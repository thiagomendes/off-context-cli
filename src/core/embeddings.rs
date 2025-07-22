use anyhow::Result;

/// Placeholder for future ML embeddings functionality
/// Currently not used since we're using simple JSON storage
#[derive(Debug)]
pub struct EmbeddingGenerator;

impl EmbeddingGenerator {
    /// Create a new embedding generator (placeholder)
    pub async fn new() -> Result<Self> {
        Ok(Self)
    }

    /// Check if Ollama is available (placeholder - always returns false)
    pub async fn is_ollama_available(&self) -> bool {
        false
    }
}

/// Type alias for compatibility
#[allow(dead_code)]
pub type EmbeddingsService = EmbeddingGenerator;