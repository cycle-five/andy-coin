use crate::{Context, Data, Error, logging};
use poise::serenity_prelude::{self as serenity, GuildId};

// Core business logic for getting leaderboard
pub fn get_leaderboard(
    data: &Data,
    guild_id: Option<serenity::GuildId>,
    is_global: bool,
    limit: usize,
) -> (Vec<(serenity::UserId, u32)>, &'static str) {
    if is_global || guild_id.is_none() {
        (data.get_global_top_users(limit), "Global")
    } else {
        #[allow(clippy::unnecessary_unwrap)]
        (data.get_guild_top_users(guild_id.unwrap(), limit), "Server")
    }
}

/// Display the AndyCoin leaderboard
#[poise::command(slash_command, prefix_command)]
pub async fn leaderboard(
    ctx: Context<'_>,
    #[description = "Number of users to show (default: 10)"] limit: Option<usize>,
    #[description = "Show global leaderboard across all servers (default: current server only)"]
    global: Option<bool>,
) -> Result<(), Error> {
    // Format arguments for logging
    let limit_arg = limit.unwrap_or(10).to_string();
    let global_arg = global.unwrap_or(false).to_string();
    let args = format!("limit: {limit_arg}, global: {global_arg}");
    let limit = limit.unwrap_or(10).min(25); // Cap at 25 to avoid too long messages
    let is_global = global.unwrap_or(false);
    let guild_id = ctx.guild_id();

    // Call the testable business logic function
    let (top_users, scope) = get_leaderboard(ctx.data(), guild_id, is_global, limit);

    if top_users.is_empty() {
        ctx.say("No one has any AndyCoins yet!").await?;
        return Ok(());
    }

    let mut response = format!("# {scope} AndyCoin Leaderboard\n");

    for (idx, (user_id, balance)) in top_users.iter().enumerate() {
        let rank = idx + 1;
        // Try to fetch the user info
        let username = match ctx.http().get_user(*user_id).await {
            Ok(user) => user.tag(),
            Err(_) => format!("User {user_id}"),
        };

        response.push_str(&format!("{rank}. **{username}**: {balance} AndyCoins\n"));
    }

    ctx.say(response).await?;

    // Log successful command execution
    logging::log_command(
        "leaderboard",
        ctx.guild_id().map(GuildId::get),
        ctx.author().id.get(),
        &args,
        true,
    );

    Ok(())
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
    fn test_get_leaderboard() {
        let data = Data::new();
        let guild1 = test_guild_id(1);
        let guild2 = test_guild_id(2);

        // Add balances for multiple users across different guilds
        data.add_coins(guild1, test_user_id(1), 100);
        data.add_coins(guild2, test_user_id(1), 50); // Same user in different guild
        data.add_coins(guild1, test_user_id(2), 50);
        data.add_coins(guild1, test_user_id(3), 75);
        data.add_coins(guild2, test_user_id(3), 70); // Same user in different guild

        // Test guild-specific leaderboard
        let (top_users, scope) = get_leaderboard(&data, Some(guild1), false, 3);
        assert_eq!(scope, "Server");
        assert_eq!(top_users.len(), 3);

        // The order might vary depending on how the DashMap iterates, so we'll just check that
        // the expected users and balances are present
        let has_user1 = top_users
            .iter()
            .any(|(id, bal)| *id == test_user_id(1) && *bal == 100);
        let has_user2 = top_users
            .iter()
            .any(|(id, bal)| *id == test_user_id(2) && *bal == 50);
        let has_user3 = top_users
            .iter()
            .any(|(id, bal)| *id == test_user_id(3) && *bal == 75);

        assert!(
            has_user1,
            "User 1 with balance 100 should be in the top users"
        );
        assert!(
            has_user2,
            "User 2 with balance 50 should be in the top users"
        );
        assert!(
            has_user3,
            "User 3 with balance 75 should be in the top users"
        );

        // Test global leaderboard
        let (top_users, scope) = get_leaderboard(&data, Some(guild1), true, 3);
        assert_eq!(scope, "Global");
        assert_eq!(top_users.len(), 3);
        assert_eq!(top_users[0].0, test_user_id(1));
        assert_eq!(top_users[0].1, 150); // 100 + 50
        assert_eq!(top_users[1].0, test_user_id(3));
        assert_eq!(top_users[1].1, 145); // 75 + 75

        // Test leaderboard in DM (should be global)
        let (top_users, scope) = get_leaderboard(&data, None, false, 3);
        assert_eq!(scope, "Global");
        assert_eq!(top_users.len(), 3);
    }
}
