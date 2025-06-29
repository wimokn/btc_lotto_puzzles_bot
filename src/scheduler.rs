use anyhow::Result;
use chrono::Utc;
use rand::seq::SliceRandom;
use std::time::Duration;
use tokio::time::{interval, Instant};

use crate::{
    puzzles::{Puzzle, PuzzleCollection},
    keygen::generate_random_key_in_range,
    checker::{check_private_key_against_puzzle, CheckStats},
    telegram::TelegramNotifier,
};

/// Configuration for the puzzle solver scheduler
#[derive(Debug, Clone)]
pub struct SchedulerConfig {
    /// Interval between puzzle solving attempts (in seconds)
    pub check_interval_seconds: u64,
    /// Minimum bits to consider (puzzles below this are ignored)
    pub min_bits: Option<u32>,
    /// Maximum bits to consider (puzzles above this are ignored)
    pub max_bits: Option<u32>,
    /// Minimum reward in BTC to consider
    pub min_reward_btc: Option<f64>,
    /// Whether to send periodic status updates
    pub send_stats_updates: bool,
    /// How often to send stats updates (in hours)
    pub stats_update_interval_hours: f64,
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self {
            check_interval_seconds: 60, // 1 minute
            min_bits: Some(14),          // Skip very small puzzles
            max_bits: None,              // No upper limit
            min_reward_btc: Some(0.0),   // Include all puzzles with rewards
            send_stats_updates: true,
            stats_update_interval_hours: 24.0, // Daily stats
        }
    }
}

/// Main scheduler that orchestrates the puzzle solving process
pub struct PuzzleSolverScheduler {
    config: SchedulerConfig,
    puzzles: PuzzleCollection,
    telegram_notifier: Option<TelegramNotifier>,
    check_stats: CheckStats,
    start_time: Instant,
}

impl PuzzleSolverScheduler {
    /// Create new scheduler
    pub fn new(
        config: SchedulerConfig,
        puzzles: PuzzleCollection,
        telegram_notifier: Option<TelegramNotifier>,
    ) -> Self {
        Self {
            config,
            puzzles,
            telegram_notifier,
            check_stats: CheckStats::new(),
            start_time: Instant::now(),
        }
    }
    
    /// Start the main solving loop
    pub async fn run(&mut self) -> Result<()> {
        log::info!("Starting puzzle solver scheduler...");
        log::info!("Configuration: {:?}", self.config);
        
        // Send startup notification
        if let Some(notifier) = &self.telegram_notifier {
            let puzzle_count = self.get_eligible_puzzles().len();
            if let Err(e) = notifier.notify_startup(puzzle_count).await {
                log::error!("Failed to send startup notification: {}", e);
            }
        }
        
        // Create intervals
        let mut check_interval = interval(Duration::from_secs(self.config.check_interval_seconds));
        let mut stats_interval = if self.config.send_stats_updates {
            let stats_seconds = (self.config.stats_update_interval_hours * 3600.0) as u64;
            Some(interval(Duration::from_secs(stats_seconds)))
        } else {
            None
        };
        
        log::info!("Scheduler started, checking every {} seconds", self.config.check_interval_seconds);
        
        loop {
            tokio::select! {
                _ = check_interval.tick() => {
                    if let Err(e) = self.run_single_check().await {
                        log::error!("Error during puzzle check: {}", e);
                        
                        // Notify about errors if configured
                        if let Some(notifier) = &self.telegram_notifier {
                            let error_msg = format!("Puzzle check error: {}", e);
                            if let Err(notify_err) = notifier.notify_error(&error_msg).await {
                                log::error!("Failed to send error notification: {}", notify_err);
                            }
                        }
                    }
                }
                
                _ = async {
                    match stats_interval.as_mut() {
                        Some(interval) => interval.tick().await,
                        None => std::future::pending().await,
                    }
                } => {
                    if let Err(e) = self.send_stats_update().await {
                        log::error!("Error sending stats update: {}", e);
                    }
                }
            }
        }
    }
    
    /// Run a single puzzle check iteration
    async fn run_single_check(&mut self) -> Result<()> {
        // Clone the puzzle collection to avoid borrow issues
        let eligible_puzzles: Vec<Puzzle> = self.get_eligible_puzzles().into_iter().cloned().collect();
        
        if eligible_puzzles.is_empty() {
            log::warn!("No eligible puzzles found with current configuration");
            return Ok(());
        }
        
        // Randomly select a puzzle
        let selected_puzzle = eligible_puzzles
            .choose(&mut rand::thread_rng())
            .ok_or_else(|| anyhow::anyhow!("Failed to select random puzzle"))?
            .clone();
        
        log::debug!("Selected puzzle {} for checking", selected_puzzle.puzzle);
        
        // Generate random private key in the puzzle's range
        let range_start = selected_puzzle.get_range_start()?;
        let range_end = selected_puzzle.get_range_end()?;
        let private_key = generate_random_key_in_range(&range_start, &range_end)?;
        
        // Check if this key solves the puzzle
        let result = check_private_key_against_puzzle(&private_key, &selected_puzzle)?;
        
        // Record the check result
        self.check_stats.record_check(&result);
        
        log::debug!(
            "Checked puzzle {}: {} (total checked: {})",
            result.puzzle_number,
            if result.is_match { "MATCH!" } else { "no match" },
            self.check_stats.total_checked
        );
        
        // If we found a match, send notification
        if result.is_match {
            log::info!("🎉 PUZZLE SOLVED! Sending Telegram notification...");
            
            if let Some(notifier) = &self.telegram_notifier {
                if let Err(e) = notifier.notify_puzzle_solved(&result, &selected_puzzle).await {
                    log::error!("Failed to send success notification: {}", e);
                } else {
                    log::info!("Success notification sent!");
                }
            }
            
            // Save the result to disk
            if let Err(e) = self.save_success_to_disk(&result, &selected_puzzle).await {
                log::error!("Failed to save success to disk: {}", e);
            }
        }
        
        Ok(())
    }
    
    /// Get puzzles that match the current configuration criteria
    fn get_eligible_puzzles(&self) -> Vec<&Puzzle> {
        self.puzzles
            .get_all_puzzles()
            .iter()
            .filter(|puzzle| {
                // Check minimum bits
                if let Some(min_bits) = self.config.min_bits {
                    if puzzle.bits < min_bits {
                        return false;
                    }
                }
                
                // Check maximum bits
                if let Some(max_bits) = self.config.max_bits {
                    if puzzle.bits > max_bits {
                        return false;
                    }
                }
                
                // Check minimum reward
                if let Some(min_reward) = self.config.min_reward_btc {
                    if puzzle.reward_btc < min_reward {
                        return false;
                    }
                }
                
                true
            })
            .collect()
    }
    
    /// Send periodic statistics update
    async fn send_stats_update(&self) -> Result<()> {
        if let Some(notifier) = &self.telegram_notifier {
            let uptime_hours = self.start_time.elapsed().as_secs_f64() / 3600.0;
            
            notifier.notify_stats(
                self.check_stats.total_checked,
                self.check_stats.current_puzzle,
                uptime_hours,
            ).await?;
        }
        
        Ok(())
    }
    
    /// Save successful puzzle solve to disk
    async fn save_success_to_disk(
        &self,
        result: &crate::checker::CheckResult,
        puzzle: &Puzzle,
    ) -> Result<()> {
        use std::fs::OpenOptions;
        use std::io::Write;
        
        let timestamp = Utc::now().format("%Y-%m-%d %H:%M:%S UTC");
        let log_entry = format!(
            "[{}] PUZZLE {} SOLVED - Private Key: {}, Address: {}, Reward: {} BTC\n",
            timestamp,
            result.puzzle_number,
            result.private_key_hex,
            result.target_address,
            puzzle.reward_btc
        );
        
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open("puzzle_solutions.log")?;
        
        file.write_all(log_entry.as_bytes())?;
        file.flush()?;
        
        log::info!("Puzzle solution saved to puzzle_solutions.log");
        
        Ok(())
    }
    
    /// Get current statistics
    pub fn get_stats(&self) -> &CheckStats {
        &self.check_stats
    }
    
    /// Get uptime in hours
    pub fn get_uptime_hours(&self) -> f64 {
        self.start_time.elapsed().as_secs_f64() / 3600.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::puzzles::Puzzle;
    
    fn create_test_puzzle(puzzle_num: u32, bits: u32, reward: f64) -> Puzzle {
        Puzzle {
            puzzle: puzzle_num,
            bits,
            range_start: "0x1000".to_string(),
            range_end: "0x2000".to_string(),
            address: "test_address".to_string(),
            reward_btc: reward,
        }
    }
    
    #[test]
    fn test_eligible_puzzles_filtering() {
        let puzzles = vec![
            create_test_puzzle(10, 10, 0.0),   // Too small bits
            create_test_puzzle(15, 15, 1.5),   // Should be included
            create_test_puzzle(20, 20, 0.5),   // Should be included
            create_test_puzzle(100, 100, 10.0), // Too large bits
        ];
        
        let collection = PuzzleCollection { puzzles };
        
        let config = SchedulerConfig {
            min_bits: Some(14),
            max_bits: Some(50),
            min_reward_btc: Some(1.0),
            ..Default::default()
        };
        
        let scheduler = PuzzleSolverScheduler::new(config, collection, None);
        let eligible = scheduler.get_eligible_puzzles();
        
        assert_eq!(eligible.len(), 1);
        assert_eq!(eligible[0].puzzle, 15);
    }
    
    #[test]
    fn test_default_config() {
        let config = SchedulerConfig::default();
        assert_eq!(config.check_interval_seconds, 60);
        assert_eq!(config.min_bits, Some(14));
        assert_eq!(config.min_reward_btc, Some(0.0));
        assert!(config.send_stats_updates);
    }
}