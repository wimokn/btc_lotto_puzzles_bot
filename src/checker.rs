use anyhow::Result;
use bitcoin::{
    address::Address,
    key::PublicKey as BitcoinPublicKey,
    network::Network,
    secp256k1::{PublicKey, SecretKey}
};
use secp256k1::Secp256k1;
use crate::puzzles::Puzzle;

/// Result of checking a private key against a puzzle
#[derive(Debug, Clone)]
pub struct CheckResult {
    pub puzzle_number: u32,
    pub private_key_hex: String,
    pub compressed_address: String,
    pub uncompressed_address: String,
    pub target_address: String,
    pub is_match: bool,
    pub match_type: Option<AddressType>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AddressType {
    Compressed,
    Uncompressed,
}

impl CheckResult {
    pub fn new(
        puzzle_number: u32,
        private_key: &SecretKey,
        compressed_address: String,
        uncompressed_address: String,
        target_address: String,
    ) -> Self {
        let private_key_hex = hex::encode(private_key.secret_bytes());
        
        let (is_match, match_type) = if compressed_address == target_address {
            (true, Some(AddressType::Compressed))
        } else if uncompressed_address == target_address {
            (true, Some(AddressType::Uncompressed))
        } else {
            (false, None)
        };
        
        Self {
            puzzle_number,
            private_key_hex,
            compressed_address,
            uncompressed_address,
            target_address,
            is_match,
            match_type,
        }
    }
}

/// Check if a private key solves a puzzle
pub fn check_private_key_against_puzzle(
    private_key: &SecretKey,
    puzzle: &Puzzle,
) -> Result<CheckResult> {
    let secp = Secp256k1::new();
    let public_key = PublicKey::from_secret_key(&secp, private_key);
    
    // Generate both compressed and uncompressed addresses
    let compressed_address = derive_bitcoin_address(&public_key, true)?;
    let uncompressed_address = derive_bitcoin_address(&public_key, false)?;
    
    log::debug!(
        "Checking puzzle {}: compressed={}, uncompressed={}, target={}",
        puzzle.puzzle,
        compressed_address,
        uncompressed_address,
        puzzle.address
    );
    
    let result = CheckResult::new(
        puzzle.puzzle,
        private_key,
        compressed_address,
        uncompressed_address,
        puzzle.address.clone(),
    );
    
    if result.is_match {
        log::info!(
            "🎉 PUZZLE {} SOLVED! Private key: {}, Address type: {:?}",
            puzzle.puzzle,
            result.private_key_hex,
            result.match_type
        );
    }
    
    Ok(result)
}

/// Derive Bitcoin address from public key
fn derive_bitcoin_address(public_key: &PublicKey, compressed: bool) -> Result<String> {
    // Convert secp256k1::PublicKey to bitcoin::PublicKey
    let bitcoin_public_key = if compressed {
        BitcoinPublicKey::new(public_key.clone())
    } else {
        BitcoinPublicKey::new_uncompressed(public_key.clone())
    };
    
    // Create P2PKH address (legacy format starting with '1')
    let address = Address::p2pkh(&bitcoin_public_key, Network::Bitcoin);
    
    Ok(address.to_string())
}

/// Batch check multiple private keys against a puzzle
pub fn batch_check_keys(
    private_keys: &[SecretKey],
    puzzle: &Puzzle,
) -> Result<Vec<CheckResult>> {
    let mut results = Vec::new();
    
    for private_key in private_keys {
        let result = check_private_key_against_puzzle(private_key, puzzle)?;
        results.push(result);
    }
    
    Ok(results)
}

/// Check statistics
#[derive(Debug, Clone)]
pub struct CheckStats {
    pub total_checked: u64,
    pub matches_found: u64,
    pub compressed_matches: u64,
    pub uncompressed_matches: u64,
    pub current_puzzle: Option<u32>,
}

impl CheckStats {
    pub fn new() -> Self {
        Self {
            total_checked: 0,
            matches_found: 0,
            compressed_matches: 0,
            uncompressed_matches: 0,
            current_puzzle: None,
        }
    }
    
    pub fn record_check(&mut self, result: &CheckResult) {
        self.total_checked += 1;
        self.current_puzzle = Some(result.puzzle_number);
        
        if result.is_match {
            self.matches_found += 1;
            match result.match_type {
                Some(AddressType::Compressed) => self.compressed_matches += 1,
                Some(AddressType::Uncompressed) => self.uncompressed_matches += 1,
                None => {} // Should not happen if is_match is true
            }
        }
    }
    
    pub fn get_match_rate(&self) -> f64 {
        if self.total_checked == 0 {
            0.0
        } else {
            self.matches_found as f64 / self.total_checked as f64
        }
    }
}

impl Default for CheckStats {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use secp256k1::SecretKey;
    use std::str::FromStr;
    
    #[test]
    fn test_address_derivation() {
        // Test with a known private key
        let private_key_hex = "0000000000000000000000000000000000000000000000000000000000000001";
        let private_key_bytes = hex::decode(private_key_hex).unwrap();
        let private_key = SecretKey::from_slice(&private_key_bytes).unwrap();
        
        let secp = Secp256k1::new();
        let public_key = PublicKey::from_secret_key(&secp, &private_key);
        
        let compressed = derive_bitcoin_address(&public_key, true).unwrap();
        let uncompressed = derive_bitcoin_address(&public_key, false).unwrap();
        
        // These are the known addresses for private key 1
        assert_eq!(compressed, "1BgGZ9tcN4rm9KBzDn7KprQz87SZ26SAMH");
        assert_eq!(uncompressed, "1EHNa6Q4Jz2uvNExL497mE43ikXhwF6kZm");
    }
    
    #[test]
    fn test_check_result() {
        let private_key_hex = "0000000000000000000000000000000000000000000000000000000000000001";
        let private_key_bytes = hex::decode(private_key_hex).unwrap();
        let private_key = SecretKey::from_slice(&private_key_bytes).unwrap();
        
        let result = CheckResult::new(
            1,
            &private_key,
            "1BgGZ9tcN4rm9KBzDn7KprQz87SZ26SAMH".to_string(),
            "1EHNa6Q4Jz2uvNExL497mE43ikXhwF6kZm".to_string(),
            "1BgGZ9tcN4rm9KBzDn7KprQz87SZ26SAMH".to_string(), // Match compressed
        );
        
        assert!(result.is_match);
        assert_eq!(result.match_type, Some(AddressType::Compressed));
        assert_eq!(result.private_key_hex, private_key_hex);
    }
    
    #[test]
    fn test_stats() {
        let mut stats = CheckStats::new();
        
        let private_key_bytes = hex::decode("0000000000000000000000000000000000000000000000000000000000000001").unwrap();
        let private_key = SecretKey::from_slice(&private_key_bytes).unwrap();
        
        // Test miss
        let miss_result = CheckResult::new(
            1,
            &private_key,
            "1BgGZ9tcN4rm9KBzDn7KprQz87SZ26SAMH".to_string(),
            "1EHNa6Q4Jz2uvNExL497mE43ikXhwF6kZm".to_string(),
            "SomeOtherAddress".to_string(),
        );
        
        stats.record_check(&miss_result);
        assert_eq!(stats.total_checked, 1);
        assert_eq!(stats.matches_found, 0);
        assert_eq!(stats.get_match_rate(), 0.0);
        
        // Test hit
        let hit_result = CheckResult::new(
            1,
            &private_key,
            "1BgGZ9tcN4rm9KBzDn7KprQz87SZ26SAMH".to_string(),
            "1EHNa6Q4Jz2uvNExL497mE43ikXhwF6kZm".to_string(),
            "1BgGZ9tcN4rm9KBzDn7KprQz87SZ26SAMH".to_string(),
        );
        
        stats.record_check(&hit_result);
        assert_eq!(stats.total_checked, 2);
        assert_eq!(stats.matches_found, 1);
        assert_eq!(stats.compressed_matches, 1);
        assert_eq!(stats.get_match_rate(), 0.5);
    }
}