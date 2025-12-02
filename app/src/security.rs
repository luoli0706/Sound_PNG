use rand::RngCore;
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use sha2::{Digest, Sha256};

/// Third Judge (TJ): Calculate SHA-256 hash of data
pub fn calculate_hash(data: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hasher.finalize().into()
}

/// Second Judge (SJ) & First Judge (FJ) & Fourth Judge (Key File):
/// Generate stream from Timestamp (SJ) and apply XOR encryption (FJ).
/// If a physical key_stream is provided (Fourth Judge), it is also XORed.
/// Modifications are done in-place.
pub fn encrypt_decrypt(data: &mut [u8], timestamp: u64, key_stream: Option<&[u8]>) {
    // Use ChaCha8 for fast, reproducible pseudo-random stream generation
    let mut rng = ChaCha8Rng::seed_from_u64(timestamp);

    // Process in 8-byte (u64) chunks for efficiency
    let mut chunks = data.chunks_exact_mut(8);
    let mut key_iter = key_stream.into_iter().flat_map(|s| s.iter().cycle()); // Infinite cycle of key bytes

    while let Some(chunk) = chunks.next() {
        let chacha_key_part = rng.next_u64();
        let mut data_part = u64::from_le_bytes(chunk.try_into().unwrap());
        
        // Apply ChaCha8 XOR
        data_part ^= chacha_key_part;

        // Apply Physical Key XOR (byte by byte for simplicity, or construct u64)
        if key_stream.is_some() {
             let mut phys_key_u64_bytes = [0u8; 8];
             for b in phys_key_u64_bytes.iter_mut() {
                 *b = *key_iter.next().unwrap();
             }
             let phys_key_part = u64::from_le_bytes(phys_key_u64_bytes);
             data_part ^= phys_key_part;
        }

        chunk.copy_from_slice(&data_part.to_le_bytes());
    }

    // Handle remaining bytes
    let remainder = chunks.into_remainder();
    if !remainder.is_empty() {
        let chacha_key_bytes = rng.next_u64().to_le_bytes();
        for (i, byte) in remainder.iter_mut().enumerate() {
            *byte ^= chacha_key_bytes[i];
            if let Some(_) = key_stream {
                 *byte ^= key_iter.next().unwrap();
            }
        }
    }
}
