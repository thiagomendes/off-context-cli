use anyhow::Result;
use serde_json::{json, Value};
use crate::core::validation::ensure_project_initialized;

pub async fn handle_clear() -> Result<()> {
    // Ensure we're in a project
    ensure_project_initialized()?;
    let project_root = std::env::current_dir()?;
    let claude_dir = project_root.join(".claude");
    let settings_file = claude_dir.join("settings.local.json");

    if !settings_file.exists() {
        println!("off-context: No settings.local.json found in the project.");
        return Ok(());
    }
    let content = std::fs::read_to_string(&settings_file)?;
    let mut existing: Value = serde_json::from_str(&content).unwrap_or(json!({}));
    let mut changed = false;
    if let Some(obj) = existing.as_object_mut() {
        if obj.remove("hooks").is_some() {
            changed = true;
        }
    }
    if changed {
        std::fs::write(&settings_file, serde_json::to_string_pretty(&existing)?)?;
        println!("off-context: hooks removed from {}", settings_file.display());
    } else {
        println!("off-context: No hooks block found in {}", settings_file.display());
    }
    Ok(())
} 