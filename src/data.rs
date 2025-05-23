use poise::serenity_prelude::{self as serenity, RoleId};
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::{
    ops::{Deref, DerefMut},
    path::Path,
};

use crate::DATA_FILE;

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct UserBalance {
    pub guild_id: u64,
    pub user_id: u64,
    pub balance: u32,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct VoteConfig {
    pub cooldown_hours: u32,
    pub duration_minutes: u32,
    pub min_votes: u32,
    pub majority_percentage: u32,
}

impl Default for VoteConfig {
    fn default() -> Self {
        Self {
            cooldown_hours: 24,      // Once per day
            duration_minutes: 30,    // Half hour voting time
            min_votes: 10,           // At least 10 votes
            majority_percentage: 70, // 7/10 majority (70%)
        }
    }
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct VoteStatus {
    pub active: bool,
    pub start_time: Option<chrono::DateTime<chrono::Utc>>,
    pub end_time: Option<chrono::DateTime<chrono::Utc>>,
    pub initiator_id: Option<u64>,
    pub yes_votes: Vec<u64>,
    pub no_votes: Vec<u64>,
    pub last_vote_time: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct GuildConfig {
    pub guild_id: u64,
    pub giver_role_id: Option<u64>,
    #[serde(default)]
    pub vote_config: VoteConfig,
    #[serde(default)]
    pub vote_status: VoteStatus,
}

#[derive(Default)]
pub struct Data(pub DataInner);

impl Deref for Data {
    type Target = DataInner;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Data {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Data {
    #[must_use]
    pub fn new() -> Self {
        Self(DataInner::new())
    }

    /// Parse YAML string into user balances and guild configs
    /// # Errors
    /// Returns an error if the YAML string is invalid
    pub fn parse_yaml(
        yaml_str: &str,
    ) -> Result<(Vec<UserBalance>, Vec<GuildConfig>), serde_yaml::Error> {
        DataInner::parse_yaml(yaml_str)
    }

    pub fn import_data(&self, balances: Vec<UserBalance>, configs: Vec<GuildConfig>) {
        self.0.import_data(balances, configs);
    }

    pub async fn load() -> Self {
        Data(DataInner::load().await)
    }

    pub fn export_data(&self) -> (Vec<UserBalance>, Vec<GuildConfig>) {
        self.0.export_data()
    }

    /// Save data to YAML file
    /// # Errors
    /// Returns an error if the file cannot be written
    pub fn to_yaml(
        balances: &[UserBalance],
        configs: &[GuildConfig],
    ) -> Result<String, serde_yaml::Error> {
        DataInner::to_yaml(balances, configs)
    }
}

/// Main centrailized data structure for the bot. Should it use the `InnerData` idiom?
pub struct DataInner {
    // Map of guild_id -> (user_id -> balance)
    pub guild_balances:
        dashmap::DashMap<serenity::GuildId, dashmap::DashMap<serenity::UserId, u32>>,
    // Map of guild_id -> guild configuration
    pub guild_configs: dashmap::DashMap<serenity::GuildId, GuildConfig>,
    // Cache from the bot's context
    pub cache: serenity::Cache,
}

impl Default for DataInner {
    fn default() -> Self {
        Self::new()
    }
}

impl DataInner {
    /// Create a new Data instance
    pub fn new() -> Self {
        Self {
            guild_balances: dashmap::DashMap::new(),
            guild_configs: dashmap::DashMap::new(),
            cache: serenity::Cache::default(),
        }
    }

    /// Parse YAML string into user balances and guild configs
    pub fn parse_yaml(
        yaml_str: &str,
    ) -> Result<(Vec<UserBalance>, Vec<GuildConfig>), serde_yaml::Error> {
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

    /// Import user balances and guild configs into the data structure
    pub fn import_data(&self, balances: Vec<UserBalance>, configs: Vec<GuildConfig>) {
        for user_balance in balances {
            let guild_id = serenity::GuildId::new(user_balance.guild_id);
            let user_id = serenity::UserId::new(user_balance.user_id);

            // Get or create the guild's balance map
            let guild_map = self
                .guild_balances
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
        let total_balances: usize = self
            .guild_balances
            .iter()
            .map(|guild_entry| guild_entry.value().len())
            .sum();

        tracing::info!(
            "Loaded {} user balances and {} guild configs",
            total_balances,
            self.guild_configs.len()
        );
    }

    /// Load data from YAML file
    pub async fn load() -> Self {
        let data = Self::new();

        if !Path::new(DATA_FILE).exists() {
            tracing::info!("No data file found. Starting with empty data.");
            return data;
        }

        // Read file contents
        let yaml_str = match tokio::fs::read_to_string(DATA_FILE).await {
            Ok(content) => content,
            Err(e) => {
                tracing::error!("Error reading data file: {}", e);
                return data;
            }
        };

        // Parse YAML and import data
        match Self::parse_yaml(&yaml_str) {
            Ok((balances, configs)) => {
                data.import_data(balances, configs);
                tracing::info!("Successfully loaded data from {}", DATA_FILE);
            }
            Err(e) => tracing::error!("Error deserializing data: {}", e),
        }

        data
    }

    /// Export balances and configs to a serializable format
    pub fn export_data(&self) -> (Vec<UserBalance>, Vec<GuildConfig>) {
        let mut balances = Vec::new();

        for guild_entry in &self.guild_balances {
            let guild_id = guild_entry.key().get();

            for user_entry in guild_entry.value() {
                balances.push(UserBalance {
                    guild_id,
                    user_id: user_entry.key().get(),
                    balance: *user_entry.value(),
                });
            }
        }

        let mut configs = Vec::new();

        for config_entry in &self.guild_configs {
            let guild_id = config_entry.key().get();
            let config = config_entry.value();

            configs.push(GuildConfig {
                guild_id,
                giver_role_id: config.giver_role_id,
                vote_config: config.vote_config.clone(),
                vote_status: config.vote_status.clone(),
            });
        }

        (balances, configs)
    }

    /// Convert data to YAML string
    pub fn to_yaml(
        balances: &[UserBalance],
        configs: &[GuildConfig],
    ) -> Result<String, serde_yaml::Error> {
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

    /// Save data to YAML file
    pub async fn save(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let (balances, configs) = self.export_data();
        let yaml_str = Self::to_yaml(&balances, &configs)?;

        tokio::fs::write(DATA_FILE, yaml_str).await?;
        tracing::info!(
            "Saved {} user balances and {} guild configs to {}",
            balances.len(),
            configs.len(),
            DATA_FILE
        );

        Ok(())
    }

    /// Expire votes that have ended
    pub fn expire_votes(&self) -> Vec<serenity::GuildId> {
        let mut expired_votes = Vec::new();
        for guild_entry in &self.guild_configs {
            let guild_id = *guild_entry.key();
            let config = guild_entry.value();

            if config.vote_status.active {
                // Check if the vote has expired
                let now = chrono::Utc::now();
                if let Some(end_time) = config.vote_status.end_time {
                    if now > end_time {
                        // Auto-end the vote
                        self.end_vote(guild_id).unwrap_or_else(|_| {
                            tracing::error!("Failed to end vote for guild {}", guild_id);
                            false
                        });
                        expired_votes.push(guild_id);
                    }
                }
            }
        }
        expired_votes
    }

    /// Get all guild IDs
    pub fn get_guild_ids(&self) -> Vec<serenity::GuildId> {
        self.guild_configs
            .iter()
            .map(|entry| {
                let guild_id = *entry.key();
                guild_id
            })
            .collect::<Vec<_>>()
    }

    /// Get a user's balance in a specific guild
    pub fn get_guild_balance(&self, guild_id: serenity::GuildId, user_id: serenity::UserId) -> u32 {
        self.guild_balances
            .get(&guild_id)
            .and_then(|guild_map| guild_map.get(&user_id).map(|bal| *bal))
            .unwrap_or(0)
    }

    /// Get a user's total balance across all guilds
    pub fn get_total_balance(&self, user_id: serenity::UserId) -> u32 {
        self.guild_balances
            .iter()
            .filter_map(|guild_entry| guild_entry.value().get(&user_id).map(|bal| *bal))
            .sum()
    }

    /// Add coins to a user's balance in a specific guild
    pub fn add_coins(
        &self,
        guild_id: serenity::GuildId,
        user_id: serenity::UserId,
        amount: u32,
    ) -> u32 {
        // Get or create the guild's balance map
        let guild_map = self
            .guild_balances
            .entry(guild_id)
            .or_insert_with(dashmap::DashMap::new);

        // Get the previous balance
        let previous_balance = guild_map.get(&user_id).map(|bal| *bal).unwrap_or(0);

        // Update the user's balance
        let new_balance = *guild_map
            .entry(user_id)
            .and_modify(|bal| *bal += amount)
            .or_insert(amount);

        // Log the balance change
        crate::logging::log_balance_change(
            guild_id.get(),
            user_id.get(),
            previous_balance,
            new_balance,
            "add_coins",
            None,
        );

        new_balance
    }

    /// Remove coins from a user's balance in a specific guild
    pub fn remove_coins(
        &self,
        guild_id: serenity::GuildId,
        user_id: serenity::UserId,
        amount: u32,
    ) -> u32 {
        // Get or create the guild's balance map
        let guild_map = self
            .guild_balances
            .entry(guild_id)
            .or_insert_with(dashmap::DashMap::new);

        // Get the previous balance
        let previous_balance = guild_map.get(&user_id).map(|bal| *bal).unwrap_or(0);

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

        let new_balance_value = *new_balance;

        // Log the balance change
        crate::logging::log_balance_change(
            guild_id.get(),
            user_id.get(),
            previous_balance,
            new_balance_value,
            "remove_coins",
            None,
        );

        new_balance_value
    }

    /// Get top users by balance in a specific guild
    pub fn get_guild_top_users(
        &self,
        guild_id: serenity::GuildId,
        limit: usize,
    ) -> Vec<(serenity::UserId, u32)> {
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

    /// Get top users by total balance across all guilds
    pub fn get_global_top_users(&self, limit: usize) -> Vec<(serenity::UserId, u32)> {
        // Collect all user balances across all guilds
        let user_totals: dashmap::DashMap<serenity::UserId, u32> = dashmap::DashMap::new();

        for guild_entry in &self.guild_balances {
            for user_entry in guild_entry.value() {
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

    /// Set the giver role for a guild
    pub fn set_giver_role(&self, guild_id: serenity::GuildId, role_id: Option<serenity::RoleId>) {
        let role_id_u64 = role_id.map(RoleId::get);

        self.guild_configs
            .entry(guild_id)
            .and_modify(|config| config.giver_role_id = role_id_u64)
            .or_insert_with(|| GuildConfig {
                guild_id: guild_id.get(),
                giver_role_id: role_id_u64,
                vote_config: VoteConfig::default(),
                vote_status: VoteStatus::default(),
            });
    }

    /// Get the giver role for a guild
    pub fn get_giver_role(&self, guild_id: serenity::GuildId) -> Option<serenity::RoleId> {
        self.guild_configs
            .get(&guild_id)
            .and_then(|config| config.giver_role_id.map(serenity::RoleId::new))
    }

    /// Check if a user has the giver role
    pub fn has_giver_role(&self, guild_id: serenity::GuildId, member: &serenity::Member) -> bool {
        // Server owner always has permission
        // Get guild owner ID.
        if member.user.id
            == member
                .guild_id
                .to_guild_cached(&self.cache)
                .map(|g| g.owner_id)
                .unwrap_or_default()
        {
            return true;
        }

        // Check if the user has the giver role
        if let Some(giver_role_id) = self.get_giver_role(guild_id) {
            return member.roles.contains(&giver_role_id);
        }

        // If no giver role is set, only the server owner can give coins
        false
    }

    /// Flip a coin and return the result
    pub fn flip_coin() -> bool {
        let mut rng = rand::rng();
        rng.random_bool(0.5)
    }

    /// Reset all data
    pub fn reset(&self) {
        self.guild_balances.clear();
        self.guild_configs.clear();
    }

    /// Get vote config for a guild
    pub fn get_vote_config(&self, guild_id: serenity::GuildId) -> VoteConfig {
        self.guild_configs
            .get(&guild_id)
            .map(|config| config.vote_config.clone())
            .unwrap_or_default()
    }

    /// Set vote config for a guild
    pub fn set_vote_config(&self, guild_id: serenity::GuildId, vote_config: &VoteConfig) {
        let my_vote_config = vote_config.clone();
        self.guild_configs
            .entry(guild_id)
            .and_modify(|config| config.vote_config = my_vote_config.clone())
            .or_insert_with(|| GuildConfig {
                guild_id: guild_id.get(),
                giver_role_id: None,
                vote_config: my_vote_config,
                vote_status: VoteStatus::default(),
            });
    }

    /// Get vote status for a guild
    pub fn get_vote_status(&self, guild_id: serenity::GuildId) -> VoteStatus {
        self.guild_configs
            .get(&guild_id)
            .map(|config| config.vote_status.clone())
            .unwrap_or_default()
    }

    /// Start a vote in a guild
    pub fn start_vote(
        &self,
        guild_id: serenity::GuildId,
        initiator_id: serenity::UserId,
    ) -> Result<chrono::DateTime<chrono::Utc>, &'static str> {
        let mut config_ref = if let Some(config) = self.guild_configs.get_mut(&guild_id) {
            config
        } else {
            // Create a new config if it doesn't exist
            let config = GuildConfig {
                guild_id: guild_id.get(),
                giver_role_id: None,
                vote_config: VoteConfig::default(),
                vote_status: VoteStatus::default(),
            };
            self.guild_configs.insert(guild_id, config);
            self.guild_configs.get_mut(&guild_id).unwrap()
        };

        // Check if a vote is already active
        if config_ref.vote_status.active {
            return Err("A vote is already active in this server");
        }

        // Check if a vote was recently completed (cooldown period)
        if let Some(last_vote_time) = config_ref.vote_status.last_vote_time {
            let cooldown_duration =
                chrono::Duration::hours(i64::from(config_ref.vote_config.cooldown_hours));
            let now = chrono::Utc::now();

            if now < last_vote_time + cooldown_duration {
                return Err(
                    "A vote was recently completed. Please wait for the cooldown period to end",
                );
            }
        }

        // Start the vote
        let now = chrono::Utc::now();
        let duration =
            chrono::Duration::minutes(i64::from(config_ref.vote_config.duration_minutes));
        let end_time = now + duration;

        config_ref.vote_status = VoteStatus {
            active: true,
            start_time: Some(now),
            end_time: Some(end_time),
            initiator_id: Some(initiator_id.get()),
            yes_votes: vec![initiator_id.get()], // Initiator automatically votes yes
            no_votes: vec![],
            last_vote_time: None,
        };

        Ok(end_time)
    }

    /// Cast a vote
    pub fn cast_vote(
        &self,
        guild_id: serenity::GuildId,
        user_id: serenity::UserId,
        vote_yes: bool,
    ) -> Result<(), &'static str> {
        let mut config_ref = match self.guild_configs.get_mut(&guild_id) {
            Some(config) => config,
            None => return Err("No vote is active in this server"),
        };

        // Check if a vote is active
        if !config_ref.vote_status.active {
            return Err("No vote is active in this server");
        }

        // Check if the vote has expired
        let now = chrono::Utc::now();
        if let Some(end_time) = config_ref.vote_status.end_time {
            if now > end_time {
                // Auto-end the vote
                self.end_vote(guild_id)?;
                return Err("The vote has ended");
            }
        }

        let user_id_u64 = user_id.get();

        // Remove user from both vote lists to avoid duplicate votes
        config_ref
            .vote_status
            .yes_votes
            .retain(|id| *id != user_id_u64);
        config_ref
            .vote_status
            .no_votes
            .retain(|id| *id != user_id_u64);

        // Add user's vote
        if vote_yes {
            config_ref.vote_status.yes_votes.push(user_id_u64);
        } else {
            config_ref.vote_status.no_votes.push(user_id_u64);
        }

        Ok(())
    }

    /// End a vote and process the results
    pub fn end_vote(&self, guild_id: serenity::GuildId) -> Result<bool, &'static str> {
        let mut config_ref = match self.guild_configs.get_mut(&guild_id) {
            Some(config) => config,
            None => return Err("No vote is active in this server"),
        };

        // Check if a vote is active
        if !config_ref.vote_status.active {
            return Err("No vote is active in this server");
        }

        let yes_votes = config_ref.vote_status.yes_votes.len();
        let no_votes = config_ref.vote_status.no_votes.len();
        let total_votes = yes_votes + no_votes;

        // Record the vote end time
        let now = chrono::Utc::now();
        config_ref.vote_status.last_vote_time = Some(now);
        config_ref.vote_status.active = false;

        // Check if there are enough votes
        if total_votes < config_ref.vote_config.min_votes as usize {
            return Ok(false); // Not enough votes, vote fails
        }

        // Calculate the percentage of yes votes
        let yes_percentage = (yes_votes as f64 / total_votes as f64) * 100.0;

        // Check if the majority threshold is met
        let vote_passed = yes_percentage >= f64::from(config_ref.vote_config.majority_percentage);

        // If the vote passed, reset all balances in the guild
        if vote_passed {
            if let Some(guild_balances) = self.guild_balances.get_mut(&guild_id) {
                guild_balances.clear();
                tracing::info!(
                    "Reset all balances in guild {} due to successful vote",
                    guild_id
                );
                // self.save().await.unwrap_or_else(|_| {
                //     tracing::error!("Failed to save data after vote");
                // });
            }
        }

        Ok(vote_passed)
    }

    /// Check if a vote has expired and end it if necessary
    pub fn check_vote_expiry(&self, guild_id: serenity::GuildId) -> Option<bool> {
        let config = self.guild_configs.get(&guild_id)?;

        // Check if a vote is active
        if !config.vote_status.active {
            return None;
        }

        // Check if the vote has expired
        let now = chrono::Utc::now();
        if let Some(end_time) = config.vote_status.end_time {
            if now > end_time {
                // End the vote
                match self.end_vote(guild_id) {
                    Ok(result) => return Some(result),
                    Err(_) => return None,
                }
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper function to create a test user ID
    fn test_user_id(id: u64) -> serenity::UserId {
        serenity::UserId::new(id)
    }

    /// Helper function to create a test guild ID
    fn test_guild_id(id: u64) -> serenity::GuildId {
        serenity::GuildId::new(id)
    }

    /// Helper function to create a test role ID
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
        data.add_coins(guild2, test_user_id(1), 50); // Same user in different guild

        data.add_coins(guild1, test_user_id(2), 50);

        data.add_coins(guild1, test_user_id(3), 75);
        data.add_coins(guild2, test_user_id(3), 75); // Same user in different guild

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

        assert_eq!(
            user1_entry.unwrap().1,
            150,
            "User 1 should have 150 coins (100 + 50)"
        );
        assert_eq!(user2_entry.unwrap().1, 50, "User 2 should have 50 coins");
        assert_eq!(
            user3_entry.unwrap().1,
            150,
            "User 3 should have 150 coins (75 + 75)"
        );

        // Check that users with the same balance (1 and 3) are sorted by their balance
        let user1_pos = top_users
            .iter()
            .position(|(id, _)| *id == test_user_id(1))
            .unwrap();
        let user3_pos = top_users
            .iter()
            .position(|(id, _)| *id == test_user_id(3))
            .unwrap();

        assert!(
            user1_pos < 2 && user3_pos < 2,
            "Users 1 and 3 should be in the top 2 positions"
        );
        assert!(
            user1_pos != user3_pos,
            "Users 1 and 3 should be in different positions"
        );
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

    // FIXME: This needs a mock for serenity::Member I think.
    // #[test]
    // fn test_has_giver_role() {
    //     let data = Data::new();
    //     let guild_id = test_guild_id(1);
    //     let role_id = test_role_id(123);

    //     // Initially, no giver role is set
    //     assert!(!data.has_giver_role(guild_id, &serenity::Member::default()));

    //     // Set a giver role
    //     data.set_giver_role(guild_id, Some(role_id));

    //     // Check that a user with the role has permission
    //     let member = serenity::MemberBuilder::new()
    //         .user(serenity::User::default())
    //         .roles(vec![role_id])
    //         .guild_id(guild_id)
    //         .build();

    //     assert!(data.has_giver_role(guild_id, &member));

    //     // Check that a user without the role doesn't have permission
    //     let asdf = Member::default().;
    //     let member = serenity::Member {
    //         user: serenity::User::default(),
    //         roles: vec![],
    //         guild_id,
    //         ..Default::default()
    //     };

    //     assert!(!data.has_giver_role(guild_id, &member));
    // }

    #[test]
    fn test_flip_coin() {
        // Flip the coin multiple times to ensure it returns both true and false
        let mut heads_count = 0;
        let mut tails_count = 0;

        for _ in 0..100 {
            if DataInner::flip_coin() {
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
            UserBalance {
                guild_id: 1,
                user_id: 123,
                balance: 100,
            },
            UserBalance {
                guild_id: 1,
                user_id: 456,
                balance: 200,
            },
            UserBalance {
                guild_id: 2,
                user_id: 123,
                balance: 50,
            },
        ];

        let configs = vec![
            GuildConfig {
                guild_id: 1,
                giver_role_id: Some(789),
                vote_config: VoteConfig::default(),
                vote_status: VoteStatus::default(),
            },
            GuildConfig {
                guild_id: 2,
                giver_role_id: None,
                vote_config: VoteConfig::default(),
                vote_status: VoteStatus::default(),
            },
        ];

        data.import_data(balances, configs);

        // Check guild-specific balances
        assert_eq!(
            data.get_guild_balance(test_guild_id(1), test_user_id(123)),
            100
        );
        assert_eq!(
            data.get_guild_balance(test_guild_id(1), test_user_id(456)),
            200
        );
        assert_eq!(
            data.get_guild_balance(test_guild_id(2), test_user_id(123)),
            50
        );
        assert_eq!(
            data.get_guild_balance(test_guild_id(1), test_user_id(789)),
            0
        ); // Non-existent user

        // Check total balances
        assert_eq!(data.get_total_balance(test_user_id(123)), 150); // 100 + 50
        assert_eq!(data.get_total_balance(test_user_id(456)), 200);

        // Check guild configs
        assert_eq!(
            data.get_giver_role(test_guild_id(1)),
            Some(test_role_id(789))
        );
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
        balances.sort_by(|a, b| a.guild_id.cmp(&b.guild_id).then(a.user_id.cmp(&b.user_id)));

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
            UserBalance {
                guild_id: 1,
                user_id: 123,
                balance: 100,
            },
            UserBalance {
                guild_id: 1,
                user_id: 456,
                balance: 200,
            },
            UserBalance {
                guild_id: 2,
                user_id: 123,
                balance: 50,
            },
        ];

        let configs = vec![
            GuildConfig {
                guild_id: 1,
                giver_role_id: Some(789),
                vote_config: VoteConfig::default(),
                vote_status: VoteStatus::default(),
            },
            GuildConfig {
                guild_id: 2,
                giver_role_id: None,
                vote_config: VoteConfig::default(),
                vote_status: VoteStatus::default(),
            },
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
