use crate::core::embedding::{
    Chunk, ChunkId, ChunkMetadata, ChunkType, SimilarChunk, VectorEntry,
};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::path::Path;
use thiserror::Error;

const META_KEY_EMBEDDING_PROVIDER: &str = "embedding.provider";
const META_KEY_EMBEDDING_MODEL: &str = "embedding.model";
const META_KEY_EMBEDDING_DIMENSIONS: &str = "embedding.dimensions";
const META_KEY_EMBEDDING_UPDATED_AT: &str = "embedding.updated_at";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VectorDbEmbeddingSignature {
    pub provider: String,
    pub model: String,
    pub dimensions: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
}

#[derive(Debug, Error)]
pub enum VectorDbError {
    #[error("SQLite error: {0}")]
    SqliteError(#[from] rusqlite::Error),

    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("Vector extension not loaded")]
    ExtensionNotLoaded,

    #[error("Chunk not found: {0}")]
    ChunkNotFound(String),

    #[error("Vector DB embedding signature missing (please rebuild vectors.db)")]
    MissingEmbeddingSignature,

    #[error("Embedding dimensions mismatch: expected {expected}, got {got}")]
    EmbeddingDimensionsMismatch { expected: u32, got: u32 },
}

pub struct VectorDb {
    conn: Connection,
    #[allow(dead_code)]
    dimensions: u32,
}

impl VectorDb {
    pub fn open<P: AsRef<Path>>(path: P, dimensions: u32) -> Result<Self, VectorDbError> {
        let conn = Connection::open(path)?;
        let db = Self { conn, dimensions };
        db.init()?;
        Ok(db)
    }

    pub fn open_in_memory(dimensions: u32) -> Result<Self, VectorDbError> {
        let conn = Connection::open_in_memory()?;
        let db = Self { conn, dimensions };
        db.init()?;
        Ok(db)
    }

    fn init(&self) -> Result<(), VectorDbError> {
        self.conn
            .execute_batch(include_str!("schema_vectors.sql"))?;

        Ok(())
    }

    pub fn get_meta_value(&self, key: &str) -> Result<Option<String>, VectorDbError> {
        self.conn
            .query_row("SELECT value FROM meta WHERE key = ?1", params![key], |row| row.get(0))
            .optional()
            .map_err(Into::into)
    }

    pub fn set_meta_value(&self, key: &str, value: &str) -> Result<(), VectorDbError> {
        self.conn.execute(
            "INSERT OR REPLACE INTO meta (key, value) VALUES (?1, ?2)",
            params![key, value],
        )?;
        Ok(())
    }

    pub fn delete_meta_key(&self, key: &str) -> Result<(), VectorDbError> {
        self.conn
            .execute("DELETE FROM meta WHERE key = ?1", params![key])?;
        Ok(())
    }

    pub fn clear_meta(&self) -> Result<(), VectorDbError> {
        self.conn.execute("DELETE FROM meta", [])?;
        Ok(())
    }

    pub fn get_embedding_signature(&self) -> Result<Option<VectorDbEmbeddingSignature>, VectorDbError> {
        let provider = self.get_meta_value(META_KEY_EMBEDDING_PROVIDER)?;
        let model = self.get_meta_value(META_KEY_EMBEDDING_MODEL)?;
        let dims_str = self.get_meta_value(META_KEY_EMBEDDING_DIMENSIONS)?;
        let updated_at = self.get_meta_value(META_KEY_EMBEDDING_UPDATED_AT)?;

        match (provider, model, dims_str) {
            (Some(provider), Some(model), Some(dims_str)) => {
                let dimensions = dims_str.parse::<u32>().unwrap_or(0);
                if dimensions == 0 {
                    return Ok(None);
                }
                Ok(Some(VectorDbEmbeddingSignature {
                    provider,
                    model,
                    dimensions,
                    updated_at,
                }))
            }
            _ => Ok(None),
        }
    }

    pub fn set_embedding_signature(
        &self,
        provider: &str,
        model: &str,
        dimensions: u32,
    ) -> Result<(), VectorDbError> {
        self.set_meta_value(META_KEY_EMBEDDING_PROVIDER, provider)?;
        self.set_meta_value(META_KEY_EMBEDDING_MODEL, model)?;
        self.set_meta_value(META_KEY_EMBEDDING_DIMENSIONS, &dimensions.to_string())?;
        self.set_meta_value(
            META_KEY_EMBEDDING_UPDATED_AT,
            &chrono::Utc::now().to_rfc3339(),
        )?;
        Ok(())
    }

    pub fn clear_embedding_signature(&self) -> Result<(), VectorDbError> {
        self.delete_meta_key(META_KEY_EMBEDDING_PROVIDER)?;
        self.delete_meta_key(META_KEY_EMBEDDING_MODEL)?;
        self.delete_meta_key(META_KEY_EMBEDDING_DIMENSIONS)?;
        self.delete_meta_key(META_KEY_EMBEDDING_UPDATED_AT)?;
        Ok(())
    }

    /// Clear all stored chunks, embeddings and metadata.
    ///
    /// Note: We delete `embeddings` first because SQLite foreign keys are not
    /// guaranteed to be enforced unless explicitly enabled.
    pub fn clear_all(&self) -> Result<(), VectorDbError> {
        self.conn.execute("DELETE FROM embeddings", [])?;
        self.conn.execute("DELETE FROM chunks", [])?;
        self.conn.execute("DELETE FROM meta", [])?;
        Ok(())
    }

    /// Delete all embeddings but keep chunks (and their metadata) intact.
    pub fn clear_embeddings(&self) -> Result<(), VectorDbError> {
        self.conn.execute("DELETE FROM embeddings", [])?;
        Ok(())
    }

    pub fn insert_chunk(&self, chunk: &Chunk) -> Result<(), VectorDbError> {
        let metadata_json = serde_json::to_string(&chunk.metadata)?;

        self.conn.execute(
            r#"
            INSERT OR REPLACE INTO chunks 
            (id, chapter_id, chunk_index, chunk_type, content, char_start, char_end, metadata_json, created_at, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
            "#,
            params![
                chunk.id.0,
                chunk.chapter_id,
                chunk.chunk_index,
                chunk.chunk_type.as_str(),
                chunk.content,
                chunk.char_start,
                chunk.char_end,
                metadata_json,
                chunk.created_at.to_rfc3339(),
                chunk.updated_at.to_rfc3339(),
            ],
        )?;

        Ok(())
    }

    pub fn insert_chunks(&self, chunks: &[Chunk]) -> Result<(), VectorDbError> {
        let tx = self.conn.unchecked_transaction()?;

        for chunk in chunks {
            let metadata_json = serde_json::to_string(&chunk.metadata)?;

            tx.execute(
                r#"
                INSERT OR REPLACE INTO chunks 
                (id, chapter_id, chunk_index, chunk_type, content, char_start, char_end, metadata_json, created_at, updated_at)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
                "#,
                params![
                    chunk.id.0,
                    chunk.chapter_id,
                    chunk.chunk_index,
                    chunk.chunk_type.as_str(),
                    chunk.content,
                    chunk.char_start,
                    chunk.char_end,
                    metadata_json,
                    chunk.created_at.to_rfc3339(),
                    chunk.updated_at.to_rfc3339(),
                ],
            )?;
        }

        tx.commit()?;
        Ok(())
    }

    pub fn insert_vector(&self, entry: &VectorEntry) -> Result<(), VectorDbError> {
        let embedding_bytes = embedding_to_bytes(&entry.embedding);

        self.conn.execute(
            "INSERT OR REPLACE INTO embeddings (chunk_id, embedding) VALUES (?1, ?2)",
            params![entry.chunk_id.0, embedding_bytes],
        )?;

        Ok(())
    }

    pub fn insert_vectors(&self, entries: &[VectorEntry]) -> Result<(), VectorDbError> {
        let tx = self.conn.unchecked_transaction()?;

        for entry in entries {
            let embedding_bytes = embedding_to_bytes(&entry.embedding);

            tx.execute(
                "INSERT OR REPLACE INTO embeddings (chunk_id, embedding) VALUES (?1, ?2)",
                params![entry.chunk_id.0, embedding_bytes],
            )?;
        }

        tx.commit()?;
        Ok(())
    }

    pub fn search_similar(
        &self,
        query_embedding: &[f32],
        top_k: usize,
        exclude_chapter_id: Option<&str>,
        chunk_type_filter: Option<ChunkType>,
    ) -> Result<Vec<SimilarChunk>, VectorDbError> {
        if query_embedding.is_empty() {
            return Ok(Vec::new());
        }

        // Enforce dimension compatibility when possible.
        // If the vectors.db already has embeddings but no signature, we surface an explicit error
        // so callers can prompt a rebuild instead of silently returning garbage scores.
        let expected_dims = match self.get_embedding_signature()? {
            Some(sig) => Some(sig.dimensions),
            None => {
                if self.count_embeddings().unwrap_or(0) > 0 {
                    return Err(VectorDbError::MissingEmbeddingSignature);
                }
                None
            }
        };

        if let Some(expected) = expected_dims {
            let got = query_embedding.len() as u32;
            if got != expected {
                return Err(VectorDbError::EmbeddingDimensionsMismatch {
                    expected,
                    got,
                });
            }
        }

        let mut results = Vec::new();

        let mut stmt = self.conn.prepare(
            r#"
            SELECT c.id, c.chapter_id, c.chunk_index, c.chunk_type, c.content, 
                   c.char_start, c.char_end, c.metadata_json, c.created_at, c.updated_at,
                   e.embedding
            FROM chunks c
            JOIN embeddings e ON c.id = e.chunk_id
            "#,
        )?;

        let rows = stmt.query_map([], |row| {
            let id: String = row.get(0)?;
            let chapter_id: String = row.get(1)?;
            let chunk_index: u32 = row.get(2)?;
            let chunk_type_str: String = row.get(3)?;
            let content: String = row.get(4)?;
            let char_start: u32 = row.get(5)?;
            let char_end: u32 = row.get(6)?;
            let metadata_json: Option<String> = row.get(7)?;
            let created_at_str: String = row.get(8)?;
            let updated_at_str: String = row.get(9)?;
            let embedding_bytes: Vec<u8> = row.get(10)?;

            Ok((
                id,
                chapter_id,
                chunk_index,
                chunk_type_str,
                content,
                char_start,
                char_end,
                metadata_json,
                created_at_str,
                updated_at_str,
                embedding_bytes,
            ))
        })?;

        for row_result in rows {
            let (
                id,
                chapter_id,
                chunk_index,
                chunk_type_str,
                content,
                char_start,
                char_end,
                metadata_json,
                created_at_str,
                updated_at_str,
                embedding_bytes,
            ) = row_result?;

            if let Some(exclude) = exclude_chapter_id {
                if chapter_id == exclude {
                    continue;
                }
            }

            let chunk_type = ChunkType::from_str(&chunk_type_str).unwrap_or(ChunkType::Paragraph);

            if let Some(ref filter) = chunk_type_filter {
                if chunk_type.as_str() != filter.as_str() {
                    continue;
                }
            }

            let embedding = bytes_to_embedding(&embedding_bytes);
            let score = cosine_similarity(query_embedding, &embedding);

            let metadata: ChunkMetadata = metadata_json
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default();

            let created_at = chrono::DateTime::parse_from_rfc3339(&created_at_str)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .unwrap_or_else(|_| chrono::Utc::now());

            let updated_at = chrono::DateTime::parse_from_rfc3339(&updated_at_str)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .unwrap_or_else(|_| chrono::Utc::now());

            let chunk = Chunk {
                id: ChunkId::from_string(id),
                chapter_id,
                chunk_index,
                chunk_type,
                content,
                char_start,
                char_end,
                metadata,
                created_at,
                updated_at,
            };

            results.push(SimilarChunk { chunk, score });
        }

        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(top_k);

        Ok(results)
    }

    pub fn get_chunk(&self, chunk_id: &ChunkId) -> Result<Option<Chunk>, VectorDbError> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT id, chapter_id, chunk_index, chunk_type, content, 
                   char_start, char_end, metadata_json, created_at, updated_at
            FROM chunks WHERE id = ?1
            "#,
        )?;

        let chunk = stmt
            .query_row(params![chunk_id.0], |row| {
                let id: String = row.get(0)?;
                let chapter_id: String = row.get(1)?;
                let chunk_index: u32 = row.get(2)?;
                let chunk_type_str: String = row.get(3)?;
                let content: String = row.get(4)?;
                let char_start: u32 = row.get(5)?;
                let char_end: u32 = row.get(6)?;
                let metadata_json: Option<String> = row.get(7)?;
                let created_at_str: String = row.get(8)?;
                let updated_at_str: String = row.get(9)?;

                Ok((
                    id,
                    chapter_id,
                    chunk_index,
                    chunk_type_str,
                    content,
                    char_start,
                    char_end,
                    metadata_json,
                    created_at_str,
                    updated_at_str,
                ))
            })
            .optional()?;

        match chunk {
            Some((
                id,
                chapter_id,
                chunk_index,
                chunk_type_str,
                content,
                char_start,
                char_end,
                metadata_json,
                created_at_str,
                updated_at_str,
            )) => {
                let chunk_type =
                    ChunkType::from_str(&chunk_type_str).unwrap_or(ChunkType::Paragraph);

                let metadata: ChunkMetadata = metadata_json
                    .and_then(|s| serde_json::from_str(&s).ok())
                    .unwrap_or_default();

                let created_at = chrono::DateTime::parse_from_rfc3339(&created_at_str)
                    .map(|dt| dt.with_timezone(&chrono::Utc))
                    .unwrap_or_else(|_| chrono::Utc::now());

                let updated_at = chrono::DateTime::parse_from_rfc3339(&updated_at_str)
                    .map(|dt| dt.with_timezone(&chrono::Utc))
                    .unwrap_or_else(|_| chrono::Utc::now());

                Ok(Some(Chunk {
                    id: ChunkId::from_string(id),
                    chapter_id,
                    chunk_index,
                    chunk_type,
                    content,
                    char_start,
                    char_end,
                    metadata,
                    created_at,
                    updated_at,
                }))
            }
            None => Ok(None),
        }
    }

    pub fn update_chunk(&self, chunk: &Chunk) -> Result<(), VectorDbError> {
        let metadata_json = serde_json::to_string(&chunk.metadata)?;

        let rows_affected = self.conn.execute(
            r#"
            UPDATE chunks SET
                chapter_id = ?2,
                chunk_index = ?3,
                chunk_type = ?4,
                content = ?5,
                char_start = ?6,
                char_end = ?7,
                metadata_json = ?8,
                updated_at = ?9
            WHERE id = ?1
            "#,
            params![
                chunk.id.0,
                chunk.chapter_id,
                chunk.chunk_index,
                chunk.chunk_type.as_str(),
                chunk.content,
                chunk.char_start,
                chunk.char_end,
                metadata_json,
                chunk.updated_at.to_rfc3339(),
            ],
        )?;

        if rows_affected == 0 {
            return Err(VectorDbError::ChunkNotFound(chunk.id.0.clone()));
        }

        Ok(())
    }

    pub fn delete_chunk(&self, chunk_id: &ChunkId) -> Result<bool, VectorDbError> {
        self.conn.execute(
            "DELETE FROM embeddings WHERE chunk_id = ?1",
            params![chunk_id.0],
        )?;

        let rows_affected = self
            .conn
            .execute("DELETE FROM chunks WHERE id = ?1", params![chunk_id.0])?;

        Ok(rows_affected > 0)
    }

    pub fn delete_chunks_by_chapter(&self, chapter_id: &str) -> Result<usize, VectorDbError> {
        self.conn.execute(
            "DELETE FROM embeddings WHERE chunk_id IN (SELECT id FROM chunks WHERE chapter_id = ?1)",
            params![chapter_id],
        )?;

        let rows_affected = self.conn.execute(
            "DELETE FROM chunks WHERE chapter_id = ?1",
            params![chapter_id],
        )?;

        Ok(rows_affected)
    }

    pub fn get_chunks_by_chapter(&self, chapter_id: &str) -> Result<Vec<Chunk>, VectorDbError> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT id, chapter_id, chunk_index, chunk_type, content, 
                   char_start, char_end, metadata_json, created_at, updated_at
            FROM chunks WHERE chapter_id = ?1
            ORDER BY chunk_index
            "#,
        )?;

        let chunks = stmt
            .query_map(params![chapter_id], |row| {
                let id: String = row.get(0)?;
                let chapter_id: String = row.get(1)?;
                let chunk_index: u32 = row.get(2)?;
                let chunk_type_str: String = row.get(3)?;
                let content: String = row.get(4)?;
                let char_start: u32 = row.get(5)?;
                let char_end: u32 = row.get(6)?;
                let metadata_json: Option<String> = row.get(7)?;
                let created_at_str: String = row.get(8)?;
                let updated_at_str: String = row.get(9)?;

                Ok((
                    id,
                    chapter_id,
                    chunk_index,
                    chunk_type_str,
                    content,
                    char_start,
                    char_end,
                    metadata_json,
                    created_at_str,
                    updated_at_str,
                ))
            })?
            .filter_map(|r| r.ok())
            .map(
                |(
                    id,
                    chapter_id,
                    chunk_index,
                    chunk_type_str,
                    content,
                    char_start,
                    char_end,
                    metadata_json,
                    created_at_str,
                    updated_at_str,
                )| {
                    let chunk_type =
                        ChunkType::from_str(&chunk_type_str).unwrap_or(ChunkType::Paragraph);

                    let metadata: ChunkMetadata = metadata_json
                        .and_then(|s| serde_json::from_str(&s).ok())
                        .unwrap_or_default();

                    let created_at = chrono::DateTime::parse_from_rfc3339(&created_at_str)
                        .map(|dt| dt.with_timezone(&chrono::Utc))
                        .unwrap_or_else(|_| chrono::Utc::now());

                    let updated_at = chrono::DateTime::parse_from_rfc3339(&updated_at_str)
                        .map(|dt| dt.with_timezone(&chrono::Utc))
                        .unwrap_or_else(|_| chrono::Utc::now());

                    Chunk {
                        id: ChunkId::from_string(id),
                        chapter_id,
                        chunk_index,
                        chunk_type,
                        content,
                        char_start,
                        char_end,
                        metadata,
                        created_at,
                        updated_at,
                    }
                },
            )
            .collect();

        Ok(chunks)
    }

    /// Get all chunks from the database
    pub fn get_all_chunks(&self) -> Result<Vec<Chunk>, VectorDbError> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT id, chapter_id, chunk_index, chunk_type, content,
                   char_start, char_end, metadata_json, created_at, updated_at
            FROM chunks
            ORDER BY chapter_id, chunk_index
            "#,
        )?;

        let chunks = stmt
            .query_map([], |row| {
                let id: String = row.get(0)?;
                let chapter_id: String = row.get(1)?;
                let chunk_index: u32 = row.get(2)?;
                let chunk_type_str: String = row.get(3)?;
                let content: String = row.get(4)?;
                let char_start: u32 = row.get(5)?;
                let char_end: u32 = row.get(6)?;
                let metadata_json: Option<String> = row.get(7)?;
                let created_at_str: String = row.get(8)?;
                let updated_at_str: String = row.get(9)?;

                Ok((
                    id,
                    chapter_id,
                    chunk_index,
                    chunk_type_str,
                    content,
                    char_start,
                    char_end,
                    metadata_json,
                    created_at_str,
                    updated_at_str,
                ))
            })?
            .filter_map(|r| r.ok())
            .map(
                |(
                    id,
                    chapter_id,
                    chunk_index,
                    chunk_type_str,
                    content,
                    char_start,
                    char_end,
                    metadata_json,
                    created_at_str,
                    updated_at_str,
                )| {
                    let chunk_type =
                        ChunkType::from_str(&chunk_type_str).unwrap_or(ChunkType::Paragraph);

                    let metadata: ChunkMetadata = metadata_json
                        .and_then(|s| serde_json::from_str(&s).ok())
                        .unwrap_or_default();

                    let created_at = chrono::DateTime::parse_from_rfc3339(&created_at_str)
                        .map(|dt| dt.with_timezone(&chrono::Utc))
                        .unwrap_or_else(|_| chrono::Utc::now());

                    let updated_at = chrono::DateTime::parse_from_rfc3339(&updated_at_str)
                        .map(|dt| dt.with_timezone(&chrono::Utc))
                        .unwrap_or_else(|_| chrono::Utc::now());

                    Chunk {
                        id: ChunkId::from_string(id),
                        chapter_id,
                        chunk_index,
                        chunk_type,
                        content,
                        char_start,
                        char_end,
                        metadata,
                        created_at,
                        updated_at,
                    }
                },
            )
            .collect();

        Ok(chunks)
    }

    pub fn count_chunks(&self) -> Result<usize, VectorDbError> {
        let count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM chunks", [], |row| row.get(0))?;
        Ok(count as usize)
    }

    pub fn count_embeddings(&self) -> Result<usize, VectorDbError> {
        let count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM embeddings", [], |row| row.get(0))?;
        Ok(count as usize)
    }

    /// Count chunks for a specific chapter
    pub fn count_chunks_by_chapter(&self, chapter_id: &str) -> Result<usize, VectorDbError> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM chunks WHERE chapter_id = ?1",
            params![chapter_id],
            |row| row.get(0),
        )?;
        Ok(count as usize)
    }

    /// Update the entities_mentioned metadata for all chunks of a chapter
    /// This is called after analysis to link chunks to the entities found in that chapter
    pub fn update_chapter_chunks_entities(
        &self,
        chapter_id: &str,
        entity_ids: &[String],
    ) -> Result<usize, VectorDbError> {
        // Get all chunks for this chapter
        let chunks = self.get_chunks_by_chapter(chapter_id)?;

        if chunks.is_empty() {
            return Ok(0);
        }

        let tx = self.conn.unchecked_transaction()?;
        let mut updated = 0;

        for chunk in &chunks {
            // Update metadata with entity IDs
            let mut metadata = chunk.metadata.clone();
            metadata.entities_mentioned = entity_ids.to_vec();
            let metadata_json = serde_json::to_string(&metadata)?;

            tx.execute(
                "UPDATE chunks SET metadata_json = ?1, updated_at = ?2 WHERE id = ?3",
                params![
                    metadata_json,
                    chrono::Utc::now().to_rfc3339(),
                    chunk.id.0
                ],
            )?;
            updated += 1;
        }

        tx.commit()?;

        tracing::debug!(
            "Updated {} chunks for chapter {} with {} entity IDs",
            updated,
            chapter_id,
            entity_ids.len()
        );

        Ok(updated)
    }

    /// Get embeddings for all chunks of a specific chapter
    /// Returns a list of (chunk_id, embedding) pairs
    pub fn get_embeddings_by_chapter(&self, chapter_id: &str) -> Result<Vec<Vec<f32>>, VectorDbError> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT e.embedding
            FROM chunks c
            JOIN embeddings e ON c.id = e.chunk_id
            WHERE c.chapter_id = ?1
            ORDER BY c.chunk_index
            "#,
        )?;

        let embeddings = stmt
            .query_map(params![chapter_id], |row| {
                let embedding_bytes: Vec<u8> = row.get(0)?;
                Ok(bytes_to_embedding(&embedding_bytes))
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(embeddings)
    }

    /// Delete all chunks with a specific entity_id in their metadata
    pub fn delete_chunks_by_entity(&self, entity_id: &str) -> Result<usize, VectorDbError> {
        // First delete from embeddings
        self.conn.execute(
            r#"
            DELETE FROM embeddings WHERE chunk_id IN (
                SELECT id FROM chunks
                WHERE chunk_type = 'entity'
                AND json_extract(metadata_json, '$.entities_mentioned[0]') = ?1
            )
            "#,
            params![entity_id],
        )?;

        // Then delete from chunks
        let rows_affected = self.conn.execute(
            r#"
            DELETE FROM chunks
            WHERE chunk_type = 'entity'
            AND json_extract(metadata_json, '$.entities_mentioned[0]') = ?1
            "#,
            params![entity_id],
        )?;

        Ok(rows_affected)
    }

    /// Get chunks by entity ID
    pub fn get_chunks_by_entity(&self, entity_id: &str) -> Result<Vec<Chunk>, VectorDbError> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT id, chapter_id, chunk_index, chunk_type, content,
                   char_start, char_end, metadata_json, created_at, updated_at
            FROM chunks
            WHERE chunk_type = 'entity'
            AND json_extract(metadata_json, '$.entities_mentioned[0]') = ?1
            "#,
        )?;

        let chunks = stmt
            .query_map(params![entity_id], |row| {
                let id: String = row.get(0)?;
                let chapter_id: String = row.get(1)?;
                let chunk_index: u32 = row.get(2)?;
                let chunk_type_str: String = row.get(3)?;
                let content: String = row.get(4)?;
                let char_start: u32 = row.get(5)?;
                let char_end: u32 = row.get(6)?;
                let metadata_json: Option<String> = row.get(7)?;
                let created_at_str: String = row.get(8)?;
                let updated_at_str: String = row.get(9)?;

                Ok((
                    id,
                    chapter_id,
                    chunk_index,
                    chunk_type_str,
                    content,
                    char_start,
                    char_end,
                    metadata_json,
                    created_at_str,
                    updated_at_str,
                ))
            })?
            .filter_map(|r| r.ok())
            .map(
                |(
                    id,
                    chapter_id,
                    chunk_index,
                    chunk_type_str,
                    content,
                    char_start,
                    char_end,
                    metadata_json,
                    created_at_str,
                    updated_at_str,
                )| {
                    let chunk_type =
                        ChunkType::from_str(&chunk_type_str).unwrap_or(ChunkType::Paragraph);

                    let metadata: ChunkMetadata = metadata_json
                        .and_then(|s| serde_json::from_str(&s).ok())
                        .unwrap_or_default();

                    let created_at = chrono::DateTime::parse_from_rfc3339(&created_at_str)
                        .map(|dt| dt.with_timezone(&chrono::Utc))
                        .unwrap_or_else(|_| chrono::Utc::now());

                    let updated_at = chrono::DateTime::parse_from_rfc3339(&updated_at_str)
                        .map(|dt| dt.with_timezone(&chrono::Utc))
                        .unwrap_or_else(|_| chrono::Utc::now());

                    Chunk {
                        id: ChunkId::from_string(id),
                        chapter_id,
                        chunk_index,
                        chunk_type,
                        content,
                        char_start,
                        char_end,
                        metadata,
                        created_at,
                        updated_at,
                    }
                },
            )
            .collect();

        Ok(chunks)
    }

    /// Count entity chunks
    pub fn count_entity_chunks(&self) -> Result<usize, VectorDbError> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM chunks WHERE chunk_type = 'entity'",
            [],
            |row| row.get(0),
        )?;
        Ok(count as usize)
    }

    /// List all entity IDs that have embeddings
    pub fn list_entity_ids(&self) -> Result<Vec<String>, VectorDbError> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT DISTINCT json_extract(metadata_json, '$.entities_mentioned[0]') as entity_id
            FROM chunks
            WHERE chunk_type = 'entity'
            AND entity_id IS NOT NULL
            "#,
        )?;

        let ids = stmt
            .query_map([], |row| row.get::<_, String>(0))?
            .filter_map(|r| r.ok())
            .collect();

        Ok(ids)
    }

    /// Keyword search using LIKE for full-text matching (P4.5-001)
    ///
    /// Returns chunks that contain the keyword, ordered by relevance.
    /// Uses simple keyword matching - for Chinese text this works reasonably well.
    pub fn search_by_keyword(
        &self,
        keyword: &str,
        top_k: usize,
        chunk_type_filter: Option<ChunkType>,
        exclude_chapter_id: Option<&str>,
    ) -> Result<Vec<KeywordSearchResult>, VectorDbError> {
        let search_pattern = format!("%{}%", keyword);

        let mut sql = String::from(
            r#"
            SELECT id, chapter_id, chunk_index, chunk_type, content,
                   char_start, char_end, metadata_json, created_at, updated_at
            FROM chunks
            WHERE content LIKE ?1
            "#,
        );

        if chunk_type_filter.is_some() {
            sql.push_str(" AND chunk_type = ?2");
        }

        if exclude_chapter_id.is_some() {
            if chunk_type_filter.is_some() {
                sql.push_str(" AND chapter_id != ?3");
            } else {
                sql.push_str(" AND chapter_id != ?2");
            }
        }

        sql.push_str(" ORDER BY length(content) ASC"); // Prefer shorter, more focused matches
        sql.push_str(&format!(" LIMIT {}", top_k * 2)); // Get more to allow ranking

        let mut stmt = self.conn.prepare(&sql)?;

        let rows: Vec<_> = match (chunk_type_filter.as_ref(), exclude_chapter_id) {
            (Some(ct), Some(exc)) => stmt
                .query_map(params![search_pattern, ct.as_str(), exc], Self::map_chunk_row)?
                .filter_map(|r| r.ok())
                .collect(),
            (Some(ct), None) => stmt
                .query_map(params![search_pattern, ct.as_str()], Self::map_chunk_row)?
                .filter_map(|r| r.ok())
                .collect(),
            (None, Some(exc)) => stmt
                .query_map(params![search_pattern, exc], Self::map_chunk_row)?
                .filter_map(|r| r.ok())
                .collect(),
            (None, None) => stmt
                .query_map(params![search_pattern], Self::map_chunk_row)?
                .filter_map(|r| r.ok())
                .collect(),
        };

        // Calculate relevance score based on keyword frequency and position
        let keyword_lower = keyword.to_lowercase();
        let mut results: Vec<KeywordSearchResult> = rows
            .into_iter()
            .map(|chunk| {
                let score = calculate_keyword_score(&chunk.content, &keyword_lower);
                KeywordSearchResult { chunk, score }
            })
            .collect();

        // Sort by score descending
        results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        results.truncate(top_k);
        Ok(results)
    }

    /// Helper function to map a row to chunk tuple
    fn map_chunk_row(
        row: &rusqlite::Row<'_>,
    ) -> rusqlite::Result<Chunk> {
        let id: String = row.get(0)?;
        let chapter_id: String = row.get(1)?;
        let chunk_index: u32 = row.get(2)?;
        let chunk_type_str: String = row.get(3)?;
        let content: String = row.get(4)?;
        let char_start: u32 = row.get(5)?;
        let char_end: u32 = row.get(6)?;
        let metadata_json: Option<String> = row.get(7)?;
        let created_at_str: String = row.get(8)?;
        let updated_at_str: String = row.get(9)?;

        let chunk_type = ChunkType::from_str(&chunk_type_str).unwrap_or(ChunkType::Paragraph);

        let metadata: ChunkMetadata = metadata_json
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();

        let created_at = chrono::DateTime::parse_from_rfc3339(&created_at_str)
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .unwrap_or_else(|_| chrono::Utc::now());

        let updated_at = chrono::DateTime::parse_from_rfc3339(&updated_at_str)
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .unwrap_or_else(|_| chrono::Utc::now());

        Ok(Chunk {
            id: ChunkId::from_string(id),
            chapter_id,
            chunk_index,
            chunk_type,
            content,
            char_start,
            char_end,
            metadata,
            created_at,
            updated_at,
        })
    }
}

/// Result of keyword search
#[derive(Debug, Clone)]
pub struct KeywordSearchResult {
    pub chunk: Chunk,
    pub score: f32,
}

/// Calculate keyword relevance score
fn calculate_keyword_score(content: &str, keyword: &str) -> f32 {
    let content_lower = content.to_lowercase();

    // Count occurrences
    let count = content_lower.matches(keyword).count() as f32;

    // Bonus for keyword appearing early in content
    let position_bonus = if let Some(pos) = content_lower.find(keyword) {
        1.0 - (pos as f32 / content.len() as f32).min(1.0)
    } else {
        0.0
    };

    // Bonus for higher keyword density
    let density = count * keyword.len() as f32 / content.len() as f32;

    // Combined score
    count * 0.5 + position_bonus * 0.3 + density * 100.0 * 0.2
}

fn embedding_to_bytes(embedding: &[f32]) -> Vec<u8> {
    embedding
        .iter()
        .flat_map(|f| f.to_le_bytes())
        .collect()
}

fn bytes_to_embedding(bytes: &[u8]) -> Vec<f32> {
    bytes
        .chunks_exact(4)
        .map(|chunk| {
            let arr: [u8; 4] = chunk.try_into().unwrap();
            f32::from_le_bytes(arr)
        })
        .collect()
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }

    let mut dot_product = 0.0f32;
    let mut norm_a = 0.0f32;
    let mut norm_b = 0.0f32;

    for (ai, bi) in a.iter().zip(b.iter()) {
        dot_product += ai * bi;
        norm_a += ai * ai;
        norm_b += bi * bi;
    }

    let denominator = (norm_a.sqrt() * norm_b.sqrt()).max(1e-8);
    dot_product / denominator
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_embedding_serialization() {
        let embedding = vec![1.0f32, 2.0, 3.0, 4.0];
        let bytes = embedding_to_bytes(&embedding);
        let restored = bytes_to_embedding(&bytes);
        assert_eq!(embedding, restored);
    }

    #[test]
    fn test_cosine_similarity() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        assert!((cosine_similarity(&a, &b) - 1.0).abs() < 0.001);

        let c = vec![0.0, 1.0, 0.0];
        assert!((cosine_similarity(&a, &c)).abs() < 0.001);
    }

    #[test]
    fn test_vector_db_basic() {
        let db = VectorDb::open_in_memory(crate::core::embedding::DEFAULT_EMBEDDING_DIMENSIONS).unwrap();

        let chunk = Chunk::new_paragraph(
            "chapter-1".to_string(),
            0,
            "Test content".to_string(),
            0,
            12,
        );

        db.insert_chunk(&chunk).unwrap();

        let retrieved = db.get_chunk(&chunk.id).unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().content, "Test content");

        let count = db.count_chunks().unwrap();
        assert_eq!(count, 1);
    }
}
