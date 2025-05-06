//! Audit tool for `AndyCoin` bot
//!
//! This tool helps analyze logs and track balance changes.

use serde_json::Value;
use std::collections::HashMap;
use std::env;
use std::fs::{self, File};
use std::io::{self, BufRead, BufReader};
use std::path::Path;

// Define log entry types
#[derive(Debug)]
enum LogEntryType {
    Command {
        timestamp: String,
        command: String,
        guild_id: String,
        user_id: String,
        arguments: String,
        result: String,
    },
    Balance {
        timestamp: String,
        guild_id: String,
        user_id: String,
        previous_balance: u32,
        new_balance: u32,
        change: i64,
        reason: String,
        initiator: String,
    },
}

fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        print_usage();
        return Ok(());
    }

    let command = &args[1];

    match command.as_str() {
        "user-commands" => {
            if args.len() < 3 {
                println!("Error: Missing user ID");
                print_usage();
                return Ok(());
            }
            let user_id = &args[2];
            list_user_commands(user_id)?;
        }
        "user-balances" => {
            if args.len() < 3 {
                println!("Error: Missing user ID");
                print_usage();
                return Ok(());
            }
            let user_id = &args[2];
            list_user_balances(user_id)?;
        }
        "balance-summary" => {
            balance_summary()?;
        }
        "help" => {
            print_usage();
        }
        _ => {
            println!("Unknown command: {command}");
            print_usage();
        }
    }

    Ok(())
}

fn print_usage() {
    println!("AndyCoin Audit Tool");
    println!("Usage:");
    println!("  audit user-commands <user_id>   - List all commands executed by a user");
    println!("  audit user-balances <user_id>   - List all balance changes for a user");
    println!("  audit balance-summary           - Show a summary of all balance changes");
    println!("  audit help                      - Show this help message");
}

fn list_user_commands(user_id: &str) -> io::Result<()> {
    println!("Commands executed by user {user_id}:");
    println!(
        "{:<27} {:<20} {:<15} {:<40} {:<10}",
        "Timestamp", "Command", "Guild", "Arguments", "Result"
    );
    println!("{}", "-".repeat(100));

    let log_entries = parse_command_logs()?;

    let mut found = false;
    for entry in log_entries {
        if let LogEntryType::Command {
            timestamp,
            command,
            guild_id,
            user_id: entry_user_id,
            arguments,
            result,
        } = entry
        {
            if entry_user_id == user_id {
                found = true;
                println!(
                    "{:<27} {:<20} {:<15} {:<40} {:<10}",
                    timestamp,
                    command,
                    guild_id,
                    // Truncate arguments if too long
                    if arguments.len() > 40 {
                        format!("{}...", &arguments[..37])
                    } else {
                        arguments
                    },
                    result
                );
            }
        }
    }

    if !found {
        println!("No commands found for user {user_id}");
    }

    Ok(())
}

fn list_user_balances(user_id: &str) -> io::Result<()> {
    println!("Balance changes for user {user_id}:");
    println!(
        "{:<27} {:<20} {:<15} {:<15} {:<10} {:<20} {:<15}",
        "Timestamp", "Guild", "Previous", "New", "Change", "Reason", "Initiator"
    );
    println!("{}", "-".repeat(110));

    let log_entries = parse_balance_logs()?;

    let mut found = false;
    for entry in log_entries {
        if let LogEntryType::Balance {
            timestamp,
            guild_id,
            user_id: entry_user_id,
            previous_balance,
            new_balance,
            change,
            reason,
            initiator,
        } = entry
        {
            if entry_user_id == user_id {
                found = true;
                println!(
                    "{:<27} {:<20} {:<15} {:<15} {:<+10} {:<20} {:<15}",
                    timestamp,
                    guild_id,
                    previous_balance,
                    new_balance,
                    change,
                    // Truncate reason if too long
                    if reason.len() > 20 {
                        format!("{}...", &reason[..17])
                    } else {
                        reason
                    },
                    initiator
                );
            }
        }
    }

    if !found {
        println!("No balance changes found for user {user_id}");
    }

    Ok(())
}

/// Print a summary of balance changes grouped by guild and user
/// TODO: Have this be a summary of all balance changes.
fn balance_summary() -> io::Result<()> {
    println!("Balance Change Summary:");

    let log_entries = parse_balance_logs()?;

    // Group by guild and user
    let mut guild_user_changes: HashMap<String, HashMap<String, i64>> = HashMap::new();
    let mut user_totals: HashMap<String, i64> = HashMap::new();

    for entry in log_entries {
        if let LogEntryType::Balance {
            guild_id,
            user_id,
            change,
            ..
        } = entry
        {
            // Update guild-user map
            let guild_map = guild_user_changes.entry(guild_id).or_default();
            *guild_map.entry(user_id.clone()).or_insert(0) += change;

            // Update user totals
            *user_totals.entry(user_id).or_insert(0) += change;
        }
    }

    // Print guild summaries
    for (guild_id, user_map) in guild_user_changes {
        println!("\nGuild: {guild_id}");
        println!("{:<20} {:<15}", "User ID", "Net Change");
        println!("{}", "-".repeat(35));

        let mut users: Vec<(&String, &i64)> = user_map.iter().collect();
        users.sort_by(|a, b| b.1.cmp(a.1)); // Sort by change (descending)

        for (user_id, change) in users {
            println!("{user_id:<20} {change:<+15}");
        }
    }

    // Print global summary
    println!("\nGlobal Summary:");
    println!("{:<20} {:<15}", "User ID", "Net Change");
    println!("{}", "-".repeat(35));

    let mut users: Vec<(&String, &i64)> = user_totals.iter().collect();
    users.sort_by(|a, b| b.1.cmp(a.1)); // Sort by change (descending)

    for (user_id, change) in users {
        println!("{user_id:<20} {change:<+15}");
    }

    Ok(())
}

fn parse_command_logs() -> io::Result<Vec<LogEntryType>> {
    let log_dir = "logs";
    let command_log_pattern = "commands";

    let mut entries = Vec::new();

    // Create logs directory if it doesn't exist
    if !Path::new(log_dir).exists() {
        println!("No logs directory found. Creating one...");
        fs::create_dir_all(log_dir)?;
        return Ok(entries);
    }

    // Find all command log files
    if let Ok(files) = fs::read_dir(log_dir) {
        #[allow(clippy::manual_flatten)]
        for file_result in files {
            if let Ok(file) = file_result {
                let file_name = file.file_name().to_string_lossy().to_string();
                if file_name.starts_with(command_log_pattern) {
                    let file_path = Path::new(log_dir).join(file.file_name());
                    let file = match File::open(&file_path) {
                        Ok(f) => f,
                        Err(e) => {
                            eprintln!("Error opening file {}: {}", file_path.display(), e);
                            continue;
                        }
                    };
                    let reader = BufReader::new(file);

                    #[allow(clippy::manual_flatten)]
                    for line in reader.lines() {
                        if let Ok(line_content) = line {
                            if let Ok(json) = serde_json::from_str::<Value>(&line_content) {
                                if let Some(timestamp) =
                                    json.get("timestamp").and_then(|v| v.as_str())
                                {
                                    if let Some(fields) = json.get("fields") {
                                        if let (
                                            Some(command),
                                            Some(guild_id),
                                            Some(user_id),
                                            Some(arguments),
                                            Some(result),
                                        ) = (
                                            fields.get("command").and_then(|v| v.as_str()),
                                            fields.get("guild_id").and_then(|v| v.as_str()),
                                            fields.get("user_id").and_then(|v| v.as_str()),
                                            fields.get("arguments").and_then(|v| v.as_str()),
                                            fields.get("result").and_then(|v| v.as_str()),
                                        ) {
                                            entries.push(LogEntryType::Command {
                                                timestamp: timestamp.to_string(),
                                                command: command.to_string(),
                                                guild_id: guild_id.to_string(),
                                                user_id: user_id.to_string(),
                                                arguments: arguments.to_string(),
                                                result: result.to_string(),
                                            });
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(entries)
}

fn parse_balance_logs() -> io::Result<Vec<LogEntryType>> {
    let log_dir = "logs";
    let balance_log_pattern = "balances";

    let mut entries = Vec::new();

    // Create logs directory if it doesn't exist
    if !Path::new(log_dir).exists() {
        println!("No logs directory found. Creating one...");
        fs::create_dir_all(log_dir)?;
        return Ok(entries);
    }

    // Find all balance log files
    if let Ok(files) = fs::read_dir(log_dir) {
        for file in files.flatten() {
            let file_name = file.file_name().to_string_lossy().to_string();
            if file_name.starts_with(balance_log_pattern) {
                let file_path = Path::new(log_dir).join(file.file_name());
                let file = match File::open(&file_path) {
                    Ok(f) => f,
                    Err(e) => {
                        eprintln!("Error opening file {}: {}", file_path.display(), e);
                        continue;
                    }
                };
                let reader = BufReader::new(file);

                #[allow(clippy::manual_flatten)]
                #[allow(clippy::cast_possible_truncation)]
                for line in reader.lines() {
                    if let Ok(line_content) = line {
                        if let Ok(json) = serde_json::from_str::<Value>(&line_content) {
                            if let Some(timestamp) = json.get("timestamp").and_then(|v| v.as_str())
                            {
                                if let Some(fields) = json.get("fields") {
                                    if let (
                                        Some(guild_id),
                                        Some(user_id),
                                        Some(previous_balance),
                                        Some(new_balance),
                                        Some(change),
                                        Some(reason),
                                        Some(initiator),
                                    ) = (
                                        fields.get("guild_id").and_then(serde_json::Value::as_str),
                                        fields.get("user_id").and_then(serde_json::Value::as_str),
                                        fields
                                            .get("previous_balance")
                                            .and_then(serde_json::Value::as_u64),
                                        fields
                                            .get("new_balance")
                                            .and_then(serde_json::Value::as_u64),
                                        fields.get("change").and_then(serde_json::Value::as_i64),
                                        fields.get("reason").and_then(serde_json::Value::as_str),
                                        fields.get("initiator").and_then(serde_json::Value::as_str),
                                    ) {
                                        entries.push(LogEntryType::Balance {
                                            timestamp: timestamp.to_string(),
                                            guild_id: guild_id.to_string(),
                                            user_id: user_id.to_string(),
                                            previous_balance: previous_balance as u32,
                                            new_balance: new_balance as u32,
                                            change,
                                            reason: reason.to_string(),
                                            initiator: initiator.to_string(),
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(entries)
}
