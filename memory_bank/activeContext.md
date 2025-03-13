# Active Development Context

## Current Focus

- Implementing the `/vote` command for server reset of AndyCoins
- Enhancing logging and auditing capabilities
- Improving error handling and user feedback

## Recent Changes

- Added comprehensive logging system with structured JSON output
- Implemented audit tool for tracking balance changes and command usage
- Added coin flipping functionality with gambling feature
- Implemented role-based permissions for giving AndyCoins

## Known Issues

- Need to improve data persistence with more robust storage solution
- Server reset voting mechanism not yet implemented
- No web interface for leaderboard yet

## Next Steps

1. Implement the `/vote` command for server reset
2. Create web version of leaderboard
3. Improve data persistence mechanism
4. Add more comprehensive error handling
5. Enhance testing coverage

## Development Environment

- Rust 1.85+
- Discord API via Serenity and Poise libraries
- YAML-based data storage
- Tracing for structured logging
