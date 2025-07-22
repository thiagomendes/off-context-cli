use anyhow::Result;
use std::fs;
use serde_json::{json, Value};
use crate::core::config::load_project_config;

pub async fn handle_init() -> Result<()> {
    let hooks_dir = dirs::home_dir().unwrap().join(".config/claude/hooks");
    let user_hook = hooks_dir.join("UserPromptSubmit.sh");
    let stop_hook = hooks_dir.join("Stop.sh");
    let project_root = std::env::current_dir()?;
    let claude_dir = project_root.join(".claude");
    let settings_file = claude_dir.join("settings.local.json");

    fs::create_dir_all(&claude_dir)?;

    // Generate hooks block
    let hooks_json = json!({
        "UserPromptSubmit": [
            { "hooks": [ { "type": "command", "command": user_hook.to_string_lossy() } ] }
        ],
        "Stop": [
            { "hooks": [ { "type": "command", "command": stop_hook.to_string_lossy() } ] }
        ]
    });

    // If it already exists, merge while preserving other configs
    let merged: Value = if settings_file.exists() {
        let content = fs::read_to_string(&settings_file)?;
        let mut existing: Value = serde_json::from_str(&content).unwrap_or(json!({}));
        // Remove old hooks
        if let Some(obj) = existing.as_object_mut() {
            obj.remove("hooks");
        }
        // Add new hooks
        if let Some(obj) = existing.as_object_mut() {
            obj.insert("hooks".to_string(), Value::Object(hooks_json.as_object().unwrap().clone()));
        }
        existing
    } else {
        let mut obj = serde_json::Map::new();
        obj.insert("hooks".to_string(), Value::Object(hooks_json.as_object().unwrap().clone()));
        Value::Object(obj)
    };

    fs::write(&settings_file, serde_json::to_string_pretty(&merged)?)?;
    println!("off-context: hooks configured in {}", settings_file.display());
    
    // Create .off-context directory in project root
    let off_context_dir = project_root.join(".off-context");
    fs::create_dir_all(&off_context_dir)?;
    println!("off-context: project directory created at {}", off_context_dir.display());
    
    // Initialize project-specific configuration
    let _config = load_project_config().await?;
    println!("off-context: project configuration initialized");
    
    Ok(())
}

pub async fn handle_uninstall() -> Result<()> {
    use std::fs;
    let hooks_dir = dirs::home_dir().unwrap().join(".config/claude/hooks");
    let offcontext_dir = dirs::home_dir().unwrap().join(".off-context");
    if hooks_dir.exists() {
        fs::remove_dir_all(&hooks_dir)?;
        println!("off-context: global hooks removed from {}", hooks_dir.display());
    } else {
        println!("off-context: global hooks already don't exist in {}", hooks_dir.display());
    }
    if offcontext_dir.exists() {
        fs::remove_dir_all(&offcontext_dir)?;
        println!("off-context: global memory removed from {}", offcontext_dir.display());
    } else {
        println!("off-context: global memory already doesn't exist in {}", offcontext_dir.display());
    }
    println!("If desired, remove the global binary with: sudo rm /usr/local/bin/off-context");
    Ok(())
} 