use poise::serenity_prelude as serenity;
use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::DATA_FILE;

#[derive(Default, Serialize, Deserialize)]
pub struct UserBalance {
    pub guild_id: u64,
    pub user_id: u64,
    pub balance: u32,
}

pub struct Data {
    // Map of guild_id -> (user_id -> balance)
    pub guild_balances: dashmap::DashMap<serenity::GuildId, dashmap::DashMap<serenity::UserId, u32>>,
} 

impl Data {
    // Create a new Data instance
    pub fn new() -> Self {
        Self {
            guild_balances: dashmap::DashMap::new(),
        }
    }

    // Parse YAML string into user balances
    pub fn parse_yaml(yaml_str: &str) -> Result<Vec<UserBalance>, serde_yaml::Error> {
        serde_yaml::from_str::<Vec<UserBalance>>(yaml_str)
    }
    
    // Import user balances into the data structure
    pub fn import_balances(&self, balances: Vec<UserBalance>) {
        for user_balance in balances {
            let guild_id = serenity::GuildId::new(user_balance.guild_id);
            let user_id = serenity::UserId::new(user_balance.user_id);
            
            // Get or create the guild's balance map
            let guild_map = self.guild_balances
                .entry(guild_id)
                .or_insert_with(dashmap::DashMap::new);
            
            // Insert the user's balance
            guild_map.insert(user_id, user_balance.balance);
        }
        
        // Count total balances across all guilds
        let total_balances: usize = self.guild_balances
            .iter()
            .map(|guild_entry| guild_entry.value().len())
            .sum();
        
        println!("Loaded {} user balances across {} guilds", 
                 total_balances, self.guild_balances.len());
    }
    
    // Load balances from YAML file
    pub async fn load() -> Self {
        let data = Self::new();
        
        if !Path::new(DATA_FILE).exists() {
            println!("No data file found. Starting with empty balances.");
            return data;
        }
        
        // Read file contents
        let yaml_str = match tokio::fs::read_to_string(DATA_FILE).await {
            Ok(content) => content,
            Err(e) => {
                eprintln!("Error reading data file: {}", e);
                return data;
            }
        };
        
        // Parse YAML and import balances
        match Self::parse_yaml(&yaml_str) {
            Ok(balances) => {
                data.import_balances(balances);
                println!("Successfully loaded balances from {}", DATA_FILE);
            }
            Err(e) => eprintln!("Error deserializing balances: {}", e),
        }
        
        data
    }

    // Export balances to a serializable format
    pub fn export_balances(&self) -> Vec<UserBalance> {
        let mut balances = Vec::new();
        
        for guild_entry in self.guild_balances.iter() {
            let guild_id = guild_entry.key().get();
            
            for user_entry in guild_entry.value().iter() {
                balances.push(UserBalance {
                    guild_id,
                    user_id: user_entry.key().get(),
                    balance: *user_entry.value(),
                });
            }
        }
        
        balances
    }
    
    // Convert balances to YAML string
    pub fn to_yaml(balances: &[UserBalance]) -> Result<String, serde_yaml::Error> {
        serde_yaml::to_string(balances)
    }
    
    // Save balances to YAML file
    pub async fn save(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let balances = self.export_balances();
        let yaml_str = Self::to_yaml(&balances)?;
        
        tokio::fs::write(DATA_FILE, yaml_str).await?;
        println!("Saved {} user balances to {}", balances.len(), DATA_FILE);
        
        Ok(())
    }

    // Get a user's balance in a specific guild
    pub fn get_guild_balance(&self, guild_id: serenity::GuildId, user_id: serenity::UserId) -> u32 {
        self.guild_balances
            .get(&guild_id)
            .and_then(|guild_map| guild_map.get(&user_id).map(|bal| *bal))
            .unwrap_or(0)
    }
    
    // Get a user's total balance across all guilds
    pub fn get_total_balance(&self, user_id: serenity::UserId) -> u32 {
        self.guild_balances
            .iter()
            .filter_map(|guild_entry| guild_entry.value().get(&user_id).map(|bal| *bal))
            .sum()
    }

    // Add coins to a user's balance in a specific guild
    pub fn add_coins(&self, guild_id: serenity::GuildId, user_id: serenity::UserId, amount: u32) -> u32 {
        // Get or create the guild's balance map
        let guild_map = self.guild_balances
            .entry(guild_id)
            .or_insert_with(dashmap::DashMap::new);
        
        // Update the user's balance
        let new_balance = guild_map
            .entry(user_id)
            .and_modify(|bal| *bal += amount)
            .or_insert(amount)
            .clone();
        
        new_balance
    }

    // Get top users by balance in a specific guild
    pub fn get_guild_top_users(&self, guild_id: serenity::GuildId, limit: usize) -> Vec<(serenity::UserId, u32)> {
        if let Some(guild_map) = self.guild_balances.get(&guild_id) {
            let mut users: Vec<(serenity::UserId, u32)> = guild_map
                .iter()
                .map(|entry| (*entry.key(), *entry.value()))
                .collect();
            
            users.sort_by(|a, b| b.1.cmp(&a.1));
            users.truncate(limit);
            
            users
        } else {
            Vec::new()
        }
    }
    
    // Get top users by total balance across all guilds
    pub fn get_global_top_users(&self, limit: usize) -> Vec<(serenity::UserId, u32)> {
        // Collect all user balances across all guilds
        let user_totals: dashmap::DashMap<serenity::UserId, u32> = dashmap::DashMap::new();
        
        for guild_entry in self.guild_balances.iter() {
            for user_entry in guild_entry.value().iter() {
                user_totals
                    .entry(*user_entry.key())
                    .and_modify(|bal| *bal += *user_entry.value())
                    .or_insert(*user_entry.value());
            }
        }
        
        // Convert to vector and sort
        let mut users: Vec<(serenity::UserId, u32)> = user_totals
            .iter()
            .map(|entry| (*entry.key(), *entry.value()))
            .collect();
        
        users.sort_by(|a, b| b.1.cmp(&a.1));
        users.truncate(limit);
        
        users
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper function to create a test user ID
    fn test_user_id(id: u64) -> serenity::UserId {
        serenity::UserId::new(id)
    }
    
    // Helper function to create a test guild ID
    fn test_guild_id(id: u64) -> serenity::GuildId {
        serenity::GuildId::new(id)
    }

    #[test]
    fn test_get_balance_empty() {
        let data = Data::new();
        let guild_id = test_guild_id(1);
        let user_id = test_user_id(123);
        
        // Balance should be 0 for a new user
        assert_eq!(data.get_guild_balance(guild_id, user_id), 0);
        assert_eq!(data.get_total_balance(user_id), 0);
    }

    #[test]
    fn test_add_coins() {
        let data = Data::new();
        let guild_id = test_guild_id(1);
        let user_id = test_user_id(123);
        
        // Add coins and check the new balance
        let new_balance = data.add_coins(guild_id, user_id, 50);
        assert_eq!(new_balance, 50);
        assert_eq!(data.get_guild_balance(guild_id, user_id), 50);
        assert_eq!(data.get_total_balance(user_id), 50);
        
        // Add more coins and check the updated balance
        let new_balance = data.add_coins(guild_id, user_id, 25);
        assert_eq!(new_balance, 75);
        assert_eq!(data.get_guild_balance(guild_id, user_id), 75);
        assert_eq!(data.get_total_balance(user_id), 75);
    }
    
    #[test]
    fn test_multi_guild_balances() {
        let data = Data::new();
        let guild1 = test_guild_id(1);
        let guild2 = test_guild_id(2);
        let user_id = test_user_id(123);
        
        // Add coins in different guilds
        data.add_coins(guild1, user_id, 50);
        data.add_coins(guild2, user_id, 30);
        
        // Check guild-specific balances
        assert_eq!(data.get_guild_balance(guild1, user_id), 50);
        assert_eq!(data.get_guild_balance(guild2, user_id), 30);
        
        // Check total balance
        assert_eq!(data.get_total_balance(user_id), 80);
    }

    #[test]
    fn test_get_guild_top_users() {
        let data = Data::new();
        let guild_id = test_guild_id(1);
        
        // Add balances for multiple users
        data.add_coins(guild_id, test_user_id(1), 100);
        data.add_coins(guild_id, test_user_id(2), 50);
        data.add_coins(guild_id, test_user_id(3), 200);
        data.add_coins(guild_id, test_user_id(4), 75);
        
        // Get top 3 users
        let top_users = data.get_guild_top_users(guild_id, 3);
        
        // Check the order and values
        assert_eq!(top_users.len(), 3);
        assert_eq!(top_users[0].0, test_user_id(3));
        assert_eq!(top_users[0].1, 200);
        assert_eq!(top_users[1].0, test_user_id(1));
        assert_eq!(top_users[1].1, 100);
        assert_eq!(top_users[2].0, test_user_id(4));
        assert_eq!(top_users[2].1, 75);
    }
    
    #[test]
    fn test_get_global_top_users() {
        let data = Data::new();
        let guild1 = test_guild_id(1);
        let guild2 = test_guild_id(2);
        
        // Add balances for multiple users across different guilds
        data.add_coins(guild1, test_user_id(1), 100);
        data.add_coins(guild2, test_user_id(1), 50);  // Same user in different guild
        
        data.add_coins(guild1, test_user_id(2), 50);
        
        data.add_coins(guild1, test_user_id(3), 75);
        data.add_coins(guild2, test_user_id(3), 75);  // Same user in different guild
        
        // Get top 3 users globally
        let top_users = data.get_global_top_users(3);
        
        // Check the number of users
        assert_eq!(top_users.len(), 3);
        
        // Since users with the same balance might be ordered differently depending on the implementation,
        // we'll check that the expected users and balances are present
        let user1_entry = top_users.iter().find(|(id, _)| *id == test_user_id(1));
        let user2_entry = top_users.iter().find(|(id, _)| *id == test_user_id(2));
        let user3_entry = top_users.iter().find(|(id, _)| *id == test_user_id(3));
        
        assert!(user1_entry.is_some(), "User 1 should be in the top users");
        assert!(user2_entry.is_some(), "User 2 should be in the top users");
        assert!(user3_entry.is_some(), "User 3 should be in the top users");
        
        assert_eq!(user1_entry.unwrap().1, 150, "User 1 should have 150 coins (100 + 50)");
        assert_eq!(user2_entry.unwrap().1, 50, "User 2 should have 50 coins");
        assert_eq!(user3_entry.unwrap().1, 150, "User 3 should have 150 coins (75 + 75)");
        
        // Check that users with the same balance (1 and 3) are sorted by their balance
        let user1_pos = top_users.iter().position(|(id, _)| *id == test_user_id(1)).unwrap();
        let user3_pos = top_users.iter().position(|(id, _)| *id == test_user_id(3)).unwrap();
        
        assert!(user1_pos < 2 && user3_pos < 2, "Users 1 and 3 should be in the top 2 positions");
        assert!(user1_pos != user3_pos, "Users 1 and 3 should be in different positions");
    }

    #[test]
    fn test_parse_yaml() {
        let yaml_str = r#"
- guild_id: 1
  user_id: 123
  balance: 100
- guild_id: 1
  user_id: 456
  balance: 200
- guild_id: 2
  user_id: 123
  balance: 50
"#;
        
        let result = Data::parse_yaml(yaml_str);
        assert!(result.is_ok());
        
        let balances = result.unwrap();
        assert_eq!(balances.len(), 3);
        assert_eq!(balances[0].guild_id, 1);
        assert_eq!(balances[0].user_id, 123);
        assert_eq!(balances[0].balance, 100);
        assert_eq!(balances[1].guild_id, 1);
        assert_eq!(balances[1].user_id, 456);
        assert_eq!(balances[1].balance, 200);
        assert_eq!(balances[2].guild_id, 2);
        assert_eq!(balances[2].user_id, 123);
        assert_eq!(balances[2].balance, 50);
    }

    #[test]
    fn test_import_balances() {
        let data = Data::new();
        let balances = vec![
            UserBalance { guild_id: 1, user_id: 123, balance: 100 },
            UserBalance { guild_id: 1, user_id: 456, balance: 200 },
            UserBalance { guild_id: 2, user_id: 123, balance: 50 },
        ];
        
        data.import_balances(balances);
        
        // Check guild-specific balances
        assert_eq!(data.get_guild_balance(test_guild_id(1), test_user_id(123)), 100);
        assert_eq!(data.get_guild_balance(test_guild_id(1), test_user_id(456)), 200);
        assert_eq!(data.get_guild_balance(test_guild_id(2), test_user_id(123)), 50);
        assert_eq!(data.get_guild_balance(test_guild_id(1), test_user_id(789)), 0); // Non-existent user
        
        // Check total balances
        assert_eq!(data.get_total_balance(test_user_id(123)), 150); // 100 + 50
        assert_eq!(data.get_total_balance(test_user_id(456)), 200);
    }
    
    #[test]
    fn test_export_balances() {
        let data = Data::new();
        
        // Add some balances across different guilds
        data.add_coins(test_guild_id(1), test_user_id(123), 100);
        data.add_coins(test_guild_id(1), test_user_id(456), 200);
        data.add_coins(test_guild_id(2), test_user_id(123), 50);
        
        // Export balances
        let mut balances = data.export_balances();
        
        // Sort by guild_id and user_id to ensure consistent order for testing
        balances.sort_by(|a, b| {
            a.guild_id.cmp(&b.guild_id).then(a.user_id.cmp(&b.user_id))
        });
        
        // Check the exported data
        assert_eq!(balances.len(), 3);
        assert_eq!(balances[0].guild_id, 1);
        assert_eq!(balances[0].user_id, 123);
        assert_eq!(balances[0].balance, 100);
        assert_eq!(balances[1].guild_id, 1);
        assert_eq!(balances[1].user_id, 456);
        assert_eq!(balances[1].balance, 200);
        assert_eq!(balances[2].guild_id, 2);
        assert_eq!(balances[2].user_id, 123);
        assert_eq!(balances[2].balance, 50);
    }
    
    #[test]
    fn test_to_yaml() {
        let balances = vec![
            UserBalance { guild_id: 1, user_id: 123, balance: 100 },
            UserBalance { guild_id: 1, user_id: 456, balance: 200 },
            UserBalance { guild_id: 2, user_id: 123, balance: 50 },
        ];
        
        let yaml_result = Data::to_yaml(&balances);
        assert!(yaml_result.is_ok());
        
        let yaml_str = yaml_result.unwrap();
        
        // Parse it back to verify
        let parsed_result = Data::parse_yaml(&yaml_str);
        assert!(parsed_result.is_ok());
        
        let parsed_balances = parsed_result.unwrap();
        assert_eq!(parsed_balances.len(), 3);
        assert_eq!(parsed_balances[0].guild_id, 1);
        assert_eq!(parsed_balances[0].user_id, 123);
        assert_eq!(parsed_balances[0].balance, 100);
        assert_eq!(parsed_balances[1].guild_id, 1);
        assert_eq!(parsed_balances[1].user_id, 456);
        assert_eq!(parsed_balances[1].balance, 200);
        assert_eq!(parsed_balances[2].guild_id, 2);
        assert_eq!(parsed_balances[2].user_id, 123);
        assert_eq!(parsed_balances[2].balance, 50);
    }
}
