use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A conversation between user and assistant
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conversation {
    pub id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub user_message: String,
    pub assistant_response: String,
    pub metadata: ConversationMetadata,
}

/// Metadata associated with a conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationMetadata {
    pub session_id: Option<String>,
    pub project_path: Option<String>,
    pub tags: Vec<String>,
    pub token_count: usize,
    pub embedding_model: Option<String>,
}

/// Claude Code transcript structure
#[derive(Debug, Deserialize)]
pub struct ClaudeTranscript {
    pub messages: Vec<TranscriptMessage>,
    pub session_id: Option<String>,
    #[allow(dead_code)]
    pub created_at: Option<String>,
}

/// Individual message in a Claude Code transcript
#[derive(Debug, Deserialize)]
pub struct TranscriptMessage {
    pub role: String,
    pub content: String,
    pub timestamp: Option<String>,
}

/// Search result from vector database
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub conversation: Conversation,
    pub score: f32,
    pub snippet: String,
}

/// Configuration for the off-context system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub database: DatabaseConfig,
    pub embeddings: EmbeddingsConfig,
    pub context: ContextConfig,
    pub hooks: HooksConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub path: String,
    pub collection_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingsConfig {
    pub provider: String,
    pub model: String,
    pub dimension: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextConfig {
    pub max_results: usize,
    pub max_tokens: usize,
    pub relevance_threshold: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HooksConfig {
    pub enabled: bool,
    pub auto_inject: bool,
}