# System Patterns

## Architecture

AndyCoin follows a modular architecture with clear separation of concerns:

```txt
AndyCoin Bot
├── Main (src/main.rs)
│   └── Bot initialization and configuration
├── Commands (src/commands/)
│   ├── Balance - Query user balances
│   ├── Give - Transfer AndyCoins to users
│   ├── Leaderboard - Display top users
│   ├── Config - Server configuration
│   └── Flip - Coin flipping and gambling
├── Data Management (src/data.rs)
│   └── YAML-based persistence
├── Logging (src/logging.rs)
│   └── Structured JSON logging
└── Audit Tool (src/bin/audit.rs)
    └── Log analysis and reporting
```

## Design Patterns

### Repository Pattern

- `Data` struct acts as a repository for user balances and guild configurations
- Provides methods for data access and manipulation
- Handles persistence to YAML files

### Command Pattern

- Each Discord command is implemented as a separate module
- Commands are registered with the Poise framework
- Consistent interface for all commands

### Observer Pattern

- Logging system observes and records all balance changes and command executions
- Audit tool can analyze these logs without affecting the main application

### Singleton Pattern

- Single instance of `Data` shared across the application
- Manages access to the shared state

## Data Flow

1. User issues a command in Discord
2. Poise framework routes the command to the appropriate handler
3. Command handler validates input and permissions
4. Data is retrieved or modified through the `Data` struct
5. Changes are logged via the logging system
6. Results are returned to the user
7. Data is periodically persisted to YAML files

## Error Handling

- Commands return `Result<(), Error>` to propagate errors
- Errors are logged with context
- User-friendly error messages are displayed in Discord

## Concurrency Model

- Uses Tokio for async runtime
- DashMap for thread-safe concurrent access to data
- Avoids locks for better performance

## Testing Strategy

- Unit tests for core functionality
- Integration tests for command behavior
- Mock objects for external dependencies

## Deployment

- Manual deployment process
- Environment variables for configuration
- Logging for monitoring and debugging
