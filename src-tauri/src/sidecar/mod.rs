pub mod api_logger;
pub mod manager;
pub mod protocol;

pub use manager::SidecarManager;
pub use protocol::{RpcRequest, RpcResponse, RpcError};

use std::sync::OnceLock;
use serde_json::json;
use crate::storage::config::{ConfigStore, EmbeddingProviderType};
use crate::storage::keychain::KeychainService;

/// Global sidecar instance
static SIDECAR: OnceLock<SidecarManager> = OnceLock::new();

/// Get the global sidecar instance
pub fn get_sidecar() -> &'static SidecarManager {
    SIDECAR.get_or_init(|| SidecarManager::new(None))
}

/// Ensure sidecar is running
pub fn ensure_sidecar_running() -> Result<(), String> {
    let sidecar = get_sidecar();
    if !sidecar.is_running() {
        sidecar.start().map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Get embedding config from settings
fn get_embedding_config_json() -> Result<serde_json::Value, String> {
    let config_store = ConfigStore::new().map_err(|e| e.to_string())?;
    let keychain = KeychainService::new();

    let config = config_store.get_embedding_config().map_err(|e| e.to_string())?;

    match config.active_provider {
        EmbeddingProviderType::Disabled => {
            Err("Embedding is disabled in settings".to_string())
        }
        EmbeddingProviderType::OpenAI => {
            let settings = &config.openai;
            let api_key = if settings.api_key_ref.is_empty() {
                String::new()
            } else {
                keychain.get_key(&settings.api_key_ref).unwrap_or_default()
            };
            Ok(json!({
                "provider": "openai",
                "api_key": api_key,
                "model": if settings.model.is_empty() { "text-embedding-3-small".to_string() } else { settings.model.clone() },
                "dimensions": settings.dimensions,
                "base_url": settings.base_url.clone().unwrap_or_else(|| "https://api.openai.com/v1".to_string())
            }))
        }
        EmbeddingProviderType::Gemini => {
            let settings = &config.gemini;
            let api_key = if settings.api_key_ref.is_empty() {
                String::new()
            } else {
                keychain.get_key(&settings.api_key_ref).unwrap_or_default()
            };
            Ok(json!({
                "provider": "gemini",
                "api_key": api_key,
                "model": if settings.model.is_empty() { "text-embedding-004".to_string() } else { settings.model.clone() },
                "dimensions": settings.dimensions,
                "proxy_url": settings.proxy_url.clone()
            }))
        }
    }
}

/// Generate embedding for a single text using Python sidecar
///
/// Returns an embedding vector (dimensions depend on model)
pub async fn generate_embedding(text: &str) -> Result<Vec<f32>, String> {
    generate_embedding_with_purpose(text, EmbeddingPurpose::Other).await
}

/// Generate embedding for a single text with explicit purpose
pub async fn generate_embedding_with_purpose(text: &str, purpose: EmbeddingPurpose) -> Result<Vec<f32>, String> {
    ensure_sidecar_running()?;

    let embedding_config = get_embedding_config_json()?;

    let sidecar = get_sidecar();
    let params = json!({
        "text": text,
        "config": embedding_config,
        "purpose": purpose.as_str()
    });

    let result = sidecar.call("generate_embedding", params)
        .await
        .map_err(|e| format!("Sidecar embedding call failed: {}", e))?;

    let embedding = result.get("embedding")
        .and_then(|v| v.as_array())
        .ok_or("Invalid embedding response: missing 'embedding' field")?
        .iter()
        .map(|v| v.as_f64().unwrap_or(0.0) as f32)
        .collect();

    Ok(embedding)
}

/// Embedding purpose for API logging
#[derive(Debug, Clone, Copy)]
pub enum EmbeddingPurpose {
    /// Storing chapter content for later retrieval
    ChapterStorage,
    /// RAG query for context building
    RagQuery,
    /// Entity description embedding
    EntityDescription,
    /// Other/unspecified
    Other,
}

impl EmbeddingPurpose {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ChapterStorage => "chapter_storage",
            Self::RagQuery => "rag_query",
            Self::EntityDescription => "entity_description",
            Self::Other => "other",
        }
    }
}

/// Generate embeddings for multiple texts using Python sidecar (batch)
///
/// More efficient than calling generate_embedding multiple times
pub async fn generate_embeddings(texts: &[String]) -> Result<Vec<Vec<f32>>, String> {
    generate_embeddings_with_purpose(texts, EmbeddingPurpose::Other).await
}

/// Generate embeddings with explicit purpose for API logging
pub async fn generate_embeddings_with_purpose(texts: &[String], purpose: EmbeddingPurpose) -> Result<Vec<Vec<f32>>, String> {
    if texts.is_empty() {
        return Ok(Vec::new());
    }

    ensure_sidecar_running()?;

    let embedding_config = get_embedding_config_json()?;

    let sidecar = get_sidecar();
    let params = json!({
        "texts": texts,
        "config": embedding_config,
        "purpose": purpose.as_str()
    });

    let result = sidecar.call("generate_embeddings", params)
        .await
        .map_err(|e| format!("Sidecar embeddings call failed: {}", e))?;

    let embeddings = result.get("embeddings")
        .and_then(|v| v.as_array())
        .ok_or("Invalid embeddings response: missing 'embeddings' field")?
        .iter()
        .map(|arr| {
            arr.as_array()
                .unwrap_or(&Vec::new())
                .iter()
                .map(|v| v.as_f64().unwrap_or(0.0) as f32)
                .collect()
        })
        .collect();

    Ok(embeddings)
}

/// Generate embedding synchronously (blocking)
///
/// Use this when you need embedding in a sync context.
/// Note: This blocks the current thread, use async version when possible.
pub fn generate_embedding_sync(text: &str) -> Result<Vec<f32>, String> {
    generate_embedding_sync_with_purpose(text, EmbeddingPurpose::Other)
}

/// Generate embedding synchronously with explicit purpose
pub fn generate_embedding_sync_with_purpose(text: &str, purpose: EmbeddingPurpose) -> Result<Vec<f32>, String> {
    // Check if we're already in a tokio runtime
    if let Ok(handle) = tokio::runtime::Handle::try_current() {
        // We're inside an async context, use block_in_place
        tokio::task::block_in_place(|| {
            handle.block_on(generate_embedding_with_purpose(text, purpose))
        })
    } else {
        // Not in a runtime, create a new one
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| format!("Failed to create runtime: {}", e))?;

        rt.block_on(generate_embedding_with_purpose(text, purpose))
    }
}

/// Generate embeddings in batch synchronously (blocking)
///
/// Use this when you need batch embeddings in a sync context.
/// Note: This blocks the current thread, use async version when possible.
pub fn generate_embeddings_sync(texts: &[String]) -> Result<Vec<Vec<f32>>, String> {
    generate_embeddings_sync_with_purpose(texts, EmbeddingPurpose::Other)
}

/// Generate embeddings in batch synchronously with explicit purpose
pub fn generate_embeddings_sync_with_purpose(texts: &[String], purpose: EmbeddingPurpose) -> Result<Vec<Vec<f32>>, String> {
    if texts.is_empty() {
        return Ok(Vec::new());
    }

    // Check if we're already in a tokio runtime
    if let Ok(handle) = tokio::runtime::Handle::try_current() {
        // We're inside an async context, use block_in_place
        tokio::task::block_in_place(|| {
            handle.block_on(generate_embeddings_with_purpose(texts, purpose))
        })
    } else {
        // Not in a runtime, create a new one
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| format!("Failed to create runtime: {}", e))?;

        rt.block_on(generate_embeddings_with_purpose(texts, purpose))
    }
}
