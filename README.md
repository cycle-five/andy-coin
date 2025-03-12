# AndyCoin Bot

AndyCoin Bot does one thing, let you give people in your server AndyCoin. It also does its best to cause chaos, indirectly.

## AndyCoin Bot Logging and Auditing

This document describes the logging and auditing system implemented for the AndyCoin Discord bot.

## Logging System

The AndyCoin bot uses the `tracing` ecosystem for structured logging. This provides:

- Detailed logs of all commands executed
- Audit trail of all balance changes
- Structured JSON output for easy parsing
- Daily log rotation

### Log Files

Logs are stored in the `logs` directory with the following files:

- `commands-YYYY-MM-DD.log` - Records all commands executed by users
- `balances-YYYY-MM-DD.log` - Records all balance changes

### Log Format

Logs are stored in JSON format with the following structure:

#### Command Logs

```json
{
  "timestamp": "2025-03-11T18:30:45.123Z",
  "level": "INFO",
  "target": "command",
  "fields": {
    "command": "give",
    "guild_id": "123456789012345678",
    "user_id": "987654321098765432",
    "arguments": "amount: 50, user: SomeUser#1234",
    "result": "success",
    "message": "Command executed successfully"
  }
}
```

#### Balance Logs

```json
{
  "timestamp": "2025-03-11T18:30:45.123Z",
  "level": "INFO",
  "target": "balance",
  "fields": {
    "guild_id": "123456789012345678",
    "user_id": "987654321098765432",
    "previous_balance": 100,
    "new_balance": 150,
    "change": 50,
    "reason": "give_command",
    "initiator": "123456789012345678",
    "message": "Balance changed"
  }
}
```

## Audit Tool

The bot includes an audit tool to help analyze logs and track balance changes. The tool is available as a binary in `src/bin/audit.rs`.

### Building the Audit Tool

```bash
cargo build --bin audit
```

### Using the Audit Tool

```bash
# List all commands executed by a specific user
cargo run --bin audit user-commands 123456789012345678

# List all balance changes for a specific user
cargo run --bin audit user-balances 123456789012345678

# Show a summary of all balance changes
cargo run --bin audit balance-summary

# Show help
cargo run --bin audit help
```

### Audit Tool Output Examples

#### User Commands

```txt
Commands executed by user 123456789012345678:
Timestamp               Command        Guild           Arguments                                 Result    
----------------------------------------------------------------------------------------------------
2025-03-11T18:30:45.123Z give           123456789012345 amount: 50, user: SomeUser#1234          success   
2025-03-11T18:35:12.456Z balance        123456789012345 user: self, global: false                success   
```

#### User Balances

```txt
Balance changes for user 987654321098765432:
Timestamp               Guild           Previous        New             Change    Reason               Initiator      
--------------------------------------------------------------------------------------------------------------
2025-03-11T18:30:45.123Z 123456789012345 100             150             +50       give_command         123456789012345
2025-03-11T19:15:22.789Z 123456789012345 150             149             -1        flip_bet             987654321098765
```

#### Balance Summary

```txt
Balance Change Summary:

Guild: 123456789012345678
User ID                Net Change     
-----------------------------------
987654321098765432     +49            
456789012345678901     +25            
234567890123456789     -10            

Global Summary:
User ID                Net Change     
-----------------------------------
987654321098765432     +49            
456789012345678901     +25            
234567890123456789     -10            
```

## Environment Variables

The logging system respects the following environment variables:

- `RUST_LOG` - Controls the log level (e.g., `info`, `debug`, `trace`)
