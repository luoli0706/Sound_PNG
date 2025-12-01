use anyhow::{Result, Context, anyhow};
use std::io::{self, Read, Write, Seek};
use flate2::write::DeflateEncoder;
use flate2::Compression;
use crate::header::{self, Header};
use std::time::{SystemTime, UNIX_EPOCH};
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use sha2::{Digest, Sha256};
use std::fs::File;
use std::path::PathBuf;
use rand::RngCore;
use hound::{WavWriter, WavSpec, SampleFormat};

/// Helper to stream bytes from Header + Encrypted Payload
struct ByteStream<R: Read> {
    header: Vec<u8>,
    header_pos: usize,
    payload_reader: R,
    buffer: Vec<u8>, // Dynamic buffer
    buf_pos: usize,
    buf_len: usize,
    rng: Option<ChaCha8Rng>, // If present, encrypt
    key_stream: Option<Box<dyn Read + Send>>, // Physical key
    key_buf: Vec<u8>, // Buffer for physical key
}

impl<R: Read> ByteStream<R> {
    fn new(header: Vec<u8>, payload_reader: R, timestamp: u64, key_stream: Option<Box<dyn Read + Send>>, encrypt: bool, buffer_size: usize) -> Self {
        let rng = if encrypt { Some(ChaCha8Rng::seed_from_u64(timestamp)) } else { None };
        Self {
            header,
            header_pos: 0,
            payload_reader,
            buffer: vec![0u8; buffer_size],
            buf_pos: 0,
            buf_len: 0,
            rng,
            key_stream,
            key_buf: vec![0u8; buffer_size],
        }
    }

    fn next_byte(&mut self) -> u8 {
        // 1. Header Phase
        if self.header_pos < self.header.len() {
            let b = self.header[self.header_pos];
            self.header_pos += 1;
            return b;
        }

        // 2. Payload Phase
        if self.buf_pos >= self.buf_len {
            match self.payload_reader.read(&mut self.buffer) {
                Ok(0) => return 0, // Padding
                Ok(n) => {
                    self.buf_len = n;
                    self.buf_pos = 0;
                    
                    // Encrypt Buffer In-Place
                    if let Some(rng) = &mut self.rng {
                        // Apply ChaCha8
                        let mut i = 0;
                        while i < n {
                            let keystream = rng.next_u64().to_le_bytes();
                            for b in keystream.iter() {
                                if i >= n { break; }
                                self.buffer[i] ^= b;
                                i += 1;
                            }
                        }
                        
                        // Apply Physical Key
                        if let Some(k_reader) = &mut self.key_stream {
                            let mut k_read = 0;
                            while k_read < n {
                                match k_reader.read(&mut self.key_buf[k_read..n]) {
                                    Ok(0) => break, // EOF
                                    Ok(kn) => k_read += kn,
                                    Err(_) => break,
                                }
                            }
                            
                            for j in 0..k_read {
                                self.buffer[j] ^= self.key_buf[j];
                            }
                        }
                    }
                },
                Err(_) => return 0,
            }
        }

        let b = self.buffer[self.buf_pos];
        self.buf_pos += 1;
        b
    }
}

/// Encodes data from a Reader source into a Container (streaming).
pub fn encode_stream(
    payload: &mut dyn Read,
    container_path: &PathBuf,
    key_path: Option<&PathBuf>, 
    output_path: &PathBuf,
    encrypt: bool,
    payload_ext: Option<&str>,
    buffer_size_kb: usize,
    on_progress: impl Fn(f32)
) -> Result<()> {
    on_progress(0.0);
    let buffer_size = buffer_size_kb * 1024;
    
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    let temp_dir = std::env::temp_dir();
    let temp_compressed = temp_dir.join(format!("spng_enc_{}.tmp", timestamp));
    
    // Step 1: Compress & Hash to Temp File
    {
        let f_out = File::create(&temp_compressed)?;
        let mut encoder = DeflateEncoder::new(f_out, Compression::default());
        let mut buf = vec![0u8; buffer_size]; 
        
        loop {
            let n = payload.read(&mut buf)?;
            if n == 0 { break; }
            encoder.write_all(&buf[..n])?;
        }
        encoder.finish()?;
    }
    
    // Step 2: Calculate Hash & Size
    let mut compressed_file = File::open(&temp_compressed)?;
    let payload_len = compressed_file.metadata()?.len();
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; buffer_size];
    loop {
        let n = compressed_file.read(&mut buf)?;
        if n == 0 { break; }
        hasher.update(&buf[..n]);
    }
    let hash: [u8; 32] = hasher.finalize().into();
    
    on_progress(0.2);
    
    // Step 3: Prepare Header
    let effective_encrypt = encrypt || key_path.is_some();
    let header = Header::new(payload_len, effective_encrypt, timestamp, hash, payload_ext.unwrap_or(""));
    let header_bytes = header.to_u16_chunks().iter().flat_map(|u| u.to_le_bytes()).collect::<Vec<u8>>();
    
    // Step 4: Embed
    compressed_file.seek(std::io::SeekFrom::Start(0))?;
    let container_ext = container_path.extension().and_then(|s| s.to_str()).unwrap_or("").to_lowercase();
    
    // Key Stream Setup
    let key_stream: Option<Box<dyn Read + Send>> = if let Some(kp) = key_path {
        Some(Box::new(File::open(kp)?))
    } else {
        None
    };

    let mut byte_stream = ByteStream::new(header_bytes, compressed_file, timestamp, key_stream, effective_encrypt, buffer_size);

    let embed_progress = |p: f32| on_progress(0.2 + 0.8 * p);

    if container_ext == "png" {
        embed_png(container_path, output_path, &mut byte_stream, embed_progress)?;
    } else if container_ext == "wav" {
        embed_wav(container_path, output_path, &mut byte_stream, embed_progress)?;
    } else {
        return Err(anyhow!("Unsupported container: {}", container_ext));
    }
    
    // Cleanup
    let _ = std::fs::remove_file(temp_compressed);
    Ok(())
}

fn embed_png(
    container: &PathBuf,
    output: &PathBuf,
    byte_stream: &mut ByteStream<File>,
    on_progress: impl Fn(f32)
) -> Result<()> {
    use png::{Decoder, Encoder, ColorType, BitDepth};
    
    let file_in = File::open(container)?;
    let decoder = Decoder::new(file_in);
    let mut reader = decoder.read_info()?;
    let info = reader.info().clone();
    
    let file_out = File::create(output)?;
    let mut encoder = Encoder::new(file_out, info.width, info.height);
    encoder.set_color(ColorType::Rgba);
    encoder.set_depth(BitDepth::Sixteen);
    let mut writer = encoder.write_header()?;
    
    let bpp = info.bytes_per_pixel();
    let mut out_row = vec![0u8; info.width as usize * 8]; // RGBA16 output
    let total_rows = info.height as usize;

    let mut row_num = 0;
    while let Ok(Some(row)) = reader.next_row() {
        let src_data = row.data();
        let mut out_idx = 0;
        let mut src_idx = 0;
        
        while src_idx < src_data.len() {
            let r = src_data[src_idx];
            let g = src_data[src_idx+1];
            let b = src_data[src_idx+2];
            let _ = if bpp == 4 { src_data[src_idx+3] } else { 255 };
            src_idx += bpp;
            
            let embed_b = byte_stream.next_byte();
            let r16 = ((r as u16) << 8) | (embed_b as u16);
            
            let embed_b = byte_stream.next_byte();
            let g16 = ((g as u16) << 8) | (embed_b as u16);
            
            let embed_b = byte_stream.next_byte();
            let b16 = ((b as u16) << 8) | (embed_b as u16);
            
            let a16: u16 = 0xFFFF;
            
            out_row[out_idx..out_idx+2].copy_from_slice(&r16.to_be_bytes());
            out_row[out_idx+2..out_idx+4].copy_from_slice(&g16.to_be_bytes());
            out_row[out_idx+4..out_idx+6].copy_from_slice(&b16.to_be_bytes());
            out_row[out_idx+6..out_idx+8].copy_from_slice(&a16.to_be_bytes());
            out_idx += 8;
        }
        writer.write_image_data(&out_row)?;
        
        if row_num % 50 == 0 { on_progress(row_num as f32 / total_rows as f32); }
        row_num += 1;
    }
    writer.finish()?;
    Ok(())
}

fn embed_wav(
    container: &PathBuf,
    output: &PathBuf,
    byte_stream: &mut ByteStream<File>,
    on_progress: impl Fn(f32)
) -> Result<()> {
    use crate::utils::WavIterator;
    let iter = WavIterator::new(container)?;
    let spec = iter.spec();
    let total_samples = iter.len();
    
    let mut writer = WavWriter::create(output, WavSpec {
        bits_per_sample: 32,
        sample_format: SampleFormat::Int,
        ..spec
    })?;

    let mut count = 0;
    for sample_res in iter.into_iter() {
        let sample = sample_res?;
        
        let b1 = byte_stream.next_byte();
        let b2 = byte_stream.next_byte();
        
        let chunk = u16::from_le_bytes([b1, b2]);
        let out = ((sample as i32) << 16) | (chunk as i32);
        writer.write_sample(out)?;
        
        if count % 10000 == 0 { on_progress(count as f32 / total_samples as f32); }
        count += 1;
    }
    writer.finalize()?;
    Ok(())
}