# Active Development Context

## Current Focus

- Creating web version of leaderboard
- Improving error handling and user feedback
- Enhancing testing coverage

## Recent Changes

- Implemented the `/vote` command for server reset of AndyCoins with custom choice parameters
- Added vote configuration options (cooldown, duration, min votes, majority percentage)
- Added vote status checking and administrative controls
- Added comprehensive logging system with structured JSON output
- Implemented audit tool for tracking balance changes and command usage
- Added coin flipping functionality with gambling feature
- Implemented role-based permissions for giving AndyCoins

## Known Issues

- Need to improve data persistence with more robust storage solution
- No web interface for leaderboard yet
- Need to implement periodic check for expired votes

## Next Steps

1. Create web version of leaderboard
2. Improve data persistence mechanism (consider using a proper database)
3. Add more comprehensive error handling
4. Enhance testing coverage
5. Implement periodic check for expired votes

## Development Environment

- Rust 1.85+
- Discord API via Serenity and Poise libraries
- YAML-based data storage
- Tracing for structured logging
- Chrono for date/time handling
