use crate::converter;
use crate::header::{self, Header};
use crate::security;
use crate::utils;
use anyhow::Result;
use flate2::write::DeflateEncoder;
use flate2::Compression;
use image::{imageops::FilterType, GenericImageView, ImageBuffer, Rgba};
use std::io::Write;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

pub fn encode(
    wav_in: &PathBuf, 
    png_in: &PathBuf, 
    key_in: Option<&PathBuf>,
    output: &PathBuf, 
    encrypt: bool,
    format: &str
) -> Result<()> {
    let is_png_mode = format.to_uppercase() == "PNG";

    // 1. Determine Payload and Container Data
    let (payload_bytes, mut wav_container_data, png_container_path) = if is_png_mode {
        // Mode: Hide Voice (WAV) inside Picture (PNG)
        // Payload = WAV File Bytes
        let payload = converter::load_file_as_bytes(wav_in)?;
        (payload, None, Some(png_in))
    } else {
        // Mode: Hide Picture (PNG) inside Voice (WAV)
        // Payload = PNG File Bytes
        let payload = converter::load_file_as_bytes(png_in)?;
        // Container = WAV PCM Data
        let wav_data = converter::load_audio_as_pcm(wav_in)?;
        (payload, Some(wav_data), None)
    };
    
    // Load Key File if provided
    let key_bytes = if let Some(path) = key_in {
        Some(converter::load_file_as_bytes(path)?)
    } else {
        None
    };
    let key_slice = key_bytes.as_deref();

    // 2. Compress the Payload
    let mut encoder = DeflateEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(&payload_bytes)?;
    let mut compressed_payload = encoder.finish()?;

    // --- Security Phase (The Three Judges) ---
    let hash = security::calculate_hash(&compressed_payload);
    // println!("[DEBUG Encoder] Payload Size: {} bytes", compressed_payload.len());
    // println!("[DEBUG Encoder] Computed Hash: {:?}", hex::encode(hash));
    
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    let effective_encrypt = encrypt || key_in.is_some();
    
    if effective_encrypt {
        security::encrypt_decrypt(&mut compressed_payload, timestamp, key_slice);
    }
    
    // --- Header Creation ---
    let payload_len = compressed_payload.len() as u64;
    let header = Header::new(payload_len, effective_encrypt, timestamp, hash);
    let header_chunks_u16 = header.to_u16_chunks();
    
    // Convert Header to Bytes (Little Endian)
    let mut full_data_stream: Vec<u8> = Vec::with_capacity(header_chunks_u16.len() * 2 + compressed_payload.len());
    for chunk in header_chunks_u16 {
        full_data_stream.extend_from_slice(&chunk.to_le_bytes());
    }
    // Append Payload
    full_data_stream.extend_from_slice(&compressed_payload);

    // --- Output Branching ---
    
    if is_png_mode {
        if let Some(container_path) = png_container_path {
            encode_as_png(container_path, output, &full_data_stream)?;
        }
    } else {
        if let Some(mut wav_data) = wav_container_data {
             // Get WavSpec (for WAV output)
            let wav_spec = if let Some(ext) = wav_in.extension().and_then(|s| s.to_str()) {
                 if ext.to_lowercase() == "wav" {
                     let (spec, _) = utils::read_and_normalize_wav(wav_in)?;
                     spec
                 } else {
                     hound::WavSpec {
                         channels: 2,
                         sample_rate: 44100,
                         bits_per_sample: 16,
                         sample_format: hound::SampleFormat::Int,
                     }
                 }
            } else {
                 hound::WavSpec {
                     channels: 2,
                     sample_rate: 44100,
                     bits_per_sample: 16,
                     sample_format: hound::SampleFormat::Int,
                     }
            };
            
            encode_as_wav(output, &mut wav_data, &full_data_stream, wav_spec)?;
        }
    }
    
    Ok(())
}

fn encode_as_wav(output: &PathBuf, wav_container: &mut Vec<i16>, data_stream: &[u8], spec: hound::WavSpec) -> Result<()> {
    let total_samples_needed = (data_stream.len() + 1) / 2;
    
    if wav_container.len() < total_samples_needed {
        wav_container.resize(total_samples_needed, 0);
    }

    let mut encoded_samples: Vec<i32> = Vec::with_capacity(wav_container.len());
    
    for (i, &wav_sample) in wav_container.iter().enumerate() {
        let data_chunk = if i < total_samples_needed {
            let byte1 = if i * 2 < data_stream.len() { data_stream[i * 2] } else { 0 };
            let byte2 = if i * 2 + 1 < data_stream.len() { data_stream[i * 2 + 1] } else { 0 };
            u16::from_le_bytes([byte1, byte2])
        } else {
            0
        };
        
        encoded_samples.push(((wav_sample as i32) << 16) | (data_chunk as i32));
    }
    
    utils::write_wav_32bit(output, spec, &encoded_samples)
}

fn encode_as_png(container_path: &PathBuf, output: &PathBuf, data_stream: &[u8]) -> Result<()> {
    let mut img = converter::load_image_object(container_path)?;
    let (mut width, mut height) = img.dimensions();

    // 1. Check Capacity & Auto-Expand
    let needed_pixels = (data_stream.len() + 2) / 3; // Ceiling division
    let current_pixels = (width * height) as usize;
    
    if needed_pixels > current_pixels {
        let scale = ((needed_pixels as f64) / (current_pixels as f64)).sqrt();
        let new_width = (width as f64 * scale).ceil() as u32 + 50; // Add some padding
        let new_height = (height as f64 * scale).ceil() as u32 + 50;
        
        // Use resize_exact to ensure we meet the calculated dimensions and capacity
        img = img.resize_exact(new_width, new_height, FilterType::Lanczos3);
        
        // Update dimensions from the actual resized image
        let (w, h) = img.dimensions();
        width = w;
        height = h;
    }
    
    // 2. Embed Data
    let mut out_img = ImageBuffer::<Rgba<u16>, Vec<u16>>::new(width, height);
    let mut data_iter = data_stream.chunks(3); // Process 3 bytes at a time (RGB)
    
    // FORCE Row-Major Iteration
    for y in 0..height {
        for x in 0..width {
            let pixel = img.get_pixel(x, y);
            
            let r = pixel[0] as u16;
            let g = pixel[1] as u16;
            let b = pixel[2] as u16;
            let a = pixel[3] as u16;
            
            // Move original 8 bits to High Byte
            let mut r_new = r << 8;
            let mut g_new = g << 8;
            let mut b_new = b << 8;
            let mut a_new = a << 8;
            
            // Embed 3 Bytes if available
            if let Some(chunk) = data_iter.next() {
                 if chunk.len() > 0 { r_new |= chunk[0] as u16; }
                 if chunk.len() > 1 { g_new |= chunk[1] as u16; }
                 if chunk.len() > 2 { b_new |= chunk[2] as u16; }
            }
            
            out_img.put_pixel(x, y, Rgba([r_new, g_new, b_new, a_new]));
        }
    }
    
    out_img.save(output)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decoder;
    use hound::{SampleFormat, WavSpec};
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_lossless_encode_decode_16bit() -> anyhow::Result<()> {
        test_encode_decode_with_spec(WavSpec {
            channels: 1,
            sample_rate: 44100,
            bits_per_sample: 16,
            sample_format: SampleFormat::Int,
        })
    }
    
    fn test_encode_decode_with_spec(spec: WavSpec) -> anyhow::Result<()> {
        let dir = tempdir()?;
        let original_wav_path = dir.path().join("original.wav");
        let original_png_path = dir.path().join("original.png");
        let encoded_path = dir.path().join("encoded.wav");
        let decoded_wav_path = dir.path().join("decoded.wav");
        let decoded_png_path = dir.path().join("decoded.png");

        match (spec.sample_format, spec.bits_per_sample) {
            (SampleFormat::Int, 16) => {
                let data: Vec<i16> = (0..500).map(|i| (i * 100) as i16).collect(); 
                utils::write_wav_16bit(&original_wav_path, spec, &data)?;
            }
            _ => {
                 let data: Vec<i16> = (0..500).map(|i| (i * 100) as i16).collect(); 
                 utils::write_wav_16bit(&original_wav_path, WavSpec { bits_per_sample: 16, sample_format: SampleFormat::Int, ..spec}, &data)?;
            }
        }

        let original_png_data: Vec<u8> = (0..2048).map(|i| (i % 256) as u8).collect();
        fs::write(&original_png_path, &original_png_data)?;

        encode(&original_wav_path, &original_png_path, None, &encoded_path, true, "WAV")?;
        assert!(encoded_path.exists());

        decoder::decode(&encoded_path, &decoded_wav_path, &decoded_png_path, None)?;
        
        let decoded_png_data = fs::read(&decoded_png_path)?;
        assert_eq!(original_png_data, decoded_png_data);
        
        Ok(())
    }
    
    #[test]
    fn test_encode_png_mode() -> anyhow::Result<()> {
        let dir = tempdir()?;
        let wav_in = dir.path().join("test.wav");
        let png_in = dir.path().join("container.png");
        let png_out = dir.path().join("encoded.png");
        let decoded_wav = dir.path().join("restored.wav");
        let decoded_png = dir.path().join("restored.png");
        
        let spec = WavSpec { channels: 1, sample_rate: 44100, bits_per_sample: 16, sample_format: SampleFormat::Int };
        let wav_data: Vec<i16> = (0..100).map(|i| (i * 100) as i16).collect();
        utils::write_wav_16bit(&wav_in, spec, &wav_data)?;
        
        let img = ImageBuffer::<Rgba<u8>, Vec<u8>>::new(100, 100);
        img.save(&png_in)?;
        
        encode(&wav_in, &png_in, None, &png_out, false, "PNG")?;
        assert!(png_out.exists());
        
        decoder::decode(&png_out, &decoded_wav, &decoded_png, None)?;
        
        let reader = hound::WavReader::open(&decoded_wav)?;
        assert_eq!(reader.spec().channels, spec.channels);
        Ok(())
    }
    
    #[test]
    fn test_encode_png_auto_expand() -> anyhow::Result<()> {
         let dir = tempdir()?;
        let wav_in = dir.path().join("large_payload.wav");
        let png_in = dir.path().join("small_container.png");
        let png_out = dir.path().join("expanded_encoded.png");
        
        let spec = WavSpec { channels: 1, sample_rate: 44100, bits_per_sample: 16, sample_format: SampleFormat::Int };
        let wav_data: Vec<i16> = (0..2500).map(|i| i as i16).collect();
        utils::write_wav_16bit(&wav_in, spec, &wav_data)?;
        
        let img = ImageBuffer::<Rgba<u8>, Vec<u8>>::new(10, 10);
        img.save(&png_in)?;
        
        encode(&wav_in, &png_in, None, &png_out, false, "PNG")?;
        assert!(png_out.exists());
        
        let out_img = image::io::Reader::open(&png_out)?.decode()?;
        let (w, h) = out_img.dimensions();
        assert!(w > 10);
        assert!(h > 10);
        
        Ok(())
    }
}
