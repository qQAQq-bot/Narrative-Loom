//! Vector Search Module (P2.5-005~007, P4.5-001~002)
//!
//! Provides advanced vector search capabilities for RAG retrieval:
//! - search_by_text: Search using text query (generates embedding internally)
//! - search_by_entities: Search for entity-related passages
//! - hybrid_search: Combines vector and keyword search with RRF fusion
//! - search_by_keyword: Pure keyword-based search

use crate::core::embedding::{
    ChunkType, SimilarChunk, DEFAULT_EMBEDDING_DIMENSIONS, DEFAULT_EMBEDDING_DIMENSIONS_USIZE,
};
use crate::core::ids::BookId;
use crate::storage::book_db::BookDb;
use crate::storage::library::Library;
use crate::storage::paths;
use crate::storage::vectors::VectorDb;
use crate::retrieval::recall_fusion::{RecallMode, RecallWeights};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum VectorSearchError {
    #[error("Vector DB error: {0}")]
    VectorDbError(#[from] crate::storage::vectors::VectorDbError),

    #[error("Book DB error: {0}")]
    BookDbError(#[from] crate::storage::book_db::BookDbError),

    #[error("Library error: {0}")]
    LibraryError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Embedding generation failed: {0}")]
    EmbeddingError(String),
}

/// Search result with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub chunk_id: String,
    pub content: String,
    pub chapter_id: String,
    pub chunk_type: String,
    pub score: f32,
    pub metadata: SearchResultMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResultMetadata {
    pub char_start: u32,
    pub char_end: u32,
    pub entities_mentioned: Vec<String>,
}

/// Entity search result - passages mentioning a specific entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntitySearchResult {
    pub entity_id: String,
    pub entity_name: String,
    pub entity_type: String,
    pub mentions: Vec<SearchResult>,
}

/// Vector searcher for a specific book
pub struct VectorSearcher {
    #[allow(dead_code)]
    book_id: BookId,
    vector_db: VectorDb,
    book_db: BookDb,
}

impl VectorSearcher {
    /// Open a vector searcher for a book
    pub fn open(book_id: &str) -> Result<Self, VectorSearchError> {
        let library = Library::open().map_err(|e| VectorSearchError::LibraryError(e.to_string()))?;
        let bid = BookId::from_string(book_id.to_string());
        let book_dir = library.book_dir(&bid);

        let vectors_path = get_vectors_db_path(book_id)?;
        let book_db_path = book_dir.join("book.db");

        if !vectors_path.exists() {
            return Err(VectorSearchError::LibraryError(
                "Vector database not found".to_string(),
            ));
        }

        if !book_db_path.exists() {
            return Err(VectorSearchError::LibraryError(
                "Book database not found".to_string(),
            ));
        }

        let vector_db = VectorDb::open(&vectors_path, DEFAULT_EMBEDDING_DIMENSIONS)?;
        let book_db = BookDb::open(&book_db_path, bid.clone())?;

        Ok(Self {
            book_id: bid,
            vector_db,
            book_db,
        })
    }

    /// Search by text query (P2.5-006)
    ///
    /// Converts text to embedding and performs vector similarity search.
    /// Currently uses dummy embedding - should be replaced with real embedding generation.
    pub fn search_by_text(
        &self,
        query: &str,
        top_k: usize,
        chunk_type_filter: Option<ChunkType>,
        exclude_chapter_id: Option<&str>,
    ) -> Result<Vec<SearchResult>, VectorSearchError> {
        // Generate query embedding (dummy for now)
        let query_embedding = generate_query_embedding(query);

        let similar_chunks = self.vector_db.search_similar(
            &query_embedding,
            top_k,
            exclude_chapter_id,
            chunk_type_filter,
        )?;

        Ok(similar_chunks
            .into_iter()
            .map(|sc| SearchResult {
                chunk_id: sc.chunk.id.to_string(),
                content: sc.chunk.content,
                chapter_id: sc.chunk.chapter_id,
                chunk_type: sc.chunk.chunk_type.as_str().to_string(),
                score: sc.score,
                metadata: SearchResultMetadata {
                    char_start: sc.chunk.char_start,
                    char_end: sc.chunk.char_end,
                    entities_mentioned: sc.chunk.metadata.entities_mentioned,
                },
            })
            .collect())
    }

    /// Search for passages related to specific entities (P2.5-007)
    ///
    /// This searches for:
    /// 1. Entity chunks (descriptions stored when entity was created/updated)
    /// 2. Paragraph chunks that mention the entity
    pub fn search_by_entities(
        &self,
        entity_ids: &[String],
        top_k_per_entity: usize,
    ) -> Result<Vec<EntitySearchResult>, VectorSearchError> {
        let mut results = Vec::new();

        for entity_id in entity_ids {
            // Try to find entity info from book_db
            let (entity_name, entity_type) = self.get_entity_info(entity_id)?;

            // Search for passages mentioning this entity
            let mentions = self.search_entity_mentions(entity_id, &entity_name, top_k_per_entity)?;

            results.push(EntitySearchResult {
                entity_id: entity_id.clone(),
                entity_name,
                entity_type,
                mentions,
            });
        }

        Ok(results)
    }

    /// Get entity info (name and type) from book_db
    fn get_entity_info(&self, entity_id: &str) -> Result<(String, String), VectorSearchError> {
        let eid = crate::core::ids::EntityId::from_string(entity_id.to_string());

        // Try character first
        if let Ok(Some(character)) = self.book_db.get_character(&eid) {
            return Ok((character.name, "character".to_string()));
        }

        // Try setting
        if let Ok(Some(setting)) = self.book_db.get_setting(&eid) {
            return Ok((setting.name, "setting".to_string()));
        }

        // Try event
        if let Ok(Some(event)) = self.book_db.get_event(&eid) {
            return Ok((event.title, "event".to_string()));
        }

        // Entity not found in book_db, return unknown
        Ok(("Unknown".to_string(), "unknown".to_string()))
    }

    /// Search for mentions of an entity in the corpus
    fn search_entity_mentions(
        &self,
        entity_id: &str,
        entity_name: &str,
        top_k: usize,
    ) -> Result<Vec<SearchResult>, VectorSearchError> {
        // Strategy 1: Search using entity name as query
        let query_embedding = generate_query_embedding(entity_name);

        let similar_chunks = self.vector_db.search_similar(
            &query_embedding,
            top_k * 2, // Get more results to filter
            None,
            Some(ChunkType::Paragraph), // Only search paragraphs, not entity chunks
        )?;

        // Filter to chunks that actually mention the entity (by name or ID in metadata)
        let mentions: Vec<SearchResult> = similar_chunks
            .into_iter()
            .filter(|sc| {
                // Check if content contains entity name
                let content_mentions = sc.chunk.content.contains(entity_name);

                // Check if metadata contains entity ID
                let metadata_mentions = sc
                    .chunk
                    .metadata
                    .entities_mentioned
                    .iter()
                    .any(|e| e == entity_id);

                content_mentions || metadata_mentions
            })
            .take(top_k)
            .map(|sc| SearchResult {
                chunk_id: sc.chunk.id.to_string(),
                content: sc.chunk.content,
                chapter_id: sc.chunk.chapter_id,
                chunk_type: sc.chunk.chunk_type.as_str().to_string(),
                score: sc.score,
                metadata: SearchResultMetadata {
                    char_start: sc.chunk.char_start,
                    char_end: sc.chunk.char_end,
                    entities_mentioned: sc.chunk.metadata.entities_mentioned,
                },
            })
            .collect();

        Ok(mentions)
    }

    /// Hybrid search combining vector similarity and keyword matching (P4.5-001)
    ///
    /// Uses Reciprocal Rank Fusion (RRF) to combine results from:
    /// 1. Vector similarity search (semantic matching)
    /// 2. Keyword search (exact matching)
    ///
    /// RRF formula: score = sum(1 / (k + rank_i)) for each result list
    /// where k is a constant (default 60) that controls the impact of lower-ranked results
    pub fn hybrid_search(
        &self,
        query: &str,
        top_k: usize,
        chunk_type_filter: Option<ChunkType>,
        exclude_chapter_id: Option<&str>,
        rrf_k: Option<u32>,
    ) -> Result<Vec<HybridSearchResult>, VectorSearchError> {
        let k = rrf_k.unwrap_or(60) as f32;

        // 1. Get vector search results
        let query_embedding = generate_query_embedding(query);
        let vector_results = self.vector_db.search_similar(
            &query_embedding,
            top_k * 2, // Get more results for fusion
            exclude_chapter_id,
            chunk_type_filter.clone(),
        )?;

        // 2. Get keyword search results
        let keyword_results = self.vector_db.search_by_keyword(
            query,
            top_k * 2,
            chunk_type_filter,
            exclude_chapter_id,
        )?;

        // 3. Apply RRF fusion (P4.5-002)
        let fused_results = rrf_fusion(vector_results, keyword_results, k, top_k);

        Ok(fused_results)
    }

    /// Pure keyword search (P4.5-001)
    pub fn search_by_keyword(
        &self,
        query: &str,
        top_k: usize,
        chunk_type_filter: Option<ChunkType>,
        exclude_chapter_id: Option<&str>,
    ) -> Result<Vec<SearchResult>, VectorSearchError> {
        let results = self.vector_db.search_by_keyword(
            query,
            top_k,
            chunk_type_filter,
            exclude_chapter_id,
        )?;

        Ok(results
            .into_iter()
            .map(|kr| SearchResult {
                chunk_id: kr.chunk.id.to_string(),
                content: kr.chunk.content,
                chapter_id: kr.chunk.chapter_id,
                chunk_type: kr.chunk.chunk_type.as_str().to_string(),
                score: kr.score,
                metadata: SearchResultMetadata {
                    char_start: kr.chunk.char_start,
                    char_end: kr.chunk.char_end,
                    entities_mentioned: kr.chunk.metadata.entities_mentioned,
                },
            })
            .collect())
    }

    /// Search for entity history across chapters
    ///
    /// Returns passages where the entity was mentioned, ordered by chapter sequence
    pub fn search_entity_history(
        &self,
        entity_id: &str,
        max_passages: usize,
    ) -> Result<Vec<SearchResult>, VectorSearchError> {
        let (entity_name, _entity_type) = self.get_entity_info(entity_id)?;

        // Get all paragraphs that mention this entity
        let mut mentions = self.search_entity_mentions(entity_id, &entity_name, max_passages * 2)?;

        // Sort by chapter_id to maintain chronological order
        mentions.sort_by(|a, b| a.chapter_id.cmp(&b.chapter_id));

        // Take top N
        mentions.truncate(max_passages);

        Ok(mentions)
    }

    /// Get all known entities from the book
    pub fn get_all_entities(&self) -> Result<AllEntities, VectorSearchError> {
        let characters = self.book_db.list_characters()?;
        let settings = self.book_db.list_settings()?;
        let events = self.book_db.list_events()?;

        Ok(AllEntities {
            characters: characters
                .into_iter()
                .map(|c| EntityInfo {
                    id: c.id.to_string(),
                    name: c.name,
                    aliases: c.aliases,
                    entity_type: "character".to_string(),
                })
                .collect(),
            settings: settings
                .into_iter()
                .map(|s| EntityInfo {
                    id: s.id.to_string(),
                    name: s.name,
                    aliases: vec![],
                    entity_type: "setting".to_string(),
                })
                .collect(),
            events: events
                .into_iter()
                .map(|e| EntityInfo {
                    id: e.id.to_string(),
                    name: e.title,
                    aliases: vec![],
                    entity_type: "event".to_string(),
                })
                .collect(),
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityInfo {
    pub id: String,
    pub name: String,
    pub aliases: Vec<String>,
    pub entity_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllEntities {
    pub characters: Vec<EntityInfo>,
    pub settings: Vec<EntityInfo>,
    pub events: Vec<EntityInfo>,
}

/// Extract entity mentions from text (P2.5-003)
///
/// Identifies which known entities are mentioned in the given text.
/// This is useful for:
/// - Tagging chunks with entity references
/// - Building entity co-occurrence graphs
/// - Filtering search results
pub fn extract_entity_mentions(
    text: &str,
    known_entities: &AllEntities,
) -> Vec<EntityMention> {
    let mut mentions = Vec::new();
    let text_lower = text.to_lowercase();
    let normalized_text = normalize_for_entity_match(&text_lower);

    // Check characters
    for entity in &known_entities.characters {
        if text_contains_entity_or_alias(&text_lower, &normalized_text, entity) {
            mentions.push(EntityMention {
                entity_id: entity.id.clone(),
                entity_name: entity.name.clone(),
                entity_type: "character".to_string(),
                positions: find_entity_positions_for_entity(text, entity),
            });
        }
    }

    // Check settings
    for entity in &known_entities.settings {
        if text_contains_entity_or_alias(&text_lower, &normalized_text, entity) {
            mentions.push(EntityMention {
                entity_id: entity.id.clone(),
                entity_name: entity.name.clone(),
                entity_type: "setting".to_string(),
                positions: find_entity_positions_for_entity(text, entity),
            });
        }
    }

    // Check events
    for entity in &known_entities.events {
        if text_contains_entity_or_alias(&text_lower, &normalized_text, entity) {
            mentions.push(EntityMention {
                entity_id: entity.id.clone(),
                entity_name: entity.name.clone(),
                entity_type: "event".to_string(),
                positions: find_entity_positions_for_entity(text, entity),
            });
        }
    }

    mentions
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityMention {
    pub entity_id: String,
    pub entity_name: String,
    pub entity_type: String,
    pub positions: Vec<(usize, usize)>, // (start, end) positions in text
}

fn normalize_for_entity_match(input: &str) -> String {
    input
        .chars()
        .filter(|c| !c.is_whitespace())
        .filter(|c| !matches!(c, '·' | '•' | '.' | ',' | '，' | '。' | '、' | '\'' | '"' | '“' | '”' | '(' | ')' | '（' | '）' | '-' | '_' | ':'))
        .collect::<String>()
        .to_lowercase()
}

/// Check if text contains entity name or aliases (case-insensitive + normalized)
fn text_contains_entity_or_alias(
    text_lower: &str,
    normalized_text: &str,
    entity: &EntityInfo,
) -> bool {
    let mut candidates: Vec<&str> = vec![entity.name.as_str()];
    candidates.extend(entity.aliases.iter().map(String::as_str));

    for candidate in candidates {
        if candidate.trim().is_empty() {
            continue;
        }

        let candidate_lower = candidate.to_lowercase();
        if text_lower.contains(&candidate_lower) {
            return true;
        }

        let normalized_candidate = normalize_for_entity_match(candidate);
        if !normalized_candidate.is_empty() && normalized_text.contains(&normalized_candidate) {
            return true;
        }
    }

    false
}

fn find_entity_positions_for_entity(text: &str, entity: &EntityInfo) -> Vec<(usize, usize)> {
    let mut candidates: Vec<&str> = vec![entity.name.as_str()];
    candidates.extend(entity.aliases.iter().map(String::as_str));

    let mut positions = Vec::new();
    for candidate in candidates {
        if candidate.trim().is_empty() {
            continue;
        }
        positions.extend(find_entity_positions(text, candidate));
    }

    let mut unique = HashSet::new();
    positions.retain(|pos| unique.insert(*pos));
    positions.sort_unstable_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));
    positions
}

/// Find all positions where entity name appears in text
fn find_entity_positions(text: &str, entity_name: &str) -> Vec<(usize, usize)> {
    let mut positions = Vec::new();
    let text_lower = text.to_lowercase();
    let name_lower = entity_name.to_lowercase();

    let mut start = 0;
    while let Some(pos) = text_lower[start..].find(&name_lower) {
        let abs_pos = start + pos;
        positions.push((abs_pos, abs_pos + entity_name.len()));
        start = abs_pos + 1;
    }

    positions
}

/// Get vectors.db path for a book
fn get_vectors_db_path(book_id: &str) -> Result<PathBuf, VectorSearchError> {
    paths::get_vectors_db_path(book_id)
        .map_err(|e| VectorSearchError::LibraryError(e.to_string()))
}

/// Generate query embedding using Python sidecar
///
/// This calls the Python sidecar to generate a real embedding using BAAI/bge-m3.
/// Falls back to dummy embedding if sidecar is not available.
fn generate_query_embedding(query: &str) -> Vec<f32> {
    let (embedding, mode) = generate_query_embedding_with_mode(query);
    let weights = RecallWeights::for_mode(mode);
    tracing::debug!(
        "Query embedding mode={:?}, vector_weight={:.2}, keyword_weight={:.2}",
        mode,
        weights.vector,
        weights.keyword
    );
    embedding
}

fn generate_query_embedding_with_mode(query: &str) -> (Vec<f32>, RecallMode) {
    // Try to use real embedding from sidecar
    match crate::sidecar::generate_embedding_sync(query) {
        Ok(embedding) => {
            tracing::debug!("Generated real embedding for query (dim={})", embedding.len());
            (embedding, RecallMode::Normal)
        }
        Err(e) => {
            tracing::warn!("Failed to generate real embedding, using fallback: {}", e);
            // Fallback to dummy embedding
            (
                generate_fallback_embedding(query),
                RecallMode::FallbackEmbedding,
            )
        }
    }
}

/// Fallback dummy embedding when sidecar is not available
fn generate_fallback_embedding(query: &str) -> Vec<f32> {
    let seed = query.len();
    let mut embedding = Vec::with_capacity(DEFAULT_EMBEDDING_DIMENSIONS_USIZE);

    for i in 0..DEFAULT_EMBEDDING_DIMENSIONS_USIZE {
        let val = ((seed + i) as f32 * 0.001).sin();
        embedding.push(val);
    }

    // Normalize
    let norm: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 0.0 {
        for x in &mut embedding {
            *x /= norm;
        }
    }

    embedding
}

/// Hybrid search result with source information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HybridSearchResult {
    pub chunk_id: String,
    pub content: String,
    pub chapter_id: String,
    pub chunk_type: String,
    pub rrf_score: f32,
    pub vector_rank: Option<usize>,
    pub keyword_rank: Option<usize>,
    pub metadata: SearchResultMetadata,
}

/// Reciprocal Rank Fusion (P4.5-002)
///
/// Combines results from vector and keyword search using RRF formula:
/// RRF(d) = sum(1 / (k + r(d))) for each ranking list
///
/// This gives higher weight to documents that appear in both lists
/// and near the top of each list.
fn rrf_fusion(
    vector_results: Vec<SimilarChunk>,
    keyword_results: Vec<crate::storage::vectors::KeywordSearchResult>,
    k: f32,
    top_k: usize,
) -> Vec<HybridSearchResult> {
    let mut scores: HashMap<String, (f32, Option<usize>, Option<usize>, SimilarChunk)> =
        HashMap::new();

    // Add vector search results
    for (rank, result) in vector_results.into_iter().enumerate() {
        let chunk_id = result.chunk.id.to_string();
        let rrf_score = 1.0 / (k + rank as f32 + 1.0);
        scores.insert(chunk_id, (rrf_score, Some(rank + 1), None, result));
    }

    // Add keyword search results
    for (rank, result) in keyword_results.into_iter().enumerate() {
        let chunk_id = result.chunk.id.to_string();
        let rrf_score = 1.0 / (k + rank as f32 + 1.0);

        if let Some(existing) = scores.get_mut(&chunk_id) {
            // Document appears in both lists - add scores
            existing.0 += rrf_score;
            existing.2 = Some(rank + 1);
        } else {
            // Create a SimilarChunk wrapper for keyword result
            let similar_chunk = SimilarChunk {
                chunk: result.chunk,
                score: result.score,
            };
            scores.insert(chunk_id, (rrf_score, None, Some(rank + 1), similar_chunk));
        }
    }

    // Convert to results and sort by RRF score
    let mut results: Vec<HybridSearchResult> = scores
        .into_iter()
        .map(|(chunk_id, (rrf_score, vector_rank, keyword_rank, sc))| HybridSearchResult {
            chunk_id,
            content: sc.chunk.content,
            chapter_id: sc.chunk.chapter_id,
            chunk_type: sc.chunk.chunk_type.as_str().to_string(),
            rrf_score,
            vector_rank,
            keyword_rank,
            metadata: SearchResultMetadata {
                char_start: sc.chunk.char_start,
                char_end: sc.chunk.char_end,
                entities_mentioned: sc.chunk.metadata.entities_mentioned,
            },
        })
        .collect();

    results.sort_by(|a, b| {
        b.rrf_score
            .partial_cmp(&a.rrf_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    results.truncate(top_k);
    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_entity_mentions() {
        let entities = AllEntities {
            characters: vec![
                EntityInfo {
                    id: "c1".to_string(),
                    name: "张三".to_string(),
                    aliases: vec![],
                    entity_type: "character".to_string(),
                },
                EntityInfo {
                    id: "c2".to_string(),
                    name: "李四".to_string(),
                    aliases: vec![],
                    entity_type: "character".to_string(),
                },
            ],
            settings: vec![EntityInfo {
                id: "s1".to_string(),
                name: "青云山".to_string(),
                aliases: vec![],
                entity_type: "setting".to_string(),
            }],
            events: vec![],
        };

        let text = "张三和李四一起去了青云山修炼。张三的剑法很厉害。";
        let mentions = extract_entity_mentions(text, &entities);

        assert_eq!(mentions.len(), 3);

        // Check 张三 appears twice
        let zhang_san = mentions.iter().find(|m| m.entity_name == "张三").unwrap();
        assert_eq!(zhang_san.positions.len(), 2);
    }

    #[test]
    fn test_extract_entity_mentions_supports_alias() {
        let entities_with_alias = AllEntities {
            characters: vec![EntityInfo {
                id: "char_1".to_string(),
                name: "费舍尔·贝纳维德斯".to_string(),
                aliases: vec!["费舍尔".to_string()],
                entity_type: "character".to_string(),
            }],
            settings: vec![],
            events: vec![],
        };

        let text = "费舍尔在雨夜里推开了旧教堂的门。";
        let mentions = extract_entity_mentions(text, &entities_with_alias);
        assert!(mentions.iter().any(|m| m.entity_id == "char_1"));
    }

    #[test]
    fn test_find_entity_positions() {
        let positions = find_entity_positions("Hello World, Hello Again", "Hello");
        assert_eq!(positions.len(), 2);
        assert_eq!(positions[0], (0, 5));
        assert_eq!(positions[1], (13, 18));
    }
}
