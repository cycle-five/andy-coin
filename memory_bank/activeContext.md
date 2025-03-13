# Active Development Context

## Current Focus

- Enhancing logging and auditing capabilities
- Improving error handling and user feedback
- Creating web version of leaderboard

## Recent Changes

- Implemented the `/vote` command for server reset of AndyCoins
- Added comprehensive logging system with structured JSON output
- Implemented audit tool for tracking balance changes and command usage
- Added coin flipping functionality with gambling feature
- Implemented role-based permissions for giving AndyCoins

## Known Issues

- Need to improve data persistence with more robust storage solution
- No web interface for leaderboard yet

## Next Steps

1. Create web version of leaderboard
2. Improve data persistence mechanism
3. Add more comprehensive error handling
4. Enhance testing coverage
5. Add periodic check for expired votes

## Development Environment

- Rust 1.85+
- Discord API via Serenity and Poise libraries
- YAML-based data storage
- Tracing for structured logging
