use anyhow::{Context, Result};
use dirs::home_dir;
use std::path::PathBuf;
use tracing::{debug, info};
use crate::core::types::*;



/// Get the default configuration
pub fn default_config() -> Config {
    let home = home_dir().unwrap_or_else(|| PathBuf::from("."));
    let config_dir = home.join(".off-context");
    
    Config {
        database: DatabaseConfig {
            path: config_dir.join("qdrant").to_string_lossy().to_string(),
            collection_name: "conversations".to_string(),
        },
        embeddings: EmbeddingsConfig {
            provider: "simple".to_string(), // Default to simple for reliability
            model: "nomic-embed-text".to_string(),
            dimension: 384, // Smaller dimension for faster processing
        },
        context: ContextConfig {
            max_results: 5,
            max_tokens: 2000,
            relevance_threshold: 0.6, // Lower threshold for more results
        },
        hooks: HooksConfig {
            enabled: true,
            auto_inject: true,
        },
    }
}

/// Get the configuration directory path
pub fn config_dir() -> Result<PathBuf> {
    let home = home_dir().ok_or_else(|| anyhow::anyhow!("Could not find home directory"))?;
    Ok(home.join(".off-context"))
}

/// Get the configuration file path
pub fn config_file_path() -> Result<PathBuf> {
    Ok(config_dir()?.join("config.toml"))
}

/// Load configuration from file or create default
pub async fn load_config() -> Result<Config> {
    let config_path = config_file_path()?;
    
    if config_path.exists() {
        debug!("Loading config from: {:?}", config_path);
        let content = tokio::fs::read_to_string(&config_path).await
            .context("Failed to read config file")?;
        let config: Config = toml::from_str(&content)
            .context("Failed to parse config file")?;
        info!("Configuration loaded successfully");
        Ok(config)
    } else {
        info!("No config file found, creating default configuration");
        let config = default_config();
        save_config(&config).await
            .context("Failed to save default config")?;
        Ok(config)
    }
}

/// Save configuration to file
pub async fn save_config(config: &Config) -> Result<()> {
    let config_dir = config_dir()?;
    tokio::fs::create_dir_all(&config_dir).await
        .context("Failed to create config directory")?;
    
    let config_path = config_file_path()?;
    let content = toml::to_string_pretty(config)
        .context("Failed to serialize config")?;
    tokio::fs::write(&config_path, content).await
        .context("Failed to write config file")?;
    
    debug!("Configuration saved to: {:?}", config_path);
    Ok(())
}

/// Initialize the global configuration (simplified - just creates directories)
pub async fn init_config() -> Result<()> {
    let config = load_config().await?;
    // Ensure config directory exists
    tokio::fs::create_dir_all(&config.database.path).await
        .context("Failed to create config directory")?;
    Ok(())
}

/// Get Claude Code configuration directory
pub fn claude_code_config_dir() -> Result<PathBuf> {
    let home = home_dir().ok_or_else(|| anyhow::anyhow!("Could not find home directory"))?;
    
    // Try common Claude Code config locations
    let candidates = [
        home.join(".config").join("claude"),
        home.join("Library").join("Application Support").join("claude"),
        home.join("AppData").join("Roaming").join("claude"),
    ];

    for candidate in &candidates {
        if candidate.exists() {
            return Ok(candidate.clone());
        }
    }

    // Default to the first option if none exist
    Ok(candidates[0].clone())
}

/// Get Claude Code hooks directory  
pub fn claude_code_hooks_dir() -> Result<PathBuf> {
    Ok(claude_code_config_dir()?.join("hooks"))
}

/// Find project root by looking for .off-context directory
pub fn find_project_root() -> Option<PathBuf> {
    let mut current = std::env::current_dir().ok()?;
    
    loop {
        if current.join(".off-context").exists() {
            return Some(current);
        }
        
        if !current.pop() {
            break;
        }
    }
    
    None
}

/// Check if we're in a project with .off-context initialized
pub fn is_in_project() -> bool {
    find_project_root().is_some()
}

/// Get project-specific configuration directory
pub fn project_config_dir() -> Result<PathBuf> {
    let project_root = find_project_root()
        .ok_or_else(|| anyhow::anyhow!("Not in a project with .off-context initialized"))?;
    Ok(project_root.join(".off-context"))
}

/// Get project-specific database path
pub fn project_database_path() -> Result<PathBuf> {
    Ok(project_config_dir()?.join("qdrant"))
}

/// Load configuration, preferring project-local if available
pub async fn load_project_config() -> Result<Config> {
    if is_in_project() {
        // Try to load project-specific config
        let project_config_path = project_config_dir()?.join("config.toml");
        if project_config_path.exists() {
            debug!("Loading project config from: {:?}", project_config_path);
            let content = tokio::fs::read_to_string(&project_config_path).await
                .context("Failed to read project config file")?;
            let mut config: Config = toml::from_str(&content)
                .context("Failed to parse project config file")?;
            
            // Update database path to be project-relative
            config.database.path = project_database_path()?.to_string_lossy().to_string();
            info!("Project configuration loaded successfully");
            return Ok(config);
        } else {
            // Create default project config
            let mut config = default_config();
            config.database.path = project_database_path()?.to_string_lossy().to_string();
            save_project_config(&config).await
                .context("Failed to save default project config")?;
            return Ok(config);
        }
    }
    
    // Fallback to global config
    load_config().await
}

/// Save project-specific configuration
pub async fn save_project_config(config: &Config) -> Result<()> {
    let project_config_dir = project_config_dir()?;
    tokio::fs::create_dir_all(&project_config_dir).await
        .context("Failed to create project config directory")?;
    
    let project_config_path = project_config_dir.join("config.toml");
    let content = toml::to_string_pretty(config)
        .context("Failed to serialize project config")?;
    tokio::fs::write(&project_config_path, content).await
        .context("Failed to write project config file")?;
    
    debug!("Project configuration saved to: {:?}", project_config_path);
    Ok(())
}