use anyhow::Result;
use crate::core::types::*;
use chrono::{DateTime, Utc};
use std::path::Path;
use uuid::Uuid;
use serde_json::Value;

/// Parse a Claude Code transcript file
pub async fn parse_transcript(transcript_path: &str) -> Result<Vec<Conversation>> {
    let content = tokio::fs::read_to_string(transcript_path).await?;
    
    // Try to parse as JSON first
    if let Ok(transcript) = serde_json::from_str::<ClaudeTranscript>(&content) {
        return extract_conversations_from_transcript(transcript, transcript_path);
    }
    
    // Try to parse as JSONL (one JSON object per line)
    let lines: Vec<&str> = content.lines().collect();
    let mut conversations = Vec::new();
    
    for line in lines {
        if line.trim().is_empty() {
            continue;
        }
        
        if let Ok(transcript) = serde_json::from_str::<ClaudeTranscript>(line) {
            let mut line_conversations = extract_conversations_from_transcript(transcript, transcript_path)?;
            conversations.append(&mut line_conversations);
        }
    }
    
    // New: If no conversations found, try Claude Code jsonl parser
    if conversations.is_empty() {
        let jsonl_convs = parse_claude_jsonl_transcript(transcript_path).await?;
        if !jsonl_convs.is_empty() {
            return Ok(jsonl_convs);
        }
    }
    Ok(conversations)
}

/// Extract conversations from a Claude Code transcript
fn extract_conversations_from_transcript(
    transcript: ClaudeTranscript,
    source_path: &str,
) -> Result<Vec<Conversation>> {
    let mut conversations = Vec::new();
    let mut current_user_message: Option<String> = None;
    
    for message in transcript.messages {
        match message.role.as_str() {
            "user" => {
                current_user_message = Some(message.content);
            }
            "assistant" => {
                if let Some(user_msg) = current_user_message.take() {
                    let conversation = Conversation {
                        id: Uuid::new_v4(),
                        timestamp: parse_timestamp(&message.timestamp)?,
                        user_message: user_msg.clone(),
                        assistant_response: message.content.clone(),
                        metadata: ConversationMetadata {
                            session_id: transcript.session_id.clone(),
                            project_path: detect_project_path(source_path),
                            tags: extract_tags(&user_msg),
                            token_count: estimate_token_count(&user_msg, &message.content),
                            embedding_model: None,
                        },
                    };
                    
                    conversations.push(conversation);
                }
            }
            _ => {
                // Ignore other message types (system, etc.)
            }
        }
    }
    
    Ok(conversations)
}

/// Novo: Parse Claude Code JSONL (um objeto por linha, tipo user/assistant)
pub async fn parse_claude_jsonl_transcript(transcript_path: &str) -> Result<Vec<Conversation>> {
    let content = tokio::fs::read_to_string(transcript_path).await?;
    let mut conversations = Vec::new();
    let mut current_user_message: Option<String> = None;
    let mut session_id: Option<String> = None;

    for line in content.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let v: Value = match serde_json::from_str(line) {
            Ok(val) => val,
            Err(_) => continue,
        };
        let msg_type = v.get("type").and_then(|t| t.as_str()).unwrap_or("");
        if msg_type == "user" {
            if let Some(msg) = v.get("message").and_then(|m| m.get("content")).and_then(|c| c.as_str()) {
                current_user_message = Some(msg.to_string());
                session_id = v.get("sessionId").and_then(|s| s.as_str()).map(|s| s.to_string());
            }
        } else if msg_type == "assistant" {
            if let (Some(user_msg), Some(msg)) = (
                current_user_message.take(),
                v.get("message").and_then(|m| m.get("content")).and_then(|c| {
                    if c.is_string() {
                        c.as_str().map(|s| s.to_string())
                    } else if c.is_array() {
                        c.as_array().and_then(|arr| arr.get(0)).and_then(|obj| obj.get("text")).and_then(|t| t.as_str()).map(|s| s.to_string())
                    } else {
                        None
                    }
                }),
            ) {
                let conversation = Conversation {
                    id: Uuid::new_v4(),
                    timestamp: Utc::now(),
                    user_message: user_msg,
                    assistant_response: msg,
                    metadata: ConversationMetadata {
                        session_id: session_id.clone(),
                        project_path: None,
                        tags: vec![],
                        token_count: 0,
                        embedding_model: None,
                    },
                };
                conversations.push(conversation);
            }
        }
    }
    Ok(conversations)
}

/// Parse timestamp from various formats
fn parse_timestamp(timestamp: &Option<String>) -> Result<DateTime<Utc>> {
    match timestamp {
        Some(ts) => {
            // Try ISO 8601 format first
            if let Ok(dt) = DateTime::parse_from_rfc3339(ts) {
                return Ok(dt.with_timezone(&Utc));
            }
            
            // Fallback to current time if parsing fails
            Ok(Utc::now())
        }
        None => Ok(Utc::now()),
    }
}

/// Detect project path from source file path
fn detect_project_path(source_path: &str) -> Option<String> {
    let path = Path::new(source_path);
    
    // Look for common project indicators
    let mut current = path.parent();
    while let Some(dir) = current {
        if dir.join(".git").exists() 
            || dir.join("Cargo.toml").exists()
            || dir.join("package.json").exists()
            || dir.join("pyproject.toml").exists() {
            return Some(dir.to_string_lossy().to_string());
        }
        current = dir.parent();
    }
    
    None
}

/// Extract tags from user message content
fn extract_tags(content: &str) -> Vec<String> {
    let mut tags = Vec::new();
    
    // Simple keyword-based tagging
    let content_lower = content.to_lowercase();
    
    let keywords = [
        ("rust", "rust"),
        ("python", "python"),
        ("javascript", "javascript"),
        ("typescript", "typescript"),
        ("react", "react"),
        ("node", "nodejs"),
        ("api", "api"),
        ("database", "database"),
        ("sql", "sql"),
        ("auth", "authentication"),
        ("test", "testing"),
        ("debug", "debugging"),
        ("performance", "performance"),
        ("security", "security"),
    ];
    
    for (keyword, tag) in keywords {
        if content_lower.contains(keyword) {
            tags.push(tag.to_string());
        }
    }
    
    tags
}

/// Estimate token count for text
fn estimate_token_count(user_msg: &str, assistant_msg: &str) -> usize {
    // Rough estimation: 4 characters per token
    (user_msg.len() + assistant_msg.len()) / 4
}