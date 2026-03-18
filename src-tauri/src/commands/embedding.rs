use crate::core::embedding::{
    Chunk, ChunkType, SimilarChunk, VectorEntry, DEFAULT_EMBEDDING_DIMENSIONS,
    DEFAULT_EMBEDDING_DIMENSIONS_USIZE,
};
use crate::core::ids::BookId;
use crate::ingestion::generate_embeddings_for_chapter;
use crate::storage::book_db::BookDb;
use crate::storage::config::{ConfigStore, EmbeddingProviderType};
use crate::storage::library::Library;
use crate::storage::paths;
use crate::storage::vectors::{VectorDb, VectorDbEmbeddingSignature};
use serde::{Deserialize, Serialize};

/// Entity info for matching in text
#[derive(Debug, Clone)]
struct EntityInfo {
    id: String,
    name: String,
    aliases: Vec<String>,
}

/// Entity type for embedding updates
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntityType {
    Character,
    Setting,
    Event,
}

impl EntityType {
    pub fn as_str(&self) -> &'static str {
        match self {
            EntityType::Character => "character",
            EntityType::Setting => "setting",
            EntityType::Event => "event",
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ChunkInfo {
    pub content: String,
    pub char_start: u32,
    pub char_end: u32,
    pub index: u32,
}

#[derive(Debug, Serialize)]
pub struct EmbeddingResult {
    pub chunk_id: String,
    pub success: bool,
    pub error: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct GenerateEmbeddingsResult {
    pub total_chunks: usize,
    pub successful: usize,
    pub failed: usize,
    pub results: Vec<EmbeddingResult>,
}

// ============================================================================
// Vector DB Compatibility (Embedding Signature) Helpers
// ============================================================================

#[derive(Debug, Clone, Serialize)]
pub struct CurrentEmbeddingSignatureInfo {
    pub provider: String,
    pub model: String,
    pub dimensions: Option<u32>,
}

#[derive(Debug, Clone, Serialize)]
pub struct VectorDbIncompatibilityInfo {
    pub book_id: String,
    pub title: String,
    pub chunk_count: usize,
    pub embedding_count: usize,
    pub db_signature: Option<VectorDbEmbeddingSignature>,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct VectorDbCompatibilityResult {
    pub current: CurrentEmbeddingSignatureInfo,
    pub incompatible: Vec<VectorDbIncompatibilityInfo>,
    pub total_books: usize,
}

fn infer_default_embedding_dimensions(provider: &str, model: &str) -> Option<u32> {
    match provider {
        // Keep this list minimal and conservative; unknown models return None.
        "openai" => match model {
            "text-embedding-3-small" => Some(1536),
            "text-embedding-3-large" => Some(3072),
            "text-embedding-ada-002" => Some(1536),
            _ => None,
        },
        "gemini" => match model {
            "text-embedding-004" => Some(768),
            "text-embedding-preview-0815" => Some(768),
            "embedding-001" => Some(768),
            _ => None,
        },
        _ => None,
    }
}

fn get_current_embedding_signature() -> Result<CurrentEmbeddingSignatureInfo, String> {
    let config_store = ConfigStore::new().map_err(|e| e.to_string())?;
    let config = config_store.get_embedding_config().map_err(|e| e.to_string())?;

    let (provider, settings, default_model) = match config.active_provider {
        EmbeddingProviderType::Disabled => {
            return Ok(CurrentEmbeddingSignatureInfo {
                provider: "disabled".to_string(),
                model: "".to_string(),
                dimensions: None,
            });
        }
        EmbeddingProviderType::OpenAI => ("openai", &config.openai, "text-embedding-3-small"),
        EmbeddingProviderType::Gemini => ("gemini", &config.gemini, "text-embedding-004"),
    };

    let model = if settings.model.trim().is_empty() {
        default_model.to_string()
    } else {
        settings.model.clone()
    };

    let dimensions = settings
        .dimensions
        .or_else(|| infer_default_embedding_dimensions(provider, &model));

    Ok(CurrentEmbeddingSignatureInfo {
        provider: provider.to_string(),
        model,
        dimensions,
    })
}

#[tauri::command]
pub async fn check_vector_db_compatibility() -> Result<VectorDbCompatibilityResult, String> {
    let current = get_current_embedding_signature()?;

    let library = Library::open().map_err(|e| e.to_string())?;
    let books = library.list_books().map_err(|e| e.to_string())?;
    let total_books = books.len();

    // If embedding is disabled, we can't meaningfully compare signatures.
    if current.provider == "disabled" {
        return Ok(VectorDbCompatibilityResult {
            current,
            incompatible: Vec::new(),
            total_books,
        });
    }

    let mut incompatible = Vec::new();

    for book in books {
        let vectors_path = library.book_dir(&book.id).join("vectors.db");
        if !vectors_path.exists() {
            continue;
        }

        let db = match VectorDb::open(&vectors_path, DEFAULT_EMBEDDING_DIMENSIONS) {
            Ok(db) => db,
            Err(e) => {
                incompatible.push(VectorDbIncompatibilityInfo {
                    book_id: book.id.to_string(),
                    title: book.title,
                    chunk_count: 0,
                    embedding_count: 0,
                    db_signature: None,
                    reason: format!("无法打开 vectors.db: {}", e),
                });
                continue;
            }
        };

        let chunk_count = db.count_chunks().unwrap_or(0);
        let embedding_count = db.count_embeddings().unwrap_or(0);

        // No embeddings yet -> nothing to rebuild.
        if embedding_count == 0 {
            continue;
        }

        let sig = db.get_embedding_signature().unwrap_or(None);

        let mut reason: Option<String> = None;
        if sig.is_none() {
            reason = Some("向量库缺少签名信息（旧版本），需要重建".to_string());
        } else if let Some(sig) = &sig {
            if sig.provider != current.provider || sig.model != current.model {
                reason = Some(format!(
                    "签名不一致：向量库为 {}/{}，当前设置为 {}/{}",
                    sig.provider, sig.model, current.provider, current.model
                ));
            } else if let Some(req_dims) = current.dimensions {
                if sig.dimensions != req_dims {
                    reason = Some(format!(
                        "维度不一致：向量库为 {}，当前设置为 {}",
                        sig.dimensions, req_dims
                    ));
                }
            }
        }

        if let Some(reason) = reason {
            incompatible.push(VectorDbIncompatibilityInfo {
                book_id: book.id.to_string(),
                title: book.title,
                chunk_count,
                embedding_count,
                db_signature: sig,
                reason,
            });
        }
    }

    Ok(VectorDbCompatibilityResult {
        current,
        incompatible,
        total_books,
    })
}

#[derive(Debug, Deserialize)]
pub struct SearchSimilarRequest {
    pub book_id: String,
    pub query: String,
    pub top_k: Option<usize>,
    pub chunk_type: Option<String>,
    pub exclude_chapter_id: Option<String>,
}

#[tauri::command]
pub async fn generate_chapter_embeddings(
    book_id: String,
    chapter_id: String,
    chapter_content: String,
) -> Result<GenerateEmbeddingsResult, String> {
    let vectors_path = get_vectors_db_path(&book_id)?;
    let db = VectorDb::open(&vectors_path, DEFAULT_EMBEDDING_DIMENSIONS).map_err(|e| e.to_string())?;

    db.delete_chunks_by_chapter(&chapter_id)
        .map_err(|e| e.to_string())?;

    // Load all entities for recognition
    let entities = load_all_entities(&book_id).unwrap_or_default();

    // Create chunks with entity recognition
    let chunks = if entities.is_empty() {
        // Fallback to simple chunking if no entities yet
        chunk_text_simple(&chapter_content, &chapter_id)
    } else {
        chunk_text_with_entities(&chapter_content, &chapter_id, &entities)
    };

    db.insert_chunks(&chunks).map_err(|e| e.to_string())?;

    // Collect all chunk contents for batch embedding
    let texts: Vec<String> = chunks.iter().map(|c| c.content.clone()).collect();

    // Generate embeddings in batch
    let embeddings = generate_real_embeddings(&texts).await;

    let mut results = Vec::new();
    let mut successful = 0;
    let mut failed = 0;

    for (chunk, embedding) in chunks.iter().zip(embeddings.into_iter()) {
        let entry = VectorEntry::new(chunk.id.clone(), embedding);

        match db.insert_vector(&entry) {
            Ok(_) => {
                successful += 1;
                results.push(EmbeddingResult {
                    chunk_id: chunk.id.to_string(),
                    success: true,
                    error: None,
                });
            }
            Err(e) => {
                failed += 1;
                results.push(EmbeddingResult {
                    chunk_id: chunk.id.to_string(),
                    success: false,
                    error: Some(e.to_string()),
                });
            }
        }
    }

    Ok(GenerateEmbeddingsResult {
        total_chunks: chunks.len(),
        successful,
        failed,
        results,
    })
}

#[tauri::command]
pub async fn search_similar_chunks(
    book_id: String,
    query: String,
    top_k: Option<usize>,
    chunk_type: Option<String>,
    exclude_chapter_id: Option<String>,
) -> Result<Vec<SimilarChunk>, String> {
    let vectors_path = get_vectors_db_path(&book_id)?;
    let db = VectorDb::open(&vectors_path, DEFAULT_EMBEDDING_DIMENSIONS).map_err(|e| e.to_string())?;

    let query_embedding = generate_real_embedding(&query).await;

    let chunk_type_filter = chunk_type.and_then(|s| ChunkType::from_str(&s));

    let results = db
        .search_similar(
            &query_embedding,
            top_k.unwrap_or(10),
            exclude_chapter_id.as_deref(),
            chunk_type_filter,
        )
        .map_err(|e| e.to_string())?;

    Ok(results)
}

#[tauri::command]
pub async fn rebuild_book_embeddings(book_id: String) -> Result<GenerateEmbeddingsResult, String> {
    let current = get_current_embedding_signature()?;
    if current.provider == "disabled" {
        return Err("Embedding 已禁用，请先在设置中配置 Embedding 服务".to_string());
    }

    // Resolve book directory
    let library = Library::open().map_err(|e| e.to_string())?;
    let bid = BookId::from_string(book_id.clone());
    let book_dir = library.book_dir(&bid);

    let vectors_path = book_dir.join("vectors.db");
    let book_db_path = book_dir.join("book.db");

    if !book_db_path.exists() {
        return Err("Book database not found".to_string());
    }

    // Open vectors.db (creates if missing) and inspect current state.
    let existing_chunk_count = {
        let db =
            VectorDb::open(&vectors_path, DEFAULT_EMBEDDING_DIMENSIONS).map_err(|e| e.to_string())?;
        db.count_chunks().unwrap_or(0)
    };

    // Rebuild strategy:
    // - If we already have chunks: keep chunks & metadata, rebuild embeddings only.
    // - If no chunks yet: generate chunks from chapter files (lazy embedding style).
    if existing_chunk_count == 0 {
        // Fresh rebuild from book chapters.
        {
            let db = VectorDb::open(&vectors_path, DEFAULT_EMBEDDING_DIMENSIONS)
                .map_err(|e| e.to_string())?;
            db.clear_all().map_err(|e| e.to_string())?;
        }

        let book_db = BookDb::open(&book_db_path, bid.clone()).map_err(|e| e.to_string())?;
        let chapters = book_db.list_chapters().map_err(|e| e.to_string())?;

        for ch in chapters {
            let content = book_db
                .get_chapter_content(&ch.id)
                .map_err(|e| e.to_string())?
                .ok_or_else(|| format!("No content found for chapter {}", ch.id))?;

            // Uses ingestion pipeline (includes signature checks + dimensions enforcement).
            let _ = generate_embeddings_for_chapter(&book_dir, &bid, &ch.id, &content, ch.index_num)
                .map_err(|e| e.to_string())?;
        }
    } else {
        // Rebuild embeddings for existing chunks.
        let db = VectorDb::open(&vectors_path, DEFAULT_EMBEDDING_DIMENSIONS)
            .map_err(|e| e.to_string())?;
        let chunks = db.get_all_chunks().map_err(|e| e.to_string())?;

        // Reset embeddings + signature before regenerating.
        db.clear_embeddings().map_err(|e| e.to_string())?;
        db.clear_embedding_signature().map_err(|e| e.to_string())?;

        const BATCH_SIZE: usize = 20;
        const BATCH_DELAY_MS: u64 = 500;

        let mut signature_set = false;
        let mut expected_dims: Option<u32> = None;

        for batch_start in (0..chunks.len()).step_by(BATCH_SIZE) {
            let batch_end = (batch_start + BATCH_SIZE).min(chunks.len());
            let batch = &chunks[batch_start..batch_end];

            let texts: Vec<String> = batch.iter().map(|c| c.content.clone()).collect();
            let embeddings = crate::sidecar::generate_embeddings_with_purpose(
                &texts,
                crate::sidecar::EmbeddingPurpose::ChapterStorage,
            )
            .await
            .map_err(|e| format!("Embedding generation failed: {}", e))?;

            if embeddings.len() != batch.len() {
                return Err(format!(
                    "Embedding response size mismatch: expected {}, got {}",
                    batch.len(),
                    embeddings.len()
                ));
            }

            if !signature_set {
                let dims = embeddings
                    .get(0)
                    .map(|v| v.len() as u32)
                    .unwrap_or(0);
                if dims == 0 {
                    return Err("Embedding API returned empty vectors".to_string());
                }

                if let Some(req) = current.dimensions {
                    if req != dims {
                        return Err(format!(
                            "Embedding API returned dim={}, but settings specify dim={}. Please adjust dimensions and rebuild.",
                            dims, req
                        ));
                    }
                }

                db.set_embedding_signature(&current.provider, &current.model, dims)
                    .map_err(|e| e.to_string())?;
                signature_set = true;
                expected_dims = Some(dims);
            } else if let Some(dims) = expected_dims {
                let got = embeddings.get(0).map(|v| v.len() as u32).unwrap_or(0);
                if got != dims {
                    return Err(format!(
                        "Embedding dimensions changed mid-rebuild: expected {}, got {}",
                        dims, got
                    ));
                }
            }

            for (chunk, embedding) in batch.iter().zip(embeddings.into_iter()) {
                let entry = VectorEntry::new(chunk.id.clone(), embedding);
                db.insert_vector(&entry).map_err(|e| e.to_string())?;
            }

            if batch_end < chunks.len() {
                std::thread::sleep(std::time::Duration::from_millis(BATCH_DELAY_MS));
            }
        }
    }

    // Return final stats
    let db = VectorDb::open(&vectors_path, DEFAULT_EMBEDDING_DIMENSIONS).map_err(|e| e.to_string())?;
    let chunk_count = db.count_chunks().map_err(|e| e.to_string())?;
    let embedding_count = db.count_embeddings().map_err(|e| e.to_string())?;

    Ok(GenerateEmbeddingsResult {
        total_chunks: chunk_count,
        successful: embedding_count,
        failed: chunk_count.saturating_sub(embedding_count),
        results: vec![],
    })
}

#[tauri::command]
pub fn get_embedding_stats(book_id: String) -> Result<EmbeddingStats, String> {
    let vectors_path = get_vectors_db_path(&book_id)?;

    if !vectors_path.exists() {
        return Ok(EmbeddingStats {
            chunk_count: 0,
            embedding_count: 0,
            has_vectors_db: false,
        });
    }

    let db = VectorDb::open(&vectors_path, DEFAULT_EMBEDDING_DIMENSIONS).map_err(|e| e.to_string())?;

    Ok(EmbeddingStats {
        chunk_count: db.count_chunks().unwrap_or(0),
        embedding_count: db.count_embeddings().unwrap_or(0),
        has_vectors_db: true,
    })
}

#[derive(Debug, Serialize)]
pub struct EmbeddingStats {
    pub chunk_count: usize,
    pub embedding_count: usize,
    pub has_vectors_db: bool,
}

fn get_vectors_db_path(book_id: &str) -> Result<std::path::PathBuf, String> {
    paths::get_vectors_db_path(book_id).map_err(|e| e.to_string())
}

const CHUNK_SIZE_TARGET: usize = 1500;  // ~1500 chars per chunk
const CHUNK_SIZE_MIN: usize = 200;

fn chunk_text_simple(text: &str, chapter_id: &str) -> Vec<Chunk> {
    let paragraphs: Vec<&str> = text.split("\n\n").filter(|p| !p.trim().is_empty()).collect();

    let mut chunks = Vec::new();
    let mut current_chunk = String::new();
    let mut chunk_start: u32 = 0;
    let mut char_offset: u32 = 0;
    let mut chunk_index: u32 = 0;

    for para in paragraphs {
        let para_trimmed = para.trim();
        if para_trimmed.is_empty() {
            continue;
        }

        let para_chars = para_trimmed.chars().count();

        if current_chunk.is_empty() {
            chunk_start = char_offset;
        }

        if !current_chunk.is_empty() {
            current_chunk.push_str("\n\n");
        }
        current_chunk.push_str(para_trimmed);

        char_offset += para_chars as u32 + 2;

        if current_chunk.chars().count() >= CHUNK_SIZE_TARGET {
            chunks.push(Chunk::new_paragraph(
                chapter_id.to_string(),
                chunk_index,
                current_chunk.clone(),
                chunk_start,
                char_offset,
            ));
            chunk_index += 1;
            current_chunk.clear();
        }
    }

    // Handle remaining content
    if current_chunk.chars().count() >= CHUNK_SIZE_MIN {
        chunks.push(Chunk::new_paragraph(
            chapter_id.to_string(),
            chunk_index,
            current_chunk,
            chunk_start,
            char_offset,
        ));
    } else if !current_chunk.is_empty() && !chunks.is_empty() {
        if let Some(last_chunk) = chunks.last_mut() {
            last_chunk.content.push_str("\n\n");
            last_chunk.content.push_str(&current_chunk);
            last_chunk.char_end = char_offset;
        }
    } else if !current_chunk.is_empty() {
        chunks.push(Chunk::new_paragraph(
            chapter_id.to_string(),
            chunk_index,
            current_chunk,
            chunk_start,
            char_offset,
        ));
    }

    chunks
}

fn generate_dummy_embedding(seed: usize) -> Vec<f32> {
    let mut embedding = Vec::with_capacity(DEFAULT_EMBEDDING_DIMENSIONS_USIZE);
    for i in 0..DEFAULT_EMBEDDING_DIMENSIONS_USIZE {
        let val = ((seed + i) as f32 * 0.001).sin();
        embedding.push(val);
    }

    let norm: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 0.0 {
        for x in &mut embedding {
            *x /= norm;
        }
    }

    embedding
}

/// Generate real embedding using sidecar, with fallback to dummy
async fn generate_real_embedding(text: &str) -> Vec<f32> {
    match crate::sidecar::generate_embedding(text).await {
        Ok(embedding) => {
            tracing::debug!("Generated real embedding (dim={})", embedding.len());
            embedding
        }
        Err(e) => {
            tracing::warn!("Failed to generate real embedding, using fallback: {}", e);
            generate_dummy_embedding(text.len())
        }
    }
}

/// Generate real embeddings in batch using sidecar, with fallback to dummy
async fn generate_real_embeddings(texts: &[String]) -> Vec<Vec<f32>> {
    match crate::sidecar::generate_embeddings(texts).await {
        Ok(embeddings) => {
            tracing::debug!("Generated {} real embeddings", embeddings.len());
            embeddings
        }
        Err(e) => {
            tracing::warn!("Failed to generate real embeddings, using fallback: {}", e);
            texts.iter().map(|t| generate_dummy_embedding(t.len())).collect()
        }
    }
}

// ============================================================================
// Entity Embedding Functions (P3.5)
// ============================================================================

/// Build a text description for an entity to be embedded
fn build_entity_description(
    entity_type: &EntityType,
    name: &str,
    description: Option<&str>,
    additional_info: Option<&str>,
) -> String {
    let type_label = match entity_type {
        EntityType::Character => "人物",
        EntityType::Setting => "设定",
        EntityType::Event => "事件",
    };

    let mut text = format!("[{}] {}", type_label, name);

    if let Some(desc) = description {
        if !desc.is_empty() {
            text.push_str(": ");
            text.push_str(desc);
        }
    }

    if let Some(info) = additional_info {
        if !info.is_empty() {
            text.push_str(" | ");
            text.push_str(info);
        }
    }

    text
}

/// Update embedding for a single entity
/// This deletes the old embedding (if exists) and creates a new one
pub fn update_entity_embedding_internal(
    book_id: &str,
    entity_id: &str,
    entity_type: &EntityType,
    name: &str,
    description: Option<&str>,
    additional_info: Option<&str>,
) -> Result<bool, String> {
    let current = get_current_embedding_signature()?;
    if current.provider == "disabled" {
        return Err("Embedding 已禁用，请先在设置中配置 Embedding 服务".to_string());
    }

    let vectors_path = get_vectors_db_path(book_id)?;
    let db = VectorDb::open(&vectors_path, DEFAULT_EMBEDDING_DIMENSIONS).map_err(|e| e.to_string())?;

    let existing_embedding_count = db.count_embeddings().unwrap_or(0);
    let existing_sig = db.get_embedding_signature().map_err(|e| e.to_string())?;

    if existing_embedding_count > 0 && existing_sig.is_none() {
        return Err("向量库缺少签名信息（旧版本），请先重建向量库".to_string());
    }

    if let Some(sig) = &existing_sig {
        if sig.provider != current.provider || sig.model != current.model {
            return Err(format!(
                "向量库签名不一致：向量库为 {}/{}，当前设置为 {}/{}。请先重建向量库。",
                sig.provider, sig.model, current.provider, current.model
            ));
        }
        if let Some(req_dims) = current.dimensions {
            if sig.dimensions != req_dims {
                return Err(format!(
                    "向量库维度不一致：向量库为 {}，当前设置为 {}。请先重建向量库。",
                    sig.dimensions, req_dims
                ));
            }
        }
    }

    // Build the entity description text
    let content = build_entity_description(entity_type, name, description, additional_info);

    if content.trim().is_empty() {
        // No content to embed
        return Ok(false);
    }

    // Generate embedding using sidecar
    let embedding = crate::sidecar::generate_embedding_sync_with_purpose(
        &content,
        crate::sidecar::EmbeddingPurpose::EntityDescription,
    )
    .map_err(|e| format!("Failed to generate embedding: {}", e))?;

    let current_dims = embedding.len() as u32;

    if let Some(sig) = &existing_sig {
        if sig.dimensions != current_dims {
            return Err(format!(
                "向量维度不一致：向量库为 {}，当前 embedding 为 {}。请先重建向量库。",
                sig.dimensions, current_dims
            ));
        }
    }

    if let Some(req_dims) = current.dimensions {
        if req_dims != current_dims {
            return Err(format!(
                "Embedding API 返回维度 {}，但设置中 dimensions 为 {}。请调整 dimensions 并重建向量库。",
                current_dims, req_dims
            ));
        }
    }

    // Fresh DB: record signature now (before writing new vectors).
    if existing_sig.is_none() && existing_embedding_count == 0 {
        db.set_embedding_signature(&current.provider, &current.model, current_dims)
            .map_err(|e| e.to_string())?;
    }

    // Delete existing entity chunks (after compatibility checks)
    db.delete_chunks_by_entity(entity_id)
        .map_err(|e| e.to_string())?;

    // Create a new entity chunk
    // Use "entity" as chapter_id prefix to distinguish from regular chapter chunks
    let chunk = Chunk::new_entity(
        format!("entity:{}", entity_type.as_str()),
        entity_id.to_string(),
        content.clone(),
    );

    // Insert the chunk
    db.insert_chunk(&chunk).map_err(|e| e.to_string())?;

    let entry = VectorEntry::new(chunk.id, embedding);

    db.insert_vector(&entry).map_err(|e| e.to_string())?;

    tracing::debug!(
        "Updated embedding for {} {} ({})",
        entity_type.as_str(),
        entity_id,
        name
    );

    Ok(true)
}

/// Delete embedding for an entity
pub fn delete_entity_embedding_internal(book_id: &str, entity_id: &str) -> Result<usize, String> {
    let vectors_path = get_vectors_db_path(book_id)?;
    let db = VectorDb::open(&vectors_path, DEFAULT_EMBEDDING_DIMENSIONS).map_err(|e| e.to_string())?;

    let deleted = db
        .delete_chunks_by_entity(entity_id)
        .map_err(|e| e.to_string())?;

    Ok(deleted)
}

/// Tauri command: Update embedding for a character
#[tauri::command]
pub async fn update_character_embedding(
    book_id: String,
    character_id: String,
    name: String,
    description: Option<String>,
    traits: Option<Vec<String>>,
    role: Option<String>,
) -> Result<bool, String> {
    // Build additional info from traits and role
    let mut additional_parts = Vec::new();
    if let Some(r) = role {
        if !r.is_empty() {
            additional_parts.push(format!("角色: {}", r));
        }
    }
    if let Some(t) = traits {
        if !t.is_empty() {
            additional_parts.push(format!("特征: {}", t.join(", ")));
        }
    }
    let additional_info = if additional_parts.is_empty() {
        None
    } else {
        Some(additional_parts.join(" | "))
    };

    update_entity_embedding_internal(
        &book_id,
        &character_id,
        &EntityType::Character,
        &name,
        description.as_deref(),
        additional_info.as_deref(),
    )
}

/// Tauri command: Update embedding for a setting
#[tauri::command]
pub async fn update_setting_embedding(
    book_id: String,
    setting_id: String,
    name: String,
    description: Option<String>,
    setting_type: Option<String>,
) -> Result<bool, String> {
    let additional_info = setting_type.map(|t| format!("类型: {}", t));

    update_entity_embedding_internal(
        &book_id,
        &setting_id,
        &EntityType::Setting,
        &name,
        description.as_deref(),
        additional_info.as_deref(),
    )
}

/// Tauri command: Update embedding for an event
#[tauri::command]
pub async fn update_event_embedding(
    book_id: String,
    event_id: String,
    title: String,
    description: Option<String>,
    importance: Option<String>,
) -> Result<bool, String> {
    let additional_info = importance.map(|i| format!("重要性: {}", i));

    update_entity_embedding_internal(
        &book_id,
        &event_id,
        &EntityType::Event,
        &title,
        description.as_deref(),
        additional_info.as_deref(),
    )
}

/// Tauri command: Delete embedding for an entity
#[tauri::command]
pub async fn delete_entity_embedding(book_id: String, entity_id: String) -> Result<usize, String> {
    delete_entity_embedding_internal(&book_id, &entity_id)
}

/// Result for rebuild_entity_embeddings
#[derive(Debug, Serialize)]
pub struct RebuildEntityEmbeddingsResult {
    pub characters_updated: usize,
    pub settings_updated: usize,
    pub events_updated: usize,
    pub total_updated: usize,
    pub errors: Vec<String>,
}

/// Tauri command: Rebuild all entity embeddings for a book
#[tauri::command]
pub async fn rebuild_entity_embeddings(
    book_id: String,
) -> Result<RebuildEntityEmbeddingsResult, String> {
    use crate::core::ids::BookId;
    use crate::storage::book_db::BookDb;
    use crate::storage::library::Library;

    // Open book database
    let library = Library::open().map_err(|e| e.to_string())?;
    let bid = BookId::from_string(book_id.clone());
    let book_dir = library.book_dir(&bid);
    let book_db_path = book_dir.join("book.db");

    if !book_db_path.exists() {
        return Err("Book database not found".to_string());
    }

    let book_db = BookDb::open(&book_db_path, bid).map_err(|e| e.to_string())?;

    let mut result = RebuildEntityEmbeddingsResult {
        characters_updated: 0,
        settings_updated: 0,
        events_updated: 0,
        total_updated: 0,
        errors: Vec::new(),
    };

    // Rebuild character embeddings
    let characters = book_db.list_characters().map_err(|e| e.to_string())?;
    for character in characters {
        let traits_str = if character.traits.is_empty() {
            None
        } else {
            Some(character.traits.join(", "))
        };

        match update_entity_embedding_internal(
            &book_id,
            &character.id.to_string(),
            &EntityType::Character,
            &character.name,
            character.description.as_deref(),
            traits_str.as_deref(),
        ) {
            Ok(true) => result.characters_updated += 1,
            Ok(false) => {} // No content to embed
            Err(e) => result.errors.push(format!("Character {}: {}", character.name, e)),
        }
    }

    // Rebuild setting embeddings
    let settings = book_db.list_settings().map_err(|e| e.to_string())?;
    for setting in settings {
        let type_info = Some(format!("类型: {}", setting.setting_type));

        match update_entity_embedding_internal(
            &book_id,
            &setting.id.to_string(),
            &EntityType::Setting,
            &setting.name,
            setting.description.as_deref(),
            type_info.as_deref(),
        ) {
            Ok(true) => result.settings_updated += 1,
            Ok(false) => {}
            Err(e) => result.errors.push(format!("Setting {}: {}", setting.name, e)),
        }
    }

    // Rebuild event embeddings
    let events = book_db.list_events().map_err(|e| e.to_string())?;
    for event in events {
        let importance_info = Some(format!("重要性: {}", event.importance));

        match update_entity_embedding_internal(
            &book_id,
            &event.id.to_string(),
            &EntityType::Event,
            &event.title,
            event.description.as_deref(),
            importance_info.as_deref(),
        ) {
            Ok(true) => result.events_updated += 1,
            Ok(false) => {}
            Err(e) => result.errors.push(format!("Event {}: {}", event.title, e)),
        }
    }

    result.total_updated =
        result.characters_updated + result.settings_updated + result.events_updated;

    tracing::info!(
        "Rebuilt entity embeddings for book {}: {} characters, {} settings, {} events",
        book_id,
        result.characters_updated,
        result.settings_updated,
        result.events_updated
    );

    Ok(result)
}

/// Get entity embedding stats for a book
#[tauri::command]
pub async fn get_entity_embedding_stats(book_id: String) -> Result<serde_json::Value, String> {
    let vectors_path = get_vectors_db_path(&book_id)?;

    if !vectors_path.exists() {
        return Ok(serde_json::json!({
            "has_vectors_db": false,
            "entity_chunk_count": 0,
            "entity_ids": []
        }));
    }

    let db = VectorDb::open(&vectors_path, DEFAULT_EMBEDDING_DIMENSIONS).map_err(|e| e.to_string())?;

    let entity_count = db.count_entity_chunks().unwrap_or(0);
    let entity_ids = db.list_entity_ids().unwrap_or_default();

    Ok(serde_json::json!({
        "has_vectors_db": true,
        "entity_chunk_count": entity_count,
        "entity_ids": entity_ids
    }))
}

// ============================================================================
// Entity Recognition Functions for Chunk Processing
// ============================================================================

/// Load all entities from book database for text matching
fn load_all_entities(book_id: &str) -> Result<Vec<EntityInfo>, String> {
    let book_db_path = paths::get_book_db_path(book_id).map_err(|e| e.to_string())?;
    let bid = BookId::from_string(book_id.to_string());
    let book_db = BookDb::open(&book_db_path, bid).map_err(|e| e.to_string())?;

    let mut entities = Vec::new();

    // Load characters with their aliases
    if let Ok(characters) = book_db.list_characters() {
        for char in characters {
            entities.push(EntityInfo {
                id: char.id.to_string(),
                name: char.name.clone(),
                aliases: char.aliases.clone(),
            });
        }
    }

    // Load settings
    if let Ok(settings) = book_db.list_settings() {
        for setting in settings {
            entities.push(EntityInfo {
                id: setting.id.to_string(),
                name: setting.name.clone(),
                aliases: vec![],
            });
        }
    }

    // Load events
    if let Ok(events) = book_db.list_events() {
        for event in events {
            entities.push(EntityInfo {
                id: event.id.to_string(),
                name: event.title.clone(),
                aliases: vec![],
            });
        }
    }

    Ok(entities)
}

/// Find all entity IDs mentioned in a text
fn find_entities_in_text(text: &str, entities: &[EntityInfo]) -> Vec<String> {
    let text_lower = text.to_lowercase();
    let mut found_ids = Vec::new();

    for entity in entities {
        // Check main name
        if text_lower.contains(&entity.name.to_lowercase()) {
            if !found_ids.contains(&entity.id) {
                found_ids.push(entity.id.clone());
            }
            continue;
        }

        // Check aliases
        for alias in &entity.aliases {
            if text_lower.contains(&alias.to_lowercase()) {
                if !found_ids.contains(&entity.id) {
                    found_ids.push(entity.id.clone());
                }
                break;
            }
        }
    }

    found_ids
}

/// Create chunks with entity recognition
fn chunk_text_with_entities(text: &str, chapter_id: &str, entities: &[EntityInfo]) -> Vec<Chunk> {
    let paragraphs: Vec<&str> = text.split("\n\n").filter(|p| !p.trim().is_empty()).collect();

    let mut chunks = Vec::new();
    let mut current_chunk = String::new();
    let mut current_entities: Vec<String> = Vec::new();
    let mut chunk_start: u32 = 0;
    let mut char_offset: u32 = 0;
    let mut chunk_index: u32 = 0;

    for para in paragraphs {
        let para_trimmed = para.trim();
        if para_trimmed.is_empty() {
            continue;
        }

        let para_chars = para_trimmed.chars().count();

        if current_chunk.is_empty() {
            chunk_start = char_offset;
        }

        if !current_chunk.is_empty() {
            current_chunk.push_str("\n\n");
        }
        current_chunk.push_str(para_trimmed);

        // Collect entities from this paragraph
        let para_entities = find_entities_in_text(para_trimmed, entities);
        for entity_id in para_entities {
            if !current_entities.contains(&entity_id) {
                current_entities.push(entity_id);
            }
        }

        char_offset += para_chars as u32 + 2;

        if current_chunk.chars().count() >= CHUNK_SIZE_TARGET {
            let chunk = Chunk::new_paragraph(
                chapter_id.to_string(),
                chunk_index,
                current_chunk.clone(),
                chunk_start,
                char_offset,
            ).with_entities(current_entities.clone());

            chunks.push(chunk);
            chunk_index += 1;
            current_chunk.clear();
            current_entities.clear();
        }
    }

    // Handle remaining content
    if current_chunk.chars().count() >= CHUNK_SIZE_MIN {
        let chunk = Chunk::new_paragraph(
            chapter_id.to_string(),
            chunk_index,
            current_chunk,
            chunk_start,
            char_offset,
        ).with_entities(current_entities);
        chunks.push(chunk);
    } else if !current_chunk.is_empty() && !chunks.is_empty() {
        if let Some(last_chunk) = chunks.last_mut() {
            last_chunk.content.push_str("\n\n");
            last_chunk.content.push_str(&current_chunk);
            last_chunk.char_end = char_offset;
            // Merge entities
            for entity_id in current_entities {
                if !last_chunk.metadata.entities_mentioned.contains(&entity_id) {
                    last_chunk.metadata.entities_mentioned.push(entity_id);
                }
            }
        }
    } else if !current_chunk.is_empty() {
        let chunk = Chunk::new_paragraph(
            chapter_id.to_string(),
            chunk_index,
            current_chunk,
            chunk_start,
            char_offset,
        ).with_entities(current_entities);
        chunks.push(chunk);
    }

    chunks
}

// ============================================================================
// Entity Co-occurrence API for Relationship Graph
// ============================================================================

/// Entity mention info for a chunk
#[derive(Debug, Clone, Serialize)]
pub struct ChunkEntityInfo {
    pub chunk_id: String,
    pub char_start: u32,
    pub char_end: u32,
    pub entity_ids: Vec<String>,
}

/// Get entity mentions for a chapter (for editor highlighting)
#[tauri::command]
pub async fn get_chapter_entity_mentions(
    book_id: String,
    chapter_id: String,
) -> Result<Vec<ChunkEntityInfo>, String> {
    let vectors_path = get_vectors_db_path(&book_id)?;

    if !vectors_path.exists() {
        return Ok(vec![]);
    }

    let db = VectorDb::open(&vectors_path, DEFAULT_EMBEDDING_DIMENSIONS).map_err(|e| e.to_string())?;

    let chunks = db
        .get_chunks_by_chapter(&chapter_id)
        .map_err(|e| e.to_string())?;

    Ok(chunks
        .into_iter()
        .filter(|c| !c.metadata.entities_mentioned.is_empty())
        .map(|c| ChunkEntityInfo {
            chunk_id: c.id.to_string(),
            char_start: c.char_start,
            char_end: c.char_end,
            entity_ids: c.metadata.entities_mentioned,
        })
        .collect())
}

/// Co-occurrence pair between two entities
#[derive(Debug, Clone, Serialize)]
pub struct EntityCooccurrence {
    pub entity1_id: String,
    pub entity2_id: String,
    pub count: u32,
    pub chapters: Vec<String>,
}

/// Get entity co-occurrence statistics for relationship graph
#[tauri::command]
pub async fn get_entity_cooccurrence(book_id: String) -> Result<Vec<EntityCooccurrence>, String> {
    let vectors_path = get_vectors_db_path(&book_id)?;

    if !vectors_path.exists() {
        return Ok(vec![]);
    }

    let db = VectorDb::open(&vectors_path, DEFAULT_EMBEDDING_DIMENSIONS).map_err(|e| e.to_string())?;

    // Get all paragraph chunks
    let all_chunks = db
        .get_all_chunks()
        .map_err(|e| e.to_string())?;

    // Count co-occurrences
    use std::collections::HashMap;
    let mut cooccurrence_map: HashMap<(String, String), (u32, Vec<String>)> = HashMap::new();

    for chunk in all_chunks {
        let entities = &chunk.metadata.entities_mentioned;
        if entities.len() < 2 {
            continue;
        }

        // Count pairs
        for i in 0..entities.len() {
            for j in (i + 1)..entities.len() {
                let (e1, e2) = if entities[i] < entities[j] {
                    (entities[i].clone(), entities[j].clone())
                } else {
                    (entities[j].clone(), entities[i].clone())
                };

                let entry = cooccurrence_map.entry((e1, e2)).or_insert((0, vec![]));
                entry.0 += 1;
                if !entry.1.contains(&chunk.chapter_id) {
                    entry.1.push(chunk.chapter_id.clone());
                }
            }
        }
    }

    // Convert to result
    let mut results: Vec<EntityCooccurrence> = cooccurrence_map
        .into_iter()
        .map(|((e1, e2), (count, chapters))| EntityCooccurrence {
            entity1_id: e1,
            entity2_id: e2,
            count,
            chapters,
        })
        .collect();

    // Sort by count descending
    results.sort_by(|a, b| b.count.cmp(&a.count));

    Ok(results)
}
