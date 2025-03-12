pub mod balance;
pub mod config;
pub mod leaderboard;
pub mod give;

pub use balance::balance;
pub use config::config;
pub use config::flip;
pub use give::give;
pub use leaderboard::leaderboard;

use crate::{Data, Error};

// Helper function to get all commands
pub fn _all_commands() -> Vec<poise::Command<Data, Error>> {
    vec![
        config(),
        give(),
        balance(),
        leaderboard(),
        flip(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_commands() {
        let commands = _all_commands();
        assert_eq!(commands.len(), 5);
    }
}
