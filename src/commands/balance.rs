use crate::{Context, Data, Error, logging};
use poise::serenity_prelude as serenity;

// Core business logic for checking balance
pub fn get_balance(
    data: &Data,
    user_id: serenity::UserId,
    opt_guild_id: Option<serenity::GuildId>,
    is_global: bool,
) -> u32 {
    if is_global {
        data.get_total_balance(user_id)
    } else if let Some(guild_id) = opt_guild_id {
        data.get_guild_balance(guild_id, user_id)
    } else {
        data.get_total_balance(user_id)
    }
}

/// Check your AndyCoin balance or another user's balance
#[poise::command(slash_command, prefix_command)]
pub async fn balance(
    ctx: Context<'_>,
    #[description = "User to check balance for (defaults to yourself)"] user: Option<
        serenity::User,
    >,
    #[description = "Show total balance across all servers (default: current server only)"] global: Option<bool>,
) -> Result<(), Error> {
    // Format arguments for logging
    let user_arg = user.as_ref().map_or("self".to_string(), |u| u.tag());
    let global_arg = global.unwrap_or(false).to_string();
    let args = format!("user: {}, global: {}", user_arg, global_arg);
    let target_user = user.as_ref().unwrap_or_else(|| ctx.author());
    let is_global = global.unwrap_or(false);
    let guild_id = ctx.guild_id();

    // Call the testable business logic function
    let balance = get_balance(ctx.data(), target_user.id, guild_id, is_global);

    let scope = if is_global || guild_id.is_none() {
        "across all servers"
    } else {
        "in this server"
    };

    let response = if target_user.id == ctx.author().id {
        format!("You have {} AndyCoins {}.", balance, scope)
    } else {
        format!("{} has {} AndyCoins {}.", target_user.tag(), balance, scope)
    };

    ctx.say(response).await?;

    // Log successful command execution
    logging::log_command(
        "balance",
        ctx.guild_id().map(|id| id.get()),
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
    fn test_get_balance() {
        let data = Data::new();
        let guild1 = test_guild_id(1);
        let guild2 = test_guild_id(2);
        let user_id = test_user_id(123);

        // Add coins in different guilds
        data.add_coins(guild1, user_id, 50);
        data.add_coins(guild2, user_id, 30);

        // Test guild-specific balance
        let balance = get_balance(&data, user_id, Some(guild1), false);
        assert_eq!(balance, 50);

        // Test global balance
        let balance = get_balance(&data, user_id, Some(guild1), true);
        assert_eq!(balance, 80);

        // Test balance in DM (should be global)
        let balance = get_balance(&data, user_id, None, false);
        assert_eq!(balance, 80);
    }
}
