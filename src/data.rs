use poise::serenity_prelude as serenity;
use serde::{Deserialize, Serialize};
use std::path::Path;
use rand::Rng;

use crate::DATA_FILE;

#[derive(Default, Serialize, Deserialize)]
pub struct UserBalance {
    pub guild_id: u64,
    pub user_id: u64,
    pub balance: u32,
}

#[derive(Default, Serialize, Deserialize)]
pub struct GuildConfig {
    pub guild_id: u64,
    pub giver_role_id: Option<u64>,
}

pub struct Data {
    // Map of guild_id -> (user_id -> balance)
    pub guild_balances: dashmap::DashMap<serenity::GuildId, dashmap::DashMap<serenity::UserId, u32>>,
    // Map of guild_id -> guild configuration
    pub guild_configs: dashmap::DashMap<serenity::GuildId, GuildConfig>,
    // Cache from the bot's context
    pub cache: serenity::Cache,
} 

impl Default for Data {
    fn default() -> Self {
        Self::new()
    }
}

impl Data {
    // Create a new Data instance
    pub fn new() -> Self {
        Self {
            guild_balances: dashmap::DashMap::new(),
            guild_configs: dashmap::DashMap::new(),
            cache: serenity::Cache::default(),
        }
    }

    // Parse YAML string into user balances and guild configs
    pub fn parse_yaml(yaml_str: &str) -> Result<(Vec<UserBalance>, Vec<GuildConfig>), serde_yaml::Error> {
        let data: serde_yaml::Value = serde_yaml::from_str(yaml_str)?;
        
        let balances = if let Some(balances_value) = data.get("balances") {
            serde_yaml::from_value(balances_value.clone())?
        } else {
            // For backward compatibility with old format
            let old_format: Result<Vec<UserBalance>, _> = serde_yaml::from_str(yaml_str);
            old_format.unwrap_or_default()
        };
        
        let configs = if let Some(configs_value) = data.get("configs") {
            serde_yaml::from_value(configs_value.clone())?
        } else {
            Vec::new()
        };
        
        Ok((balances, configs))
    }
    
    // Import user balances and guild configs into the data structure
    pub fn import_data(&self, balances: Vec<UserBalance>, configs: Vec<GuildConfig>) {
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
        
        // Import guild configs
        for guild_config in configs {
            let guild_id = serenity::GuildId::new(guild_config.guild_id);
            self.guild_configs.insert(guild_id, guild_config);
        }
        
        // Count total balances across all guilds
        let total_balances: usize = self.guild_balances
            .iter()
            .map(|guild_entry| guild_entry.value().len())
            .sum();
        
        println!("Loaded {} user balances and {} guild configs", 
                 total_balances, self.guild_configs.len());
    }
    
    // Load data from YAML file
    pub async fn load() -> Self {
        let data = Self::new();
        
        if !Path::new(DATA_FILE).exists() {
            println!("No data file found. Starting with empty data.");
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
        
        // Parse YAML and import data
        match Self::parse_yaml(&yaml_str) {
            Ok((balances, configs)) => {
                data.import_data(balances, configs);
                println!("Successfully loaded data from {}", DATA_FILE);
            }
            Err(e) => eprintln!("Error deserializing data: {}", e),
        }
        
        data
    }

    // Export balances and configs to a serializable format
    pub fn export_data(&self) -> (Vec<UserBalance>, Vec<GuildConfig>) {
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
        
        let mut configs = Vec::new();
        
        for config_entry in self.guild_configs.iter() {
            let guild_id = config_entry.key().get();
            let config = config_entry.value();
            
            configs.push(GuildConfig {
                guild_id,
                giver_role_id: config.giver_role_id,
            });
        }
        
        (balances, configs)
    }
    
    // Convert data to YAML string
    pub fn to_yaml(balances: &[UserBalance], configs: &[GuildConfig]) -> Result<String, serde_yaml::Error> {
        let mut data = serde_yaml::Mapping::new();
        
        data.insert(
            serde_yaml::Value::String("balances".to_string()),
            serde_yaml::to_value(balances)?,
        );
        
        data.insert(
            serde_yaml::Value::String("configs".to_string()),
            serde_yaml::to_value(configs)?,
        );
        
        serde_yaml::to_string(&serde_yaml::Value::Mapping(data))
    }
    
    // Save data to YAML file
    pub async fn save(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let (balances, configs) = self.export_data();
        let yaml_str = Self::to_yaml(&balances, &configs)?;
        
        tokio::fs::write(DATA_FILE, yaml_str).await?;
        println!("Saved {} user balances and {} guild configs to {}", 
                 balances.len(), configs.len(), DATA_FILE);
        
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
        let new_balance = *guild_map
            .entry(user_id)
            .and_modify(|bal| *bal += amount)
            .or_insert(amount);
        
        new_balance
    }

    // Remove coins from a user's balance in a specific guild
    pub fn remove_coins(&self, guild_id: serenity::GuildId, user_id: serenity::UserId, amount: u32) -> u32 {
        // Get or create the guild's balance map
        let guild_map = self.guild_balances
            .entry(guild_id)
            .or_insert_with(dashmap::DashMap::new);
        
        // Update the user's balance
        let new_balance = guild_map
            .entry(user_id)
            .and_modify(|bal| {
                if *bal >= amount {
                    *bal -= amount;
                } else {
                    *bal = 0;
                }
            })
            .or_insert(0);

        *new_balance
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

    // Set the giver role for a guild
    pub fn set_giver_role(&self, guild_id: serenity::GuildId, role_id: Option<serenity::RoleId>) {
        let role_id_u64 = role_id.map(|r| r.get());
        
        self.guild_configs
            .entry(guild_id)
            .and_modify(|config| config.giver_role_id = role_id_u64)
            .or_insert_with(|| GuildConfig {
                guild_id: guild_id.get(),
                giver_role_id: role_id_u64,
            });
    }

    // Get the giver role for a guild
    pub fn get_giver_role(&self, guild_id: serenity::GuildId) -> Option<serenity::RoleId> {
        self.guild_configs
            .get(&guild_id)
            .and_then(|config| config.giver_role_id.map(serenity::RoleId::new))
    }

    // Check if a user has the giver role
    pub fn has_giver_role(&self, guild_id: serenity::GuildId, member: &serenity::Member) -> bool {
        // Server owner always has permission
        // Get guild owner ID.
        if member.user.id == member.guild_id.to_guild_cached(&self.cache).map(|g| g.owner_id).unwrap_or_default() {
            return true;
        }
        
        // Check if the user has the giver role
        if let Some(giver_role_id) = self.get_giver_role(guild_id) {
            return member.roles.contains(&giver_role_id);
        }
        
        // If no giver role is set, only the server owner can give coins
        false
    }

    // Flip a coin and return the result
    pub fn flip_coin(&self) -> bool {
        let mut rng = rand::thread_rng();
        rng.gen_bool(0.5)
    }

    // Reset all data
    pub fn reset(&self) {
        self.guild_balances.clear();
        self.guild_configs.clear();
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

    // Helper function to create a test role ID
    fn test_role_id(id: u64) -> serenity::RoleId {
        serenity::RoleId::new(id)
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
    fn test_set_get_giver_role() {
        let data = Data::new();
        let guild_id = test_guild_id(1);
        let role_id = test_role_id(123);
        
        // Initially, no giver role is set
        assert_eq!(data.get_giver_role(guild_id), None);
        
        // Set a giver role
        data.set_giver_role(guild_id, Some(role_id));
        
        // Check that the giver role is set
        assert_eq!(data.get_giver_role(guild_id), Some(role_id));
        
        // Clear the giver role
        data.set_giver_role(guild_id, None);
        
        // Check that the giver role is cleared
        assert_eq!(data.get_giver_role(guild_id), None);
    }

    #[test]
    fn test_flip_coin() {
        let data = Data::new();
        
        // Flip the coin multiple times to ensure it returns both true and false
        let mut heads_count = 0;
        let mut tails_count = 0;
        
        for _ in 0..100 {
            if data.flip_coin() {
                heads_count += 1;
            } else {
                tails_count += 1;
            }
        }
        
        // Both heads and tails should occur
        assert!(heads_count > 0);
        assert!(tails_count > 0);
    }

    #[test]
    fn test_parse_yaml() {
        let yaml_str = r#"
balances:
  - guild_id: 1
    user_id: 123
    balance: 100
  - guild_id: 1
    user_id: 456
    balance: 200
  - guild_id: 2
    user_id: 123
    balance: 50
configs:
  - guild_id: 1
    giver_role_id: 789
  - guild_id: 2
    giver_role_id: null
"#;
        
        let result = Data::parse_yaml(yaml_str);
        assert!(result.is_ok());
        
        let (balances, configs) = result.unwrap();
        
        // Check balances
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
        
        // Check configs
        assert_eq!(configs.len(), 2);
        assert_eq!(configs[0].guild_id, 1);
        assert_eq!(configs[0].giver_role_id, Some(789));
        assert_eq!(configs[1].guild_id, 2);
        assert_eq!(configs[1].giver_role_id, None);
    }

    #[test]
    fn test_import_data() {
        let data = Data::new();
        let balances = vec![
            UserBalance { guild_id: 1, user_id: 123, balance: 100 },
            UserBalance { guild_id: 1, user_id: 456, balance: 200 },
            UserBalance { guild_id: 2, user_id: 123, balance: 50 },
        ];
        
        let configs = vec![
            GuildConfig { guild_id: 1, giver_role_id: Some(789) },
            GuildConfig { guild_id: 2, giver_role_id: None },
        ];
        
        data.import_data(balances, configs);
        
        // Check guild-specific balances
        assert_eq!(data.get_guild_balance(test_guild_id(1), test_user_id(123)), 100);
        assert_eq!(data.get_guild_balance(test_guild_id(1), test_user_id(456)), 200);
        assert_eq!(data.get_guild_balance(test_guild_id(2), test_user_id(123)), 50);
        assert_eq!(data.get_guild_balance(test_guild_id(1), test_user_id(789)), 0); // Non-existent user
        
        // Check total balances
        assert_eq!(data.get_total_balance(test_user_id(123)), 150); // 100 + 50
        assert_eq!(data.get_total_balance(test_user_id(456)), 200);
        
        // Check guild configs
        assert_eq!(data.get_giver_role(test_guild_id(1)), Some(test_role_id(789)));
        assert_eq!(data.get_giver_role(test_guild_id(2)), None);
    }
    
    #[test]
    fn test_export_data() {
        let data = Data::new();
        
        // Add some balances across different guilds
        data.add_coins(test_guild_id(1), test_user_id(123), 100);
        data.add_coins(test_guild_id(1), test_user_id(456), 200);
        data.add_coins(test_guild_id(2), test_user_id(123), 50);
        
        // Set some guild configs
        data.set_giver_role(test_guild_id(1), Some(test_role_id(789)));
        data.set_giver_role(test_guild_id(2), None);
        
        // Export data
        let (mut balances, mut configs) = data.export_data();
        
        // Sort by guild_id and user_id to ensure consistent order for testing
        balances.sort_by(|a, b| {
            a.guild_id.cmp(&b.guild_id).then(a.user_id.cmp(&b.user_id))
        });
        
        configs.sort_by(|a, b| a.guild_id.cmp(&b.guild_id));
        
        // Check the exported balances
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
        
        // Check the exported configs
        assert_eq!(configs.len(), 2);
        assert_eq!(configs[0].guild_id, 1);
        assert_eq!(configs[0].giver_role_id, Some(789));
        assert_eq!(configs[1].guild_id, 2);
        assert_eq!(configs[1].giver_role_id, None);
    }
    
    #[test]
    fn test_to_yaml() {
        let balances = vec![
            UserBalance { guild_id: 1, user_id: 123, balance: 100 },
            UserBalance { guild_id: 1, user_id: 456, balance: 200 },
            UserBalance { guild_id: 2, user_id: 123, balance: 50 },
        ];
        
        let configs = vec![
            GuildConfig { guild_id: 1, giver_role_id: Some(789) },
            GuildConfig { guild_id: 2, giver_role_id: None },
        ];
        
        let yaml_result = Data::to_yaml(&balances, &configs);
        assert!(yaml_result.is_ok());
        
        let yaml_str = yaml_result.unwrap();
        
        // Parse it back to verify
        let parsed_result = Data::parse_yaml(&yaml_str);
        assert!(parsed_result.is_ok());
        
        let (parsed_balances, parsed_configs) = parsed_result.unwrap();
        
        // Check balances
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
        
        // Check configs
        assert_eq!(parsed_configs.len(), 2);
        assert_eq!(parsed_configs[0].guild_id, 1);
        assert_eq!(parsed_configs[0].giver_role_id, Some(789));
        assert_eq!(parsed_configs[1].guild_id, 2);
        assert_eq!(parsed_configs[1].giver_role_id, None);
    }
}
