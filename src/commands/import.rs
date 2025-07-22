use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};
use walkdir::WalkDir;

use crate::core::{
    config::load_project_config,
    memory::Memory,
    parser::parse_transcript,
    validation::ensure_project_initialized,
};

pub async fn handle_import(path: Option<&str>) -> Result<()> {
    // Ensure we're in a project
    ensure_project_initialized()?;
    
    println!("📥 Importing Claude Code conversations to project...");
    
    let import_path = if let Some(path) = path {
        PathBuf::from(shellexpand::tilde(path).as_ref())
    } else {
        // Try to find Claude Code data directory
        find_claude_code_data_dir()?
    };
    
    println!("📁 Scanning: {}", import_path.display());
    
    if !import_path.exists() {
        println!("❌ Path does not exist: {}", import_path.display());
        println!("💡 Try specifying the path manually:");
        println!("   off-context import --path /path/to/claude/data");
        return Ok(());
    }
    
    // Find transcript files
    let transcript_files = find_transcript_files(&import_path)?;
    
    if transcript_files.is_empty() {
        println!("❌ No transcript files found in {}", import_path.display());
        println!("💡 Make sure Claude Code has been used and transcripts are available");
        return Ok(());
    }
    
    println!("🔍 Found {} potential transcript files", transcript_files.len());
    
    // Initialize memory and configuration
    let config = load_project_config().await.context("Failed to load configuration")?;
    let memory = Memory::new(&config.database).await
        .context("Failed to initialize memory store")?;
    
    let mut total_conversations = 0;
    let mut processed_files = 0;
    let mut failed_files = 0;
    
    println!("⚙️ Processing transcript files...");
    
    for (i, transcript_file) in transcript_files.iter().enumerate() {
        let progress = format!("[{}/{}]", i + 1, transcript_files.len());
        
        match process_transcript_file(&memory, transcript_file).await {
            Ok(conversation_count) => {
                if conversation_count > 0 {
                    println!("  {} ✅ {}: {} conversations", 
                           progress, 
                           transcript_file.file_name().unwrap_or_default().to_string_lossy(),
                           conversation_count);
                    total_conversations += conversation_count;
                }
                processed_files += 1;
            }
            Err(e) => {
                debug!("Failed to process {}: {}", transcript_file.display(), e);
                println!("  {} ⚠️ {}: skipped ({})", 
                       progress,
                       transcript_file.file_name().unwrap_or_default().to_string_lossy(),
                       e);
                failed_files += 1;
            }
        }
    }
    
    // Show summary
    println!();
    println!("📊 Import Summary:");
    println!("   📁 Files scanned: {}", transcript_files.len());
    println!("   ✅ Files processed: {}", processed_files);
    println!("   ⚠️ Files failed: {}", failed_files);
    println!("   💬 Total conversations imported: {}", total_conversations);
    
    // Show current database size
    match memory.conversation_count().await {
        Ok(total) => println!("   📚 Total conversations in database: {}", total),
        Err(e) => debug!("Failed to get conversation count: {}", e),
    }
    
    if total_conversations > 0 {
        println!();
        println!("✅ Import complete!");
        println!("🔍 Try: off-context search \"your query\"");
    } else {
        println!();
        println!("⚠️ No conversations were imported");
        println!("💡 Check if Claude Code has created transcript files");
    }
    
    Ok(())
}

async fn process_transcript_file(memory: &Memory, file_path: &Path) -> Result<usize> {
    let conversations = parse_transcript(&file_path.to_string_lossy()).await
        .context("Failed to parse transcript")?;
    
    if conversations.is_empty() {
        return Ok(0);
    }
    
    // Store conversations
    for conversation in &conversations {
        memory.store_conversation(conversation).await
            .context("Failed to store conversation")?;
    }
    
    Ok(conversations.len())
}

fn find_transcript_files(base_path: &Path) -> Result<Vec<PathBuf>> {
    let mut transcript_files = Vec::new();
    
    for entry in WalkDir::new(base_path)
        .follow_links(true)
        .max_depth(5) // Reasonable depth limit
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        
        if path.is_file() && is_transcript_file(path) {
            transcript_files.push(path.to_path_buf());
        }
    }
    
    // Sort by modification time (newest first)
    transcript_files.sort_by(|a, b| {
        let a_time = a.metadata().and_then(|m| m.modified()).unwrap_or(std::time::UNIX_EPOCH);
        let b_time = b.metadata().and_then(|m| m.modified()).unwrap_or(std::time::UNIX_EPOCH);
        b_time.cmp(&a_time)
    });
    
    Ok(transcript_files)
}

fn is_transcript_file(path: &Path) -> bool {
    if let Some(filename) = path.file_name().and_then(|f| f.to_str()) {
        // Look for common Claude Code transcript patterns
        filename.ends_with(".json") && (
            filename.contains("transcript") ||
            filename.contains("conversation") ||
            filename.contains("claude") ||
            // Check file size - transcript files are usually substantial
            path.metadata().map(|m| m.len() > 100).unwrap_or(false)
        )
    } else {
        false
    }
}

fn find_claude_code_data_dir() -> Result<PathBuf> {
    let home = dirs::home_dir().context("Could not find home directory")?;
    
    // Common Claude Code data locations
    let candidates = [
        home.join(".claude"),
        home.join(".config").join("claude"),
        home.join("Library").join("Application Support").join("claude"),
        home.join("AppData").join("Roaming").join("claude"),
        home.join("AppData").join("Local").join("claude"),
    ];
    
    for candidate in &candidates {
        if candidate.exists() {
            info!("Found Claude Code directory: {:?}", candidate);
            return Ok(candidate.clone());
        }
    }
    
    // Default to the first candidate
    warn!("Claude Code directory not found, using default: {:?}", candidates[0]);
    Ok(candidates[0].clone())
}