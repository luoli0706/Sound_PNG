use anyhow::{Result, Context, anyhow};
use std::io::{self, Read, Write};
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use rand::RngCore;

/// Helper to stream bytes from Header + Encrypted Payload
/// Moved from stream_encoder.rs to be shared
pub struct ByteStream<R: Read> {
    header: Vec<u8>,
    header_pos: usize,
    payload_reader: R,
    buffer: Vec<u8>, // Dynamic buffer
    buf_pos: usize,
    buf_len: usize,
    rng: Option<ChaCha8Rng>, // If present, encrypt
    key_stream: Option<Box<dyn Read + Send>>, // Physical key
    key_buf: Vec<u8>, // Buffer for physical key
    // Added for plugins to know total size for distribution
    total_payload_len: u64, 
}

impl<R: Read> ByteStream<R> {
    pub fn new(header: Vec<u8>, payload_reader: R, timestamp: u64, key_stream: Option<Box<dyn Read + Send>>, encrypt: bool, buffer_size: usize, payload_len: u64) -> Self {
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
            total_payload_len: payload_len,
        }
    }

    pub fn total_len(&self) -> u64 {
        self.header.len() as u64 + self.total_payload_len
    }

    pub fn next_byte(&mut self) -> u8 {
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

#[derive(Debug, Clone)]
pub struct PluginMetadata {
    pub name: String,
    pub description: String,
    pub version: String,
    pub author: String,
}

/// Trait for Plugins that can encode data into a container
pub trait ContainerEncoder: Send + Sync {
    fn metadata(&self) -> PluginMetadata;
    
    /// Returns a list of supported file extensions (e.g., ["png", "jpg"])
    /// If it supports directories, it can return specific markers or just handle `encode` logic safely.
    fn supported_extensions(&self) -> Vec<String>;
    
    /// Encodes the byte stream into the container.
    /// `container_path` might be a file or a directory depending on the plugin logic.
    fn encode(
        &self, 
        container_path: &std::path::Path, 
        output_path: &std::path::Path, 
        byte_stream: &mut ByteStream<std::fs::File>,
        on_progress: Box<dyn Fn(f32) + Send + Sync>
    ) -> Result<()>;
}

/// Trait for Plugins that can decode data from a container
pub trait ContainerDecoder: Send + Sync {
    fn metadata(&self) -> PluginMetadata;
    
    /// Returns a list of supported file extensions
    fn supported_extensions(&self) -> Vec<String>;

    /// Returns a Reader that yields the raw payload stream (Header + Payload).
    /// The core will handle header parsing and decryption.
    /// This reader just extracts bits/bytes from the container(s).
    fn decode(
        &self,
        input_path: &std::path::Path,
        on_progress: Box<dyn Fn(f32) + Send + Sync>
    ) -> Result<Box<dyn Read + Send>>;
}