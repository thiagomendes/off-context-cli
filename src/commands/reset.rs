use anyhow::{Context, Result};
use std::io::{self, Write};
use tracing::{info, warn};

use crate::core::{config::{load_project_config, project_config_dir}, memory::Memory, validation::ensure_project_initialized};

pub async fn handle_reset(yes: bool) -> Result<()> {
    // Ensure we're in a project
    ensure_project_initialized()?;
    
    // Show current status before reset
    let config = load_project_config().await.context("Failed to load configuration")?;
    
    let conversation_count = match Memory::new(&config.database).await {
        Ok(memory) => memory.conversation_count().await.unwrap_or(0),
        Err(_) => 0,
    };
    
    println!("🗑️ off-context Reset (Project-local)");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("📊 Current status:");
    println!("   💾 Conversations to delete: {}", conversation_count);
    println!("   📁 Database path: {}", config.database.path);
    
    if !yes {
        println!();
        print!("⚠️ This will delete ALL stored conversations. Continue? (y/N): ");
        io::stdout().flush()?;
        
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        
        if input.trim().to_lowercase() != "y" {
            println!("❌ Reset cancelled");
            return Ok(());
        }
    }
    
    println!("🗑️ Resetting off-context memory...");
    info!("Starting memory reset");
    
    // Clear database
    match Memory::new(&config.database).await {
        Ok(memory) => {
            memory.clear().await.context("Failed to clear memory database")?;
            println!("  ✅ Database cleared");
        }
        Err(e) => {
            warn!("Failed to initialize memory for clearing: {}", e);
            println!("  ⚠️ Database clear failed (may not exist)");
        }
    }
    
    // Clean up session injection tracking files
    if let Ok(config_dir) = project_config_dir() {
        if let Ok(entries) = std::fs::read_dir(&config_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(name) = path.file_name() {
                    if name.to_string_lossy().starts_with("session_injected_") {
                        match tokio::fs::remove_file(&path).await {
                            Ok(_) => println!("  ✅ Session tracking file removed: {}", name.to_string_lossy()),
                            Err(_) => println!("  ⚠️ Could not remove session file: {}", name.to_string_lossy()),
                        }
                    }
                }
            }
        }
    }
    
    // Optionally remove project config directory entirely
    if yes {
        let project_config = project_config_dir()?;
        if project_config.exists() {
            match tokio::fs::remove_dir_all(&project_config).await {
                Ok(_) => {
                    println!("  ✅ Project configuration directory removed");
                    info!("Project configuration directory removed: {:?}", project_config);
                }
                Err(e) => {
                    warn!("Failed to remove project config directory: {}", e);
                    println!("  ⚠️ Could not remove project config directory");
                }
            }
        }
    }
    
    println!("✅ Memory reset complete!");
    println!();
    println!("📝 What was reset (project-only):");
    println!("   🗑️ Project conversation history");
    println!("   🧠 Project embeddings and search indices");
    if yes {
        println!("   ⚙️ Project configuration files");
    }
    println!();
    println!("🔧 Next steps:");
    println!("   💡 Run 'off-context init' to reinitialize project");
    println!("   📥 Run 'off-context import' to reimport conversations");
    
    Ok(())
}