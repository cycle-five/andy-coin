use std::path::Path;
use tracing::info;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{
    EnvFilter,
    fmt::{self, format::FmtSpan},
    layer::SubscriberExt,
    util::SubscriberInitExt,
};

/// Log directory name
pub const LOG_DIR: &str = "logs";
/// Command log file name
pub const COMMAND_LOG_FILE: &str = "commands";
/// Balance log file name
pub const BALANCE_LOG_FILE: &str = "balances";

/// Initialize the logging system with console and file outputs
pub fn init() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Create log directory if it doesn't exist
    if !Path::new(LOG_DIR).exists() {
        std::fs::create_dir_all(LOG_DIR)?;
    }

    // Set up file appenders with daily rotation
    let command_file = RollingFileAppender::new(Rotation::DAILY, LOG_DIR, COMMAND_LOG_FILE);
    let balance_file = RollingFileAppender::new(Rotation::DAILY, LOG_DIR, BALANCE_LOG_FILE);

    // Create a layer for console output (human-readable format)
    let console_layer = fmt::layer()
        .with_span_events(FmtSpan::CLOSE)
        .with_target(true)
        .with_ansi(true);

    // Create a layer for command logs (JSON format)
    let command_layer = fmt::layer()
        .with_span_events(FmtSpan::CLOSE)
        .with_target(true)
        .with_ansi(false)
        .json()
        .with_writer(command_file);

    // Create a layer for balance logs (JSON format)
    let balance_layer = fmt::layer()
        .with_span_events(FmtSpan::CLOSE)
        .with_target(true)
        .with_ansi(false)
        .json()
        .with_writer(balance_file);

    // Set up the subscriber with all layers
    // Use env filter to allow runtime configuration of log levels
    // Default to INFO level if not specified
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    tracing_subscriber::registry()
        .with(env_filter)
        .with(console_layer)
        .with(command_layer)
        .with(balance_layer)
        .init();

    info!("Logging system initialized");
    Ok(())
}

/// Log a command execution
pub fn log_command(
    command_name: &str,
    guild_id: Option<u64>,
    user_id: u64,
    args: &str,
    success: bool,
) {
    let guild_id_str = guild_id
        .map(|id| id.to_string())
        .unwrap_or_else(|| "DM".to_string());

    if success {
        info!(
            target: "command",
            command = command_name,
            guild_id = guild_id_str,
            user_id = user_id.to_string(),
            arguments = args,
            result = "success",
            "Command executed successfully"
        );
    } else {
        info!(
            target: "command",
            command = command_name,
            guild_id = guild_id_str,
            user_id = user_id.to_string(),
            arguments = args,
            result = "failure",
            "Command execution failed"
        );
    }
}

/// Log a balance change
pub fn log_balance_change(
    guild_id: u64,
    user_id: u64,
    previous_balance: u32,
    new_balance: u32,
    reason: &str,
    initiator_id: Option<u64>,
) {
    let change = i64::from(new_balance) - i64::from(previous_balance);
    let initiator = initiator_id
        .map(|id| id.to_string())
        .unwrap_or_else(|| "System".to_string());

    info!(
        target: "balance",
        guild_id = guild_id.to_string(),
        user_id = user_id.to_string(),
        previous_balance = previous_balance,
        new_balance = new_balance,
        change = change,
        reason = reason,
        initiator = initiator,
        "Balance changed"
    );
}
