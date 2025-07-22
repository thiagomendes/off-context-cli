use anyhow::{Context, Result};
use tracing::{debug, warn};

use crate::core::{
    config::{load_project_config, is_in_project},
    memory::Memory,
    parser::parse_transcript,
};

/// Handle hook processing - called by Claude Code hooks
pub async fn handle_hook(transcript_path: &str) -> Result<()> {
    debug!("🪝 Processing Claude Code hook: {}", transcript_path);
    
    // This should be fast and silent (< 100ms)
    let start = std::time::Instant::now();
    
    // Only process if we're in a project directory with .off-context
    if !is_in_project() {
        debug!("Not in project directory, skipping hook processing");
        return Ok(());
    }
    
    // Load configuration
    let config = load_project_config().await.context("Failed to load configuration")?;
    
    // Parse transcript file to extract conversations
    let conversations = parse_transcript(transcript_path).await
        .context("Failed to parse transcript file")?;
    
    if conversations.is_empty() {
        debug!("No conversations found in transcript");
        return Ok(());
    }
    
    let conversation_count = conversations.len();
    
    // Initialize memory store
    match Memory::new(&config.database).await {
        Ok(memory) => {
            // Store each conversation
            for conversation in conversations {
                if let Err(e) = memory.store_conversation(&conversation).await {
                    warn!("Failed to store conversation {}: {}", conversation.id, e);
                    // Continue processing other conversations
                }
            }
            
            let duration = start.elapsed();
            debug!("Stored {} conversations in {:?}", conversation_count, duration);
        }
        Err(e) => {
            warn!("Failed to initialize memory store: {}", e);
            // Don't fail the hook - just log the error
        }
    }
    
    let total_duration = start.elapsed();
    if total_duration.as_millis() > 100 {
        warn!("Hook processing took {:?} (target: <100ms)", total_duration);
    } else {
        debug!("Hook processing completed in {:?}", total_duration);
    }
    
    Ok(())
}