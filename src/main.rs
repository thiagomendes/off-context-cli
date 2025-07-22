use anyhow::Result;
use clap::{Parser, Subcommand};
use tracing::info;
use std::io::{self, Read};
use futures::executor::block_on;

mod commands;
mod core;

use commands::*;

#[derive(Parser)]
#[command(
    name = "off-context",
    about = "Claude Code Memory System using Official Hooks",
    version,
    long_about = "Gives Claude Code perfect memory by automatically capturing and injecting context from past conversations."
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Enable verbose logging
    #[arg(short, long, global = true)]
    verbose: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// One-time setup: configure Claude Code hooks automatically
    Setup {
        /// Force reconfiguration even if already set up
        #[arg(short, long)]
        force: bool,
    },

    /// Show memory system status and statistics
    Status,

    /// Search conversation history manually
    Search {
        /// Query to search for
        query: String,
        /// Maximum number of results
        #[arg(short, long, default_value = "5")]
        limit: usize,
    },

    /// Reset/clear all stored memory
    Reset {
        /// Skip confirmation prompt
        #[arg(short, long)]
        yes: bool,
    },

    /// Import existing Claude Code conversations
    Import {
        /// Path to Claude Code conversation files
        #[arg(short, long)]
        path: Option<String>,
    },

    /// Export conversation history
    Export {
        /// Output format (json, md)
        #[arg(short, long, default_value = "md")]
        format: String,
        /// Output file path
        #[arg(short, long)]
        output: Option<String>,
    },

    /// Process Claude Code transcript (internal command)
    #[command(hide = true)]
    Hook {
        /// Path to Claude Code transcript file
        transcript_path: String,
    },

    /// Inject context for query (internal command)
    #[command(hide = true)]
    Inject {
        /// User query to enhance with context
        query: String,
    },

    /// Initialize local project integration with off-context (creates/updates .claude/settings.local.json)
    Init,

    /// Remove hooks from current project (.claude/settings.local.json)
    Clear,

    /// Remove global hooks and off-context global memory
    Uninstall,

    /// Inject context into Claude Code JSON prompt
    SmartInject,

    /// Process UserPromptSubmit hook JSON (official format)
    InjectPrompt,

    /// Start web admin interface
    Admin {
        /// Port to bind the server to
        #[arg(short, long, default_value = "8080")]
        port: u16,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize logging
    init_logging(cli.verbose)?;

    // Handle commands
    match cli.command {
        Some(Commands::Setup { force }) => {
            info!("Starting setup process...");
            setup::handle_setup(force).await
        }
        Some(Commands::Status) => {
            status::handle_status().await
        }
        Some(Commands::Search { query, limit }) => {
            search::handle_search(&query, limit).await
        }
        Some(Commands::Reset { yes }) => {
            reset::handle_reset(yes).await
        }
        Some(Commands::Import { path }) => {
            import::handle_import(path.as_deref()).await
        }
        Some(Commands::Export { format, output }) => {
            export::handle_export(&format, output.as_deref()).await
        }
        Some(Commands::Hook { transcript_path }) => {
            hook::handle_hook(&transcript_path).await
        }
        Some(Commands::Inject { query }) => {
            inject::handle_inject(&query).await
        }
        Some(Commands::Init) => {
            commands::init::handle_init().await
        }
        Some(Commands::Clear) => {
            commands::clear::handle_clear().await
        }
        Some(Commands::Uninstall) => {
            commands::init::handle_uninstall().await
        }
        Some(Commands::SmartInject) => {
            // Read JSON from stdin
            let mut buffer = String::new();
            io::stdin().read_to_string(&mut buffer).expect("Failed to read stdin");
            let mut json: serde_json::Value = serde_json::from_str(&buffer).expect("Invalid JSON input");

            // Extract session_id and check if it's a new session
            let session_id = json.get("session_id").and_then(|v| v.as_str()).unwrap_or("");
            let is_new_session = if !session_id.is_empty() {
                if let Ok(config_dir) = crate::core::config::project_config_dir() {
                    std::fs::create_dir_all(&config_dir).ok();
                    let session_file = config_dir.join(format!("session_injected_{}", session_id));
                    let already_injected = session_file.exists();
                    
                    if !already_injected {
                        // Mark this session as having received context
                        std::fs::write(&session_file, "").ok();
                        true
                    } else {
                        false
                    }
                } else {
                    true
                }
            } else {
                true
            };

            // Extract prompt
            let prompt_text = json.get("prompt").and_then(|v| v.as_str()).unwrap_or("");
            if prompt_text.is_empty() {
                println!("{}", buffer);
                return Ok(());
            }

            // Inject context if new session, otherwise just pass through
            let enhanced_prompt = if is_new_session {
                // Use existing inject logic (now async)
                match block_on(crate::commands::inject::inject_context_internal(prompt_text)) {
                    Ok(ctx) => ctx,
                    Err(_) => prompt_text.to_string(),
                }
            } else {
                prompt_text.to_string()
            };
            json["prompt"] = serde_json::Value::String(enhanced_prompt);
            println!("{}", serde_json::to_string(&json).unwrap());
            Ok(())
        }
        Some(Commands::InjectPrompt) => {
            // Process standard UserPromptSubmit hook JSON
            let mut buffer = String::new();
            io::stdin().read_to_string(&mut buffer).expect("Failed to read stdin");
            let json: serde_json::Value = serde_json::from_str(&buffer).expect("Invalid JSON input");

            // Extract standard hook fields
            let session_id = json.get("session_id").and_then(|v| v.as_str()).unwrap_or("");
            let prompt_text = json.get("prompt").and_then(|v| v.as_str()).unwrap_or("");
            
            if prompt_text.is_empty() {
                println!("{}", prompt_text);
                return Ok(());
            }

            // Check if we already injected context for this session  
            let is_new_session = if !session_id.is_empty() {
                if let Ok(config_dir) = crate::core::config::project_config_dir() {
                    std::fs::create_dir_all(&config_dir).ok();
                    let session_file = config_dir.join(format!("session_injected_{}", session_id));
                    let already_injected = session_file.exists();
                    
                    if !already_injected {
                        // Mark this session as having received context
                        std::fs::write(&session_file, "").ok();
                        true
                    } else {
                        false
                    }
                } else {
                    true
                }
            } else {
                true
            };

            // Inject context only if new session
            if is_new_session {
                match block_on(crate::commands::inject::inject_context_simple(prompt_text)) {
                    Ok(enhanced) => println!("{}", enhanced),
                    Err(_) => println!("{}", prompt_text),
                }
            } else {
                println!("{}", prompt_text);
            }
            Ok(())
        }
        Some(Commands::Admin { port }) => {
            admin::handle_admin(port).await
        }
        None => {
            // No subcommand provided - show help
            println!("off-context: Claude Code Memory System");
            println!();
            println!("Quick start:");
            println!("  off-context setup    # One-time configuration");
            println!("  # Then just use Claude Code normally!");
            println!();
            println!("Management:");
            println!("  off-context status   # Check what's remembered");
            println!("  off-context search   # Search conversation history");
            println!();
            println!("Use 'off-context --help' for more information.");
            Ok(())
        }
    }
}

fn init_logging(verbose: bool) -> Result<()> {
    use tracing_subscriber::{fmt, EnvFilter};

    let level = if verbose { "debug" } else { "warn" };
    
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(level));

    fmt()
        .with_env_filter(filter)
        .with_target(false)
        .init();

    Ok(())
}