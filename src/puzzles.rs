use anyhow::Result;
use num_bigint::BigUint;
use num_traits::Num;
use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Puzzle {
    pub puzzle: u32,
    pub bits: u32,
    pub range_start: String,
    pub range_end: String,
    pub address: String,
    pub reward_btc: f64,
}

impl Puzzle {
    /// Parse the hex range start as a BigUint
    pub fn get_range_start(&self) -> Result<BigUint> {
        let hex_str = self.range_start.strip_prefix("0x").unwrap_or(&self.range_start);
        Ok(BigUint::from_str_radix(hex_str, 16)?)
    }

    /// Parse the hex range end as a BigUint
    pub fn get_range_end(&self) -> Result<BigUint> {
        let hex_str = self.range_end.strip_prefix("0x").unwrap_or(&self.range_end);
        Ok(BigUint::from_str_radix(hex_str, 16)?)
    }

    /// Get the size of the search space for this puzzle
    pub fn get_range_size(&self) -> Result<BigUint> {
        let start = self.get_range_start()?;
        let end = self.get_range_end()?;
        Ok(&end - &start + 1u32)
    }
}

/// Container for all puzzles
#[derive(Debug, Clone)]
pub struct PuzzleCollection {
    pub puzzles: Vec<Puzzle>,
}

impl PuzzleCollection {
    /// Load puzzles from JSON file
    pub fn load_from_file(path: &str) -> Result<Self> {
        let content = fs::read_to_string(path)?;
        let puzzles: Vec<Puzzle> = serde_json::from_str(&content)?;
        
        log::info!("Loaded {} puzzles from {}", puzzles.len(), path);
        
        Ok(PuzzleCollection { puzzles })
    }

    /// Get a puzzle by number
    pub fn get_puzzle(&self, puzzle_number: u32) -> Option<&Puzzle> {
        self.puzzles.iter().find(|p| p.puzzle == puzzle_number)
    }

    /// Get all puzzles
    pub fn get_all_puzzles(&self) -> &[Puzzle] {
        &self.puzzles
    }

    /// Get puzzles with rewards above a threshold
    pub fn get_puzzles_with_min_reward(&self, min_reward: f64) -> Vec<&Puzzle> {
        self.puzzles
            .iter()
            .filter(|p| p.reward_btc >= min_reward)
            .collect()
    }

    /// Get puzzles within a specific bit range
    pub fn get_puzzles_by_bit_range(&self, min_bits: u32, max_bits: u32) -> Vec<&Puzzle> {
        self.puzzles
            .iter()
            .filter(|p| p.bits >= min_bits && p.bits <= max_bits)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_puzzle_range_parsing() {
        let puzzle = Puzzle {
            puzzle: 14,
            bits: 14,
            range_start: "0x2000".to_string(),
            range_end: "0x3fff".to_string(),
            address: "1ErZWg5cFCe4Vw5BzgfzB74VNLaXEiEkhk".to_string(),
            reward_btc: 0.0,
        };

        let start = puzzle.get_range_start().unwrap();
        let end = puzzle.get_range_end().unwrap();
        
        assert_eq!(start, BigUint::from(0x2000u32));
        assert_eq!(end, BigUint::from(0x3fffu32));
        
        let range_size = puzzle.get_range_size().unwrap();
        assert_eq!(range_size, BigUint::from(0x2000u32)); // 0x3fff - 0x2000 + 1 = 0x2000
    }
}