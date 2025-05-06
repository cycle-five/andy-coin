use crate::{Context, Error, data::VoteConfig, logging};
use std::fmt::Write;

/// Vote decision options
#[derive(Debug, poise::ChoiceParameter)]
pub enum VoteDecision {
    #[name = "Start a new vote"]
    Start,
    #[name = "Vote yes"]
    Yes,
    #[name = "Vote no"]
    No,
}

/// Start a vote to reset all AndyCoins in the server or cast your vote
#[poise::command(slash_command, guild_only)]
pub async fn vote(
    ctx: Context<'_>,
    #[description = "Your vote decision"] decision: VoteDecision,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap();
    let user_id = ctx.author().id;

    match decision {
        VoteDecision::Yes => {
            match ctx.data().cast_vote(guild_id, user_id, true) {
                Ok(()) => {
                    ctx.say("You have voted YES on the current reset proposal.")
                        .await?;

                    // Log successful vote
                    logging::log_command(
                        "vote_cast",
                        Some(guild_id.get()),
                        ctx.author().id.get(),
                        "vote: YES",
                        true,
                    );
                }
                Err(e) => {
                    ctx.say(format!("Error: {e}")).await?;
                }
            }
        }
        VoteDecision::No => {
            match ctx.data().cast_vote(guild_id, user_id, false) {
                Ok(()) => {
                    ctx.say("You have voted NO on the current reset proposal.")
                        .await?;

                    // Log successful vote
                    logging::log_command(
                        "vote_cast",
                        Some(guild_id.get()),
                        ctx.author().id.get(),
                        "vote: NO",
                        true,
                    );
                }
                Err(e) => {
                    ctx.say(format!("Error: {e}")).await?;
                }
            }
        }
        VoteDecision::Start => {
            // Start a new vote
            match ctx.data().start_vote(guild_id, user_id) {
                Ok(end_time) => {
                    let vote_config = ctx.data().get_vote_config(guild_id);
                    let end_time_str = end_time.format("%H:%M:%S UTC");

                    let mut response = String::new();
                    writeln!(&mut response, "🗳️ **AndyCoin Reset Vote Started**")?;
                    writeln!(
                        &mut response,
                        "A vote to reset all AndyCoins in this server has been started by {}.",
                        ctx.author().name
                    )?;
                    writeln!(&mut response, "The vote will end at {end_time_str}.")?;
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
                        &format!("end_time: {end_time_str}"),
                        true,
                    );
                }
                Err(e) => {
                    ctx.say(format!("Error: {e}")).await?;
                }
            }
        }
    }

    Ok(())
}

/// Configure vote settings
#[poise::command(
    slash_command,
    guild_only,
    subcommands("status", "config"),
    subcommand_required
)]
pub async fn vote_admin(_: Context<'_>) -> Result<(), Error> {
    Ok(())
}

/// Check the status of the current vote
#[poise::command(slash_command, guild_only)]
pub async fn status(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap();
    let vote_config = ctx.data().get_vote_config(guild_id);

    // Check if the vote has expired
    if let Some(vote_passed) = ctx.data().check_vote_expiry(guild_id) {
        let result_str = if vote_passed {
            "The vote has ended and PASSED. All AndyCoins have been reset to 0."
        } else {
            "The vote has ended and FAILED. Not enough votes or majority not reached."
        };
        ctx.say(result_str).await?;
        return Ok(());
    }

    let vote_status = ctx.data().get_vote_status(guild_id);

    // Check if a vote is active
    if !vote_status.active {
        // Check if there's a cooldown
        if let Some(last_vote_time) = vote_status.last_vote_time {
            let cooldown_duration = chrono::Duration::hours(i64::from(vote_config.cooldown_hours));
            let now = chrono::Utc::now();

            if now < last_vote_time + cooldown_duration {
                let cooldown_end = last_vote_time + cooldown_duration;
                let cooldown_end_str = cooldown_end.format("%H:%M:%S UTC on %Y-%m-%d");

                ctx.say(format!("No active vote. A vote was recently completed. The next vote can be started at {cooldown_end_str}.")).await?;
                return Ok(());
            }
        }

        ctx.say("No active vote. Use `/vote` to start a new vote.")
            .await?;
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
        format!("<@{initiator_id}>")
    } else {
        "Unknown".to_string()
    };

    // Build response
    let mut response = String::new();
    writeln!(&mut response, "🗳️ **AndyCoin Reset Vote Status**")?;
    writeln!(&mut response, "Initiator: {initiator_str}")?;
    writeln!(&mut response, "End Time: {end_time_str}")?;
    writeln!(
        &mut response,
        "Votes: {yes_votes} YES / {no_votes} NO (Total: {total_votes})",
    )?;
    writeln!(
        &mut response,
        "Current YES Percentage: {yes_percentage:.1}%",
    )?;
    writeln!(
        &mut response,
        "Required: At least {} votes with {}% majority",
        vote_config.min_votes, vote_config.majority_percentage
    )?;

    // Check if the vote would pass with current numbers
    if total_votes >= vote_config.min_votes as usize {
        if yes_percentage >= f64::from(vote_config.majority_percentage) {
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
        &format!("yes: {yes_votes}, no: {no_votes}, total: {total_votes}"),
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
    #[description = "Percentage of YES votes required to pass (default: 70)"]
    majority_percentage: Option<u32>,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().expect("Guild ID not found");

    // Check if user has permission (server owner or admin)
    #[allow(deprecated)]
    let permissions = ctx
        .author_member()
        .await
        .unwrap()
        .permissions(ctx.cache())
        .unwrap();
    if !permissions.administrator() && ctx.author().id != ctx.guild().unwrap().owner_id {
        ctx.say("You need to be a server administrator to configure vote settings.")
            .await?;
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
            ctx.say("Majority percentage cannot be greater than 100%.")
                .await?;
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
    writeln!(
        &mut response,
        "Cooldown between votes: {cooldown_hours} hours",
    )?;
    writeln!(&mut response, "Vote duration: {duration_minutes} minutes")?;
    writeln!(&mut response, "Minimum votes required: {min_votes}")?;
    writeln!(&mut response, "Majority percentage required: {majority}%")?;

    ctx.say(response).await?;

    // Log config update
    logging::log_command(
        "vote_config",
        Some(guild_id.get()),
        ctx.author().id.get(),
        &format!(
            "cooldown: {cooldown_hours}, duration: {duration_minutes}, min_votes: {min_votes}, majority: {majority}"
        ),
        true,
    );

    // Save data to file
    if let Err(e) = ctx.data().save().await {
        ctx.say(format!("Warning: Failed to save settings: {e}"))
            .await?;
    }

    Ok(())
}

// The #[poise::command] macro automatically generates the necessary code
// to export these commands, so we don't need to manually define them.
