use anyhow::Result;
use std::convert::TryInto;

pub const HEADER_SIZE_SAMPLES: usize = 64; // 64 samples = 128 bytes (LSB 16-bit)
pub const HEADER_SIZE_BYTES: usize = HEADER_SIZE_SAMPLES * 2;
pub const MAGIC: &[u8; 4] = b"SPNG";
pub const VERSION: u8 = 1;

#[derive(Debug, Clone)]
pub struct Header {
    pub magic: [u8; 4],
    pub version: u8,
    pub flags: u8, // Bit 0: Encrypted, Bit 1: Compressed (Always 1 for now)
    pub payload_len: u64,
    pub timestamp: u64,
    pub hash: [u8; 32],
    pub extension: [u8; 8], // New field for file extension
}

impl Header {
    pub fn new(payload_len: u64, encrypted: bool, timestamp: u64, hash: [u8; 32], ext_str: &str) -> Self {
        let mut flags = 0;
        if encrypted {
            flags |= 0x01;
        }
        flags |= 0x02; // Compressed

        let mut extension = [0u8; 8];
        let bytes = ext_str.as_bytes();
        let len = bytes.len().min(8);
        extension[0..len].copy_from_slice(&bytes[0..len]);

        Self {
            magic: *MAGIC,
            version: VERSION,
            flags,
            payload_len,
            timestamp,
            hash,
            extension,
        }
    }

    pub fn to_u16_chunks(&self) -> Vec<u16> {
        let mut bytes = vec![0u8; HEADER_SIZE_BYTES];

        // Write fields to byte buffer
        bytes[0..4].copy_from_slice(&self.magic);
        bytes[4..12].copy_from_slice(&self.payload_len.to_le_bytes());
        bytes[12] = self.version;
        bytes[13] = self.flags;
        bytes[14..22].copy_from_slice(&self.timestamp.to_le_bytes());
        bytes[22..54].copy_from_slice(&self.hash);
        bytes[54..62].copy_from_slice(&self.extension);
        // Remaining bytes are zero-padded by default

        // Convert bytes to u16 chunks (Little Endian)
        let mut chunks = Vec::with_capacity(HEADER_SIZE_SAMPLES);
        for chunk in bytes.chunks(2) {
            chunks.push(u16::from_le_bytes([chunk[0], chunk[1]]));
        }
        chunks
    }

    pub fn from_u16_chunks(chunks: &[u16]) -> Result<Self> {
        if chunks.len() < HEADER_SIZE_SAMPLES {
            return Err(anyhow::anyhow!("Header too short"));
        }

        let mut bytes = Vec::with_capacity(HEADER_SIZE_BYTES);
        for chunk in chunks.iter().take(HEADER_SIZE_SAMPLES) {
            bytes.extend_from_slice(&chunk.to_le_bytes());
        }

        let magic: [u8; 4] = bytes[0..4].try_into().unwrap();
        if &magic != MAGIC {
            return Err(anyhow::anyhow!("Invalid Magic Bytes. Not a SPNG file?"));
        }

        let payload_len = u64::from_le_bytes(bytes[4..12].try_into().unwrap());
        let version = bytes[12];
        let flags = bytes[13];
        let timestamp = u64::from_le_bytes(bytes[14..22].try_into().unwrap());
        let hash: [u8; 32] = bytes[22..54].try_into().unwrap();
        let extension: [u8; 8] = bytes[54..62].try_into().unwrap();

        Ok(Self {
            magic,
            version,
            flags,
            payload_len,
            timestamp,
            hash,
            extension,
        })
    }

    pub fn read_from_stream<R: std::io::Read>(reader: &mut R) -> Result<Self> {
        let mut bytes = vec![0u8; HEADER_SIZE_BYTES];
        reader.read_exact(&mut bytes)?;
        
        let chunks: Vec<u16> = bytes.chunks(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
        Self::from_u16_chunks(&chunks)
    }
}
