# Technical Context

## Technology Stack

### Core Technologies

- **Rust** (v1.85+) - Primary programming language
- **Serenity** - Discord API wrapper for Rust
- **Poise** - Command framework for Serenity
- **Tokio** - Asynchronous runtime
- **YAML** - Data storage format

### Key Libraries

- **DashMap** - Thread-safe concurrent hash map
- **Serde** - Serialization/deserialization framework
- **Tracing** - Structured logging
- **Rand** - Random number generation
- **Chrono** - Date and time handling for vote timing

## Development Environment

- **Cargo** - Rust package manager and build tool
- **GitHub** - Version control
- **VSCode** - Recommended IDE with rust-analyzer extension

## Discord Bot Setup

### Bot Permissions

- Read Messages/View Channels
- Send Messages
- Use Slash Commands
- Read Message History

### Environment Variables

- `DISCORD_TOKEN` - Discord bot token

## Data Storage

### File Structure

- `andy_coin_data.yaml` - Main data file for balances and configurations

### Data Format

```yaml
balances:
  - guild_id: u64
    user_id: u64
    balance: u32
configs:
  - guild_id: u64
    giver_role_id: Option<u64>
    vote_config:
      cooldown_hours: u32
      duration_minutes: u32
      min_votes: u32
      majority_percentage: u32
    vote_status:
      active: bool
      start_time: Option<DateTime<Utc>>
      end_time: Option<DateTime<Utc>>
      initiator_id: Option<u64>
      yes_votes: Vec<u64>
      no_votes: Vec<u64>
      last_vote_time: Option<DateTime<Utc>>
```

## Logging System

### Log Files

- `logs/commands-YYYY-MM-DD.log` - Command execution logs
- `logs/balances-YYYY-MM-DD.log` - Balance change logs

### Log Format

- JSON structured logs
- Daily rotation
- Configurable log level via `RUST_LOG` environment variable

## Command Structure

All commands use Discord's slash command interface:

- `/give <user> <amount>` - Give AndyCoins to a user
- `/balance [user] [global]` - Check balance for self or another user
- `/leaderboard [global]` - View server or global leaderboard
- `/config role <role>` - Set the role that can give AndyCoins
- `/flip [guess] [bet]` - Flip a coin, optionally with a bet flag
- `/vote <decision>` - Start a vote or cast a vote (yes/no) on resetting AndyCoins
- `/vote_admin status` - Check the status of the current vote
- `/vote_admin config [cooldown_hours] [duration_minutes] [min_votes] [majority_percentage]` - Configure vote settings
- `/vote_admin end` - Force end the current vote (admin only)

## Concurrency Handling

- Thread-safe data structures with DashMap
- Async/await for non-blocking operations
- Tokio for async runtime

## Error Handling

- Custom error type with `Box<dyn Error + Send + Sync>`
- Structured error logging
- User-friendly error messages

## Testing

- Unit tests for core functionality
- Integration tests for command behavior
- Test utilities for mocking Discord context

## Deployment

- Manual deployment process
- Environment variables for configuration
- Logging for monitoring and debugging

## Performance Considerations

- In-memory data structure with periodic persistence
- Efficient concurrent access with DashMap
- Minimal external dependencies
