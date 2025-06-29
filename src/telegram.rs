use anyhow::{Context, Result};
use reqwest::Client;
use serde_json::json;
use std::env;
use crate::checker::CheckResult;
use crate::puzzles::Puzzle;

/// Telegram notification client
#[derive(Debug, Clone)]
pub struct TelegramNotifier {
    bot_token: String,
    chat_id: String,
    client: Client,
}

impl TelegramNotifier {
    /// Create new Telegram notifier from environment variables
    pub fn new() -> Result<Self> {
        let bot_token = env::var("TELOXIDE_TOKEN")
            .context("TELOXIDE_TOKEN environment variable not set")?;
        
        let chat_id = env::var("CHAT_ID")
            .context("CHAT_ID environment variable not set")?;
        
        let client = Client::new();
        
        log::info!("Telegram notifier initialized for chat ID: {}", chat_id);
        
        Ok(Self {
            bot_token,
            chat_id,
            client,
        })
    }
    
    /// Create new Telegram notifier with explicit credentials
    pub fn with_credentials(bot_token: String, chat_id: String) -> Self {
        let client = Client::new();
        
        Self {
            bot_token,
            chat_id,
            client,
        }
    }
    
    /// Send a success notification when a puzzle is solved
    pub async fn notify_puzzle_solved(
        &self,
        result: &CheckResult,
        puzzle: &Puzzle,
    ) -> Result<()> {
        let message = self.format_success_message(result, puzzle);
        self.send_message(&message).await
    }
    
    /// Send a general status update
    pub async fn notify_status(&self, message: &str) -> Result<()> {
        let formatted_message = format!("🤖 **BTC Lotto Bot Status**\n\n{}", message);
        self.send_message(&formatted_message).await
    }
    
    /// Send an error notification
    pub async fn notify_error(&self, error: &str) -> Result<()> {
        let message = format!("❌ **BTC Lotto Bot Error**\n\n```\n{}\n```", error);
        self.send_message(&message).await
    }
    
    /// Send bot startup notification
    pub async fn notify_startup(&self, puzzles_count: usize) -> Result<()> {
        let message = format!(
            "🚀 **BTC Lotto Bot Started**\n\n\
            • Loaded {} puzzles\n\
            • Ready to search for private keys\n\
            • Will notify on any matches found\n\n\
            Good luck! 🍀",
            puzzles_count
        );
        self.send_message(&message).await
    }
    
    /// Send statistics update
    pub async fn notify_stats(
        &self,
        total_checked: u64,
        current_puzzle: Option<u32>,
        uptime_hours: f64,
    ) -> Result<()> {
        let puzzle_info = current_puzzle
            .map(|p| format!("Puzzle #{}", p))
            .unwrap_or_else(|| "None".to_string());
        
        let rate = if uptime_hours > 0.0 {
            total_checked as f64 / uptime_hours
        } else {
            0.0
        };
        
        let message = format!(
            "📊 **BTC Lotto Bot Statistics**\n\n\
            • Total keys checked: {}\n\
            • Current puzzle: {}\n\
            • Uptime: {:.2} hours\n\
            • Rate: {:.0} keys/hour\n\n\
            Still searching... 🔍",
            total_checked,
            puzzle_info,
            uptime_hours,
            rate
        );
        
        self.send_message(&message).await
    }
    
    /// Format success message when puzzle is solved
    fn format_success_message(&self, result: &CheckResult, puzzle: &Puzzle) -> String {
        format!(
            "🎉🎉🎉 **BITCOIN PUZZLE SOLVED!** 🎉🎉🎉\n\n\
            **Puzzle:** #{}\n\
            **Bits:** {}\n\
            **Reward:** {} BTC\n\n\
            **Target Address:**\n`{}`\n\n\
            **Private Key (HEX):**\n`{}`\n\n\
            **Generated Address:**\n`{}`\n\n\
            🚨 **IMPORTANT:** Secure this private key immediately! 🚨\n\n\
            💰 **Estimated Value:** ${:.2} USD (at current BTC price)",
            result.puzzle_number,
            puzzle.bits,
            puzzle.reward_btc,
            result.target_address,
            result.private_key_hex,
            result.address,
            puzzle.reward_btc * 50000.0 // Rough BTC price estimate
        )
    }
    
    /// Send a message to Telegram
    async fn send_message(&self, text: &str) -> Result<()> {
        let url = format!(
            "https://api.telegram.org/bot{}/sendMessage",
            self.bot_token
        );
        
        let payload = json!({
            "chat_id": self.chat_id,
            "text": text,
            "parse_mode": "Markdown",
            "disable_web_page_preview": true
        });
        
        log::debug!("Sending Telegram message: {}", text);
        
        let response = self.client
            .post(&url)
            .json(&payload)
            .send()
            .await
            .context("Failed to send Telegram message")?;
        
        let status = response.status();
        if status.is_success() {
            log::info!("Telegram notification sent successfully");
            Ok(())
        } else {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            Err(anyhow::anyhow!(
                "Telegram API error: {} - {}", 
                status, 
                error_text
            ))
        }
    }
    
    /// Test the Telegram connection
    pub async fn test_connection(&self) -> Result<()> {
        self.notify_status("🧪 Testing Telegram connection...").await?;
        log::info!("Telegram connection test successful");
        Ok(())
    }
}

/// Telegram notification configuration
#[derive(Debug, Clone)]
pub struct TelegramConfig {
    pub enabled: bool,
    pub notify_on_startup: bool,
    pub notify_on_error: bool,
    pub stats_interval_hours: Option<f64>,
}

impl Default for TelegramConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            notify_on_startup: true,
            notify_on_error: true,
            stats_interval_hours: Some(24.0), // Send stats every 24 hours
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::checker::CheckResult;
    use secp256k1::SecretKey;
    
    #[test]
    fn test_format_success_message() {
        // Mock data
        let bot_token = "test_token".to_string();
        let chat_id = "test_chat".to_string();
        let notifier = TelegramNotifier::with_credentials(bot_token, chat_id);
        
        let private_key_bytes = hex::decode("0000000000000000000000000000000000000000000000000000000000000001").unwrap();
        let _private_key = SecretKey::from_slice(&private_key_bytes).unwrap();
        
        let result = CheckResult {
            puzzle_number: 14,
            private_key_hex: "0000000000000000000000000000000000000000000000000000000000000001".to_string(),
            address: "1BgGZ9tcN4rm9KBzDn7KprQz87SZ26SAMH".to_string(),
            target_address: "1BgGZ9tcN4rm9KBzDn7KprQz87SZ26SAMH".to_string(),
            is_match: true,
        };
        
        let puzzle = Puzzle {
            puzzle: 14,
            bits: 14,
            range_start: "0x2000".to_string(),
            range_end: "0x3fff".to_string(),
            address: "1BgGZ9tcN4rm9KBzDn7KprQz87SZ26SAMH".to_string(),
            reward_btc: 0.0,
        };
        
        let message = notifier.format_success_message(&result, &puzzle);
        
        assert!(message.contains("BITCOIN PUZZLE SOLVED"));
        assert!(message.contains("Puzzle:** #14"));
        assert!(message.contains("Bits:** 14"));
        assert!(message.contains(&result.private_key_hex));
        assert!(message.contains(&result.target_address));
    }
}