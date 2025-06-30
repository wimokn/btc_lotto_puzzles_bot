use anyhow::Result;
use num_bigint::{BigUint, RandBigInt};
use rand::thread_rng;
use secp256k1::{PublicKey, Secp256k1, SecretKey};

/// Generate a random private key within the specified range
pub fn generate_random_key_in_range(
    range_start: &BigUint,
    range_end: &BigUint,
) -> Result<SecretKey> {
    let mut rng = thread_rng();

    // Generate random BigUint in range [range_start, range_end]
    let range_size = range_end - range_start;
    let random_offset = rng.gen_biguint_below(&range_size);
    let private_key_bigint = range_start + random_offset;

    // Convert BigUint to 32-byte array for secp256k1
    let key_bytes = private_key_to_bytes(&private_key_bigint)?;

    // Create SecretKey from bytes
    let secret_key = SecretKey::from_slice(&key_bytes)?;

    // log::debug!("Generated private key in range: {:#x}", private_key_bigint);

    Ok(secret_key)
}

/// Convert BigUint to 32-byte array suitable for secp256k1 SecretKey
fn private_key_to_bytes(key: &BigUint) -> Result<[u8; 32]> {
    let mut bytes = [0u8; 32];
    let key_bytes = key.to_bytes_be();

    // Ensure we don't exceed 32 bytes
    if key_bytes.len() > 32 {
        return Err(anyhow::anyhow!("Private key too large for secp256k1"));
    }

    // Copy bytes to the end of the array (big-endian)
    let start_index = 32 - key_bytes.len();
    bytes[start_index..].copy_from_slice(&key_bytes);

    Ok(bytes)
}

/// Generate public key from private key
pub fn generate_public_key(secret_key: &SecretKey) -> PublicKey {
    let secp = Secp256k1::new();
    PublicKey::from_secret_key(&secp, secret_key)
}

/// Key generation statistics
#[derive(Debug, Clone)]
pub struct KeyGenStats {
    pub total_generated: u64,
    pub range_start: BigUint,
    pub range_end: BigUint,
    pub puzzle_number: u32,
}

impl KeyGenStats {
    pub fn new(range_start: BigUint, range_end: BigUint, puzzle_number: u32) -> Self {
        Self {
            total_generated: 0,
            range_start,
            range_end,
            puzzle_number,
        }
    }

    pub fn increment(&mut self) {
        self.total_generated += 1;
    }

    pub fn get_search_space_size(&self) -> BigUint {
        &self.range_end - &self.range_start + 1u32
    }

    pub fn get_progress_percentage(&self) -> f64 {
        let search_space = self.get_search_space_size();
        if search_space == BigUint::from(0u32) {
            return 0.0;
        }

        // This is a very rough approximation since the search space is enormous
        let generated = BigUint::from(self.total_generated);
        let ratio = &generated * BigUint::from(100u32) / &search_space;

        // Convert to f64, but it will be essentially 0% for all practical purposes
        ratio.to_string().parse::<f64>().unwrap_or(0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use num_bigint::BigUint;

    #[test]
    fn test_private_key_to_bytes() {
        let key = BigUint::from(0x1234u32);
        let bytes = private_key_to_bytes(&key).unwrap();

        // Should be 32 bytes with the value at the end
        assert_eq!(bytes.len(), 32);
        assert_eq!(bytes[30], 0x12);
        assert_eq!(bytes[31], 0x34);

        // All other bytes should be zero
        for i in 0..30 {
            assert_eq!(bytes[i], 0);
        }
    }

    #[test]
    fn test_key_generation_in_range() {
        let start = BigUint::from(0x1000u32);
        let end = BigUint::from(0x2000u32);

        // Generate several keys and verify they're in range
        for _ in 0..10 {
            let secret_key = generate_random_key_in_range(&start, &end).unwrap();
            let key_bytes = secret_key.secret_bytes();
            let key_bigint = BigUint::from_bytes_be(&key_bytes);

            assert!(key_bigint >= start);
            assert!(key_bigint <= end);
        }
    }

    #[test]
    fn test_stats() {
        let start = BigUint::from(0x1000u32);
        let end = BigUint::from(0x2000u32);
        let mut stats = KeyGenStats::new(start, end, 14);

        assert_eq!(stats.total_generated, 0);
        assert_eq!(stats.puzzle_number, 14);

        stats.increment();
        assert_eq!(stats.total_generated, 1);

        let space_size = stats.get_search_space_size();
        assert_eq!(space_size, BigUint::from(0x1001u32)); // 0x2000 - 0x1000 + 1
    }
}
