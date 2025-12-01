use anyhow::{Result, anyhow, Context};
use std::io::{self, Read, Write};
use flate2::read::DeflateDecoder;
use crate::header::{self, Header};
use crate::security;
use std::fs::File;
use std::path::PathBuf;
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use rand::RngCore;

/// Reads extracted payload bytes (encrypted/compressed) from a container stream.
/// This reader yields the raw byte stream hidden in the container (Header + Payload).
enum ContainerReader {
    Png {
        reader: png::Reader<File>,
        bpp: usize,
        extracted_buf: std::collections::VecDeque<u8>, 
    },
    Wav {
        iter: hound::WavIntoSamples<io::BufReader<File>, i32>,
        extracted_buf: std::collections::VecDeque<u8>,
    }
}

impl ContainerReader {
    fn new_png(file: File) -> Result<Self> {
        let decoder = png::Decoder::new(file);
        let reader = decoder.read_info()?;
        let info = reader.info().clone();
        let bpp = info.bytes_per_pixel();
        // row_buf is internal to png reader usually if we iterate rows.
        
        Ok(Self::Png {
            reader,
            bpp,
            extracted_buf: std::collections::VecDeque::new(),
        })
    }
    
    fn new_wav(file: File) -> Result<Self> {
        let reader = hound::WavReader::new(io::BufReader::new(file))?;
        Ok(Self::Wav {
            iter: reader.into_samples::<i32>(),
            extracted_buf: std::collections::VecDeque::new(),
        })
    }
}

impl Read for ContainerReader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let mut total_read = 0;
        
        while total_read < buf.len() {
            // Serve from buffer first
            match self {
                Self::Png { extracted_buf, reader, bpp } => {
                    if let Some(b) = extracted_buf.pop_front() {
                        buf[total_read] = b;
                        total_read += 1;
                        continue;
                    }
                    
                    // Refill buffer from next row
                    match reader.next_row() {
                        Ok(Some(row)) => {
                            let data = row.data();
                            let mut i = 0;
                            while i < data.len() {
                                // R
                                if i+1 < data.len() { extracted_buf.push_back(data[i+1]); }
                                // G
                                if i+3 < data.len() { extracted_buf.push_back(data[i+3]); }
                                // B
                                if i+5 < data.len() { extracted_buf.push_back(data[i+5]); }
                                // A (Skip)
                                
                                i += *bpp;
                            }
                        },
                        Ok(None) => break, // EOF
                        Err(e) => return Err(io::Error::new(io::ErrorKind::InvalidData, e.to_string())),
                    }
                },
                Self::Wav { extracted_buf, iter } => {
                    if let Some(b) = extracted_buf.pop_front() {
                        buf[total_read] = b;
                        total_read += 1;
                        continue;
                    }
                    
                    if let Some(sample_res) = iter.next() {
                        match sample_res {
                            Ok(sample) => {
                                let chunk = (sample & 0xFFFF) as u16;
                                let bytes = chunk.to_le_bytes();
                                extracted_buf.push_back(bytes[0]);
                                extracted_buf.push_back(bytes[1]);
                            },
                            Err(e) => return Err(io::Error::new(io::ErrorKind::InvalidData, e.to_string())),
                        }
                    } else {
                        break; // EOF
                    }
                }
            }
        }
        
        Ok(total_read)
    }
}

struct DecryptReader<R: Read> {
    inner: R,
    rng: Option<ChaCha8Rng>,
    key_stream: Option<Box<dyn Read + Send>>,
    key_buf: Vec<u8>, // Dynamic
}

impl<R: Read> DecryptReader<R> {
    fn new(inner: R, timestamp: u64, key_stream: Option<Box<dyn Read + Send>>, encrypt: bool, buffer_size: usize) -> Self {
        let rng = if encrypt { Some(ChaCha8Rng::seed_from_u64(timestamp)) } else { None };
        Self { inner, rng, key_stream, key_buf: vec![0u8; buffer_size] }
    }
}

impl<R: Read> Read for DecryptReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let n = self.inner.read(buf)?;
        if n == 0 { return Ok(0); }
        
        // Decrypt (XOR)
        if let Some(rng) = &mut self.rng {
             let mut i = 0;
             while i < n {
                 let keystream = rng.next_u64().to_le_bytes();
                 for b in keystream.iter() {
                     if i >= n { break; }
                     buf[i] ^= b;
                     i += 1;
                 }
             }
             
             if let Some(k_reader) = &mut self.key_stream {
                 let mut k_read = 0;
                 while k_read < n {
                     // We need to ensure key_buf is large enough? 
                     // buf.len() might be > key_buf.len() if buffer_size mismatch?
                     // We'll resize key_buf on fly or just loop logic.
                     // For safety, we iterate.
                     
                     // Actually we initialized key_buf with buffer_size.
                     // If read request > buffer_size, we might overflow key_buf if we try to read n bytes.
                     // We should cap read to key_buf.len().
                     // But `buf` here is the caller's buffer.
                     
                     let to_read = std::cmp::min(n - k_read, self.key_buf.len());
                     
                     match k_reader.read(&mut self.key_buf[..to_read]) {
                         Ok(0) => break,
                         Ok(kn) => {
                             for j in 0..kn {
                                 buf[k_read + j] ^= self.key_buf[j];
                             }
                             k_read += kn;
                         },
                         Err(_) => break,
                     }
                 }
             }
        }
        
        Ok(n)
    }
}

pub fn decode_stream(
    input_path: &PathBuf,
    output_path: &PathBuf,
    key_path: Option<&PathBuf>,
    buffer_size_kb: usize,
    on_progress: impl Fn(f32)
) -> Result<String> {
    on_progress(0.0);
    let buffer_size = buffer_size_kb * 1024;
    
    let ext = input_path.extension().and_then(|s| s.to_str()).unwrap_or("").to_lowercase();
    let file_in = File::open(input_path)?;
    
    let mut raw_extractor = if ext == "png" {
        ContainerReader::new_png(file_in)?
    } else if ext == "wav" {
        ContainerReader::new_wav(file_in)?
    } else {
        return Err(anyhow!("Unsupported container: {}", ext));
    };

    let mut header_bytes = vec![0u8; header::HEADER_SIZE_BYTES];
    raw_extractor.read_exact(&mut header_bytes).context("Failed to read header")?;
    
    let chunks: Vec<u16> = header_bytes.chunks(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
    let header = Header::from_u16_chunks(&chunks)?;
    
    on_progress(0.05);

    // 2. Setup Pipeline
    let effective_encrypt = (header.flags & 0x01) != 0;
    let key_stream: Option<Box<dyn Read + Send>> = if let Some(kp) = key_path {
        Some(Box::new(File::open(kp)?))
    } else {
        None
    };
    
    let decryptor = DecryptReader::new(raw_extractor, header.timestamp, key_stream, effective_encrypt, buffer_size);
    let limited_reader = decryptor.take(header.payload_len);
    let mut decompressor = DeflateDecoder::new(limited_reader);
    
    // 3. Write Output
    let mut file_out = File::create(output_path)?;
    let mut buf = vec![0u8; buffer_size];
    let mut total_written = 0;
    
    loop {
        let n = decompressor.read(&mut buf)?;
        if n == 0 { break; }
        file_out.write_all(&buf[..n])?;
        total_written += n as u64;
        
        if total_written % (1024*1024) == 0 {
             on_progress(0.1); 
        }
    }
    
    on_progress(1.0);
    
    let ext_str = String::from_utf8_lossy(&header.extension).to_string();
    let clean_ext = ext_str.trim_matches(char::from(0)).to_string();
    
    Ok(clean_ext)
}