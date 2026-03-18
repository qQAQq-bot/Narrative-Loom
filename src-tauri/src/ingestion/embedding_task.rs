use crate::core::embedding::{Chunk, VectorEntry, DEFAULT_EMBEDDING_DIMENSIONS};
use crate::core::ids::{BookId, ChapterId};
use crate::sidecar::EmbeddingPurpose;
use crate::storage::book_db::BookDb;
use crate::storage::config::{ConfigStore, EmbeddingProviderType};
use crate::storage::vectors::VectorDb;
use std::path::Path;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum EmbeddingTaskError {
    #[error("Vector DB error: {0}")]
    VectorDbError(#[from] crate::storage::vectors::VectorDbError),

    #[error("Book DB error: {0}")]
    BookDbError(#[from] crate::storage::book_db::BookDbError),

    #[error("Embedding generation failed: {0}")]
    EmbeddingFailed(String),

    #[error("Task cancelled")]
    Cancelled,
}

#[derive(Debug, Clone)]
pub struct EmbeddingProgress {
    pub book_id: String,
    pub total_chapters: u32,
    pub processed_chapters: u32,
    pub total_chunks: u32,
    pub processed_chunks: u32,
    pub current_chapter: Option<String>,
    pub status: EmbeddingStatus,
}

#[derive(Debug, Clone, PartialEq)]
pub enum EmbeddingStatus {
    Pending,
    InProgress,
    Completed,
    Failed(String),
}

#[derive(Debug)]
pub struct EmbeddingTask {
    pub book_id: BookId,
    pub book_dir: std::path::PathBuf,
    pub dimensions: u32,
}

pub struct EmbeddingResult {
    pub book_id: String,
    pub total_chunks: u32,
    pub successful_chunks: u32,
    pub failed_chunks: u32,
}

#[derive(Debug, Clone)]
struct ActiveEmbeddingSignature {
    provider: String,
    model: String,
    requested_dimensions: Option<u32>,
}

fn get_active_embedding_signature() -> Result<ActiveEmbeddingSignature, EmbeddingTaskError> {
    let config_store =
        ConfigStore::new().map_err(|e| EmbeddingTaskError::EmbeddingFailed(e.to_string()))?;
    let config = config_store
        .get_embedding_config()
        .map_err(|e| EmbeddingTaskError::EmbeddingFailed(e.to_string()))?;

    let (provider, settings) = match config.active_provider {
        EmbeddingProviderType::OpenAI => ("openai", &config.openai),
        EmbeddingProviderType::Gemini => ("gemini", &config.gemini),
        EmbeddingProviderType::Disabled => {
            return Err(EmbeddingTaskError::EmbeddingFailed(
                "Embedding is disabled in settings".to_string(),
            ));
        }
    };

    let default_model = match provider {
        "openai" => "text-embedding-3-small",
        "gemini" => "text-embedding-004",
        _ => "",
    };

    let model = if settings.model.trim().is_empty() {
        default_model.to_string()
    } else {
        settings.model.clone()
    };

    Ok(ActiveEmbeddingSignature {
        provider: provider.to_string(),
        model,
        requested_dimensions: settings.dimensions,
    })
}

fn require_compatible_vectors_db(
    vector_db: &VectorDb,
    embedding_count: usize,
    expected: &ActiveEmbeddingSignature,
) -> Result<Option<crate::storage::vectors::VectorDbEmbeddingSignature>, EmbeddingTaskError> {
    let existing_sig = vector_db.get_embedding_signature()?;

    // If there are already embeddings but no signature recorded, we cannot safely add new vectors
    // without risking dimension mismatches. Force a rebuild to make it explicit.
    if embedding_count > 0 && existing_sig.is_none() {
        return Err(EmbeddingTaskError::EmbeddingFailed(
            "Vector DB signature missing (old vectors.db). Please rebuild embeddings.".to_string(),
        ));
    }

    if let Some(sig) = &existing_sig {
        if sig.provider != expected.provider || sig.model != expected.model {
            return Err(EmbeddingTaskError::EmbeddingFailed(format!(
                "Vector DB embedding signature mismatch. DB uses {} / {}, but settings are {} / {}. Please rebuild embeddings.",
                sig.provider, sig.model, expected.provider, expected.model
            )));
        }

        if let Some(req_dims) = expected.requested_dimensions {
            if sig.dimensions != req_dims {
                return Err(EmbeddingTaskError::EmbeddingFailed(format!(
                    "Vector DB embedding dimensions mismatch. DB dim={}, settings dim={}. Please rebuild embeddings.",
                    sig.dimensions, req_dims
                )));
            }
        }
    }

    Ok(existing_sig)
}

const CHUNK_SIZE_TARGET: usize = 1500;  // ~1500 chars per chunk, within embedding token limit
const CHUNK_SIZE_MIN: usize = 200;
const EMBEDDING_BATCH_SIZE: usize = 20;  // Process embeddings in batches
const EMBEDDING_BATCH_DELAY_MS: u64 = 500;  // Delay between batches to avoid rate limiting

pub fn generate_embeddings_for_book(
    book_dir: &Path,
    book_id: &BookId,
) -> Result<EmbeddingResult, EmbeddingTaskError> {
    let vectors_path = book_dir.join("vectors.db");
    let vector_db = VectorDb::open(&vectors_path, DEFAULT_EMBEDDING_DIMENSIONS)?;

    let book_db_path = book_dir.join("book.db");
    let book_db = BookDb::open(&book_db_path, book_id.clone())?;

    let chapters = book_db.list_chapters()?;

    let mut total_chunks: u32 = 0;
    let mut successful_chunks: u32 = 0;
    let mut failed_chunks: u32 = 0;

    // Collect all chunks first
    let mut all_chunks: Vec<Chunk> = Vec::new();

    for chapter_summary in &chapters {
        let chapter_id = &chapter_summary.id;

        let content = match book_db.get_chapter_content(chapter_id) {
            Ok(Some(c)) => c,
            Ok(None) => {
                tracing::warn!("No content found for chapter {}", chapter_id);
                continue;
            }
            Err(e) => {
                tracing::warn!("Failed to read chapter {}: {}", chapter_id, e);
                continue;
            }
        };

        let chunks = chunk_text_for_embedding(&content, chapter_id, chapter_summary.index_num);
        total_chunks += chunks.len() as u32;
        all_chunks.extend(chunks);
    }

    if all_chunks.is_empty() {
        return Ok(EmbeddingResult {
            book_id: book_id.to_string(),
            total_chunks: 0,
            successful_chunks: 0,
            failed_chunks: 0,
        });
    }

    // Batch insert all chunks
    vector_db.insert_chunks(&all_chunks)?;

    // Generate embeddings in batches
    tracing::info!(
        "Generating embeddings for {} chunks in book {}",
        all_chunks.len(),
        book_id
    );

    for batch_start in (0..all_chunks.len()).step_by(EMBEDDING_BATCH_SIZE) {
        let batch_end = (batch_start + EMBEDDING_BATCH_SIZE).min(all_chunks.len());
        let batch_chunks = &all_chunks[batch_start..batch_end];

        // Collect texts for batch embedding
        let texts: Vec<String> = batch_chunks.iter().map(|c| c.content.clone()).collect();

        // Generate embeddings
        match crate::sidecar::generate_embeddings_sync_with_purpose(&texts, EmbeddingPurpose::ChapterStorage) {
            Ok(embeddings) => {
                // Insert embeddings
                for (chunk, embedding) in batch_chunks.iter().zip(embeddings.into_iter()) {
                    let entry = VectorEntry::new(chunk.id.clone(), embedding);
                    match vector_db.insert_vector(&entry) {
                        Ok(_) => successful_chunks += 1,
                        Err(e) => {
                            tracing::warn!("Failed to insert embedding for chunk {}: {}", chunk.id, e);
                            failed_chunks += 1;
                        }
                    }
                }
            }
            Err(e) => {
                tracing::warn!(
                    "Failed to generate embeddings for batch {}-{}: {}",
                    batch_start,
                    batch_end,
                    e
                );
                failed_chunks += batch_chunks.len() as u32;
            }
        }

        tracing::debug!(
            "Processed embedding batch {}-{} / {}",
            batch_start,
            batch_end,
            all_chunks.len()
        );

        // Add delay between batches to avoid rate limiting
        if batch_end < all_chunks.len() {
            std::thread::sleep(std::time::Duration::from_millis(EMBEDDING_BATCH_DELAY_MS));
        }
    }

    tracing::info!(
        "Embedding generation complete for book {}: {} successful, {} failed",
        book_id,
        successful_chunks,
        failed_chunks
    );

    Ok(EmbeddingResult {
        book_id: book_id.to_string(),
        total_chunks,
        successful_chunks,
        failed_chunks,
    })
}

pub fn generate_embeddings_for_chapter(
    book_dir: &Path,
    _book_id: &BookId,
    chapter_id: &ChapterId,
    content: &str,
    chapter_index: u32,
) -> Result<u32, EmbeddingTaskError> {
    let expected = get_active_embedding_signature()?;

    let vectors_path = book_dir.join("vectors.db");
    let vector_db = VectorDb::open(&vectors_path, DEFAULT_EMBEDDING_DIMENSIONS)?;

    let existing_embedding_count = vector_db.count_embeddings().unwrap_or(0);
    let existing_sig =
        require_compatible_vectors_db(&vector_db, existing_embedding_count, &expected)?;

    let chunks = chunk_text_for_embedding(content, chapter_id, chapter_index);

    if chunks.is_empty() {
        return Ok(0);
    }

    // Generate embeddings in batches (do not write into vectors.db until we're sure
    // dimensions match the existing DB signature).
    let mut embeddings_by_chunk: Vec<Option<Vec<f32>>> = vec![None; chunks.len()];
    let mut first_dims: Option<u32> = None;

    for batch_start in (0..chunks.len()).step_by(EMBEDDING_BATCH_SIZE) {
        let batch_end = (batch_start + EMBEDDING_BATCH_SIZE).min(chunks.len());
        let batch_chunks = &chunks[batch_start..batch_end];

        let texts: Vec<String> = batch_chunks.iter().map(|c| c.content.clone()).collect();

        match crate::sidecar::generate_embeddings_sync_with_purpose(
            &texts,
            EmbeddingPurpose::ChapterStorage,
        ) {
            Ok(embeddings) => {
                for (idx, embedding) in (batch_start..batch_end).zip(embeddings.into_iter()) {
                    if first_dims.is_none() {
                        first_dims = Some(embedding.len() as u32);
                    }
                    embeddings_by_chunk[idx] = Some(embedding);
                }
            }
            Err(e) => {
                tracing::warn!("Failed to generate embeddings for chapter {}: {}", chapter_id, e);
            }
        }

        // Add delay between batches to avoid rate limiting
        if batch_end < chunks.len() {
            std::thread::sleep(std::time::Duration::from_millis(EMBEDDING_BATCH_DELAY_MS));
        }
    }

    let Some(current_dims) = first_dims else {
        // No embeddings generated; keep DB unchanged so we can retry later.
        return Ok(0);
    };

    if let Some(sig) = &existing_sig {
        if sig.dimensions != current_dims {
            return Err(EmbeddingTaskError::EmbeddingFailed(format!(
                "Vector DB embedding dimensions mismatch. DB dim={}, current request dim={}. Please rebuild embeddings.",
                sig.dimensions, current_dims
            )));
        }
    }

    if let Some(req_dims) = expected.requested_dimensions {
        if req_dims != current_dims {
            return Err(EmbeddingTaskError::EmbeddingFailed(format!(
                "Embedding API returned dim={}, but settings specify dim={}. Please adjust dimensions and rebuild embeddings.",
                current_dims, req_dims
            )));
        }
    }

    // If this vectors.db doesn't have a signature yet (fresh DB), record it now.
    if existing_sig.is_none() && existing_embedding_count == 0 {
        vector_db.set_embedding_signature(&expected.provider, &expected.model, current_dims)?;
    }

    // Now write to vectors.db
    vector_db.delete_chunks_by_chapter(chapter_id.as_str())?;
    vector_db.insert_chunks(&chunks)?;

    let mut inserted = 0u32;
    for (chunk, embedding_opt) in chunks.iter().zip(embeddings_by_chunk.into_iter()) {
        if let Some(embedding) = embedding_opt {
            let entry = VectorEntry::new(chunk.id.clone(), embedding);
            if vector_db.insert_vector(&entry).is_ok() {
                inserted += 1;
            }
        }
    }

    Ok(inserted)
}

fn chunk_text_for_embedding(content: &str, chapter_id: &ChapterId, chapter_index: u32) -> Vec<Chunk> {
    let mut chunks = Vec::new();
    let paragraphs: Vec<&str> = content
        .split("\n\n")
        .map(|p| p.trim())
        .filter(|p| !p.is_empty())
        .collect();
    
    let mut current_chunk = String::new();
    let mut chunk_start: u32 = 0;
    let mut char_offset: u32 = 0;
    let mut chunk_index: u32 = 0;
    
    for para in paragraphs {
        let para_chars = para.chars().count();
        
        if current_chunk.is_empty() {
            chunk_start = char_offset;
        }
        
        if !current_chunk.is_empty() {
            current_chunk.push_str("\n\n");
        }
        current_chunk.push_str(para);
        
        char_offset += para_chars as u32 + 2;
        
        if current_chunk.chars().count() >= CHUNK_SIZE_TARGET {
            let chunk = create_chunk(
                &current_chunk,
                chapter_id,
                chapter_index,
                chunk_index,
                chunk_start,
                char_offset,
            );
            chunks.push(chunk);
            chunk_index += 1;
            current_chunk.clear();
        }
    }
    
    if current_chunk.chars().count() >= CHUNK_SIZE_MIN {
        let chunk = create_chunk(
            &current_chunk,
            chapter_id,
            chapter_index,
            chunk_index,
            chunk_start,
            char_offset,
        );
        chunks.push(chunk);
    } else if !current_chunk.is_empty() && !chunks.is_empty() {
        if let Some(last_chunk) = chunks.last_mut() {
            last_chunk.content.push_str("\n\n");
            last_chunk.content.push_str(&current_chunk);
            last_chunk.char_end = char_offset;
        }
    } else if !current_chunk.is_empty() {
        let chunk = create_chunk(
            &current_chunk,
            chapter_id,
            chapter_index,
            chunk_index,
            chunk_start,
            char_offset,
        );
        chunks.push(chunk);
    }
    
    chunks
}

fn create_chunk(
    content: &str,
    chapter_id: &ChapterId,
    _chapter_index: u32,
    chunk_index: u32,
    char_start: u32,
    char_end: u32,
) -> Chunk {
    Chunk::new_paragraph(
        chapter_id.to_string(),
        chunk_index,
        content.to_string(),
        char_start,
        char_end,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunk_text_short() {
        let content = "第一段内容。\n\n第二段内容。";
        let chapter_id = ChapterId::from_string("ch-1".to_string());
        let chunks = chunk_text_for_embedding(content, &chapter_id, 1);
        
        assert_eq!(chunks.len(), 1);
        assert!(chunks[0].content.contains("第一段"));
        assert!(chunks[0].content.contains("第二段"));
    }

    #[test]
    fn test_chunk_text_long() {
        let long_para = "这是一个很长的段落。".repeat(100);
        let content = format!("{}\n\n{}", long_para, long_para);
        let chapter_id = ChapterId::from_string("ch-1".to_string());
        let chunks = chunk_text_for_embedding(&content, &chapter_id, 1);
        
        assert!(chunks.len() >= 2);
    }
}
