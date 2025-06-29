use anyhow::Result;
use chrono::Utc;
use rand::seq::SliceRandom;
use std::sync::Arc;
use std::time::Duration;
use tokio::{
    sync::{Notify, RwLock},
    time::{Instant, interval},
};

use crate::{
    checker::{CheckStats, check_private_key_against_puzzle},
    keygen::generate_random_key_in_range,
    puzzles::{Puzzle, PuzzleCollection},
    telegram::TelegramNotifier,
    telegram_bot::{BotState, update_bot_state},
};

/// Configuration for the puzzle solver scheduler
#[derive(Debug, Clone)]
pub struct SchedulerConfig {
    /// Duration to run each solving session (in seconds)
    pub run_duration_seconds: u64,
    /// Interval between solving sessions (in seconds)
    pub check_interval_seconds: u64,
    /// Number of threads to use for parallel processing
    pub threads: usize,
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
            run_duration_seconds: 600,  // 10 minutes per session
            check_interval_seconds: 60, // 1 minute between sessions
            threads: 8,                 // 8 threads for parallel processing
            min_bits: Some(14),         // Skip very small puzzles
            max_bits: None,             // No upper limit
            min_reward_btc: Some(0.0),  // Include all puzzles with rewards
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
    bot_state: Option<Arc<RwLock<BotState>>>,
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
            bot_state: None,
        }
    }

    /// Create new scheduler with bot state integration
    pub fn with_bot_state(
        config: SchedulerConfig,
        puzzles: PuzzleCollection,
        telegram_notifier: Option<TelegramNotifier>,
        bot_state: Arc<RwLock<BotState>>,
    ) -> Self {
        Self {
            config,
            puzzles,
            telegram_notifier,
            check_stats: CheckStats::new(),
            start_time: Instant::now(),
            bot_state: Some(bot_state),
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

        log::info!(
            "Scheduler started, checking every {} seconds",
            self.config.check_interval_seconds
        );

        loop {
            tokio::select! {
                _ = check_interval.tick() => {
                    // Check if bot state allows running (if bot state is available)
                    let should_run = if let Some(bot_state) = &self.bot_state {
                        bot_state.read().await.is_running
                    } else {
                        true // Always run if no bot state
                    };

                    if should_run {
                        if let Err(e) = self.run_solving_session().await {
                            log::error!("Error during puzzle solving session: {}", e);

                            // Notify about errors if configured
                            if let Some(notifier) = &self.telegram_notifier {
                                let error_msg = format!("Puzzle solving session error: {}", e);
                                if let Err(notify_err) = notifier.notify_error(&error_msg).await {
                                    log::error!("Failed to send error notification: {}", notify_err);
                                }
                            }
                        }
                    } else {
                        log::debug!("Puzzle solver is paused via bot command");
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

    /// Run a solving session for the configured duration using multiple threads
    async fn run_solving_session(&mut self) -> Result<()> {
        // Clone the puzzle collection to avoid borrow issues
        let eligible_puzzles: Vec<Puzzle> =
            self.get_eligible_puzzles().into_iter().cloned().collect();

        if eligible_puzzles.is_empty() {
            log::warn!("No eligible puzzles found with current configuration");
            return Ok(());
        }

        log::info!(
            "Starting solving session: {} threads for {} seconds on {} puzzles",
            self.config.threads,
            self.config.run_duration_seconds,
            eligible_puzzles.len()
        );

        let session_start = Instant::now();
        let duration = Duration::from_secs(self.config.run_duration_seconds);

        // Use a channel to collect results from worker threads
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

        // Create a shutdown signal for all threads when a match is found
        let shutdown_notify = Arc::new(Notify::new());

        // Start worker threads
        let mut handles = Vec::new();
        for thread_id in 0..self.config.threads {
            let puzzles = eligible_puzzles.clone();
            let tx = tx.clone();
            let thread_duration = duration;
            let shutdown_notify = shutdown_notify.clone();

            let handle = tokio::spawn(async move {
                let mut thread_stats = 0u64;
                let thread_start = Instant::now();

                log::info!("Worker thread {} started", thread_id);

                loop {
                    // Check if we should stop (time limit or shutdown signal)
                    tokio::select! {
                        _ = shutdown_notify.notified() => {
                            log::debug!("Worker thread {} received shutdown signal", thread_id);
                            break;
                        }
                        _ = tokio::time::sleep(Duration::from_millis(1)) => {
                            if thread_start.elapsed() >= thread_duration {
                                break;
                            }
                        }
                    }
                    // Randomly select a puzzle
                    let selected_puzzle = match puzzles.choose(&mut rand::thread_rng()) {
                        Some(puzzle) => puzzle,
                        None => break,
                    };

                    // Generate random private key in the puzzle's range
                    match selected_puzzle
                        .get_range_start()
                        .and_then(|start| selected_puzzle.get_range_end().map(|end| (start, end)))
                    {
                        Ok((range_start, range_end)) => {
                            match generate_random_key_in_range(&range_start, &range_end) {
                                Ok(private_key) => {
                                    // Check if this key solves the puzzle
                                    match check_private_key_against_puzzle(
                                        &private_key,
                                        selected_puzzle,
                                    ) {
                                        Ok(result) => {
                                            thread_stats += 1;

                                            // Send result back to main thread
                                            if let Err(_) =
                                                tx.send((result, selected_puzzle.clone()))
                                            {
                                                log::warn!(
                                                    "Worker thread {} failed to send result",
                                                    thread_id
                                                );
                                                break;
                                            }
                                        }
                                        Err(e) => {
                                            log::warn!(
                                                "Worker thread {} check error: {}",
                                                thread_id,
                                                e
                                            );
                                        }
                                    }
                                }
                                Err(e) => {
                                    log::warn!(
                                        "Worker thread {} key generation error: {}",
                                        thread_id,
                                        e
                                    );
                                }
                            }
                        }
                        Err(e) => {
                            log::warn!("Worker thread {} range parsing error: {}", thread_id, e);
                        }
                    }
                }

                log::debug!(
                    "Worker thread {} completed {} checks",
                    thread_id,
                    thread_stats
                );
                thread_stats
            });

            handles.push(handle);
        }

        // Drop the sender so the receiver will close when all workers are done
        drop(tx);

        // Collect results from worker threads
        let mut session_checks = 0u64;
        let mut matches_found = Vec::new();
        let mut found_solution = false;

        while let Some((result, puzzle)) = rx.recv().await {
            session_checks += 1;
            self.check_stats.record_check(&result);

            // Update bot state periodically (every 100 checks to avoid overhead)
            if session_checks % 100 == 0 {
                if let Some(bot_state) = &self.bot_state {
                    update_bot_state(bot_state, &self.check_stats, Some(result.puzzle_number))
                        .await;
                }
            }

            // Handle matches
            if result.is_match && !found_solution {
                found_solution = true;
                matches_found.push((result, puzzle));

                // Signal all threads to stop immediately
                shutdown_notify.notify_waiters();
                log::info!("Signaling all worker threads to stop after finding solution");
            }
        }

        // Wait for all worker threads to complete
        for handle in handles {
            if let Err(e) = handle.await {
                log::error!("Worker thread error: {}", e);
            }
        }

        let session_duration = session_start.elapsed();
        let checks_per_second = session_checks as f64 / session_duration.as_secs_f64();

        log::info!(
            "Solving session completed: {} checks in {:.2}s ({:.0} checks/sec)",
            session_checks,
            session_duration.as_secs_f64(),
            checks_per_second
        );

        // Update final bot state
        if let Some(bot_state) = &self.bot_state {
            update_bot_state(
                bot_state,
                &self.check_stats,
                self.check_stats.current_puzzle,
            )
            .await;
        }

        // Process any matches found
        for (result, puzzle) in matches_found {
            // Send notification
            if let Some(notifier) = &self.telegram_notifier {
                if let Err(e) = notifier.notify_puzzle_solved(&result, &puzzle).await {
                    log::error!("Failed to send success notification: {}", e);
                } else {
                    log::info!(
                        "Success notification sent for puzzle {}",
                        result.puzzle_number
                    );
                }
            }

            // Save the result to disk
            if let Err(e) = self.save_success_to_disk(&result, &puzzle).await {
                log::error!("Failed to save success to disk: {}", e);
            }

            // Stop the bot after finding a match - requires manual restart
            if let Some(bot_state) = &self.bot_state {
                let mut state_guard = bot_state.write().await;
                state_guard.is_running = false;
                log::info!(
                    "🛑 Bot automatically stopped after finding puzzle solution. Manual restart required."
                );
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

            notifier
                .notify_stats(
                    self.check_stats.total_checked,
                    self.check_stats.current_puzzle,
                    uptime_hours,
                )
                .await?;
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
            create_test_puzzle(10, 10, 0.0),    // Too small bits
            create_test_puzzle(15, 15, 1.5),    // Should be included
            create_test_puzzle(20, 20, 0.5),    // Should be included
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
        assert_eq!(config.run_duration_seconds, 600);
        assert_eq!(config.check_interval_seconds, 60);
        assert_eq!(config.threads, 8);
        assert_eq!(config.min_bits, Some(14));
        assert_eq!(config.min_reward_btc, Some(0.0));
        assert!(config.send_stats_updates);
    }
}
