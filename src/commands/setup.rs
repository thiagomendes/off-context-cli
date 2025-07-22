use anyhow::{Context, Result};
use std::process::Command;
use tracing::{info, warn};

use crate::core::{
    config::{claude_code_hooks_dir, config_dir, init_config},
    memory::Memory,
};

/// Handle the setup command - configure Claude Code hooks
pub async fn handle_setup(force: bool) -> Result<()> {
    info!("🚀 Starting off-context setup...");
    
    if !force {
        // Check if already configured
        if is_already_configured().await? {
            println!("✅ off-context is already configured!");
            println!("💡 Use --force to reconfigure");
            return Ok(());
        }
    }
    
    // 1. Initialize configuration
    info!("⚙️ Initializing configuration...");
    init_config().await?;
    println!("  Configuration initialized ✅");
    
    // 2. Detect Claude Code installation
    info!("🔍 Detecting Claude Code installation...");
    detect_claude_code().await?;
    
    // 3. Configure hooks
    info!("🔗 Configuring Claude Code hooks...");
    configure_hooks().await?;
    
    // 4. Initialize local database (create config dir structure)
    info!("🗄️ Initializing memory database...");
    initialize_database().await?;
    
    println!("✅ Setup complete!");
    println!();
    println!("🎉 off-context is now active!");
    println!("💡 Just use Claude Code normally - memory works automatically");
    println!();
    println!("Optional next steps:");
    println!("  off-context import    # Import existing conversations");
    println!("  off-context status    # Check system status");
    
    Ok(())
}

async fn is_already_configured() -> Result<bool> {
    // Check if hooks are configured
    let hooks_dir = claude_code_hooks_dir()?;
    let user_prompt_hook = hooks_dir.join("UserPromptSubmit.sh");
    let stop_hook = hooks_dir.join("Stop.sh");
    
    // Check if config directory exists
    let config_dir = config_dir()?;
    let config_exists = config_dir.exists();
    
    let hooks_configured = user_prompt_hook.exists() && stop_hook.exists();
    
    Ok(config_exists && hooks_configured)
}

async fn detect_claude_code() -> Result<()> {
    // Try to find Claude Code binary
    let claude_cmd = if cfg!(windows) {
        Command::new("where").arg("claude").output()
    } else {
        Command::new("which").arg("claude").output()
    };

    match claude_cmd {
        Ok(output) if output.status.success() => {
            let path_str = String::from_utf8_lossy(&output.stdout);
            let path = path_str.trim();
            println!("  Found Claude Code at: {} ✅", path);
        }
        _ => {
            warn!("Claude Code binary not found in PATH");
            println!("  Claude Code binary not found in PATH ⚠️");
            println!("  💡 Make sure Claude Code is installed and accessible");
            println!("  💡 This won't prevent setup, but hooks may not work");
        }
    }

    Ok(())
}

async fn configure_hooks() -> Result<()> {
    let hooks_dir = claude_code_hooks_dir()?;
    
    // Create hooks directory if it doesn't exist
    tokio::fs::create_dir_all(&hooks_dir).await
        .context("Failed to create hooks directory")?;

    // Create UserPromptSubmit hook for context injection
    let user_prompt_hook = hooks_dir.join("UserPromptSubmit.sh");
    let user_prompt_script = r#"#!/bin/bash
# off-context UserPromptSubmit hook
# Receives JSON via stdin with session_id, transcript_path, prompt
if command -v off-context >/dev/null 2>&1; then
    off-context inject-prompt
else
    cat
fi
"#;

    tokio::fs::write(&user_prompt_hook, user_prompt_script).await
        .context("Failed to write UserPromptSubmit hook")?;

    // Make the hook executable (Unix systems)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = tokio::fs::metadata(&user_prompt_hook).await?.permissions();
        perms.set_mode(0o755);
        tokio::fs::set_permissions(&user_prompt_hook, perms).await
            .context("Failed to make UserPromptSubmit hook executable")?;
    }

    // Create Stop hook for conversation capture
    let stop_hook = hooks_dir.join("Stop.sh");
    let stop_script = r#"#!/bin/bash
# off-context Stop hook
# Receives JSON via stdin with session_id, transcript_path

LOG_FILE="$HOME/.off-context/hooks.log"

log() {
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] [Stop] $1" >> "$LOG_FILE"
}

# Read JSON from stdin
INPUT_JSON=$(cat)
log "Stop hook input: $INPUT_JSON"

# Extract transcript_path from JSON
if command -v jq >/dev/null 2>&1; then
    TRANSCRIPT_FILE=$(echo "$INPUT_JSON" | jq -r '.transcript_path // empty')
    SESSION_ID=$(echo "$INPUT_JSON" | jq -r '.session_id // empty')
    log "Extracted transcript: $TRANSCRIPT_FILE, session: $SESSION_ID"
else
    log "jq not found, cannot parse JSON input"
    exit 1
fi

if [ -n "$TRANSCRIPT_FILE" ] && [ -f "$TRANSCRIPT_FILE" ]; then
    log "Processing transcript: $TRANSCRIPT_FILE"
    if command -v off-context >/dev/null 2>&1; then
        off-context hook "$TRANSCRIPT_FILE" >>"$LOG_FILE" 2>&1 &
        log "off-context hook called for $TRANSCRIPT_FILE"
    else
        log "off-context not found, skipping hook call"
    fi
else
    log "No valid transcript file to process: $TRANSCRIPT_FILE"
fi
"#;

    tokio::fs::write(&stop_hook, stop_script).await
        .context("Failed to write Stop hook")?;

    // Make the hook executable (Unix systems)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = tokio::fs::metadata(&stop_hook).await?.permissions();
        perms.set_mode(0o755);
        tokio::fs::set_permissions(&stop_hook, perms).await
            .context("Failed to make Stop hook executable")?;
    }

    println!("  UserPromptSubmit hook created ✅");
    println!("  Stop hook created ✅");
    println!("  Hooks directory: {}", hooks_dir.display());

    Ok(())
}

async fn initialize_database() -> Result<()> {
    // Create the off-context config directory
    let config_dir = config_dir()?;
    tokio::fs::create_dir_all(&config_dir).await
        .context("Failed to create config directory")?;

    // Test database connection by creating a Memory instance
    // This will create the collection if needed
    let config = crate::core::config::load_config().await?;
    match Memory::new(&config.database).await {
        Ok(_) => {
            println!("  Database connection test passed ✅");
        }
        Err(e) => {
            warn!("Database initialization failed: {}", e);
            println!("  Database initialization failed ⚠️");
            println!("  💡 This is normal if Qdrant is not running");
            println!("  💡 The system will work with Qdrant when it's available");
        }
    }
    
    println!("  Config directory: {}", config_dir.display());

    Ok(())
}