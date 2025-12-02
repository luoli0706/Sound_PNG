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

/// Beta 1.0 Wrapper: Infers Payload/Container based on Format (Voice vs Picture).
pub fn encode(
    wav_in: &PathBuf, 
    png_in: &PathBuf, 
    key_in: Option<&PathBuf>,
    output: &PathBuf, 
    encrypt: bool,
    format: &str,
    on_progress: impl Fn(f32)
) -> Result<()> {
    let is_png_mode = format.to_uppercase() == "PNG";

    // Determine Payload and Container
    let (payload_bytes, container_path, ext) = if is_png_mode {
        // Mode: Hide Voice (WAV) inside Picture (PNG)
        let payload = converter::load_file_as_bytes(wav_in)?;
        (payload, png_in, "wav")
    } else {
        // Mode: Hide Picture (PNG) inside Voice (WAV)
        let payload = converter::load_file_as_bytes(png_in)?;
        (payload, wav_in, "png")
    };
    
    // Load Key
    let key_bytes = if let Some(path) = key_in {
        Some(converter::load_file_as_bytes(path)?)
    } else {
        None
    };

    // Call Generic Encoder
    encode_data(&payload_bytes, container_path, key_bytes.as_deref(), output, encrypt, Some(ext), on_progress)
}

/// Beta 2.0 Generic Interface: Encodes arbitrary payload into specific container.
pub fn encode_data(
    payload: &[u8],
    container_path: &PathBuf,
    key: Option<&[u8]>,
    output_path: &PathBuf,
    encrypt: bool,
    payload_ext: Option<&str>,
    on_progress: impl Fn(f32)
) -> Result<()> {
    on_progress(0.0);
    
    // 1. Compress Payload
    let mut encoder = DeflateEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(payload)?;
    let mut compressed_payload = encoder.finish()?;
    
    on_progress(0.05);

    // 2. Security
    let hash = security::calculate_hash(&compressed_payload);
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    let effective_encrypt = encrypt || key.is_some();
    
    if effective_encrypt {
        security::encrypt_decrypt(&mut compressed_payload, timestamp, key);
    }
    
    on_progress(0.10);

    // 3. Header
    let payload_len = compressed_payload.len() as u64;
    let header = Header::new(payload_len, effective_encrypt, timestamp, hash, payload_ext.unwrap_or(""));
    let header_chunks_u16 = header.to_u16_chunks();
    
    // 4. Combine Stream
    let mut full_data_stream: Vec<u8> = Vec::with_capacity(header_chunks_u16.len() * 2 + compressed_payload.len());
    for chunk in header_chunks_u16 {
        full_data_stream.extend_from_slice(&chunk.to_le_bytes());
    }
    full_data_stream.extend_from_slice(&compressed_payload);

    // 5. Dispatch based on Output Container Type
    let ext = container_path.extension().and_then(|s| s.to_str()).unwrap_or("").to_uppercase();
    
    let embed_progress = |p: f32| on_progress(0.10 + 0.90 * p);
    
    if ext == "PNG" || ext == "JPG" || ext == "JPEG" {
        encode_as_png(container_path, output_path, &full_data_stream, embed_progress)
    } else if ext == "WAV" {
        let mut wav_data = converter::load_audio_as_pcm(container_path)?;
        let (spec, _) = utils::read_and_normalize_wav(container_path)?;
        encode_as_wav(output_path, &mut wav_data, &full_data_stream, spec, embed_progress)
    } else {
        Err(anyhow::anyhow!("Unsupported Container Format: {}", ext))
    }
}

fn encode_as_wav(output: &PathBuf, wav_container: &mut Vec<i16>, data_stream: &[u8], spec: hound::WavSpec, on_progress: impl Fn(f32)) -> Result<()> {
    let total_samples_needed = (data_stream.len() + 1) / 2;
    
    if wav_container.len() < total_samples_needed {
        wav_container.resize(total_samples_needed, 0);
    }

    let mut encoded_samples: Vec<i32> = Vec::with_capacity(wav_container.len());
    let total_samples = wav_container.len();
    
    for (i, &wav_sample) in wav_container.iter().enumerate() {
        let data_chunk = if i < total_samples_needed {
            let byte1 = if i * 2 < data_stream.len() { data_stream[i * 2] } else { 0 };
            let byte2 = if i * 2 + 1 < data_stream.len() { data_stream[i * 2 + 1] } else { 0 };
            u16::from_le_bytes([byte1, byte2])
        } else {
            0
        };
        
        encoded_samples.push(((wav_sample as i32) << 16) | (data_chunk as i32));
        
        if i % 10000 == 0 {
             on_progress(i as f32 / total_samples as f32);
        }
    }
    on_progress(1.0);
    
    utils::write_wav_32bit(output, spec, &encoded_samples)
}

fn encode_as_png(container_path: &PathBuf, output: &PathBuf, data_stream: &[u8], on_progress: impl Fn(f32)) -> Result<()> {
    let mut img = converter::load_image_object(container_path)?;
    let (mut width, mut height) = img.dimensions();

    let needed_pixels = (data_stream.len() + 2) / 3;
    let current_pixels = (width * height) as usize;
    
    if needed_pixels > current_pixels {
        let scale = ((needed_pixels as f64) / (current_pixels as f64)).sqrt();
        let new_width = (width as f64 * scale).ceil() as u32 + 50;
        let new_height = (height as f64 * scale).ceil() as u32 + 50;
        
        img = img.resize_exact(new_width, new_height, FilterType::Lanczos3);
        let (w, h) = img.dimensions();
        width = w;
        height = h;
    }
    
    let mut out_img = ImageBuffer::<Rgba<u16>, Vec<u16>>::new(width, height);
    let mut data_iter = data_stream.chunks(3);
    let total_rows = height;
    
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
            // Force Opaque to prevent PNG optimization from discarding RGB values of transparent pixels
            let a_new = 0xFFFF; 
            
            if let Some(chunk) = data_iter.next() {
                 if chunk.len() > 0 { r_new |= chunk[0] as u16; }
                 if chunk.len() > 1 { g_new |= chunk[1] as u16; }
                 if chunk.len() > 2 { b_new |= chunk[2] as u16; }
            }
            
            out_img.put_pixel(x, y, Rgba([r_new, g_new, b_new, a_new]));
        }
        
        if y % 10 == 0 {
             on_progress(y as f32 / total_rows as f32);
        }
    }
    on_progress(1.0);
    
    out_img.save_with_format(output, image::ImageFormat::Png)?;
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
    fn test_arbitrary_binary_in_png() -> anyhow::Result<()> {
        let dir = tempdir()?;
        let payload_path = dir.path().join("data.bin");
        let container_path = dir.path().join("container.png");
        let output_path = dir.path().join("output.png");
        let restored_path = dir.path().join("restored.bin");
        
        // Create Arbitrary Binary (e.g. random bytes)
        let payload_data: Vec<u8> = (0..1000).map(|i| (i % 255) as u8).collect();
        fs::write(&payload_path, &payload_data)?;
        
        // Create Container (Transparent is fine now because we force opaque)
        let mut img = ImageBuffer::<Rgba<u8>, Vec<u8>>::new(100, 100);
        for pixel in img.pixels_mut() {
            *pixel = Rgba([0, 0, 0, 255]); // Opaque Black
        }
        img.save_with_format(&container_path, image::ImageFormat::Png)?;
        
        // Encode Generic
        encode_data(&payload_data, &container_path, None, &output_path, false, Some("bin"), |_|{})?;
        assert!(output_path.exists());
        
        // Decode (Generic)
        let dummy_path = dir.path().join("dummy");
        decoder::decode_data(&output_path, &restored_path, Some(&dummy_path), None, |_|{})?;
        
        let restored_data = fs::read(&restored_path)?;
        assert_eq!(payload_data, restored_data);
        
        Ok(())
    }

    #[test]
    fn test_homomorphic_png_in_png() -> anyhow::Result<()> {
        let dir = tempdir()?;
        let payload_png = dir.path().join("payload.png");
        let container_png = dir.path().join("container.png");
        let output_png = dir.path().join("output_homo.png");
        let restored_payload = dir.path().join("restored_payload.png");
        
        // Create Payload PNG
        let mut img1 = ImageBuffer::<Rgba<u8>, Vec<u8>>::new(50, 50);
        for pixel in img1.pixels_mut() { *pixel = Rgba([255, 0, 0, 255]); }
        img1.save(&payload_png)?;
        let payload_bytes = fs::read(&payload_png)?;
        
        // Create Container PNG
        let mut img2 = ImageBuffer::<Rgba<u8>, Vec<u8>>::new(200, 200);
        for pixel in img2.pixels_mut() { *pixel = Rgba([0, 0, 255, 255]); }
        img2.save(&container_png)?;
        
        // Encode
        encode_data(&payload_bytes, &container_png, None, &output_png, false, Some("png"), |_|{})?;
        assert!(output_png.exists());
        
        // Decode
        let dummy = dir.path().join("dummy");
        decoder::decode_data(&output_png, &restored_payload, Some(&dummy), None, |_|{})?;
        
        let restored_bytes = fs::read(&restored_payload)?;
        assert_eq!(payload_bytes, restored_bytes);
        
        Ok(())
    }

    #[test]
    fn test_homomorphic_wav_in_wav() -> anyhow::Result<()> {
        let dir = tempdir()?;
        let payload_wav = dir.path().join("payload.wav");
        let container_wav = dir.path().join("container.wav");
        let output_wav = dir.path().join("output_homo.wav");
        let restored_payload = dir.path().join("restored_payload.wav");
        
        let spec = WavSpec { channels: 1, sample_rate: 44100, bits_per_sample: 16, sample_format: SampleFormat::Int };
        
        // Create Payload WAV
        let data1: Vec<i16> = (0..100).map(|i| i as i16).collect();
        utils::write_wav_16bit(&payload_wav, spec, &data1)?;
        let payload_bytes = fs::read(&payload_wav)?;
        
        // Create Container WAV (Must be larger)
        // 100 samples payload -> ~200 bytes compressed?
        // Container capacity: 2 bytes per sample.
        // Need > 100 samples.
        let data2: Vec<i16> = (0..5000).map(|i| (i*2) as i16).collect();
        utils::write_wav_16bit(&container_wav, spec, &data2)?;
        
        // Encode
        encode_data(&payload_bytes, &container_wav, None, &output_wav, false, Some("wav"), |_|{})?;
        assert!(output_wav.exists());
        
        // Decode
        let dummy = dir.path().join("dummy");
        decoder::decode_data(&output_wav, &restored_payload, Some(&dummy), None, |_|{})?;
        
        let restored_bytes = fs::read(&restored_payload)?;
        assert_eq!(payload_bytes, restored_bytes);
        
        Ok(())
    }
}
