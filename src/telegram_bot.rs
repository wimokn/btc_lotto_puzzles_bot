use anyhow::Result;
use chrono::{DateTime, Utc};
use std::sync::Arc;
use teloxide::{
    prelude::*,
    types::{InlineKeyboardButton, InlineKeyboardMarkup, ParseMode},
    utils::command::BotCommands,
};
use tokio::sync::RwLock;

use crate::{checker::CheckStats, scheduler::SchedulerConfig};

/// Shared state between the scheduler and bot
#[derive(Debug, Clone)]
pub struct BotState {
    pub check_stats: CheckStats,
    pub start_time: DateTime<Utc>,
    pub current_puzzle: Option<u32>,
    pub config: SchedulerConfig,
    pub total_puzzles: usize,
    pub is_running: bool,
}

impl BotState {
    pub fn new(config: SchedulerConfig, total_puzzles: usize) -> Self {
        Self {
            check_stats: CheckStats::new(),
            start_time: Utc::now(),
            current_puzzle: None,
            config,
            total_puzzles,
            is_running: false,
        }
    }

    pub fn get_uptime_hours(&self) -> f64 {
        let now = Utc::now();
        let duration = now.signed_duration_since(self.start_time);
        duration.num_seconds() as f64 / 3600.0
    }

    pub fn get_keys_per_hour(&self) -> f64 {
        let uptime = self.get_uptime_hours();
        if uptime > 0.0 {
            self.check_stats.total_checked as f64 / uptime
        } else {
            0.0
        }
    }
}

/// Bot commands
#[derive(BotCommands, Clone)]
#[command(
    rename_rule = "lowercase",
    description = "BTC Lotto Puzzles Bot Commands"
)]
pub enum Command {
    #[command(description = "Display help message")]
    Help,
    #[command(description = "Show current bot status")]
    Status,
    #[command(description = "Show detailed statistics")]
    Stats,
    #[command(description = "Show bot configuration")]
    Config,
    #[command(description = "Start the puzzle solver")]
    Start,
    #[command(description = "Stop the puzzle solver")]
    Stop,
}

/// Callback data for inline keyboards
#[derive(Debug, Clone, PartialEq)]
pub enum CallbackData {
    Refresh,
    DetailedStats,
    Config,
    Help,
    StartSolver,
    StopSolver,
}

impl CallbackData {
    pub fn as_str(&self) -> &'static str {
        match self {
            CallbackData::Refresh => "refresh",
            CallbackData::DetailedStats => "detailed_stats",
            CallbackData::Config => "config",
            CallbackData::Help => "help",
            CallbackData::StartSolver => "start_solver",
            CallbackData::StopSolver => "stop_solver",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "refresh" => Some(CallbackData::Refresh),
            "detailed_stats" => Some(CallbackData::DetailedStats),
            "config" => Some(CallbackData::Config),
            "help" => Some(CallbackData::Help),
            "start_solver" => Some(CallbackData::StartSolver),
            "stop_solver" => Some(CallbackData::StopSolver),
            _ => None,
        }
    }
}

/// Interactive Telegram bot handler
pub struct InteractiveTelegramBot {
    pub bot: Bot,
    pub state: Arc<RwLock<BotState>>,
}

impl InteractiveTelegramBot {
    pub fn new(token: String, state: Arc<RwLock<BotState>>) -> Self {
        let bot = Bot::new(token);
        Self { bot, state }
    }

    /// Start the bot and handle commands
    pub async fn run(&self) -> Result<()> {
        log::info!("Starting interactive Telegram bot...");

        let handler = Update::filter_message()
            .branch(
                dptree::entry()
                    .filter_command::<Command>()
                    .endpoint(Self::handle_command),
            )
            .branch(dptree::endpoint(Self::handle_message));

        let callback_handler = Update::filter_callback_query().endpoint(Self::handle_callback);

        Dispatcher::builder(
            self.bot.clone(),
            dptree::entry().branch(handler).branch(callback_handler),
        )
        .dependencies(dptree::deps![self.state.clone()])
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;

        Ok(())
    }

    /// Handle bot commands
    async fn handle_command(
        bot: Bot,
        msg: Message,
        cmd: Command,
        state: Arc<RwLock<BotState>>,
    ) -> ResponseResult<()> {
        match cmd {
            Command::Help => {
                bot.send_message(msg.chat.id, Self::get_help_text())
                    .parse_mode(ParseMode::MarkdownV2)
                    .reply_markup(Self::get_main_keyboard())
                    .await?;
            }
            Command::Status => {
                let status_text = Self::get_status_text(&state).await;
                bot.send_message(msg.chat.id, status_text)
                    .parse_mode(ParseMode::MarkdownV2)
                    .reply_markup(Self::get_main_keyboard())
                    .await?;
            }
            Command::Stats => {
                let stats_text = Self::get_detailed_stats(&state).await;
                bot.send_message(msg.chat.id, stats_text)
                    .parse_mode(ParseMode::MarkdownV2)
                    .reply_markup(Self::get_main_keyboard())
                    .await?;
            }
            Command::Config => {
                let config_text = Self::get_config_text(&state).await;
                bot.send_message(msg.chat.id, config_text)
                    .parse_mode(ParseMode::MarkdownV2)
                    .reply_markup(Self::get_main_keyboard())
                    .await?;
            }
            Command::Start => {
                {
                    let mut state_guard = state.write().await;
                    state_guard.is_running = true;
                }
                bot.send_message(msg.chat.id, "🚀 *Puzzle solver started*")
                    .parse_mode(ParseMode::MarkdownV2)
                    .reply_markup(Self::get_main_keyboard())
                    .await?;
            }
            Command::Stop => {
                {
                    let mut state_guard = state.write().await;
                    state_guard.is_running = false;
                }
                bot.send_message(msg.chat.id, "⏹️ *Puzzle solver stopped*")
                    .parse_mode(ParseMode::MarkdownV2)
                    .reply_markup(Self::get_main_keyboard())
                    .await?;
            }
        }
        Ok(())
    }

    /// Handle callback queries from inline keyboards
    async fn handle_callback(
        bot: Bot,
        q: CallbackQuery,
        state: Arc<RwLock<BotState>>,
    ) -> ResponseResult<()> {
        if let Some(data) = &q.data {
            if let Some(callback_data) = CallbackData::from_str(data) {
                let response_text = match callback_data {
                    CallbackData::Refresh => Self::get_status_text(&state).await,
                    CallbackData::DetailedStats => Self::get_detailed_stats(&state).await,
                    CallbackData::Config => Self::get_config_text(&state).await,
                    CallbackData::Help => Self::get_help_text(),
                    CallbackData::StartSolver => {
                        {
                            let mut state_guard = state.write().await;
                            state_guard.is_running = true;
                        }
                        "🚀 *Puzzle solver started*".to_string()
                    }
                    CallbackData::StopSolver => {
                        {
                            let mut state_guard = state.write().await;
                            state_guard.is_running = false;
                        }
                        "⏹️ *Puzzle solver stopped*".to_string()
                    }
                };

                if let Some(Message { id, chat, .. }) = q.message {
                    bot.edit_message_text(chat.id, id, response_text)
                        .parse_mode(ParseMode::MarkdownV2)
                        .reply_markup(Self::get_main_keyboard())
                        .await?;
                }
            }
        }

        bot.answer_callback_query(q.id).await?;
        Ok(())
    }

    /// Handle regular messages
    async fn handle_message(bot: Bot, msg: Message) -> ResponseResult<()> {
        bot.send_message(
            msg.chat.id,
            "👋 Welcome to BTC Lotto Puzzles Bot\n\nUse /help to see available commands or click the buttons below:",
        )
        .parse_mode(ParseMode::MarkdownV2)
        .reply_markup(Self::get_main_keyboard())
        .await?;
        Ok(())
    }

    /// Get main keyboard with buttons
    fn get_main_keyboard() -> InlineKeyboardMarkup {
        let mut keyboard = Vec::new();

        // First row: Status and Stats
        keyboard.push(vec![
            InlineKeyboardButton::callback("📊 Status", CallbackData::Refresh.as_str()),
            InlineKeyboardButton::callback(
                "📈 Detailed Stats",
                CallbackData::DetailedStats.as_str(),
            ),
        ]);

        // Second row: Config and Help
        keyboard.push(vec![
            InlineKeyboardButton::callback("⚙️ Config", CallbackData::Config.as_str()),
            InlineKeyboardButton::callback("❓ Help", CallbackData::Help.as_str()),
        ]);

        // Third row: Start/Stop controls
        keyboard.push(vec![
            InlineKeyboardButton::callback("🚀 Start", CallbackData::StartSolver.as_str()),
            InlineKeyboardButton::callback("⏹️ Stop", CallbackData::StopSolver.as_str()),
        ]);

        InlineKeyboardMarkup::new(keyboard)
    }

    /// Get status text
    async fn get_status_text(state: &Arc<RwLock<BotState>>) -> String {
        let state_guard = state.read().await;
        let status_icon = if state_guard.is_running {
            "🟢"
        } else {
            "🔴"
        };
        let current_puzzle = state_guard
            .current_puzzle
            .map(|p| format!("#{}", p))
            .unwrap_or_else(|| "None".to_string());

        format!(
            "📊 *BTC Lotto Puzzles Bot Status*\n\n\
            *Status:* {} {}\n\
            *Total Keys Checked:* `{}`\n\
            *Matches Found:* `{}`\n\
            *Current Puzzle:* `{}`\n\
            *Uptime:* `{:.2}` hours\n\
            *Rate:* `{:.0}` keys/hour\n\
            *Total Puzzles Loaded:* `{}`\n\n\
            _Last updated: {}_",
            status_icon,
            if state_guard.is_running {
                "Running"
            } else {
                "Stopped"
            },
            state_guard.check_stats.total_checked,
            state_guard.check_stats.matches_found,
            current_puzzle,
            state_guard.get_uptime_hours(),
            state_guard.get_keys_per_hour(),
            state_guard.total_puzzles,
            Utc::now().format("%Y/%m/%d %H:%M:%S UTC")
        )
    }

    /// Get detailed statistics text
    async fn get_detailed_stats(state: &Arc<RwLock<BotState>>) -> String {
        let state_guard = state.read().await;
        let match_rate = if state_guard.check_stats.total_checked > 0 {
            state_guard.check_stats.matches_found as f64
                / state_guard.check_stats.total_checked as f64
                * 100.0
        } else {
            0.0
        };

        format!(
            "📈 *Detailed Statistics*\n\n\
            *Performance Metrics:*\n\
            • Total Keys Generated: `{}`\n\
            • Keys per Hour: `{:.0}`\n\
            • Keys per Minute: `{:.1}`\n\
            • Average per Check: `{:.2}ms`\n\n\
            *Success Metrics:*\n\
            • Total Matches: `{}`\n\
            • Success Rate: `{:.8}%`\n\n\
            *Runtime Info:*\n\
            • Started: `{}`\n\
            • Uptime: `{:.2}` hours\n\
            • Current Status: `{}`\n\n\
            _Statistics updated: {}_",
            state_guard.check_stats.total_checked,
            state_guard.get_keys_per_hour(),
            state_guard.get_keys_per_hour() / 60.0,
            if state_guard.get_keys_per_hour() > 0.0 {
                3600000.0 / state_guard.get_keys_per_hour()
            } else {
                0.0
            },
            state_guard.check_stats.matches_found,
            match_rate,
            state_guard.start_time.format("%Y/%m/%d %H:%M:%S UTC"),
            state_guard.get_uptime_hours(),
            if state_guard.is_running {
                "Running"
            } else {
                "Stopped"
            },
            Utc::now().format("%Y/%m/%d %H:%M:%S UTC")
        )
    }

    /// Get configuration text
    async fn get_config_text(state: &Arc<RwLock<BotState>>) -> String {
        let state_guard = state.read().await;
        let min_bits = state_guard
            .config
            .min_bits
            .map(|b| b.to_string())
            .unwrap_or_else(|| "None".to_string());
        let max_bits = state_guard
            .config
            .max_bits
            .map(|b| b.to_string())
            .unwrap_or_else(|| "None".to_string());
        let min_reward = state_guard
            .config
            .min_reward_btc
            .map(|r| format!("{:.1}", r))
            .unwrap_or_else(|| "None".to_string());

        format!(
            "⚙️ *Bot Configuration*\n\n\
            *Performance Settings:*\n\
            • Threads: `{}`\n\
            • Run Duration: `{}` seconds\n\
            • Check Interval: `{}` seconds\n\
            • Stats Update Interval: `{:.1}` hours\n\n\
            *Puzzle Filters:*\n\
            • Minimum Bits: `{}`\n\
            • Maximum Bits: `{}`\n\
            • Minimum Reward: `{}` BTC\n\n\
            *Features:*\n\
            • Stats Updates: `{}`\n\
            • Total Puzzles Available: `{}`\n\n\
            _Configuration loaded at startup_",
            state_guard.config.threads,
            state_guard.config.run_duration_seconds,
            state_guard.config.check_interval_seconds,
            state_guard.config.stats_update_interval_hours,
            min_bits,
            max_bits,
            min_reward,
            if state_guard.config.send_stats_updates {
                "Enabled"
            } else {
                "Disabled"
            },
            state_guard.total_puzzles
        )
    }

    /// Get help text
    fn get_help_text() -> String {
        "❓ *BTC Lotto Puzzles Bot Help*\n\n\
        *Available Commands:*\n\
        • `/help` Show this help message\n\
        • `/status` Show current bot status\n\
        • `/stats` Show detailed statistics\n\
        • `/config` Show bot configuration\n\
        • `/start` Start the puzzle solver\n\
        • `/stop` Stop the puzzle solver\n\n\
        *Interactive Buttons:*\n\
        • 📊 *Status* Quick status overview\n\
        • 📈 *Detailed Stats* Comprehensive statistics\n\
        • ⚙️ *Config* View current configuration\n\
        • 🚀 *Start* Begin puzzle solving\n\
        • ⏹️ *Stop* Pause puzzle solving\n\n\
        *About the Bot:*\n\
        This bot randomly generates private keys within Bitcoin puzzle ranges and checks if they match target addresses\\. When a match is found, you'll receive an immediate notification with the private key and reward information\\.\n\n\
        *Security Note:* 🔒\n\
        Private keys are sensitive information\\. This bot is for educational purposes and legitimate cryptographic research only\\.\n\n\
        _Use the buttons below for quick access to bot functions\\._".to_string()
    }
}

/// Update bot state from scheduler
pub async fn update_bot_state(
    state: &Arc<RwLock<BotState>>,
    check_stats: &crate::checker::CheckStats,
    current_puzzle: Option<u32>,
) {
    let mut state_guard = state.write().await;
    state_guard.check_stats = check_stats.clone();
    state_guard.current_puzzle = current_puzzle;
}
