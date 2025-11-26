use crate::header::{self, Header};
use crate::security;
use crate::utils;
use crate::converter;
use anyhow::{anyhow, Context, Result};
use flate2::read::DeflateDecoder;
use hound::WavReader;
use image::{io::Reader as ImageReader, GenericImageView, Pixel};
use std::io::Read;
use std::path::PathBuf;

/// Analyzes the input file header to determine if it is encrypted.
/// Supports both WAV and PNG encoded files.
pub fn analyze_header(input: &PathBuf) -> Result<bool> {
    let ext = input.extension().and_then(|s| s.to_str()).unwrap_or("").to_lowercase();
    
    let header = if ext == "png" {
        read_header_from_png(input)?
    } else {
        read_header_from_wav(input)?
    };
    
    Ok((header.flags & 0x01) != 0)
}

pub fn decode(input: &PathBuf, wav_out: &PathBuf, png_out: &PathBuf, key_in: Option<&PathBuf>) -> Result<()> {
    let ext = input.extension().and_then(|s| s.to_str()).unwrap_or("").to_lowercase();
    
    // 1. Extract Full Data Stream (Header + Payload)
    let raw_data_stream = if ext == "png" {
        extract_stream_from_png(input)?
    } else {
        extract_stream_from_wav(input)?
    };

    // 2. Parse Header
    let metadata_len_bytes = header::HEADER_SIZE_SAMPLES * 2; // 128 bytes
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
        // In PNG we might read whole image, so stream is huge.
        // In WAV we might have padding.
        // Just check we have *enough*.
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

    // 5. Check Hash
    let calculated_hash = security::calculate_hash(&compressed_payload);
    println!("[DEBUG Decoder] Extracted Payload Size: {} bytes", compressed_payload.len());
    println!("[DEBUG Decoder] Header Hash:   {:?}", hex::encode(header.hash));
    println!("[DEBUG Decoder] Computed Hash: {:?}", hex::encode(calculated_hash));

    if calculated_hash != header.hash {
        // Dump first 16 bytes for comparison
        if compressed_payload.len() >= 16 {
             println!("[DEBUG Decoder] First 16 bytes: {:?}", &compressed_payload[0..16]);
        }
        return Err(anyhow!("Security Alert: Data Integrity Check Failed!"));
    }

    // 6. Decompress
    let mut decoder = DeflateDecoder::new(&compressed_payload[..]);
    let mut decompressed_bytes = Vec::new();
    decoder.read_to_end(&mut decompressed_bytes)?;

    // 7. Output (Wait, decode separates Audio vs Image?)
    // The Payload is ALWAYS "Audio + Image" ??
    // No, in Encode:
    // Encode(WAV In, PNG In) -> Output.
    // Output format decides container.
    // But Payload is ALWAYS "PNG In (Image)" ??
    // WAIT. The prompt says "Bi-directional".
    // 1. Voice Carrier (WAV) carries Picture Payload.
    // 2. Picture Carrier (PNG) carries Voice Payload.
    
    // Current Encoder Logic:
    // `wav_data` is Container A.
    // `png_data` is Payload B.
    // `encode` takes `wav_in` and `png_in`.
    // If Format=WAV: Container=WavIn, Payload=PngIn. -> Correct.
    // If Format=PNG: Container=PngIn, Payload=PngIn?? NO!
    // Logic error in Encoder?
    // User: "Voice <-> Picture".
    // If I output PNG, the Payload should be VOICE (WAV).
    // If I output WAV, the Payload should be PICTURE (PNG).
    
    // Let's check Encoder again.
    // Encoder loads `wav_data` (as PCM) and `png_data_bytes` (as Bytes).
    // Then `compressed_payload` is `png_data_bytes` (Image).
    // Then `encode_as_png` writes `compressed_payload` into `png_in`.
    // So: PNG Container holds PNG Payload? That's useless.
    // It should be: PNG Container holds WAV Payload.
    
    // I need to Fix Encoder Logic first to support true Bi-Directional.
    // But for now, I must finish Decoder logic assuming Generic Payload.
    // The `decode` function writes `wav_out` AND `png_out`.
    // If Payload was Image, `wav_out` is empty?
    // If Payload was Audio, `png_out` is empty?
    // Currently `decode` writes `wav_out` (from Container High Bits) and `png_out` (from Payload).
    // This assumes WAV Container.
    
    // If Input is PNG:
    // Container = Image. High Bits = Original Image.
    // Payload = Hidden Data.
    // If Hidden Data is Audio -> Write to `wav_out`.
    // If Hidden Data is Image -> Write to `png_out`.
    // How do we know what the payload is?
    // We don't. Unless we add a metadata flag "Payload Type".
    // Or we just write the bytes to the requested output file.
    // In `gui.rs`, user selects "Save Voice As" and "Save Picture As".
    // If I hid Audio in PNG, user should use "Save Voice As".
    // So `decode` should probably write the *Payload* to the target determined by the user.
    // But `decode` signature has `wav_out` and `png_out`.
    
    // Let's look at `decode` signature again.
    // `voice_out` and `picture_out`.
    // Logic:
    // 1. Extract Container content (Visual/Audio) -> Write to X.
    // 2. Extract Payload content (Hidden) -> Write to Y.
    
    // If Input == WAV:
    // Container = Audio (Restored Voice). Write to `wav_out`.
    // Payload = Image (Restored Picture). Write to `png_out`.
    
    // If Input == PNG:
    // Container = Image (Restored Picture). Write to `png_out`.
    // Payload = Audio (Restored Voice). Write to `wav_out`.
    
    // I will implement this switching logic.
    
    // --- Container Restoration ---
    if ext == "png" {
        // Restore PNG container (High 8 bits)
        // Need to read pixels again? 
        // `extract_stream_from_png` just gets LSB.
        // I need a separate function to restore container?
        // Actually, `decode` usually just extracts payload.
        // "Restoring Container" means removing noise?
        // The "Noise" is in LSB.
        // If we just want the "carrier" back, we strip LSB.
        // `sound_png` original design: `wav_out` is the CARRIER (cleaned).
        // `png_out` is the PAYLOAD.
        
        // So:
        // If Input PNG:
        // `png_out` = Carrier (Image).
        // `wav_out` = Payload (Audio).
        
        restore_container_png(input, png_out)?;
        std::fs::write(wav_out, &decompressed_bytes)?; // Payload is Audio
    } else {
        // Input WAV
        // `wav_out` = Carrier (Audio).
        // `png_out` = Payload (Image).
        restore_container_wav(input, wav_out)?;
        std::fs::write(png_out, &decompressed_bytes)?; // Payload is Image
    }

    Ok(())
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
    let stream = extract_stream_from_png_limit(input, header::HEADER_SIZE_SAMPLES * 2)?; // 128 bytes
    let chunks: Vec<u16> = stream.chunks(2).map(|c| u16::from_le_bytes([c[0],c[1]])).collect();
    Header::from_u16_chunks(&chunks)
}

fn extract_stream_from_wav(input: &PathBuf) -> Result<Vec<u8>> {
    let mut reader = WavReader::open(input)?;
    let samples: Vec<i32> = reader.samples::<i32>().collect::<Result<_,_>>()?;
    // Low 16 bits are data.
    // Data is [u8, u8] from u16.
    let mut data = Vec::with_capacity(samples.len() * 2);
    for s in samples {
        let chunk = (s & 0xFFFF) as u16;
        data.extend_from_slice(&chunk.to_le_bytes());
    }
    Ok(data)
}

fn extract_stream_from_png(input: &PathBuf) -> Result<Vec<u8>> {
    let img = ImageReader::open(input)?.decode()?.into_rgba16();
    let (width, height) = img.dimensions();
    
    // Get raw buffer [R, G, B, A, R, G, B, A, ...]
    let raw_pixels = img.into_raw();
    
    let mut data = Vec::with_capacity((width * height * 3) as usize);
    
    // Process chunks of 4 (R, G, B, A)
    for pixel in raw_pixels.chunks(4) {
         // pixel[0]=R, pixel[1]=G, pixel[2]=B, pixel[3]=A
         // Data is in Low Byte (& 0xFF)
         data.push((pixel[0] & 0xFF) as u8);
         data.push((pixel[1] & 0xFF) as u8);
         data.push((pixel[2] & 0xFF) as u8);
         // Skip Alpha
    }
    Ok(data)
}

fn extract_stream_from_png_limit(input: &PathBuf, limit_bytes: usize) -> Result<Vec<u8>> {
    let img = ImageReader::open(input)?.decode()?.into_rgba16();
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
    // Container is in High 8 bits of 16-bit PNG?
    // If input is 16-bit PNG.
    // We need to read as Rgba16.
    let reader = ImageReader::open(input)?.decode()?;
    let img = reader.to_rgba16(); // Ensure we work with 16-bit
    let (width, height) = img.dimensions();
    
    use image::{ImageBuffer, Rgba};
    let mut out_img = ImageBuffer::<Rgba<u8>, Vec<u8>>::new(width, height);
    
    for (x, y, pixel) in img.enumerate_pixels() {
        // High byte is original 8-bit value.
        // Pixel is [u16; 4]
        let r = (pixel[0] >> 8) as u8;
        let g = (pixel[1] >> 8) as u8;
        let b = (pixel[2] >> 8) as u8;
        let a = (pixel[3] >> 8) as u8;
        out_img.put_pixel(x, y, Rgba([r, g, b, a]));
    }
    out_img.save(output)?;
    Ok(())
}
