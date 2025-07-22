use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tracing::{debug, info};
use uuid::Uuid;

use crate::core::types::{Conversation, SearchResult, DatabaseConfig};

/// Simple file-based storage for development/testing
pub struct Memory {
    conversations: Arc<Mutex<HashMap<Uuid, Conversation>>>,
    storage_path: PathBuf,
}

impl Memory {
    /// Create a new memory instance
    pub async fn new(config: &DatabaseConfig) -> Result<Self> {
        let storage_path = PathBuf::from(&config.path).join("conversations.json");
        
        // Ensure the directory exists
        if let Some(parent) = storage_path.parent() {
            tokio::fs::create_dir_all(parent).await
                .context("Failed to create storage directory")?;
        }
        
        // Load existing conversations from file
        let conversations = Self::load_from_file(&storage_path).await?;
        
        Ok(Self {
            conversations: Arc::new(Mutex::new(conversations)),
            storage_path,
        })
    }
    
    /// Store a conversation in memory and save to file
    pub async fn store_conversation(&self, conversation: &Conversation) -> Result<()> {
        {
            let mut conversations = self.conversations.lock().unwrap();
            conversations.insert(conversation.id, conversation.clone());
            debug!("Stored conversation {} in memory", conversation.id);
        }
        
        // Save to file
        self.save_to_file().await?;
        Ok(())
    }
    
    /// Search for relevant conversations using simple text matching
    pub async fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>> {
        let conversations = self.conversations.lock().unwrap();
        let mut results = Vec::new();
        let query_lower = query.to_lowercase();
        
        for conversation in conversations.values() {
            let user_msg_lower = conversation.user_message.to_lowercase();
            let assistant_msg_lower = conversation.assistant_response.to_lowercase();
            
            // Simple score based on keyword matches
            let mut score = 0.0;
            let query_words: Vec<&str> = query_lower.split_whitespace().collect();
            
            for word in query_words {
                if user_msg_lower.contains(word) {
                    score += 0.5;
                }
                if assistant_msg_lower.contains(word) {
                    score += 0.3;
                }
            }
            
            if score > 0.0 {
                let snippet = self.create_snippet(conversation);
                results.push(SearchResult {
                    conversation: conversation.clone(),
                    score,
                    snippet,
                });
            }
        }
        
        // Sort by score (highest first)
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        
        // Limit results
        results.truncate(limit);
        
        debug!("Found {} search results for query: {}", results.len(), query);
        Ok(results)
    }
    
    /// Get conversation count
    pub async fn conversation_count(&self) -> Result<usize> {
        let conversations = self.conversations.lock().unwrap();
        Ok(conversations.len())
    }

    
    /// Clear all conversations
    pub async fn clear(&self) -> Result<()> {
        {
            let mut conversations = self.conversations.lock().unwrap();
            conversations.clear();
            info!("Memory cleared");
        }
        
        // Save empty state to file
        self.save_to_file().await?;
        Ok(())
    }


    /// Retorna todas as conversas salvas
    pub async fn all_conversations(&self) -> Result<Vec<Conversation>> {
        let conversations = self.conversations.lock().unwrap();
        Ok(conversations.values().cloned().collect())
    }

    /// Create a snippet from a conversation for display
    fn create_snippet(&self, conversation: &Conversation) -> String {
        let user_preview = if conversation.user_message.len() > 100 {
            format!("{}...", &conversation.user_message[..100])
        } else {
            conversation.user_message.clone()
        };

        let assistant_preview = if conversation.assistant_response.len() > 200 {
            format!("{}...", &conversation.assistant_response[..200])
        } else {
            conversation.assistant_response.clone()
        };

        format!("User: {}\nAssistant: {}", user_preview, assistant_preview)
    }
    
    /// Load conversations from JSON file
    async fn load_from_file(storage_path: &PathBuf) -> Result<HashMap<Uuid, Conversation>> {
        if !storage_path.exists() {
            debug!("Storage file does not exist, starting with empty memory");
            return Ok(HashMap::new());
        }
        
        let content = tokio::fs::read_to_string(storage_path).await
            .context("Failed to read storage file")?;
        
        if content.trim().is_empty() {
            return Ok(HashMap::new());
        }
        
        let conversations: Vec<Conversation> = serde_json::from_str(&content)
            .context("Failed to parse storage file")?;
        
        let mut map = HashMap::new();
        for conversation in conversations {
            map.insert(conversation.id, conversation);
        }
        
        debug!("Loaded {} conversations from storage file", map.len());
        Ok(map)
    }
    
    /// Save conversations to JSON file
    async fn save_to_file(&self) -> Result<()> {
        let json_content = {
            let conversations = self.conversations.lock().unwrap();
            let conversations_vec: Vec<&Conversation> = conversations.values().collect();
            
            serde_json::to_string_pretty(&conversations_vec)
                .context("Failed to serialize conversations")?
        };
        
        tokio::fs::write(&self.storage_path, json_content).await
            .context("Failed to write storage file")?;
        
        debug!("Saved conversations to storage file");
        Ok(())
    }
}