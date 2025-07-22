use anyhow::{Context, Result};
use std::io::{self, Read};
use tracing::warn;

use crate::core::{
    config::{load_project_config, is_in_project},
    memory::Memory,
};

/// Handle context injection - called by UserPromptSubmit hook
pub async fn handle_inject(query: &str) -> Result<()> {
    // If no query provided as argument, read from stdin
    let query = if query.is_empty() {
        let mut buffer = String::new();
        io::stdin().read_to_string(&mut buffer)
            .context("Failed to read from stdin")?;
        buffer
    } else {
        query.to_string()
    };
    
    let _start = std::time::Instant::now();
    
    // Fast path: if query is very short or looks like a simple command, don't inject context
    if query.trim().len() < 10 || is_simple_command(&query) {
        print!("{}", query);
        return Ok(());
    }
    
    match inject_context_internal(&query).await {
        Ok(enhanced_query) => {
            print!("{}", enhanced_query);
            // Context injection completed successfully
        }
        Err(e) => {
            warn!("Context injection failed: {}, using original query", e);
            print!("{}", query);
        }
    }
    
    Ok(())
}

pub async fn inject_context_internal(query: &str) -> Result<String> {
    // Try to parse the prompt JSON
    let prompt_json: serde_json::Value = match serde_json::from_str(query) {
        Ok(val) => val,
        Err(_) => return Ok(query.to_string()), // If not JSON, return the original text
    };
    // Extract current session_id
    let current_session_id = prompt_json.get("session_id").and_then(|v| v.as_str()).map(|s| s.to_string());
    
    // Only inject if we're in a project - otherwise pass through original query
    if !is_in_project() {
        return Ok(query.to_string());
    }
    
    // Load configuration
    let config = load_project_config().await.context("Failed to load configuration")?;
    if !config.hooks.auto_inject {
        return Ok(query.to_string());
    }
    let memory = match Memory::new(&config.database).await {
        Ok(memory) => memory,
        Err(_) => {
            return Ok(query.to_string());
        }
    };
    // Search all saved conversations
    let all_convs = memory.all_conversations().await.unwrap_or_default();
    // Group by session_id and sort by timestamp
    use std::collections::BTreeMap;
    let mut sessions: BTreeMap<String, Vec<&crate::core::types::Conversation>> = BTreeMap::new();
    for conv in &all_convs {
        if let Some(sid) = &conv.metadata.session_id {
            sessions.entry(sid.clone()).or_default().push(conv);
        }
    }
    // Sort sessions by timestamp of last conversation
    let mut session_vec: Vec<_> = sessions.into_iter().collect();
    session_vec.sort_by_key(|(_, v)| v.last().map(|c| c.timestamp));
    // Find sessions different from current one, sorted by timestamp
    let mut prev_session: Option<&Vec<&crate::core::types::Conversation>> = None;
    
    if let Some(current_sid) = &current_session_id {
        // Get the most recent session that's not the current one
        for (sid, convs) in session_vec.iter().rev() {
            if sid != current_sid {
                prev_session = Some(convs);
                break;
            }
        }
    } else {
        // If no current session_id, get the most recent session
        prev_session = session_vec.last().map(|(_, v)| v);
    }
    // Build memory block
    let mut instruction_block = String::from("[INSTRUCTION]\n");
    if let Some(convs) = prev_session {
        let n = 3;
        for conv in convs.iter().rev().take(n).rev() {
            instruction_block.push_str(&format!(
                "Remember: in the last conversation, you answered \"{}\" to the question \"{}\".\n",
                conv.assistant_response, conv.user_message
            ));
        }
    }
    instruction_block.push_str("[/INSTRUCTION]\n\n");
    // Replace the .prompt field in JSON with the expanded prompt
    let mut new_json = prompt_json.clone();
    if let Some(prompt_field) = new_json.get_mut("prompt") {
        *prompt_field = serde_json::Value::String(instruction_block.clone() + prompt_field.as_str().unwrap_or(""));
    }
    let result = serde_json::to_string(&new_json).unwrap_or(query.to_string());
    Ok(result)
}

/// Simple context injection for UserPromptSubmit hook
pub async fn inject_context_simple(prompt: &str) -> Result<String> {
    // Only inject if we're in a project - otherwise pass through original query
    if !is_in_project() {
        return Ok(prompt.to_string());
    }
    
    // Load configuration
    let config = load_project_config().await.context("Failed to load configuration")?;
    if !config.hooks.auto_inject {
        return Ok(prompt.to_string());
    }

    let memory = match Memory::new(&config.database).await {
        Ok(memory) => {
            memory
        },
        Err(_) => {
            return Ok(prompt.to_string());
        },
    };

    // Get last few conversations from memory
    let all_convs = memory.all_conversations().await.unwrap_or_default();
    if all_convs.is_empty() {
        return Ok(prompt.to_string());
    }

    // Sort by timestamp and get latest conversations
    let mut sorted_convs = all_convs;
    sorted_convs.sort_by_key(|c| c.timestamp);
    
    // Take only last 2 conversations to reduce token usage
    let recent_convs: Vec<_> = sorted_convs.iter().rev().take(2).rev().collect();
    
    let mut context_block = String::from("[PREV: ");
    let mut first = true;
    for conv in recent_convs {
        // Clean user message from all log artifacts and system noise
        let clean_user_msg = conv.user_message
            .replace("<user-prompt-submit-hook>", "")
            .replace("[CONTEXT FROM PREVIOUS CONVERSATIONS]", "")
            .replace("[END CONTEXT]", "")
            .split("INFO Configuration loaded successfully").last().unwrap_or("")
            .split("Previous: User said").last().unwrap_or("")
            .replace("[2m", "")
            .replace("[0m", "")
            .replace("[32m", "")
            .trim()
            .chars().take(80).collect::<String>();
        
        let clean_assistant_msg = conv.assistant_response
            .replace("<user-prompt-submit-hook>", "")
            .replace("[2m", "")
            .replace("[0m", "")
            .replace("[32m", "")
            .trim()
            .chars().take(200).collect::<String>();
            
        if !clean_user_msg.is_empty() && !clean_assistant_msg.is_empty() {
            if !first {
                context_block.push_str("; ");
            }
            context_block.push_str(&format!("U:\"{}\" A:\"{}\"", clean_user_msg, clean_assistant_msg));
            first = false;
        }
    }
    context_block.push_str("]\n\n");
    
    Ok(format!("{}{}", context_block, prompt))
}

/// Check if the query looks like a simple command that doesn't need context
fn is_simple_command(query: &str) -> bool {
    let query_lower = query.trim().to_lowercase();
    
    // Simple greetings or short commands
    let simple_patterns = [
        "hi", "hello", "help", "thanks", "thank you", "yes", "no", "ok", "okay",
        "continue", "go on", "next", "stop", "quit", "exit",
    ];
    
    simple_patterns.iter().any(|pattern| query_lower == *pattern)
}