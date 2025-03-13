# Product Context

## Product Vision

AndyCoin is a Discord bot that enables server owners to create a virtual economy within their servers. It's designed to be simple, fun, and slightly chaotic, allowing users to earn, give, and gamble with a virtual currency called "AndyCoin".

## User Personas

### Server Owners/Admins

- Want to reward active members
- Need to configure who can give out AndyCoins
- Interested in maintaining server engagement

### Regular Server Members

- Want to earn and accumulate AndyCoins
- Enjoy competing with others on the leaderboard
- May use AndyCoins as a status symbol within the community

### Power Users

- Track their AndyCoin balance across multiple servers
- Actively participate in the economy
- May strategize around the voting system to cause chaos

## User Journeys

### Server Setup

1. Server owner adds AndyCoin bot to their server
2. Owner configures the giver role using `/config role`
3. Owner assigns the giver role to trusted members
4. Givers start distributing AndyCoins to active members

### Regular Usage

1. User participates in server activities
2. User receives AndyCoins from givers
3. User checks their balance with `/balance`
4. User views the leaderboard with `/leaderboard`
5. User gambles coins with `/flip`

### Server Reset (Planned)

1. User initiates a vote to reset server AndyCoins
2. Server members vote on the proposal
3. If vote passes, all balances in the server are reset
4. Economy starts fresh, potentially causing chaos and excitement

## Feature Prioritization

### Core (Implemented)

- Giving AndyCoins to users
- Checking balances (server and global)
- Viewing leaderboards (server and global)
- Configuring giver roles
- Coin flipping and gambling

### Planned

- Server reset voting system
- Web interface for leaderboard
- Improved data persistence

### Future Possibilities

- Scheduled rewards
- Integration with server activities
- Custom server economy settings
- AndyCoin transfer between users
