use crate::{Context, Error, logging};
use poise::serenity_prelude as serenity;

/// Set the giver role for a server
#[poise::command(slash_command, guild_only)]
pub async fn role(
    ctx: Context<'_>,
    #[description = "Role that can give AndyCoins"] role: Option<serenity::Role>,
) -> Result<(), Error> {
    let guild_id = match ctx.guild_id() {
        Some(id) => id,
        None => {
            ctx.say("This command can only be used in a server!")
                .await?;
            return Ok(());
        }
    };

    // Check if the command user is the server owner
    let is_owner = if let Some(guild) = ctx.guild() {
        guild.owner_id == ctx.author().id
    } else {
        false
    };

    if !is_owner {
        ctx.say("Only the server owner can set the giver role!")
            .await?;
        return Ok(());
    }

    let response;
    let role_name_for_log;

    if let Some(r) = role {
        // Set the giver role
        let role_name = r.name.clone();
        role_name_for_log = role_name.clone();
        ctx.data().set_giver_role(guild_id, Some(r.id));
        response = format!(
            "Set {} as the giver role. Users with this role can now give AndyCoins.",
            role_name
        );
    } else {
        // Clear the giver role
        role_name_for_log = "None".to_string();
        ctx.data().set_giver_role(guild_id, None);
        response =
            "Cleared the giver role. Only the server owner can give AndyCoins now.".to_string();
    }

    // Save the updated data
    ctx.data().save().await?;

    ctx.say(response).await?;

    // Log successful command execution
    logging::log_command(
        "role",
        Some(guild_id.get()),
        ctx.author().id.get(),
        &format!("role: {}", role_name_for_log),
        true,
    );

    Ok(())
}

/// Flip a coin
#[poise::command(slash_command, prefix_command)]
pub async fn flip(
    ctx: Context<'_>,
    #[description = "guess if you want"] guess: Option<String>,
    #[description = "bet you won't figure this out"] bet: Option<bool>,
) -> Result<(), Error> {
    let result = ctx.data().flip_coin();
    let result_str = if result { "heads" } else { "tails" };

    // If no guess is provided, just show the result
    if guess.is_none() && bet.is_none() {
        ctx.say(format!("The coin landed on **{}**!", result_str))
            .await?;

        // Log simple flip
        logging::log_command(
            "flip",
            ctx.guild_id().map(|id| id.get()),
            ctx.author().id.get(),
            &format!("result: {}", result_str),
            true,
        );

        return Ok(());
    }

    // If a guess is provided, check if it's correct
    if let Some(guess_str) = guess {
        let guess_lower = guess_str.to_lowercase();
        let is_heads = guess_lower == "heads" || guess_lower == "head" || guess_lower == "h";
        let is_tails = guess_lower == "tails" || guess_lower == "tail" || guess_lower == "t";

        if !is_heads && !is_tails {
            ctx.say("Invalid guess! Please use 'heads' or 'tails'.")
                .await?;
            return Ok(());
        }

        let guess_result = is_heads;

        // If bet flag is provided, this is a betting game
        if bet.unwrap_or(false) {
            let guild_id = match ctx.guild_id() {
                Some(id) => id,
                None => {
                    ctx.say("The betting game can only be played in a server!")
                        .await?;
                    return Ok(());
                }
            };

            let user_id = ctx.author().id;
            let current_balance = ctx.data().get_guild_balance(guild_id, user_id);

            if current_balance < 1 {
                ctx.say("You need at least 1 AndyCoin to play the betting game!")
                    .await?;
                return Ok(());
            }

            if guess_result == result {
                // Win: add a coin
                let new_balance = ctx.data().add_coins(guild_id, user_id, 1);
                ctx.say(format!("The coin landed on **{}**! You guessed correctly and won 1 AndyCoin! Your new balance is {} AndyCoins.", 
                               result_str, new_balance)).await?;
            } else {
                // Lose: remove a coin
                let new_balance = ctx.data().remove_coins(guild_id, user_id, 1);
                ctx.say(format!("The coin landed on **{}**! You guessed wrong and lost 1 AndyCoin. Your new balance is {} AndyCoins.", 
                               result_str, new_balance)).await?;
            }

            // Save the updated balances
            ctx.data().save().await?;

            // Log the bet result
            let outcome = if guess_result == result {
                "win"
            } else {
                "lose"
            };
            logging::log_command(
                "flip_bet",
                Some(guild_id.get()),
                ctx.author().id.get(),
                &format!(
                    "guess: {}, result: {}, outcome: {}",
                    if guess_result { "heads" } else { "tails" },
                    if result { "heads" } else { "tails" },
                    outcome
                ),
                true,
            );
        } else {
            // Regular guess without betting
            if guess_result == result {
                ctx.say(format!(
                    "The coin landed on **{}**! You guessed correctly!",
                    result_str
                ))
                .await?;
            } else {
                ctx.say(format!(
                    "The coin landed on **{}**! You guessed wrong.",
                    result_str
                ))
                .await?;
            }

            // Log the guess result
            let outcome = if guess_result == result {
                "correct"
            } else {
                "wrong"
            };
            logging::log_command(
                "flip_guess",
                ctx.guild_id().map(|id| id.get()),
                ctx.author().id.get(),
                &format!(
                    "guess: {}, result: {}, outcome: {}",
                    if guess_result { "heads" } else { "tails" },
                    if result { "heads" } else { "tails" },
                    outcome
                ),
                true,
            );
        }

        return Ok(());
    }

    Ok(())
}

/// Command to configure the bot. Uses a subcommand structure via poise.
#[poise::command(slash_command, subcommands("role"), owners_only)]
pub async fn config(ctx: Context<'_>) -> Result<(), Error> {
    ctx.say("Use one of the subcommands: role").await?;

    // Log command execution
    logging::log_command(
        "config",
        ctx.guild_id().map(|id| id.get()),
        ctx.author().id.get(),
        "help message displayed",
        true,
    );

    Ok(())
}

// /// Reset all AndyCoin data (server owner only)
// #[poise::command(slash_command, prefix_command, guild_only)]
// pub async fn reset(
//     ctx: Context<'_>,
// ) -> Result<(), Error> {
//     // Check if the command user is the server owner
//     let is_owner = if let Some(guild) = ctx.guild() {
//         guild.owner_id == ctx.author().id
//     } else {
//         false
//     };

//     if !is_owner {
//         ctx.say("Only the server owner can reset AndyCoin data!").await?;
//         return Ok(());
//     }

//     ctx.data().reset();
//     ctx.data().save().await?;
//     ctx.say("Reset all AndyCoin data.").await?;

//     Ok(())
// }
