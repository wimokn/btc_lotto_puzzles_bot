# Bitcoin Lottery Puzzles Bot

A Rust application that automatically searches for solutions to Bitcoin cryptographic puzzles by generating random private keys within known puzzle ranges and checking if they match target addresses.

## Features

- 🔍 **Automated Search**: Periodically generates random private keys within puzzle ranges
- 📊 **Multi-Format Support**: Checks both compressed and uncompressed Bitcoin addresses
- 📱 **Telegram Notifications**: Instant alerts when puzzles are solved
- ⚙️ **Configurable**: Customizable intervals, puzzle filters, and search parameters
- 🔒 **Secure**: Follows security best practices for private key handling
- 📈 **Statistics**: Tracks search progress and performance metrics
- 💾 **Persistence**: Logs successful finds to disk

## Quick Start

### 1. Setup Telegram Bot

1. Create a Telegram bot via [@BotFather](https://t.me/botfather)
2. Get your bot token
3. Get your chat ID (message your bot and visit `https://api.telegram.org/bot<YOUR_BOT_TOKEN>/getUpdates`)

### 2. Configure Environment

```bash
# Copy the example environment file
cp .env.example .env

# Edit .env with your credentials
TELOXIDE_TOKEN=your_telegram_bot_token_here
CHAT_ID=your_telegram_chat_id_here
```

### 3. Build and Run

```bash
# Build the project
cargo build --release

# Run the bot
./target/release/btc_lotto_puzzles_bot
```

## Configuration Options

Configure the bot behavior via environment variables in `.env`:

| Variable | Default | Description |
|----------|---------|-------------|
| `TELOXIDE_TOKEN` | - | Telegram bot token (required) |
| `CHAT_ID` | - | Telegram chat ID for notifications (required) |
| `CHECK_INTERVAL_SECONDS` | `60` | How often to generate and check keys |
| `MIN_BITS` | `14` | Minimum puzzle difficulty to attempt |
| `MAX_BITS` | `160` | Maximum puzzle difficulty to attempt |
| `MIN_REWARD_BTC` | `0.0` | Only attempt puzzles with this minimum reward |
| `SEND_STATS_UPDATES` | `true` | Send periodic statistics via Telegram |
| `STATS_UPDATE_INTERVAL_HOURS` | `24.0` | How often to send stats updates |

## Puzzle Data

The bot loads puzzle information from `unsolved_puzzles.json`. Each puzzle contains:

- Puzzle number and bit length
- Private key search range (hex format)
- Target Bitcoin address
- Current BTC reward amount

## How It Works

1. **Load Puzzles**: Reads unsolved puzzle data from JSON file
2. **Random Selection**: Picks a random eligible puzzle based on your filters
3. **Key Generation**: Generates a random private key within the puzzle's range
4. **Address Derivation**: Creates both compressed and uncompressed Bitcoin addresses
5. **Matching**: Compares generated addresses with the target address
6. **Notification**: Sends Telegram alert if a match is found
7. **Logging**: Records all successful finds to `puzzle_solutions.log`

## Expected Success Rate

Bitcoin puzzles are cryptographically secure by design. The probability of finding a solution depends on the puzzle's bit length:

- **14-bit puzzle**: ~16,000 possible keys
- **64-bit puzzle**: ~18 quintillion possible keys  
- **80-bit puzzle**: ~1.2 × 10²⁴ possible keys

This bot is primarily educational and demonstrates Bitcoin cryptography concepts. Real puzzle solutions typically require specialized hardware and significant computational resources.

## Security Considerations

- Private keys are generated in memory and not persisted unless a match is found
- Successful finds are logged locally to `puzzle_solutions.log`
- Telegram messages contain sensitive private key information
- Run in a secure environment and ensure proper access controls

## Development

```bash
# Run with debug logging
RUST_LOG=debug cargo run

# Run tests
cargo test

# Check code without building
cargo check
```

## Legal and Ethical Use

This software is provided for educational purposes and legitimate cryptographic research. Users are responsible for:

- Compliance with local laws and regulations
- Ethical use of any discovered private keys
- Proper security measures when handling cryptocurrency

## License

See [LICENSE](LICENSE) file for details.