use poise::serenity_prelude as serenity;
use crate::{Context, Error, Data};

// Core business logic for giving coins
pub fn give_coins(
    data: &Data,
    guild_id: serenity::GuildId,
    user_id: serenity::UserId,
    amount: u32,
) -> u32 {
    data.add_coins(guild_id, user_id, amount)
}

/// Give AndyCoins to a user (server owner only)
#[poise::command(slash_command, prefix_command, guild_only)]
pub async fn give(
    ctx: Context<'_>,
    #[description = "Amount of AndyCoins to give"] amount: u32,
    #[description = "User to give the AndyCoins to"] user: serenity::User,
) -> Result<(), Error> {
    let guild_id = match ctx.guild_id() {
        Some(id) => id,
        None => {
            ctx.say("This command can only be used in a server!").await?;
            return Ok(());
        }
    };
    
    // Get the member who is giving coins
    let member = match ctx.author_member().await {
        Some(member) => member,
        None => {
            ctx.say("Failed to get your member information.").await?;
            return Ok(());
        }
    };
    
    // Check if the user has permission to give coins
    if !ctx.data().has_giver_role(guild_id, &member) {
        ctx.say("You don't have permission to give AndyCoins! Only the server owner or users with the giver role can do this.").await?;
        return Ok(());
    }

    // Call the testable business logic function
    let new_balance = give_coins(ctx.data(), guild_id, user.id, amount);
    
    // Save the updated balances
    ctx.data().save().await?;
    
    let response = format!("Gave {} AndyCoins to {}. Their new balance in this server is {} AndyCoins.", 
                          amount, user.tag(), new_balance);
    ctx.say(response).await?;
    
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
        
        // Test giving coins
        let new_balance = give_coins(&data, guild_id, user_id, 50);
        assert_eq!(new_balance, 50);
        
        // Test giving more coins
        let new_balance = give_coins(&data, guild_id, user_id, 25);
        assert_eq!(new_balance, 75);
    }
}
