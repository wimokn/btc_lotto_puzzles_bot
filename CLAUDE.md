# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This repository contains Bitcoin lottery puzzle data and appears to be designed for analyzing or solving Bitcoin puzzle challenges. The main data structure is in `unsolved_puzzles.json`, which contains:

- Bitcoin puzzle numbers (ranging from 14 to 160)
- Bit lengths for each puzzle
- Private key search ranges (hex format)
- Target Bitcoin addresses
- Current BTC rewards for each puzzle

## Data Structure

The `unsolved_puzzles.json` file contains an array of puzzle objects with the following structure:
- `puzzle`: Puzzle number
- `bits`: Number of bits in the private key
- `range_start`: Lower bound of private key search space (hex)
- `range_end`: Upper bound of private key search space (hex)  
- `address`: Target Bitcoin address to match
- `reward_btc`: Current BTC reward amount

## Security Notice

This project involves Bitcoin cryptographic puzzles. When working with this codebase:
- Never commit private keys or wallet information
- Be cautious when handling cryptographic operations
- Ensure any puzzle-solving code is for legitimate educational/research purposes only
- NEVER read or process .env files
- STOP immediately if you encounter API keys or passwords
- Do not access any file containing credentials
- Respect all .claudeignore entries without exception

## Build and Run Commands

### Development
```bash
# Build the project
cargo build

# Run with debug logging
RUST_LOG=debug cargo run

# Run tests
cargo test

# Check code without building
cargo check
```

### Production
```bash
# Build optimized release version
cargo build --release

# Run the release binary
./target/release/btc_lotto_puzzles_bot
```

## Configuration

The application uses environment variables for configuration. Copy `.env.example` to `.env` and configure:

- `TELOXIDE_TOKEN`: Your Telegram bot token
- `CHAT_ID`: Telegram chat ID for notifications
- `CHECK_INTERVAL_SECONDS`: How often to check (default: 60)
- `MIN_BITS`/`MAX_BITS`: Puzzle difficulty range to target
- `MIN_REWARD_BTC`: Minimum reward threshold

## Architecture

The application consists of several modules:

- `puzzles.rs`: JSON data loading and puzzle structures
- `keygen.rs`: Random private key generation within hex ranges
- `checker.rs`: Bitcoin address derivation and matching logic
- `telegram.rs`: Notification system for successful finds
- `scheduler.rs`: Main loop orchestrating the puzzle solving process
- `main.rs`: Application entry point and configuration

## Key Dependencies

- `bitcoin`: Bitcoin cryptography and address generation
- `secp256k1`: Elliptic curve operations for key generation
- `tokio`: Async runtime for concurrent operations
- `reqwest`: HTTP client for Telegram API calls
- `serde_json`: JSON parsing for puzzle data
- `num-bigint`: Large integer arithmetic for key ranges