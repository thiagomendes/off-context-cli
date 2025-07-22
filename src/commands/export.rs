use anyhow::{Context, Result};
use chrono::DateTime;

use crate::core::{config::load_project_config, memory::Memory, types::Conversation, validation::ensure_project_initialized};

pub async fn handle_export(format: &str, output: Option<&str>) -> Result<()> {
    // Ensure we're in a project
    ensure_project_initialized()?;
    
    println!("📤 Exporting project conversations...");
    println!("📋 Format: {}", format);
    
    let output_file = output.unwrap_or_else(|| {
        match format {
            "json" => "conversations.json",
            "md" => "conversations.md",
            _ => "conversations.txt",
        }
    });
    
    println!("📁 Output: {}", output_file);
    
    // Load configuration and initialize memory
    let config = load_project_config().await.context("Failed to load configuration")?;
    let memory = Memory::new(&config.database).await
        .context("Failed to initialize memory store")?;
    
    // Get all conversations via search (empty query returns all)
    let search_results = memory
        .search("", 10000) // Large limit to get all conversations
        .await
        .context("Failed to retrieve conversations")?;
    
    if search_results.is_empty() {
        println!("❌ No conversations found to export");
        println!("💡 Make sure conversations have been imported first");
        return Ok(());
    }
    
    println!("📊 Found {} conversations to export", search_results.len());
    
    // Export in the requested format
    let content = match format.to_lowercase().as_str() {
        "json" => export_as_json(&search_results)?,
        "md" | "markdown" => export_as_markdown(&search_results)?,
        "txt" | "text" => export_as_text(&search_results)?,
        _ => {
            println!("❌ Unsupported format: {}", format);
            println!("💡 Supported formats: json, md, txt");
            return Ok(());
        }
    };
    
    // Write to file
    tokio::fs::write(output_file, content).await
        .context("Failed to write export file")?;
    
    // Get file size for display
    let file_size = tokio::fs::metadata(output_file).await
        .map(|m| m.len())
        .unwrap_or(0);
    
    println!("✅ Export complete!");
    println!("   📁 File: {}", output_file);
    println!("   📊 Conversations: {}", search_results.len());
    println!("   📦 Size: {}", format_size(file_size));
    
    Ok(())
}

pub fn export_as_json(search_results: &[crate::core::types::SearchResult]) -> Result<String> {
    let conversations: Vec<&Conversation> = search_results
        .iter()
        .map(|r| &r.conversation)
        .collect();
    
    serde_json::to_string_pretty(&conversations)
        .context("Failed to serialize conversations as JSON")
}

pub fn export_as_markdown(search_results: &[crate::core::types::SearchResult]) -> Result<String> {
    let mut content = String::new();
    
    content.push_str("# Conversation History Export\n\n");
    content.push_str(&format!("*Exported on {} UTC*\n\n", chrono::Utc::now().format("%Y-%m-%d %H:%M:%S")));
    content.push_str(&format!("**Total conversations:** {}\n\n", search_results.len()));
    content.push_str("---\n\n");
    
    for (i, result) in search_results.iter().enumerate() {
        let conversation = &result.conversation;
        
        content.push_str(&format!("## Conversation {} - {}\n\n", 
                                i + 1, 
                                format_timestamp(&conversation.timestamp)));
        
        // Metadata
        if let Some(session_id) = &conversation.metadata.session_id {
            content.push_str(&format!("**Session ID:** {}\n\n", session_id));
        }
        
        if let Some(project_path) = &conversation.metadata.project_path {
            content.push_str(&format!("**Project:** {}\n\n", project_path));
        }
        
        if !conversation.metadata.tags.is_empty() {
            content.push_str(&format!("**Tags:** {}\n\n", conversation.metadata.tags.join(", ")));
        }
        
        content.push_str(&format!("**Tokens:** {}\n\n", conversation.metadata.token_count));
        
        // User message
        content.push_str("### User\n\n");
        content.push_str(&conversation.user_message);
        content.push_str("\n\n");
        
        // Assistant response
        content.push_str("### Assistant\n\n");
        content.push_str(&conversation.assistant_response);
        content.push_str("\n\n");
        
        if i < search_results.len() - 1 {
            content.push_str("---\n\n");
        }
    }
    
    Ok(content)
}

pub fn export_as_text(search_results: &[crate::core::types::SearchResult]) -> Result<String> {
    let mut content = String::new();
    
    content.push_str("CONVERSATION HISTORY EXPORT\n");
    content.push_str("==========================\n\n");
    content.push_str(&format!("Exported on: {} UTC\n", chrono::Utc::now().format("%Y-%m-%d %H:%M:%S")));
    content.push_str(&format!("Total conversations: {}\n\n", search_results.len()));
    
    for (i, result) in search_results.iter().enumerate() {
        let conversation = &result.conversation;
        
        content.push_str(&format!("CONVERSATION {} - {}\n", 
                                i + 1, 
                                format_timestamp(&conversation.timestamp)));
        content.push_str(&"-".repeat(50));
        content.push_str("\n");
        
        // Metadata
        if let Some(project_path) = &conversation.metadata.project_path {
            content.push_str(&format!("Project: {}\n", project_path));
        }
        
        if !conversation.metadata.tags.is_empty() {
            content.push_str(&format!("Tags: {}\n", conversation.metadata.tags.join(", ")));
        }
        
        content.push_str(&format!("Tokens: {}\n\n", conversation.metadata.token_count));
        
        // User message
        content.push_str("USER:\n");
        for line in conversation.user_message.lines() {
            content.push_str(&format!("> {}\n", line));
        }
        content.push_str("\n");
        
        // Assistant response
        content.push_str("ASSISTANT:\n");
        for line in conversation.assistant_response.lines() {
            content.push_str(&format!("< {}\n", line));
        }
        content.push_str("\n");
        
        if i < search_results.len() - 1 {
            content.push_str(&"=".repeat(50));
            content.push_str("\n\n");
        }
    }
    
    Ok(content)
}

fn format_timestamp(timestamp: &DateTime<chrono::Utc>) -> String {
    timestamp.format("%Y-%m-%d %H:%M:%S").to_string()
}

fn format_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{}B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1}KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1}MB", bytes as f64 / (1024.0 * 1024.0))
    }
}