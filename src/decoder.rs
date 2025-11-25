use crate::utils;
use anyhow::{anyhow, Result};
use flate2::read::DeflateDecoder;
use hound::WavReader;
use std::io::Read;
use std::path::PathBuf;

pub fn decode(input: &PathBuf, wav_out: &PathBuf, png_out: &PathBuf) -> Result<()> {
    // 1. Read the 32-bit WAV file
    let mut reader = WavReader::open(input)?;
    let spec = reader.spec();

    // Ensure we are reading a 32-bit integer WAV file as expected
    if spec.sample_format != hound::SampleFormat::Int || spec.bits_per_sample != 32 {
        return Err(anyhow!(
            "Invalid encoded file: must be a 32-bit integer WAV file."
        ));
    }

    let encoded_data: Vec<i32> = reader.samples::<i32>().collect::<Result<_, _>>()?;

    let metadata_len = 4;
    if encoded_data.len() < metadata_len {
        return Err(anyhow!(
            "Invalid encoded file: not enough data for metadata."
        ));
    }

    // --- Bitwise Decoding ---

    // 1. Decode Metadata to get original PNG length
    let len_chunks: [u16; 4] = [
        (encoded_data[0] & 0xFFFF) as u16,
        (encoded_data[1] & 0xFFFF) as u16,
        (encoded_data[2] & 0xFFFF) as u16,
        (encoded_data[3] & 0xFFFF) as u16,
    ];

    let original_png_len = ((len_chunks[0] as u64) << 48)
        | ((len_chunks[1] as u64) << 32)
        | ((len_chunks[2] as u64) << 16)
        | (len_chunks[3] as u64);

    // 2. Decode Payload
    let mut decoded_wav_data: Vec<i16> = Vec::with_capacity(encoded_data.len());
    let mut compressed_png_bytes: Vec<u8> = Vec::with_capacity(encoded_data.len() * 2);

    // Decode WAV data (MSB) from ALL samples to ensure it's lossless
    for &sample in &encoded_data {
        decoded_wav_data.push((sample >> 16) as i16);
    }

    // Decode PNG data (LSB) from the payload section
    let payload = &encoded_data[metadata_len..];
    for &sample in payload {
        let png_chunk = (sample & 0xFFFF) as u16;
        compressed_png_bytes.extend_from_slice(&png_chunk.to_le_bytes());
    }

    // --- Decompress and Finalize Data ---

    // 3. Decompress the extracted PNG data
    let mut decoder = DeflateDecoder::new(&compressed_png_bytes[..]);
    let mut decompressed_png_bytes = Vec::new();

    // Read the exact number of bytes of the original file from the decompressor
    decoder.read_to_end(&mut decompressed_png_bytes)?;

    // Truncate to the original size just in case of any padding/stream issues
    decompressed_png_bytes.truncate(original_png_len as usize);
    if decompressed_png_bytes.len() != original_png_len as usize {
        return Err(anyhow!(
            "Decompression failed: expected {} bytes, got {}",
            original_png_len,
            decompressed_png_bytes.len()
        ));
    }

    // 4. Write the new 16-bit WAV file and the restored PNG file
    utils::write_wav_16bit(wav_out, spec, &decoded_wav_data)?;
    utils::write_png(png_out, &decompressed_png_bytes)?;

    Ok(())
}
