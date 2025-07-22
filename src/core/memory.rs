use anyhow::{Context, Result};
use qdrant_client::{
    prelude::*,
    qdrant::{
        vectors::VectorsConfig, CollectionOperationResponse, CreateCollection, Distance,
        PointStruct, SearchPoints, VectorParams, Value,
    },
    client::QdrantClient,
};
use std::collections::HashMap;
use tracing::{debug, info, warn};
use uuid::Uuid;

use crate::core::{
    config::get_config,
    embeddings::EmbeddingGenerator,
    types::{Conversation, SearchResult, DatabaseConfig},
};

const COLLECTION_NAME: &str = "conversations";

/// Memory interface for storing and retrieving conversations
pub struct Memory {
    client: QdrantClient,
    collection_name: String,
    embeddings: EmbeddingGenerator,
}

impl Memory {
    /// Create a new memory instance
    pub async fn new(config: &DatabaseConfig) -> Result<Self> {
        // Initialize Qdrant client (embedded mode)
        let client = QdrantClient::from_url("http://localhost:6334")
            .build()
            .context("Failed to create Qdrant client")?;

        let embeddings = EmbeddingGenerator::new().await?;

        let memory = Self {
            client,
            collection_name: config.collection_name.clone(),
            embeddings,
        };

        // Initialize the collection if it doesn't exist
        memory.ensure_collection_exists().await?;

        Ok(memory)
    }

    /// Ensure the conversations collection exists
    async fn ensure_collection_exists(&self) -> Result<()> {
        let config = get_config()?;
        
        // Check if collection exists
        match self.client.collection_info(&self.collection_name).await {
            Ok(_) => {
                debug!("Collection '{}' already exists", self.collection_name);
                return Ok(());
            }
            Err(_) => {
                info!("Creating collection '{}'", self.collection_name);
            }
        }

        // Create collection
        let result: CollectionOperationResponse = self
            .client
            .create_collection(&CreateCollection {
                collection_name: self.collection_name.clone(),
                vectors_config: Some(VectorsConfig::Params(VectorParams {
                    size: config.embeddings.dimension as u64,
                    distance: Distance::Cosine.into(),
                    ..Default::default()
                })),
                ..Default::default()
            })
            .await
            .context("Failed to create collection")?;

        if result.result {
            info!("Successfully created collection '{}'", self.collection_name);
        } else {
            warn!("Failed to create collection '{}'", self.collection_name);
        }

        Ok(())
    }
    
    /// Store a conversation in the memory database
    pub async fn store_conversation(&self, conversation: &Conversation) -> Result<()> {
        // Generate embedding for the conversation
        let text = format!("{}\n{}", conversation.user_message, conversation.assistant_response);
        let embedding = self.embeddings.generate_embedding(&text).await?;

        let mut payload = HashMap::new();
        payload.insert("id".to_string(), Value::from(conversation.id.to_string()));
        payload.insert("timestamp".to_string(), Value::from(conversation.timestamp.to_rfc3339()));
        payload.insert("user_message".to_string(), Value::from(conversation.user_message.clone()));
        payload.insert("assistant_response".to_string(), Value::from(conversation.assistant_response.clone()));
        payload.insert("token_count".to_string(), Value::from(conversation.metadata.token_count as i64));

        if let Some(session_id) = &conversation.metadata.session_id {
            payload.insert("session_id".to_string(), Value::from(session_id.clone()));
        }

        if let Some(project_path) = &conversation.metadata.project_path {
            payload.insert("project_path".to_string(), Value::from(project_path.clone()));
        }

        if !conversation.metadata.tags.is_empty() {
            payload.insert("tags".to_string(), Value::from(conversation.metadata.tags.join(",")));
        }

        let point = PointStruct::new(conversation.id.to_string(), embedding, payload);

        self.client
            .upsert_points_blocking(&self.collection_name, None, vec![point], None)
            .await
            .context("Failed to store conversation")?;

        debug!("Stored conversation {}", conversation.id);
        Ok(())
    }
    
    /// Search for relevant conversations
    pub async fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>> {
        let config = get_config()?;
        
        // Generate embedding for the query
        let query_embedding = self.embeddings.generate_embedding(query).await?;

        let search_result = self
            .client
            .search_points(&SearchPoints {
                collection_name: self.collection_name.clone(),
                vector: query_embedding,
                limit: limit as u64,
                score_threshold: Some(config.context.relevance_threshold),
                with_payload: Some(true.into()),
                ..Default::default()
            })
            .await
            .context("Failed to search conversations")?;

        let mut results = Vec::new();

        for scored_point in search_result.result {
            if let Some(payload) = scored_point.payload {
                let conversation = self.payload_to_conversation(payload)?;
                let snippet = self.create_snippet(&conversation);

                results.push(SearchResult {
                    conversation,
                    score: scored_point.score,
                    snippet,
                });
            }
        }

        Ok(results)
    }
    
    /// Get conversation count
    pub async fn conversation_count(&self) -> Result<usize> {
        let info = self
            .client
            .collection_info(&self.collection_name)
            .await
            .context("Failed to get collection info")?;

        Ok(info.result.unwrap().points_count.unwrap() as usize)
    }

    
    /// Clear all conversations
    pub async fn clear(&self) -> Result<()> {
        self.client
            .delete_collection(&self.collection_name)
            .await
            .context("Failed to delete collection")?;

        self.ensure_collection_exists().await?;
        info!("Memory cleared");
        Ok(())
    }

    /// Get conversation by ID
    pub async fn get_conversation(&self, id: &Uuid) -> Result<Option<Conversation>> {
        let points = self
            .client
            .get_points(
                &self.collection_name,
                None,
                &[id.to_string().into()],
                Some(true),
                Some(false),
                None,
            )
            .await
            .context("Failed to get conversation")?;

        if let Some(point) = points.result.first() {
            if let Some(payload) = &point.payload {
                return Ok(Some(self.payload_to_conversation(payload.clone())?));
            }
        }

        Ok(None)
    }

    /// Retorna todas as conversas salvas
    pub async fn all_conversations(&self) -> Result<Vec<Conversation>> {
        use qdrant_client::qdrant::WithPayloadSelector;
        let points = self.client.scroll_points(
            &self.collection_name,
            None,
            None,
            Some(10000), // Limite alto para garantir tudo
            Some(WithPayloadSelector::Enable(true)),
            None,
            None,
        ).await.context("Failed to scroll all conversations")?;
        let mut conversations = Vec::new();
        for point in points.result.points {
            if let Some(payload) = point.payload {
                if let Ok(conv) = self.payload_to_conversation(payload) {
                    conversations.push(conv);
                }
            }
        }
        Ok(conversations)
    }

    /// Convert Qdrant payload back to Conversation
    fn payload_to_conversation(
        &self,
        payload: HashMap<String, Value>,
    ) -> Result<Conversation> {
        let id = payload
            .get("id")
            .and_then(|v| v.as_str())
            .and_then(|s| Uuid::parse_str(s).ok())
            .context("Invalid conversation ID")?;

        let timestamp = payload
            .get("timestamp")
            .and_then(|v| v.as_str())
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
            .context("Invalid timestamp")?
            .with_timezone(&chrono::Utc);

        let user_message = payload
            .get("user_message")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let assistant_response = payload
            .get("assistant_response")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let token_count = payload
            .get("token_count")
            .and_then(|v| v.as_integer())
            .unwrap_or(0) as usize;

        let session_id = payload
            .get("session_id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let project_path = payload
            .get("project_path")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let tags = payload
            .get("tags")
            .and_then(|v| v.as_str())
            .map(|s| s.split(',').map(|tag| tag.to_string()).collect())
            .unwrap_or_default();

        Ok(Conversation {
            id,
            timestamp,
            user_message,
            assistant_response,
            metadata: crate::core::types::ConversationMetadata {
                session_id,
                project_path,
                tags,
                token_count,
                embedding_model: None,
            },
        })
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
}