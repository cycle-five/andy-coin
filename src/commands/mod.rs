pub mod give;
pub mod balance;
pub mod leaderboard;

pub use give::give;
pub use balance::balance;
pub use leaderboard::leaderboard;

use crate::{Data, Error};

// Helper function to get all commands
pub fn all_commands() -> Vec<poise::Command<Data, Error>> {
    vec![
        give(),
        balance(),
        leaderboard(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper function to create a test user ID
    #[test]
    fn test_all_commands() {
        let commands = all_commands();
        assert_eq!(commands.len(), 3);
    }
}