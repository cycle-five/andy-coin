use crate::{Context, Data, Error, logging};
use poise::serenity_prelude::{self as serenity, UserId};

/// Core business logic for giving coins
pub fn give_coins(
    data: &Data,
    guild_id: serenity::GuildId,
    user_id: serenity::UserId,
    amount: u32,
    initiator_id: Option<serenity::UserId>,
) -> u32 {
    // Get the previous balance
    let previous_balance = data.get_guild_balance(guild_id, user_id);

    // Add coins
    let guild_map = data
        .guild_balances
        .entry(guild_id)
        .or_insert_with(dashmap::DashMap::new);

    let new_balance = *guild_map
        .entry(user_id)
        .and_modify(|bal| *bal += amount)
        .or_insert(amount);

    // Log the balance change
    logging::log_balance_change(
        guild_id.get(),
        user_id.get(),
        previous_balance,
        new_balance,
        "give_command",
        initiator_id.map(UserId::get),
    );

    new_balance
}

/// Give AndyCoins to a user (server owner only)
#[poise::command(slash_command, prefix_command, guild_only)]
pub async fn give(
    ctx: Context<'_>,
    #[description = "Amount of AndyCoins to give"] amount: u32,
    #[description = "User to give the AndyCoins to"] user: serenity::User,
) -> Result<(), Error> {
    // Log command execution
    let args = format!("amount: {amount}, user: {}", user.tag());
    let guild_id = if let Some(id) = ctx.guild_id() {
        id
    } else {
        ctx.say("This command can only be used in a server!")
            .await?;
        return Ok(());
    };

    // Get the member who is giving coins
    let member = if let Some(member) = ctx.author_member().await {
        member
    } else {
        ctx.say("Failed to get your member information.").await?;
        return Ok(());
    };

    // Check if the user has permission to give coins
    if !ctx.data().has_giver_role(guild_id, &member) {
        ctx.say("You don't have permission to give AndyCoins! Only the server owner or users with the giver role can do this.").await?;
        return Ok(());
    }

    // Call the testable business logic function
    let new_balance = give_coins(ctx.data(), guild_id, user.id, amount, Some(ctx.author().id));

    // Save the updated balances
    ctx.data().save().await?;

    let response = format!(
        "Gave {amount} AndyCoins to {}. Their new balance in this server is {new_balance} AndyCoins.",
        user.tag(),
    );
    ctx.say(response).await?;

    // Log successful command execution
    logging::log_command(
        "give",
        Some(guild_id.get()),
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
    fn test_give_coins() {
        let data = Data::new();
        let guild_id = test_guild_id(1);
        let user_id = test_user_id(123);
        let initiator_id = test_user_id(456);

        // Test giving coins
        let new_balance = give_coins(&data, guild_id, user_id, 50, Some(initiator_id));
        assert_eq!(new_balance, 50);

        // Test giving more coins
        let new_balance = give_coins(&data, guild_id, user_id, 25, Some(initiator_id));
        assert_eq!(new_balance, 75);
    }
}
