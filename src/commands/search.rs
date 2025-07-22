use anyhow::{Context, Result};
use chrono::DateTime;
use tracing::debug;

use crate::core::{config::load_project_config, memory::Memory, validation::ensure_project_initialized};

pub async fn handle_search(query: &str, limit: usize) -> Result<()> {
    // Ensure we're in a project
    ensure_project_initialized()?;
    
    println!("🔍 Searching project for: \"{}\"", query);
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    
    let start = std::time::Instant::now();
    
    // Load configuration
    let config = load_project_config().await.context("Failed to load configuration")?;
    
    // Initialize memory store
    let memory = Memory::new(&config.database).await
        .context("Failed to initialize memory store")?;
    
    // Perform search
    let search_results = memory
        .search(query, limit)
        .await
        .context("Failed to search conversations")?;
    
    let search_duration = start.elapsed();
    
    if search_results.is_empty() {
        println!("❌ No conversations found matching \"{}\"", query);
        println!();
        println!("💡 Tips:");
        println!("   • Try different keywords");
        println!("   • Check if conversations have been imported");
        println!("   • Run 'off-context status' to verify setup");
        return Ok(());
    }
    
    println!("✅ Found {} results in {:?}\n", search_results.len(), search_duration);
    
    for (i, result) in search_results.iter().enumerate() {
        let conversation = &result.conversation;
        
        println!("📝 Result {} (similarity: {:.2})", i + 1, result.score);
        println!("   ⏰ {}", format_timestamp(&conversation.timestamp));
        
        if let Some(project_path) = &conversation.metadata.project_path {
            println!("   📁 Project: {}", project_path);
        }
        
        if !conversation.metadata.tags.is_empty() {
            println!("   🏷️  Tags: {}", conversation.metadata.tags.join(", "));
        }
        
        println!("   💬 Tokens: {}", conversation.metadata.token_count);
        println!();
        
        // Show conversation snippet
        println!("   💭 Conversation snippet:");
        println!("   ┌─────────────────────────────────────────────");
        
        // Format the snippet with proper indentation
        for line in result.snippet.lines() {
            println!("   │ {}", line);
        }
        
        println!("   └─────────────────────────────────────────────");
        
        if i < search_results.len() - 1 {
            println!();
        }
    }
    
    // Show summary
    println!();
    println!("📊 Search Summary:");
    println!("   🔍 Query: \"{}\"", query);
    println!("   📋 Results: {} of max {}", search_results.len(), limit);
    println!("   ⚡ Duration: {:?}", search_duration);
    
    // Show total conversation count
    match memory.conversation_count().await {
        Ok(total) => println!("   📚 Total project conversations: {}", total),
        Err(e) => debug!("Failed to get conversation count: {}", e),
    }
    
    Ok(())
}

fn format_timestamp(timestamp: &DateTime<chrono::Utc>) -> String {
    let now = chrono::Utc::now();
    let duration = now.signed_duration_since(*timestamp);
    
    if duration.num_days() > 0 {
        format!("{} ({} days ago)", timestamp.format("%Y-%m-%d %H:%M:%S"), duration.num_days())
    } else if duration.num_hours() > 0 {
        format!("{} ({} hours ago)", timestamp.format("%Y-%m-%d %H:%M:%S"), duration.num_hours())
    } else if duration.num_minutes() > 0 {
        format!("{} ({} minutes ago)", timestamp.format("%Y-%m-%d %H:%M:%S"), duration.num_minutes())
    } else {
        format!("{} (just now)", timestamp.format("%Y-%m-%d %H:%M:%S"))
    }
}