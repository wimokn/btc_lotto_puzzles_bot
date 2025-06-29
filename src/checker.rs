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
    pub address: String,
    pub target_address: String,
    pub is_match: bool,
}


impl CheckResult {
    pub fn new(
        puzzle_number: u32,
        private_key: &SecretKey,
        address: String,
        target_address: String,
    ) -> Self {
        let private_key_hex = hex::encode(private_key.secret_bytes());
        let is_match = address == target_address;
        
        Self {
            puzzle_number,
            private_key_hex,
            address,
            target_address,
            is_match,
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
    
    // Generate compressed address (standard format for puzzles)
    let address = derive_bitcoin_address(&public_key, true)?;
    
    log::debug!(
        "Checking puzzle {}: address={}, target={}",
        puzzle.puzzle,
        address,
        puzzle.address
    );
    
    let result = CheckResult::new(
        puzzle.puzzle,
        private_key,
        address,
        puzzle.address.clone(),
    );
    
    if result.is_match {
        log::info!(
            "🎉 PUZZLE {} SOLVED! Private key: {}",
            puzzle.puzzle,
            result.private_key_hex
        );
    }
    
    Ok(result)
}

/// Derive Bitcoin address from public key (compressed format)
fn derive_bitcoin_address(public_key: &PublicKey, _compressed: bool) -> Result<String> {
    // Convert secp256k1::PublicKey to bitcoin::PublicKey (always compressed)
    let bitcoin_public_key = BitcoinPublicKey::new(public_key.clone());
    
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
    pub current_puzzle: Option<u32>,
}

impl CheckStats {
    pub fn new() -> Self {
        Self {
            total_checked: 0,
            matches_found: 0,
            current_puzzle: None,
        }
    }
    
    pub fn record_check(&mut self, result: &CheckResult) {
        self.total_checked += 1;
        self.current_puzzle = Some(result.puzzle_number);
        
        if result.is_match {
            self.matches_found += 1;
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
    
    #[test]
    fn test_address_derivation() {
        // Test with a known private key
        let private_key_hex = "0000000000000000000000000000000000000000000000000000000000000001";
        let private_key_bytes = hex::decode(private_key_hex).unwrap();
        let private_key = SecretKey::from_slice(&private_key_bytes).unwrap();
        
        let secp = Secp256k1::new();
        let public_key = PublicKey::from_secret_key(&secp, &private_key);
        
        let compressed = derive_bitcoin_address(&public_key, true).unwrap();
        
        // Known compressed address for private key 1
        assert_eq!(compressed, "1BgGZ9tcN4rm9KBzDn7KprQz87SZ26SAMH");
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
            "1BgGZ9tcN4rm9KBzDn7KprQz87SZ26SAMH".to_string(), // Match
        );
        
        assert!(result.is_match);
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
            "1BgGZ9tcN4rm9KBzDn7KprQz87SZ26SAMH".to_string(),
        );
        
        stats.record_check(&hit_result);
        assert_eq!(stats.total_checked, 2);
        assert_eq!(stats.matches_found, 1);
        assert_eq!(stats.get_match_rate(), 0.5);
    }
}