use anyhow::{Context, Result};
use axum::{
    extract::Query,
    http::StatusCode,
    response::{Html, Json},
    routing::{get, post},
    Router,
};
use rust_embed::RustEmbed;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tower_http::cors::CorsLayer;
use regex::Regex;

use crate::core::{
    config::load_project_config,
    memory::Memory,
    validation::ensure_project_initialized,
};

// Embed static web files
#[derive(RustEmbed)]
#[folder = "src/web/static"]
struct WebAssets;

#[derive(Serialize)]
struct StatusResponse {
    hooks_active: bool,
    database_exists: bool,
    conversation_count: usize,
    database_size: String,
    embeddings_available: bool,
    embeddings_provider: String,
    project_name: String,
    project_root: String,
    config_dir: String,
    database_path: String,
    hooks_path: Option<String>,
    last_activity: Option<String>,
}

#[derive(Deserialize)]
struct SearchQuery {
    q: String,
    #[serde(default = "default_limit")]
    limit: usize,
}

fn default_limit() -> usize {
    10
}

#[derive(Serialize)]
struct SearchResponse {
    query: String,
    results: Vec<SearchResultItem>,
    total_conversations: usize,
}

#[derive(Serialize)]
struct SearchResultItem {
    score: f32,
    timestamp: String,
    user_message: String,
    assistant_response: String,
    snippet: String,
    highlighted_snippet: String,
    project_path: Option<String>,
    tags: Vec<String>,
    token_count: usize,
}

/// Handle the admin command - start web interface
pub async fn handle_admin(port: u16) -> Result<()> {
    ensure_project_initialized()?;
    
    println!("🌐 Starting off-context admin interface...");
    println!("📡 Server: http://localhost:{}", port);
    println!("🔧 Press Ctrl+C to stop");
    
    let app = create_app().await?;
    
    let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{}", port))
        .await
        .context("Failed to bind to port")?;
        
    axum::serve(listener, app)
        .await
        .context("Server error")?;
    
    Ok(())
}

async fn create_app() -> Result<Router> {
    let app = Router::new()
        .route("/", get(serve_index))
        .route("/api/status", get(api_status))
        .route("/api/search", get(api_search))
        .route("/api/export", post(api_export))
        .route("/api/init", post(|| async { api_init().await }))
        .route("/api/clear", post(|| async { api_clear().await }))
        .route("/api/reset", post(|| async { api_reset().await }))
        .route("/static/*file", get(serve_static))
        .layer(CorsLayer::permissive());
    
    Ok(app)
}

async fn serve_index() -> Result<Html<String>, StatusCode> {
    match WebAssets::get("index.html") {
        Some(content) => {
            let content_str = std::str::from_utf8(&content.data)
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            Ok(Html(content_str.to_string()))
        }
        None => {
            // Fallback HTML if file not embedded
            let html = r#"<!DOCTYPE html>
<html><head><title>off-context Admin</title></head>
<body><h1>🧠 off-context Admin</h1>
<p>Web interface prototype - static files not embedded yet</p>
<p>Try: <a href="/api/status">/api/status</a></p>
</body></html>"#;
            Ok(Html(html.to_string()))
        }
    }
}

async fn serve_static(
    axum::extract::Path(file): axum::extract::Path<String>,
) -> Result<axum::response::Response, StatusCode> {
    match WebAssets::get(&file) {
        Some(content) => {
            let mime_type = mime_guess::from_path(&file).first_or_octet_stream();
            
            Ok(axum::response::Response::builder()
                .header(axum::http::header::CONTENT_TYPE, mime_type.as_ref())
                .body(axum::body::Body::from(content.data))
                .unwrap())
        }
        None => Err(StatusCode::NOT_FOUND),
    }
}

async fn api_status() -> Result<Json<StatusResponse>, StatusCode> {
    use crate::core::config::{find_project_root, project_config_dir, claude_code_hooks_dir};
    
    // Reuse logic from status command
    let hooks_active = crate::commands::status::check_hooks_status()
        .await
        .unwrap_or(false);
    
    let db_status = crate::commands::status::check_database_status()
        .await
        .unwrap_or_default();
    
    let embeddings_status = crate::commands::status::check_embeddings_status()
        .await
        .unwrap_or_else(|_| crate::commands::status::EmbeddingsStatus {
            available: false,
            provider: "error".to_string(),
            dimensions: 0,
        });

    // Get project info
    let project_root = find_project_root().unwrap_or_else(|| std::env::current_dir().unwrap());
    let project_name = project_root.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("Unknown Project")
        .to_string();
    
    let config_dir = project_config_dir().unwrap_or_else(|_| project_root.join(".off-context"));
    let database_path = config_dir.join("qdrant");
    let hooks_path = claude_code_hooks_dir().ok();
    
    let response = StatusResponse {
        hooks_active,
        database_exists: db_status.exists,
        conversation_count: db_status.conversation_count,
        database_size: crate::commands::status::format_size(db_status.size_bytes),
        embeddings_available: embeddings_status.available,
        embeddings_provider: embeddings_status.provider,
        project_name,
        project_root: project_root.display().to_string(),
        config_dir: config_dir.display().to_string(),
        database_path: database_path.display().to_string(),
        hooks_path: hooks_path.map(|p| p.display().to_string()),
        last_activity: db_status.last_activity,
    };
    
    Ok(Json(response))
}

async fn api_search(Query(params): Query<SearchQuery>) -> Result<Json<SearchResponse>, StatusCode> {
    // Reuse logic from search command
    let config = load_project_config()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let memory = Memory::new(&config.database)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let search_results = memory
        .search(&params.q, params.limit)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let total_conversations = memory
        .conversation_count()
        .await
        .unwrap_or(0);
    
    let results: Vec<SearchResultItem> = search_results
        .into_iter()
        .map(|r| {
            let conv = &r.conversation;
            SearchResultItem {
                score: r.score,
                timestamp: conv.timestamp.format("%Y-%m-%d %H:%M:%S").to_string(),
                user_message: conv.user_message.clone(),
                assistant_response: conv.assistant_response.clone(),
                snippet: r.snippet.clone(),
                highlighted_snippet: highlight_search_terms(&r.snippet, &params.q),
                project_path: conv.metadata.project_path.clone(),
                tags: conv.metadata.tags.clone(),
                token_count: conv.metadata.token_count,
            }
        })
        .collect();
    
    let response = SearchResponse {
        query: params.q,
        results,
        total_conversations,
    };
    
    Ok(Json(response))
}

async fn api_export(
    Json(payload): Json<HashMap<String, String>>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let format = payload.get("format").unwrap_or(&"json".to_string()).clone();
    
    // Reuse logic from export command
    let config = load_project_config()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let memory = Memory::new(&config.database)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    // Get ALL conversations from database, not just search results
    let all_conversations = memory
        .all_conversations()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    // Convert to SearchResult format for compatibility with export functions
    let search_results: Vec<crate::core::types::SearchResult> = all_conversations
        .into_iter()
        .map(|conv| crate::core::types::SearchResult {
            conversation: conv,
            score: 1.0, // Perfect score since we want all conversations
            snippet: "Full conversation".to_string(), // Not used in export
        })
        .collect();
    
    let content = match format.as_str() {
        "json" => crate::commands::export::export_as_json(&search_results),
        "md" => crate::commands::export::export_as_markdown(&search_results),
        "txt" => crate::commands::export::export_as_text(&search_results),
        _ => return Err(StatusCode::BAD_REQUEST),
    }
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let response = serde_json::json!({
        "format": format,
        "content": content,
        "conversation_count": search_results.len()
    });
    
    Ok(Json(response))
}

async fn api_init() -> Result<Json<serde_json::Value>, StatusCode> {
    println!("API init called");
    match crate::commands::init::handle_init().await {
        Ok(_) => Ok(Json(serde_json::json!({
            "success": true,
            "message": "Project initialized successfully"
        }))),
        Err(e) => {
            eprintln!("Init failed: {}", e);
            Ok(Json(serde_json::json!({
                "success": false,
                "message": format!("Init failed: {}", e)
            })))
        }
    }
}

async fn api_clear() -> Result<Json<serde_json::Value>, StatusCode> {
    match crate::commands::clear::handle_clear().await {
        Ok(_) => Ok(Json(serde_json::json!({
            "success": true,
            "message": "Project hooks cleared successfully"
        }))),
        Err(e) => {
            eprintln!("Clear failed: {}", e);
            Ok(Json(serde_json::json!({
                "success": false,
                "message": format!("Clear failed: {}", e)
            })))
        }
    }
}

async fn api_reset() -> Result<Json<serde_json::Value>, StatusCode> {
    println!("API reset called");
    match reset_memory_only().await {
        Ok(_) => Ok(Json(serde_json::json!({
            "success": true,
            "message": "Memory database reset successfully"
        }))),
        Err(e) => {
            eprintln!("Reset failed: {}", e);
            Ok(Json(serde_json::json!({
                "success": false,
                "message": format!("Reset failed: {}", e)
            })))
        }
    }
}

/// Reset only the memory database, without user interaction (for web API)
async fn reset_memory_only() -> Result<()> {
    use crate::core::{config::{load_project_config, project_config_dir}, memory::Memory, validation::ensure_project_initialized};
    use anyhow::Context;
    
    // Ensure we're in a project
    ensure_project_initialized()?;
    
    // Load config
    let config = load_project_config().await.context("Failed to load configuration")?;
    println!("Resetting off-context memory...");
    
    // Clear database
    match Memory::new(&config.database).await {
        Ok(memory) => {
            memory.clear().await.context("Failed to clear memory database")?;
            println!("Database cleared successfully");
        }
        Err(e) => {
            println!("Failed to initialize memory for clearing: {}", e);
            return Err(anyhow::anyhow!("Database clear failed"));
        }
    }
    
    // Clean up session injection tracking files
    if let Ok(config_dir) = project_config_dir() {
        if let Ok(entries) = std::fs::read_dir(&config_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(name) = path.file_name() {
                    if name.to_string_lossy().starts_with("session_injected_") {
                        match tokio::fs::remove_file(&path).await {
                            Ok(_) => println!("Session tracking file removed: {}", name.to_string_lossy()),
                            Err(_) => println!("Could not remove session file: {}", name.to_string_lossy()),
                        }
                    }
                }
            }
        }
    }
    
    Ok(())
}

/// Highlight search terms in text with HTML <mark> tags
fn highlight_search_terms(text: &str, query: &str) -> String {
    if query.trim().is_empty() {
        return text.to_string();
    }
    
    // Split query into individual words and escape special regex characters
    let terms: Vec<String> = query
        .split_whitespace()
        .filter(|term| !term.is_empty())
        .map(|term| regex::escape(term))
        .collect();
    
    if terms.is_empty() {
        return text.to_string();
    }
    
    // Create a regex pattern that matches any of the terms (case insensitive)
    let pattern = format!("(?i)({})", terms.join("|"));
    
    match Regex::new(&pattern) {
        Ok(re) => {
            // Replace all matches with highlighted versions
            re.replace_all(text, "<mark class=\"bg-yellow-200 text-yellow-900 px-1 rounded\">$1</mark>").to_string()
        }
        Err(_) => {
            // If regex fails, return original text
            text.to_string()
        }
    }
}