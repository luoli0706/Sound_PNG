use crate::header::{self, Header};
use crate::security;
use crate::utils;
use crate::converter;
use anyhow::{anyhow, Result};
use flate2::read::DeflateDecoder;
use hound::WavReader;
use image::{io::Reader as ImageReader, GenericImageView, Pixel};
use std::io::Read;
use std::path::PathBuf;

/// Analyzes header to check encryption.
pub fn analyze_header(input: &PathBuf) -> Result<bool> {
    let ext = input.extension().and_then(|s| s.to_str()).unwrap_or("").to_lowercase();
    
    let header = if ext == "png" {
        read_header_from_png(input)?
    } else {
        read_header_from_wav(input)?
    };
    
    Ok((header.flags & 0x01) != 0)
}

/// Beta 1.0 Wrapper
pub fn decode(
    input: &PathBuf, 
    wav_out: &PathBuf, 
    png_out: &PathBuf, 
    key_in: Option<&PathBuf>,
    on_progress: impl Fn(f32)
) -> Result<()> {
    let ext = input.extension().and_then(|s| s.to_str()).unwrap_or("").to_lowercase();
    
    if ext == "png" {
        decode_data(input, wav_out, Some(png_out), key_in, on_progress)?;
    } else {
        decode_data(input, png_out, Some(wav_out), key_in, on_progress)?;
    }
    Ok(())
}

/// Beta 2.0 Generic Interface: Returns detected extension of payload.
pub fn decode_data(
    input: &PathBuf, 
    payload_out: &PathBuf, 
    container_out: Option<&PathBuf>, 
    key_in: Option<&PathBuf>,
    on_progress: impl Fn(f32)
) -> Result<String> {
    let ext = input.extension().and_then(|s| s.to_str()).unwrap_or("").to_lowercase();
    
    on_progress(0.0);

    // 1. Extract Full Data Stream
    let extract_progress = |p: f32| on_progress(0.5 * p);
    
    let raw_data_stream = if ext == "png" {
        extract_stream_from_png(input, extract_progress)?
    } else {
        extract_stream_from_wav(input, extract_progress)?
    };
    
    on_progress(0.5);

    // 2. Parse Header
    let metadata_len_bytes = header::HEADER_SIZE_SAMPLES * 2;
    if raw_data_stream.len() < metadata_len_bytes {
        return Err(anyhow!("Invalid encoded file: not enough data."));
    }
    
    let header_bytes = &raw_data_stream[0..metadata_len_bytes];
    let header_chunks: Vec<u16> = header_bytes
        .chunks(2)
        .map(|c| u16::from_le_bytes([c[0], c[1]]))
        .collect();
        
    let header = Header::from_u16_chunks(&header_chunks)?;

    // 3. Extract Payload
    let payload_len = header.payload_len as usize;
    let total_needed = metadata_len_bytes + payload_len;
    
    if raw_data_stream.len() < total_needed {
        return Err(anyhow!("Invalid encoded file: payload truncated."));
    }
    
    let mut compressed_payload = raw_data_stream[metadata_len_bytes..metadata_len_bytes+payload_len].to_vec();

    // 4. Security
    let key_bytes = if let Some(path) = key_in {
        Some(converter::load_file_as_bytes(path)?)
    } else {
        None
    };
    let key_slice = key_bytes.as_deref();

    let encrypted = (header.flags & 0x01) != 0;
    if encrypted {
        security::encrypt_decrypt(&mut compressed_payload, header.timestamp, key_slice);
    }
    
    on_progress(0.6);

    // 5. Check Hash
    let calculated_hash = security::calculate_hash(&compressed_payload);
    if calculated_hash != header.hash {
        return Err(anyhow!("Security Alert: Data Integrity Check Failed!"));
    }

    // 6. Decompress
    let mut decoder = DeflateDecoder::new(&compressed_payload[..]);
    let mut decompressed_bytes = Vec::new();
    decoder.read_to_end(&mut decompressed_bytes)?;
    
    on_progress(0.8);

    // 7. Save Payload
    // If payload_out is generic, we might want to rename it? 
    // Logic is handled by caller usually. We just write to payload_out.
    std::fs::write(payload_out, &decompressed_bytes)?;

    // 8. Save Restored Container (Optional)
    if let Some(cont_out) = container_out {
        if ext == "png" {
            restore_container_png(input, cont_out)?;
        } else {
            restore_container_wav(input, cont_out)?;
        }
    }
    
    on_progress(1.0);
    
    // Extract extension string
    let ext_str = String::from_utf8_lossy(&header.extension).to_string();
    let clean_ext = ext_str.trim_matches(char::from(0)).to_string();
    
    Ok(clean_ext)
}

// --- Helpers ---

fn read_header_from_wav(input: &PathBuf) -> Result<Header> {
    let mut reader = WavReader::open(input)?;
    let samples: Vec<i32> = reader.samples::<i32>().take(header::HEADER_SIZE_SAMPLES).collect::<Result<_,_>>()?;
    if samples.len() < header::HEADER_SIZE_SAMPLES {
        return Err(anyhow!("File too short"));
    }
    let chunks: Vec<u16> = samples.iter().map(|s| (s & 0xFFFF) as u16).collect();
    Header::from_u16_chunks(&chunks)
}

fn read_header_from_png(input: &PathBuf) -> Result<Header> {
    let stream = extract_stream_from_png_limit(input, header::HEADER_SIZE_SAMPLES * 2)?; 
    let chunks: Vec<u16> = stream.chunks(2).map(|c| u16::from_le_bytes([c[0],c[1]])).collect();
    Header::from_u16_chunks(&chunks)
}

fn extract_stream_from_wav(input: &PathBuf, on_progress: impl Fn(f32)) -> Result<Vec<u8>> {
    let mut reader = WavReader::open(input)?;
    let samples: Vec<i32> = reader.samples::<i32>().collect::<Result<_,_>>()?;
    let mut data = Vec::with_capacity(samples.len() * 2);
    let total = samples.len();
    
    for (i, s) in samples.iter().enumerate() {
        let chunk = (s & 0xFFFF) as u16;
        data.extend_from_slice(&chunk.to_le_bytes());
        
        if i % 10000 == 0 { on_progress(i as f32 / total as f32); }
    }
    Ok(data)
}

use std::io::Cursor;

fn extract_stream_from_png(input: &PathBuf, on_progress: impl Fn(f32)) -> Result<Vec<u8>> {
    // Load bytes first to avoid file locking/path issues
    let bytes = std::fs::read(input)?;
    
    let img = image::load_from_memory_with_format(&bytes, image::ImageFormat::Png)?.into_rgba16();
    let (width, height) = img.dimensions();
    
    // Get raw buffer [R, G, B, A, R, G, B, A, ...]
    let raw_pixels = img.into_raw();
    let total_pixels = width * height;
    
    let mut data = Vec::with_capacity((width * height * 3) as usize);
    
    // Process chunks of 4 (R, G, B, A)
    for (i, pixel) in raw_pixels.chunks(4).enumerate() {
         data.push((pixel[0] & 0xFF) as u8);
         data.push((pixel[1] & 0xFF) as u8);
         data.push((pixel[2] & 0xFF) as u8);
         
         if i % width as usize == 0 { on_progress(i as f32 / total_pixels as f32); }
    }
    Ok(data)
}

fn extract_stream_from_png_limit(input: &PathBuf, limit_bytes: usize) -> Result<Vec<u8>> {
    let bytes = std::fs::read(input)?;
    if bytes.len() == 0 {
        return Err(anyhow::anyhow!("Input file is empty: {:?}", input));
    }
    
    let img_res = image::load_from_memory_with_format(&bytes, image::ImageFormat::Png);
    let img = match img_res {
        Ok(i) => i.into_rgba16(),
        Err(e) => return Err(anyhow::anyhow!("Image decode failed: {}", e)),
    };

    let raw_pixels = img.into_raw();
    let mut data = Vec::with_capacity(limit_bytes);
    
    for pixel in raw_pixels.chunks(4) {
         if data.len() >= limit_bytes { break; }
         data.push((pixel[0] & 0xFF) as u8);
         
         if data.len() >= limit_bytes { break; }
         data.push((pixel[1] & 0xFF) as u8);
         
         if data.len() >= limit_bytes { break; }
         data.push((pixel[2] & 0xFF) as u8);
    }
    Ok(data)
}

fn restore_container_wav(input: &PathBuf, output: &PathBuf) -> Result<()> {
    let mut reader = WavReader::open(input)?;
    let spec = reader.spec();
    let samples: Vec<i32> = reader.samples::<i32>().collect::<Result<_,_>>()?;
    
    let restored: Vec<i16> = samples.iter().map(|s| (s >> 16) as i16).collect();
    utils::write_wav_16bit(output, spec, &restored)
}

fn restore_container_png(input: &PathBuf, output: &PathBuf) -> Result<()> {
    let bytes = std::fs::read(input)?;
    let mut reader = ImageReader::new(Cursor::new(bytes));
    reader.set_format(image::ImageFormat::Png);
    let img = reader.decode()?.into_rgba16();
    let (width, height) = img.dimensions();
    
    use image::{ImageBuffer, Rgba};
    let mut out_img = ImageBuffer::<Rgba<u8>, Vec<u8>>::new(width, height);
    
    for (x, y, pixel) in img.enumerate_pixels() {
        let r = (pixel[0] >> 8) as u8;
        let g = (pixel[1] >> 8) as u8;
        let b = (pixel[2] >> 8) as u8;
        let a = (pixel[3] >> 8) as u8;
        out_img.put_pixel(x, y, Rgba([r, g, b, a]));
    }
    out_img.save(output)?;
    Ok(())
}