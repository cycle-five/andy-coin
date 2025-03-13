use crate::{data::VoteConfig, logging, Context, Error};
use poise::serenity_prelude::{self as serenity, Guild};
use std::fmt::Write;

/// Start a vote to reset all AndyCoins in the server
#[poise::command(slash_command, guild_only)]
pub async fn vote(
    ctx: Context<'_>,
    #[description = "Vote yes or no (default: yes)"] vote_yes: Option<bool>,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap();
    let user_id = ctx.author().id;

    // If vote_yes is provided, cast a vote
    if let Some(vote_yes) = vote_yes {
        match ctx.data().cast_vote(guild_id, user_id, vote_yes) {
            Ok(_) => {
                let vote_type = if vote_yes { "YES" } else { "NO" };
                ctx.say(format!("You have voted {vote_type} on the current reset proposal."))
                    .await?;
                
                // Log successful vote
                logging::log_command(
                    "vote_cast",
                    Some(guild_id.get()),
                    ctx.author().id.get(),
                    &format!("vote: {}", vote_type),
                    true,
                );
            }
            Err(e) => {
                ctx.say(format!("Error: {e}")).await?;
            }
        }
        return Ok(());
    }

    // Otherwise, start a new vote
    match ctx.data().start_vote(guild_id, user_id) {
        Ok(end_time) => {
            let vote_config = ctx.data().get_vote_config(guild_id);
            let end_time_str = end_time.format("%H:%M:%S UTC");
            
            let mut response = String::new();
            writeln!(
                &mut response,
                "🗳️ **AndyCoin Reset Vote Started**"
            )?;
            writeln!(
                &mut response,
                "A vote to reset all AndyCoins in this server has been started by {}.",
                ctx.author().name
            )?;
            writeln!(
                &mut response,
                "The vote will end at {end_time_str}."
            )?;
            writeln!(
                &mut response,
                "Requirements: At least {} votes with {}% majority to pass.",
                vote_config.min_votes, vote_config.majority_percentage
            )?;
            writeln!(
                &mut response,
                "Use `/vote yes` to vote in favor or `/vote no` to vote against."
            )?;
            writeln!(
                &mut response,
                "⚠️ If the vote passes, all AndyCoins in this server will be reset to 0!"
            )?;
            
            ctx.say(response).await?;
            
            // Log successful vote start
            logging::log_command(
                "vote_start",
                Some(guild_id.get()),
                ctx.author().id.get(),
                &format!("end_time: {}", end_time_str),
                true,
            );
        }
        Err(e) => {
            ctx.say(format!("Error: {e}")).await?;
        }
    }

    Ok(())
}

/// Configure vote settings
#[poise::command(
    slash_command,
    guild_only,
    subcommands("status", "config", "end"),
    subcommand_required
)]
pub async fn vote_admin(_: Context<'_>) -> Result<(), Error> {
    Ok(())
}

/// Check the status of the current vote
#[poise::command(slash_command, guild_only)]
pub async fn status(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap();
    let vote_status = ctx.data().get_vote_status(guild_id);
    let vote_config = ctx.data().get_vote_config(guild_id);

    // Check if a vote is active
    if !vote_status.active {
        // Check if there's a cooldown
        if let Some(last_vote_time) = vote_status.last_vote_time {
            let cooldown_duration = chrono::Duration::hours(vote_config.cooldown_hours as i64);
            let now = chrono::Utc::now();
            
            if now < last_vote_time + cooldown_duration {
                let cooldown_end = last_vote_time + cooldown_duration;
                let cooldown_end_str = cooldown_end.format("%H:%M:%S UTC on %Y-%m-%d");
                
                ctx.say(format!(
                    "No active vote. A vote was recently completed. The next vote can be started at {}.",
                    cooldown_end_str
                )).await?;
                return Ok(());
            }
        }
        
        ctx.say("No active vote. Use `/vote` to start a new vote.").await?;
        return Ok(());
    }

    // Get vote information
    let yes_votes = vote_status.yes_votes.len();
    let no_votes = vote_status.no_votes.len();
    let total_votes = yes_votes + no_votes;
    let yes_percentage = if total_votes > 0 {
        (yes_votes as f64 / total_votes as f64) * 100.0
    } else {
        0.0
    };

    // Format end time
    let end_time_str = if let Some(end_time) = vote_status.end_time {
        end_time.format("%H:%M:%S UTC").to_string()
    } else {
        "Unknown".to_string()
    };

    // Format initiator
    let initiator_str = if let Some(initiator_id) = vote_status.initiator_id {
        format!("<@{}>", initiator_id)
    } else {
        "Unknown".to_string()
    };

    // Build response
    let mut response = String::new();
    writeln!(&mut response, "🗳️ **AndyCoin Reset Vote Status**")?;
    writeln!(&mut response, "Initiator: {}", initiator_str)?;
    writeln!(&mut response, "End Time: {}", end_time_str)?;
    writeln!(&mut response, "Votes: {} YES / {} NO (Total: {})", yes_votes, no_votes, total_votes)?;
    writeln!(&mut response, "Current YES Percentage: {:.1}%", yes_percentage)?;
    writeln!(&mut response, "Required: At least {} votes with {}% majority", vote_config.min_votes, vote_config.majority_percentage)?;
    
    // Check if the vote would pass with current numbers
    if total_votes >= vote_config.min_votes as usize {
        if yes_percentage >= vote_config.majority_percentage as f64 {
            writeln!(&mut response, "Status: Would PASS with current votes")?;
        } else {
            writeln!(&mut response, "Status: Would FAIL with current votes")?;
        }
    } else {
        writeln!(&mut response, "Status: Not enough votes yet")?;
    }

    ctx.say(response).await?;
    
    // Log status check
    logging::log_command(
        "vote_status",
        Some(guild_id.get()),
        ctx.author().id.get(),
        &format!("yes: {}, no: {}, total: {}", yes_votes, no_votes, total_votes),
        true,
    );
    Ok(())
}

/// Configure vote settings
#[poise::command(slash_command, guild_only)]
pub async fn config(
    ctx: Context<'_>,
    #[description = "Cooldown hours between votes (default: 24)"] cooldown_hours: Option<u32>,
    #[description = "Duration of voting in minutes (default: 30)"] duration_minutes: Option<u32>,
    #[description = "Minimum number of votes required (default: 10)"] min_votes: Option<u32>,
    #[description = "Percentage of YES votes required to pass (default: 70)"] majority_percentage: Option<u32>,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap();
    
    // Check if user has permission (server owner or admin)
    let permissions = ctx.author_member().await.unwrap().permissions(ctx.cache()).unwrap();
    if !permissions.administrator() && ctx.author().id != ctx.guild().unwrap().owner_id {
        ctx.say("You need to be a server administrator to configure vote settings.").await?;
        return Ok(());
    }
    
    // Get current config
    let mut vote_config = ctx.data().get_vote_config(guild_id);
    
    // Update config with provided values
    if let Some(hours) = cooldown_hours {
        vote_config.cooldown_hours = hours;
    }
    
    if let Some(minutes) = duration_minutes {
        vote_config.duration_minutes = minutes;
    }
    
    if let Some(votes) = min_votes {
        vote_config.min_votes = votes;
    }
    
    if let Some(percentage) = majority_percentage {
        if percentage > 100 {
            ctx.say("Majority percentage cannot be greater than 100%.").await?;
            return Ok(());
        }
        vote_config.majority_percentage = percentage;
    }
    
    // Save the updated config
    ctx.data().set_vote_config(guild_id, &vote_config);

    
    let VoteConfig {
        cooldown_hours,
        duration_minutes,
        min_votes,
        majority_percentage: majority,
    } = vote_config.clone();
    
    // Build response
    let mut response = String::new();
    writeln!(&mut response, "✅ **Vote Settings Updated**")?;
    writeln!(&mut response, "Cooldown between votes: {} hours", cooldown_hours)?;
    writeln!(&mut response, "Vote duration: {} minutes", duration_minutes)?;
    writeln!(&mut response, "Minimum votes required: {}", min_votes)?;
    writeln!(&mut response, "Majority percentage required: {}%", majority)?;
    
    ctx.say(response).await?;

    // Log config update
    logging::log_command(
        "vote_config",
        Some(guild_id.get()),
        ctx.author().id.get(),
        &format!(
            "cooldown: {}, duration: {}, min_votes: {}, majority: {}",
            cooldown_hours,
            duration_minutes,
            min_votes,
            majority
        ),
        true,
    );
    
    // Save data to file
    if let Err(e) = ctx.data().save().await {
        ctx.say(format!("Warning: Failed to save settings: {}", e)).await?;
    }
    
    Ok(())
}

/// Force end the current vote (admin only)
#[poise::command(slash_command, guild_only)]
pub async fn end(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap();
    
    // Check if user has permission (server owner or admin)
    let permissions = ctx.author_member().await.unwrap().permissions(ctx.cache()).unwrap();
    if !permissions.administrator() && ctx.author().id != ctx.guild().unwrap().owner_id {
        ctx.say("You need to be a server administrator to force end a vote.").await?;
        return Ok(());
    }
    
    // End the vote
    match ctx.data().end_vote(guild_id) {
        Ok(vote_passed) => {
            let result = if vote_passed {
                "PASSED"
            } else {
                "FAILED"
            };
            
            ctx.say(format!("Vote has been force ended by an administrator. Result: {result}")).await?;
            
            // Log vote end
            logging::log_command(
                "vote_end",
                Some(guild_id.get()),
                ctx.author().id.get(),
                &format!("result: {}", result),
                true,
            );
        }
        Err(e) => {
            ctx.say(format!("Error: {e}")).await?;
        }
    }
    
    Ok(())
}

// The #[poise::command] macro automatically generates the necessary code
// to export these commands, so we don't need to manually define them.
