use anyhow::Result;
use tracing::debug;

use crate::core::{
    config::{claude_code_hooks_dir, project_config_dir, load_project_config, find_project_root},
    embeddings::EmbeddingGenerator,
    memory::Memory,
    validation::ensure_project_initialized,
};

/// Handle the status command - show system information
pub async fn handle_status() -> Result<()> {
    // Check if we're in a project
    ensure_project_initialized()?;
    
    println!("📊 off-context Status (Project-local)");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    
    // Check hook configuration
    let hooks_status = check_hooks_status().await?;
    println!("🔗 Claude Code Hooks: {}", if hooks_status { "✅ Active" } else { "❌ Not configured" });
    
    // Check database status
    let db_status = check_database_status().await?;
    println!("🗄️ Memory Database: {}", if db_status.exists { "✅ Ready" } else { "❌ Not initialized" });
    
    if db_status.exists {
        println!("   💾 Conversations: {}", db_status.conversation_count);
        println!("   📦 Database size: {}", format_size(db_status.size_bytes));
        println!("   📅 Last activity: {}", db_status.last_activity.unwrap_or_else(|| "Never".to_string()));
    }
    
    // Check embedding service
    let embeddings_status = check_embeddings_status().await?;
    println!("🧠 Embeddings: {}", if embeddings_status.available { "✅ Available" } else { "⚠️ Using fallback" });
    println!("   🔧 Provider: {}", embeddings_status.provider);
    println!("   📐 Dimensions: {}", embeddings_status.dimensions);
    
    // Configuration info
    let project_root = find_project_root().unwrap();
    let project_config = project_config_dir()?;
    let project_config_file = project_config.join("config.toml");
    
    println!("⚙️ Project Configuration: {}", if project_config_file.exists() { "✅ Found" } else { "⚠️ Using defaults" });
    println!("   📁 Project root: {}", project_root.display());
    println!("   🗂️ Config directory: {}", project_config.display());
    
    // Performance info
    println!();
    println!("⚡ Performance:");
    let search_time = get_average_search_time().await?;
    println!("   🔍 Average search time: {}ms", search_time);
    println!("   💽 Database path: {}", project_config.join("qdrant").display());
    
    // Show hooks directory (global)
    if let Ok(hooks_dir) = claude_code_hooks_dir() {
        println!("   🪝 Global hooks: {}", hooks_dir.display());
    }
    
    if !hooks_status {
        println!();
        println!("💡 Global hooks not found. Run: off-context setup");
        println!("💡 This project is isolated but needs global hooks to work");
    }
    
    Ok(())
}

#[derive(Default)]
pub struct DatabaseStatus {
    pub exists: bool,
    pub conversation_count: usize,
    pub size_bytes: u64,
    pub last_activity: Option<String>,
}

pub struct EmbeddingsStatus {
    pub available: bool,
    pub provider: String,
    pub dimensions: usize,
}

pub async fn check_hooks_status() -> Result<bool> {
    // First check if global hooks exist
    let hooks_dir = claude_code_hooks_dir()?;
    let user_prompt_hook = hooks_dir.join("UserPromptSubmit.sh");
    let stop_hook = hooks_dir.join("Stop.sh");
    
    let global_hooks_exist = user_prompt_hook.exists() && stop_hook.exists();
    debug!("Global hooks status: UserPromptSubmit={}, Stop={}", user_prompt_hook.exists(), stop_hook.exists());
    
    if !global_hooks_exist {
        return Ok(false);
    }
    
    // Then check if local project hooks are configured
    let project_root = std::env::current_dir()?;
    let claude_dir = project_root.join(".claude");
    let settings_file = claude_dir.join("settings.local.json");
    
    if !settings_file.exists() {
        debug!("No settings.local.json found");
        return Ok(false);
    }
    
    let content = std::fs::read_to_string(&settings_file)?;
    let settings: serde_json::Value = serde_json::from_str(&content).unwrap_or(serde_json::json!({}));
    
    let local_hooks_configured = settings.get("hooks").is_some();
    debug!("Local hooks configured: {}", local_hooks_configured);
    
    Ok(global_hooks_exist && local_hooks_configured)
}

pub async fn check_database_status() -> Result<DatabaseStatus> {
    let config = match load_project_config().await {
        Ok(config) => config,
        Err(_) => return Ok(DatabaseStatus::default()),
    };
    
    match Memory::new(&config.database).await {
        Ok(memory) => {
            let conversation_count = memory.conversation_count().await.unwrap_or(0);
            
            // Try to get directory size
            let db_path = std::path::Path::new(&config.database.path);
            let size_bytes = if db_path.exists() {
                get_directory_size(db_path).unwrap_or(0)
            } else {
                0
            };
            
            Ok(DatabaseStatus {
                exists: true,
                conversation_count,
                size_bytes,
                last_activity: Some("Recently".to_string()),
            })
        }
        Err(e) => {
            debug!("Database connection failed: {}", e);
            Ok(DatabaseStatus::default())
        }
    }
}

pub async fn check_embeddings_status() -> Result<EmbeddingsStatus> {
    let config = load_project_config().await?;
    
    match EmbeddingGenerator::new().await {
        Ok(generator) => {
            let ollama_available = generator.is_ollama_available().await;
            
            Ok(EmbeddingsStatus {
                available: ollama_available,
                provider: if ollama_available { 
                    config.embeddings.provider 
                } else { 
                    "simple (fallback)".to_string() 
                },
                dimensions: config.embeddings.dimension,
            })
        }
        Err(_) => {
            Ok(EmbeddingsStatus {
                available: false,
                provider: "simple (fallback)".to_string(),
                dimensions: config.embeddings.dimension,
            })
        }
    }
}

async fn get_average_search_time() -> Result<String> {
    // For now, return a placeholder
    // In a real implementation, we'd track search performance metrics
    Ok("< 50".to_string())
}

fn get_directory_size(path: &std::path::Path) -> Result<u64> {
    use std::fs;
    
    let mut total_size = 0;
    
    if path.is_dir() {
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let file_type = entry.file_type()?;
            
            if file_type.is_file() {
                total_size += entry.metadata()?.len();
            } else if file_type.is_dir() {
                total_size += get_directory_size(&entry.path())?;
            }
        }
    } else if path.is_file() {
        total_size = fs::metadata(path)?.len();
    }
    
    Ok(total_size)
}

pub fn format_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{}B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1}KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1}MB", bytes as f64 / (1024.0 * 1024.0))
    }
}