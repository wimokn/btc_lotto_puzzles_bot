mod puzzles;
mod keygen;
mod checker;
mod telegram;
mod telegram_bot;
mod scheduler;

use anyhow::Result;
use dotenv::dotenv;
use log::{info, error};
use std::{env, sync::Arc};
use tokio::sync::RwLock;

use puzzles::PuzzleCollection;
use scheduler::{PuzzleSolverScheduler, SchedulerConfig};
use telegram::TelegramNotifier;
use telegram_bot::{BotState, InteractiveTelegramBot};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    
    info!("🚀 Starting BTC Lotto Puzzles Bot");
    
    // Load environment variables
    dotenv().ok();
    
    // Load configuration from environment or use defaults
    let config = load_config_from_env();
    info!("Loaded configuration: {:?}", config);
    
    // Load puzzles from JSON file
    let puzzles_file = env::var("PUZZLES_FILE").unwrap_or_else(|_| "unsolved_puzzles.json".to_string());
    let puzzles = match PuzzleCollection::load_from_file(&puzzles_file) {
        Ok(puzzles) => {
            info!("Successfully loaded {} puzzles from {}", puzzles.get_all_puzzles().len(), puzzles_file);
            puzzles
        }
        Err(e) => {
            error!("Failed to load puzzles from {}: {}", puzzles_file, e);
            return Err(e);
        }
    };
    
    // Initialize Telegram notifier if credentials are available
    let telegram_notifier = match TelegramNotifier::new() {
        Ok(notifier) => {
            info!("Telegram notifications enabled");
            
            // Test the connection
            if let Err(e) = notifier.test_connection().await {
                error!("Telegram connection test failed: {}", e);
                error!("Continuing without Telegram notifications...");
                None
            } else {
                info!("Telegram connection test successful");
                Some(notifier)
            }
        }
        Err(e) => {
            error!("Failed to initialize Telegram notifier: {}", e);
            error!("Continuing without Telegram notifications...");
            None
        }
    };

    // Check if we should enable interactive bot mode
    let enable_interactive_bot = env::var("ENABLE_INTERACTIVE_BOT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(true);

    if enable_interactive_bot {
        // Create shared bot state
        let bot_state = Arc::new(RwLock::new(BotState::new(
            config.clone(),
            puzzles.get_all_puzzles().len(),
        )));

        // Initialize interactive Telegram bot
        if let Ok(bot_token) = env::var("TELOXIDE_TOKEN") {
            let interactive_bot = InteractiveTelegramBot::new(bot_token, bot_state.clone());
            
            // Create scheduler with bot state integration
            let mut scheduler = PuzzleSolverScheduler::with_bot_state(
                config,
                puzzles,
                telegram_notifier,
                bot_state.clone(),
            );

            info!("🤖 Starting with interactive Telegram bot enabled");
            info!("🎯 Starting puzzle solving loop...");
            info!("Press Ctrl+C to stop the bot");

            // Start both the interactive bot and the scheduler
            let scheduler_handle = tokio::spawn(async move {
                scheduler.run().await
            });

            let bot_handle = tokio::spawn(async move {
                if let Err(e) = interactive_bot.run().await {
                    error!("Interactive bot error: {}", e);
                }
            });

            // Handle graceful shutdown
            let shutdown_result = tokio::select! {
                result = scheduler_handle => {
                    error!("Scheduler exited unexpectedly: {:?}", result);
                    result.unwrap_or_else(|e| Err(anyhow::anyhow!("Scheduler task error: {}", e)))
                }
                result = bot_handle => {
                    error!("Interactive bot exited unexpectedly: {:?}", result);
                    match result {
                        Ok(_) => Ok(()),
                        Err(e) => Err(anyhow::anyhow!("Bot task error: {}", e)),
                    }
                }
                _ = tokio::signal::ctrl_c() => {
                    info!("Received Ctrl+C, shutting down gracefully...");
                    Ok(())
                }
            };

            shutdown_result
        } else {
            error!("TELOXIDE_TOKEN not found, falling back to basic mode");
            run_basic_mode(config, puzzles, telegram_notifier).await
        }
    } else {
        info!("Interactive bot disabled, running in basic mode");
        run_basic_mode(config, puzzles, telegram_notifier).await
    }
}

/// Run the bot in basic mode without interactive features
async fn run_basic_mode(
    config: SchedulerConfig,
    puzzles: PuzzleCollection,
    telegram_notifier: Option<TelegramNotifier>,
) -> Result<()> {
    // Create and start the scheduler
    let mut scheduler = PuzzleSolverScheduler::new(config, puzzles, telegram_notifier);
    
    info!("🎯 Starting puzzle solving loop...");
    info!("Press Ctrl+C to stop the bot");
    
    // Handle graceful shutdown
    let shutdown_result = tokio::select! {
        result = scheduler.run() => {
            error!("Scheduler exited unexpectedly: {:?}", result);
            result
        }
        _ = tokio::signal::ctrl_c() => {
            info!("Received Ctrl+C, shutting down gracefully...");
            print_final_stats(&scheduler);
            Ok(())
        }
    };
    
    info!("🛑 BTC Lotto Puzzles Bot stopped");
    shutdown_result
}

/// Load scheduler configuration from environment variables
fn load_config_from_env() -> SchedulerConfig {
    let run_duration_seconds = env::var("RUN_DURATION_SECONDS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(600); // Default: 10 minutes
    
    let check_interval_seconds = env::var("CHECK_INTERVAL_SECONDS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(60); // Default: 1 minute
    
    let threads = env::var("THREADS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(8); // Default: 8 threads
    
    let min_bits = env::var("MIN_BITS")
        .ok()
        .and_then(|s| s.parse().ok());
    
    let max_bits = env::var("MAX_BITS")
        .ok()
        .and_then(|s| s.parse().ok());
    
    let min_reward_btc = env::var("MIN_REWARD_BTC")
        .ok()
        .and_then(|s| s.parse().ok());
    
    let send_stats_updates = env::var("SEND_STATS_UPDATES")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(true);
    
    let stats_update_interval_hours = env::var("STATS_UPDATE_INTERVAL_HOURS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(24.0);
    
    SchedulerConfig {
        run_duration_seconds,
        check_interval_seconds,
        threads,
        min_bits,
        max_bits,
        min_reward_btc,
        send_stats_updates,
        stats_update_interval_hours,
    }
}

/// Print final statistics when shutting down
fn print_final_stats(scheduler: &PuzzleSolverScheduler) {
    let stats = scheduler.get_stats();
    let uptime_hours = scheduler.get_uptime_hours();
    
    info!("=== FINAL STATISTICS ===");
    info!("Total keys checked: {}", stats.total_checked);
    info!("Matches found: {}", stats.matches_found);
    info!("Uptime: {:.2} hours", uptime_hours);
    
    if uptime_hours > 0.0 {
        let rate = stats.total_checked as f64 / uptime_hours;
        info!("Average rate: {:.0} keys/hour", rate);
    }
    
    if let Some(current_puzzle) = stats.current_puzzle {
        info!("Last puzzle checked: #{}", current_puzzle);
    }
    
    info!("======================");
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    
    #[test]
    fn test_config_loading() {
        // Set some test environment variables
        unsafe {
            env::set_var("RUN_DURATION_SECONDS", "300");
            env::set_var("CHECK_INTERVAL_SECONDS", "30");
            env::set_var("THREADS", "4");
            env::set_var("MIN_BITS", "20");
            env::set_var("MAX_BITS", "80");
            env::set_var("MIN_REWARD_BTC", "1.0");
        }
        
        let config = load_config_from_env();
        
        assert_eq!(config.run_duration_seconds, 300);
        assert_eq!(config.check_interval_seconds, 30);
        assert_eq!(config.threads, 4);
        assert_eq!(config.min_bits, Some(20));
        assert_eq!(config.max_bits, Some(80));
        assert_eq!(config.min_reward_btc, Some(1.0));
        
        // Clean up
        unsafe {
            env::remove_var("RUN_DURATION_SECONDS");
            env::remove_var("CHECK_INTERVAL_SECONDS");
            env::remove_var("THREADS");
            env::remove_var("MIN_BITS");
            env::remove_var("MAX_BITS");
            env::remove_var("MIN_REWARD_BTC");
        }
    }
    
    #[test]
    fn test_default_config_loading() {
        // Make sure these env vars don't exist
        unsafe {
            env::remove_var("RUN_DURATION_SECONDS");
            env::remove_var("CHECK_INTERVAL_SECONDS");
            env::remove_var("THREADS");
            env::remove_var("MIN_BITS");
        }
        
        let config = load_config_from_env();
        
        assert_eq!(config.run_duration_seconds, 600); // Default value
        assert_eq!(config.check_interval_seconds, 60); // Default value
        assert_eq!(config.threads, 8); // Default value
    }
}