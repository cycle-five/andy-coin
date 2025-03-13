use poise::serenity_prelude as serenity;

mod commands;
mod data;
mod logging;

pub use data::Data;

const DATA_FILE: &str = "andy_coin_data.yaml";

pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Context<'a> = poise::Context<'a, Data, Error>;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Initialize logging system
    logging::init()?;

    let token = std::env::var("DISCORD_TOKEN").expect("missing DISCORD_TOKEN");
    let intents = serenity::GatewayIntents::non_privileged();

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![
                commands::give::give(),
                commands::balance::balance(),
                commands::leaderboard::leaderboard(),
                commands::config::config(),
                commands::config::role(),
                commands::config::flip(),
                commands::vote::vote(),
                commands::vote::vote_admin(),
            ],
            ..Default::default()
        })
        .setup(|ctx, _ready, framework| {
            Box::pin(async move {
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                tracing::info!(
                    "Bot is ready! Registered {} commands.",
                    framework.options().commands.len()
                );

                // Load data from file
                let data = Data::load().await;
                Ok(data)
            })
        })
        .build();

    let mut client = serenity::ClientBuilder::new(token, intents)
        .framework(framework)
        .await
        .expect("Failed to create client");

    // Start the client
    tracing::info!("Starting bot...");
    if let Err(e) = client.start().await {
        tracing::error!("Client error: {}", e);
        return Err(e.into());
    }

    Ok(())
}
