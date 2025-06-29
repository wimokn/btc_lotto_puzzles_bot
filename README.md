# Bitcoin Lottery Puzzles Bot

A Rust application that automatically searches for solutions to Bitcoin cryptographic puzzles by generating random private keys within known puzzle ranges and checking if they match target addresses.

## Features

- 🔍 **Automated Search**: Periodically generates random private keys within puzzle ranges
- 📊 **Optimized Checking**: Efficiently checks compressed Bitcoin addresses (standard format)
- 📱 **Telegram Notifications**: Instant alerts when puzzles are solved
- 🤖 **Interactive Bot**: Control and monitor via Telegram buttons and commands
- ⚙️ **Configurable**: Customizable intervals, puzzle filters, and search parameters
- 🔒 **Secure**: Follows security best practices for private key handling
- 📈 **Statistics**: Real-time tracking of search progress and performance metrics
- 💾 **Persistence**: Logs successful finds to disk
- ⏯️ **Remote Control**: Start/stop puzzle solving via Telegram commands

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
| `ENABLE_INTERACTIVE_BOT` | `true` | Enable interactive Telegram bot with buttons |
| `RUN_DURATION_SECONDS` | `600` | How long each solving session runs |
| `CHECK_INTERVAL_SECONDS` | `60` | Pause between solving sessions |
| `THREADS` | `8` | Number of parallel worker threads |
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
2. **Interactive Control**: Send commands via Telegram to start/stop the solver
3. **Session-Based Processing**: Runs solving sessions for configured duration (default: 10 minutes)
4. **Multi-Threading**: Uses parallel worker threads for maximum performance (default: 8 threads)
5. **Random Selection**: Each thread picks random eligible puzzles based on your filters
6. **Key Generation**: Generates random private keys within puzzle ranges
7. **Address Derivation**: Creates compressed Bitcoin addresses (standard format)
8. **Matching**: Compares generated addresses with target addresses
9. **Notification**: Sends Telegram alert if a match is found
10. **Real-time Monitoring**: Check status and statistics via Telegram buttons
11. **Session Rest**: Pauses between sessions (default: 1 minute) to prevent overheating
12. **Logging**: Records all successful finds to `puzzle_solutions.log`

## Performance Architecture

### Multi-Threaded Design
- **Parallel Processing**: Multiple worker threads run simultaneously
- **Session-Based**: Configurable duration sessions with rest periods
- **Efficient Resource Use**: Optimized for both CPU cores and memory usage
- **Scalable**: Adjust thread count based on your hardware capabilities

### Example Performance (8 threads, 600-second sessions):
- **Small Puzzles (14-20 bits)**: ~50,000-100,000 keys/session
- **Medium Puzzles (21-40 bits)**: ~30,000-80,000 keys/session  
- **Large Puzzles (40+ bits)**: ~20,000-60,000 keys/session

*Performance varies based on CPU, puzzle complexity, and system resources*

## Interactive Telegram Bot Commands

The bot supports both traditional commands and interactive buttons:

### Commands
- `/help` - Show help message and available commands
- `/status` - Display current bot status and statistics
- `/stats` - Show detailed performance metrics
- `/config` - View current configuration settings
- `/start` - Start the puzzle solver
- `/stop` - Stop the puzzle solver

### Command Examples

#### `/status` Command Response:
```
📊 BTC Lotto Puzzles Bot Status

Status: 🟢 Running
Total Keys Checked: 1,247
Matches Found: 0
Current Puzzle: #71
Uptime: 2.45 hours
Rate: 509 keys/hour
Total Puzzles Loaded: 47

Last updated: 2024-06-29 15:30:45 UTC
```

#### `/stats` Command Response:
```
📈 Detailed Statistics

Performance Metrics:
• Total Keys Generated: 1,247
• Keys per Hour: 509
• Keys per Minute: 8.5
• Average per Check: 7.08ms

Success Metrics:
• Total Matches: 0
• Success Rate: 0.00000000%

Runtime Info:
• Started: 2024-06-29 13:05:30 UTC
• Uptime: 2.45 hours
• Current Status: Running

Statistics updated: 2024-06-29 15:30:45 UTC
```

#### `/start` Command Response:
```
🚀 Puzzle solver started!
```

#### `/stop` Command Response:
```
⏹️ Puzzle solver stopped!
```

#### `/config` Command Response:
```
⚙️ Bot Configuration

Performance Settings:
• Threads: 8
• Run Duration: 600 seconds
• Check Interval: 60 seconds
• Stats Update Interval: 24.0 hours

Puzzle Filters:
• Minimum Bits: 14
• Maximum Bits: 160
• Minimum Reward: 0.0 BTC

Features:
• Stats Updates: Enabled
• Total Puzzles Available: 47

Configuration loaded at startup
```

#### `/help` Command Response:
```
❓ BTC Lotto Puzzles Bot Help

Available Commands:
• /help - Show this help message
• /status - Show current bot status
• /stats - Show detailed statistics
• /config - Show bot configuration
• /start - Start the puzzle solver
• /stop - Stop the puzzle solver

Interactive Buttons:
• 📊 Status - Quick status overview
• 📈 Detailed Stats - Comprehensive statistics
• ⚙️ Config - View current configuration
• 🚀 Start - Begin puzzle solving
• ⏹️ Stop - Pause puzzle solving

About the Bot:
This bot randomly generates private keys within Bitcoin puzzle ranges and checks if they match target addresses. When a match is found, you'll receive an immediate notification with the private key and reward information.

Security Note: 🔒
Private keys are sensitive information. This bot is for educational purposes and legitimate cryptographic research only.

Use the buttons below for quick access to bot functions.
```

### Interactive Buttons Interface

The bot displays an interactive keyboard with buttons for easy access:

```
┌─────────────────────────────────────┐
│  📊 Status     │  📈 Detailed Stats │
├─────────────────────────────────────┤
│  ⚙️ Config     │      ❓ Help       │
├─────────────────────────────────────┤
│   🚀 Start     │      ⏹️ Stop       │
└─────────────────────────────────────┘
```

**Button Functions:**
- 📊 **Status** - Quick overview of current state
- 📈 **Detailed Stats** - Comprehensive performance metrics
- ⚙️ **Config** - View configuration settings
- ❓ **Help** - Show help and command information
- 🚀 **Start** - Begin puzzle solving
- ⏹️ **Stop** - Pause puzzle solving

**Button Behavior:**
- Clicking any button updates the message with new information
- Buttons work instantly without typing commands
- All information is refreshed in real-time
- Start/Stop buttons provide immediate control over the solver

### Status Information
The bot provides real-time information including:
- **Total Keys Checked**: Number of random private keys generated and tested
- **Random Rounds**: Total iterations of the puzzle solving loop
- **Current Puzzle**: Which puzzle number is currently being processed
- **Keys per Hour**: Performance rate showing generation speed
- **Total Runtime**: How long the bot has been running
- **Matches Found**: Number of successful puzzle solutions
- **Start/Stop Status**: Current operational state

### Typical Usage Flow

1. **Start the Bot**: Send `/start` command or click 🚀 Start button
2. **Monitor Progress**: Click 📊 Status button to check current progress
3. **View Detailed Stats**: Click 📈 Detailed Stats for comprehensive metrics
4. **Check Configuration**: Click ⚙️ Config to verify settings
5. **Control Operation**: Use 🚀 Start / ⏹️ Stop buttons to control the solver
6. **Get Help**: Click ❓ Help if you need assistance

### Real-time Monitoring Example

```
User: /status
Bot: 📊 BTC Lotto Puzzles Bot Status
     Status: 🟢 Running
     Total Keys Checked: 3,456
     Current Puzzle: #14
     Rate: 1,152 keys/hour
     
User: [Clicks 📈 Detailed Stats button]
Bot: 📈 Detailed Statistics
     Performance Metrics:
     • Total Keys Generated: 3,456
     • Keys per Hour: 1,152
     • Keys per Minute: 19.2
     
User: [Clicks ⏹️ Stop button]
Bot: ⏹️ Puzzle solver stopped!
```

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