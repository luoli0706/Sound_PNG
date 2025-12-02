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
use crate::plugin_loader::PluginManager;
use std::sync::{Arc, Mutex};

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
    },
    Plugin(Box<dyn Read + Send>), // Added for plugins
}

impl ContainerReader {
    fn new_png(file: File) -> Result<Self> {
        let decoder = png::Decoder::new(file);
        let reader = decoder.read_info()?;
        let info = reader.info().clone();
        let bpp = info.bytes_per_pixel();
        
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
        match self {
            Self::Plugin(r) => r.read(buf),
            Self::Png { extracted_buf, reader, bpp } => {
                let mut total_read = 0;
                while total_read < buf.len() {
                    if let Some(b) = extracted_buf.pop_front() {
                        buf[total_read] = b;
                        total_read += 1;
                        continue;
                    }
                    match reader.next_row() {
                        Ok(Some(row)) => {
                            let data = row.data();
                            let mut i = 0;
                            while i < data.len() {
                                if i+1 < data.len() { extracted_buf.push_back(data[i+1]); }
                                if i+3 < data.len() { extracted_buf.push_back(data[i+3]); }
                                if i+5 < data.len() { extracted_buf.push_back(data[i+5]); }
                                i += *bpp;
                            }
                        },
                        Ok(None) => break,
                        Err(e) => return Err(io::Error::new(io::ErrorKind::InvalidData, e.to_string())),
                    }
                }
                Ok(total_read)
            },
            Self::Wav { extracted_buf, iter } => {
                let mut total_read = 0;
                while total_read < buf.len() {
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
                        break;
                    }
                }
                Ok(total_read)
            }
        }
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
    plugins: &Arc<Mutex<PluginManager>>,
    input_ext_hint: String,
    on_progress: impl Fn(f32) + Send + Sync + 'static
) -> Result<String> {
    on_progress(0.0);
    let buffer_size = buffer_size_kb * 1024;
    
    // Plugin Check
    let on_progress = Arc::new(on_progress);

    let mut raw_extractor = {
        let pm = plugins.lock().unwrap();
        if let Some(decoder) = pm.get_decoder_by_ext(&input_ext_hint) {
             let cb = on_progress.clone();
             let reader = decoder.decode(input_path, Box::new(move |p| cb(p)))?;
             ContainerReader::Plugin(reader)
        } else if input_ext_hint == "png" {
             ContainerReader::new_png(File::open(input_path)?)?
        } else if input_ext_hint == "wav" {
             ContainerReader::new_wav(File::open(input_path)?)?
        } else {
             return Err(anyhow!("Unsupported container: {}", input_ext_hint));
        }
    };

    // Header parsing...
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