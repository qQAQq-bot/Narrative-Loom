// Search commands for full-text and entity search

use crate::core::ids::BookId;
use crate::storage::book_db::{BookDb, Character, Event, Setting};
use crate::storage::library::Library;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub result_type: String, // "chapter", "character", "setting", "event"
    pub id: String,
    pub title: String,
    pub content: String,
    pub chapter_id: Option<String>,
    pub chapter_title: Option<String>,
    pub highlights: Vec<HighlightRange>,
    pub score: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HighlightRange {
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchOptions {
    pub query: String,
    pub search_chapters: bool,
    pub search_characters: bool,
    pub search_settings: bool,
    pub search_events: bool,
    pub limit: Option<usize>,
}

impl Default for SearchOptions {
    fn default() -> Self {
        Self {
            query: String::new(),
            search_chapters: true,
            search_characters: true,
            search_settings: true,
            search_events: true,
            limit: Some(50),
        }
    }
}

/// Search across chapters and entities
#[tauri::command]
pub async fn search(book_id: String, options: SearchOptions) -> Result<Vec<SearchResult>, String> {
    if options.query.trim().is_empty() {
        return Ok(vec![]);
    }

    let library = Library::open().map_err(|e| e.to_string())?;
    let bid = BookId::from_string(book_id.clone());
    let book_dir = library.book_dir(&bid);
    let db_path = book_dir.join("book.db");

    if !db_path.exists() {
        return Err("Book database not found".to_string());
    }

    let db = BookDb::open(&db_path, bid.clone())
        .map_err(|e| format!("Failed to open book database: {}", e))?;

    let query = options.query.to_lowercase();
    let limit = options.limit.unwrap_or(50);
    let mut results = Vec::new();

    // Search chapters (full-text search in chapter content)
    if options.search_chapters {
        let chapter_results = search_chapters(&library, &bid, &query, limit)?;
        results.extend(chapter_results);
    }

    // Search characters
    if options.search_characters {
        let characters = db.list_characters().map_err(|e| e.to_string())?;
        for character in characters {
            if let Some(result) = match_character(&character, &query) {
                results.push(result);
            }
        }
    }

    // Search settings
    if options.search_settings {
        let settings = db.list_settings().map_err(|e| e.to_string())?;
        for setting in settings {
            if let Some(result) = match_setting(&setting, &query) {
                results.push(result);
            }
        }
    }

    // Search events
    if options.search_events {
        let events = db.list_events().map_err(|e| e.to_string())?;
        for event in events {
            if let Some(result) = match_event(&event, &query) {
                results.push(result);
            }
        }
    }

    // Sort by score and limit results
    results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    results.truncate(limit);

    Ok(results)
}

/// Search chapter content for matching text
#[tauri::command]
pub async fn search_chapters_content(
    book_id: String,
    query: String,
    limit: Option<usize>,
) -> Result<Vec<SearchResult>, String> {
    if query.trim().is_empty() {
        return Ok(vec![]);
    }

    let library = Library::open().map_err(|e| e.to_string())?;
    let bid = BookId::from_string(book_id);
    let query_lower = query.to_lowercase();
    let limit = limit.unwrap_or(50);

    search_chapters(&library, &bid, &query_lower, limit)
}

/// Search entities only (characters, settings, events)
#[tauri::command]
pub async fn search_entities(
    book_id: String,
    query: String,
    entity_types: Option<Vec<String>>,
) -> Result<Vec<SearchResult>, String> {
    if query.trim().is_empty() {
        return Ok(vec![]);
    }

    let library = Library::open().map_err(|e| e.to_string())?;
    let bid = BookId::from_string(book_id);
    let book_dir = library.book_dir(&bid);
    let db_path = book_dir.join("book.db");

    if !db_path.exists() {
        return Err("Book database not found".to_string());
    }

    let db = BookDb::open(&db_path, bid)
        .map_err(|e| format!("Failed to open book database: {}", e))?;

    let query_lower = query.to_lowercase();
    let types = entity_types.unwrap_or_else(|| {
        vec![
            "character".to_string(),
            "setting".to_string(),
            "event".to_string(),
        ]
    });

    let mut results = Vec::new();

    if types.contains(&"character".to_string()) {
        let characters = db.list_characters().map_err(|e| e.to_string())?;
        for character in characters {
            if let Some(result) = match_character(&character, &query_lower) {
                results.push(result);
            }
        }
    }

    if types.contains(&"setting".to_string()) {
        let settings = db.list_settings().map_err(|e| e.to_string())?;
        for setting in settings {
            if let Some(result) = match_setting(&setting, &query_lower) {
                results.push(result);
            }
        }
    }

    if types.contains(&"event".to_string()) {
        let events = db.list_events().map_err(|e| e.to_string())?;
        for event in events {
            if let Some(result) = match_event(&event, &query_lower) {
                results.push(result);
            }
        }
    }

    results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

    Ok(results)
}

fn search_chapters(
    library: &Library,
    book_id: &BookId,
    query: &str,
    limit: usize,
) -> Result<Vec<SearchResult>, String> {
    let book_dir = library.book_dir(book_id);
    let db_path = book_dir.join("book.db");

    if !db_path.exists() {
        return Err("Book database not found".to_string());
    }

    let db = BookDb::open(&db_path, book_id.clone())
        .map_err(|e| format!("Failed to open book database: {}", e))?;

    // Use FTS5 full-text search
    let fts_results = db
        .search_chapter_fts(query, limit)
        .map_err(|e| format!("FTS search failed: {}", e))?;

    let results = fts_results
        .into_iter()
        .map(|r| {
            let title = r
                .chapter_title
                .unwrap_or_else(|| format!("第 {} 章", r.chapter_index));
            // FTS5 rank is negative (lower = better match), normalize to 0.0-1.0 score
            let score = (1.0 / (1.0 - r.rank as f32)).min(1.0);

            SearchResult {
                result_type: "chapter".to_string(),
                id: r.chapter_id.clone(),
                title,
                content: r.snippet,
                chapter_id: Some(r.chapter_id),
                chapter_title: None,
                highlights: vec![],
                score,
            }
        })
        .collect();

    Ok(results)
}

fn match_character(character: &Character, query: &str) -> Option<SearchResult> {
    let name_lower = character.name.to_lowercase();
    let desc_lower = character
        .description
        .as_ref()
        .map(|d| d.to_lowercase())
        .unwrap_or_default();
    let aliases_lower: Vec<String> = character.aliases.iter().map(|a| a.to_lowercase()).collect();

    let mut score = 0.0;
    let mut highlights = Vec::new();

    // Check name match (highest weight)
    if name_lower.contains(query) {
        score += 0.5;
        if let Some(pos) = name_lower.find(query) {
            highlights.push(HighlightRange {
                start: pos,
                end: pos + query.len(),
            });
        }
    }

    // Check aliases
    for alias in &aliases_lower {
        if alias.contains(query) {
            score += 0.3;
        }
    }

    // Check description
    if desc_lower.contains(query) {
        score += 0.2;
    }

    // Check traits
    for trait_item in &character.traits {
        if trait_item.to_lowercase().contains(query) {
            score += 0.1;
        }
    }

    if score > 0.0 {
        let content = character
            .description
            .clone()
            .unwrap_or_else(|| format!("角色: {}", character.role));

        Some(SearchResult {
            result_type: "character".to_string(),
            id: character.id.to_string(),
            title: character.name.clone(),
            content,
            chapter_id: character.first_appearance_chapter_id.as_ref().map(|c| c.to_string()),
            chapter_title: None,
            highlights,
            score,
        })
    } else {
        None
    }
}

fn match_setting(setting: &Setting, query: &str) -> Option<SearchResult> {
    let name_lower = setting.name.to_lowercase();
    let desc_lower = setting
        .description
        .as_ref()
        .map(|d| d.to_lowercase())
        .unwrap_or_default();
    let type_lower = setting.setting_type.to_lowercase();

    let mut score = 0.0;
    let mut highlights = Vec::new();

    // Check name match
    if name_lower.contains(query) {
        score += 0.5;
        if let Some(pos) = name_lower.find(query) {
            highlights.push(HighlightRange {
                start: pos,
                end: pos + query.len(),
            });
        }
    }

    // Check type
    if type_lower.contains(query) {
        score += 0.2;
    }

    // Check description
    if desc_lower.contains(query) {
        score += 0.3;
    }

    if score > 0.0 {
        let content = setting
            .description
            .clone()
            .unwrap_or_else(|| format!("类型: {}", setting.setting_type));

        Some(SearchResult {
            result_type: "setting".to_string(),
            id: setting.id.to_string(),
            title: setting.name.clone(),
            content,
            chapter_id: None,
            chapter_title: None,
            highlights,
            score,
        })
    } else {
        None
    }
}

fn match_event(event: &Event, query: &str) -> Option<SearchResult> {
    let title_lower = event.title.to_lowercase();
    let desc_lower = event
        .description
        .as_ref()
        .map(|d| d.to_lowercase())
        .unwrap_or_default();

    let mut score = 0.0;
    let mut highlights = Vec::new();

    // Check title match
    if title_lower.contains(query) {
        score += 0.5;
        if let Some(pos) = title_lower.find(query) {
            highlights.push(HighlightRange {
                start: pos,
                end: pos + query.len(),
            });
        }
    }

    // Check description
    if desc_lower.contains(query) {
        score += 0.3;
    }

    if score > 0.0 {
        let content = event
            .description
            .clone()
            .unwrap_or_else(|| format!("重要性: {}", event.importance));

        Some(SearchResult {
            result_type: "event".to_string(),
            id: event.id.to_string(),
            title: event.title.clone(),
            content,
            chapter_id: event.chapter_id.as_ref().map(|c| c.to_string()),
            chapter_title: None,
            highlights,
            score,
        })
    } else {
        None
    }
}

// ============================================================================
// Vector Search Commands (P2.5-005~007)
// ============================================================================

use crate::retrieval::vector_search::VectorSearcher;

/// Semantic search using vector similarity (P2.5-006)
#[tauri::command]
pub async fn vector_search(
    book_id: String,
    query: String,
    top_k: Option<usize>,
    chunk_type: Option<String>,
    exclude_chapter_id: Option<String>,
) -> Result<Vec<VectorSearchResult>, String> {
    use crate::core::embedding::ChunkType;

    let searcher = VectorSearcher::open(&book_id).map_err(|e| e.to_string())?;

    let chunk_type_filter = chunk_type.and_then(|t| ChunkType::from_str(&t));

    let results = searcher
        .search_by_text(
            &query,
            top_k.unwrap_or(10),
            chunk_type_filter,
            exclude_chapter_id.as_deref(),
        )
        .map_err(|e| e.to_string())?;

    Ok(results
        .into_iter()
        .map(|r| VectorSearchResult {
            chunk_id: r.chunk_id,
            content: r.content,
            chapter_id: r.chapter_id,
            chunk_type: r.chunk_type,
            score: r.score,
            char_start: r.metadata.char_start,
            char_end: r.metadata.char_end,
            entities_mentioned: r.metadata.entities_mentioned,
        })
        .collect())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorSearchResult {
    pub chunk_id: String,
    pub content: String,
    pub chapter_id: String,
    pub chunk_type: String,
    pub score: f32,
    pub char_start: u32,
    pub char_end: u32,
    pub entities_mentioned: Vec<String>,
}

/// Search for passages related to specific entities (P2.5-007)
#[tauri::command]
pub async fn search_entity_mentions(
    book_id: String,
    entity_ids: Vec<String>,
    top_k_per_entity: Option<usize>,
) -> Result<Vec<EntitySearchResultInfo>, String> {
    let searcher = VectorSearcher::open(&book_id).map_err(|e| e.to_string())?;

    let results = searcher
        .search_by_entities(&entity_ids, top_k_per_entity.unwrap_or(5))
        .map_err(|e| e.to_string())?;

    Ok(results
        .into_iter()
        .map(|r| EntitySearchResultInfo {
            entity_id: r.entity_id,
            entity_name: r.entity_name,
            entity_type: r.entity_type,
            mentions: r
                .mentions
                .into_iter()
                .map(|m| VectorSearchResult {
                    chunk_id: m.chunk_id,
                    content: m.content,
                    chapter_id: m.chapter_id,
                    chunk_type: m.chunk_type,
                    score: m.score,
                    char_start: m.metadata.char_start,
                    char_end: m.metadata.char_end,
                    entities_mentioned: m.metadata.entities_mentioned,
                })
                .collect(),
        })
        .collect())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntitySearchResultInfo {
    pub entity_id: String,
    pub entity_name: String,
    pub entity_type: String,
    pub mentions: Vec<VectorSearchResult>,
}

/// Get entity history - passages where entity was mentioned across chapters
#[tauri::command]
pub async fn get_entity_history(
    book_id: String,
    entity_id: String,
    max_passages: Option<usize>,
) -> Result<Vec<VectorSearchResult>, String> {
    let searcher = VectorSearcher::open(&book_id).map_err(|e| e.to_string())?;

    let results = searcher
        .search_entity_history(&entity_id, max_passages.unwrap_or(20))
        .map_err(|e| e.to_string())?;

    Ok(results
        .into_iter()
        .map(|r| VectorSearchResult {
            chunk_id: r.chunk_id,
            content: r.content,
            chapter_id: r.chapter_id,
            chunk_type: r.chunk_type,
            score: r.score,
            char_start: r.metadata.char_start,
            char_end: r.metadata.char_end,
            entities_mentioned: r.metadata.entities_mentioned,
        })
        .collect())
}

/// Extract entity mentions from text (P2.5-003)
#[tauri::command]
pub async fn extract_mentions_from_text(
    book_id: String,
    text: String,
) -> Result<Vec<EntityMentionInfo>, String> {
    use crate::retrieval::vector_search::extract_entity_mentions;

    let searcher = VectorSearcher::open(&book_id).map_err(|e| e.to_string())?;
    let all_entities = searcher.get_all_entities().map_err(|e| e.to_string())?;

    let mentions = extract_entity_mentions(&text, &all_entities);

    Ok(mentions
        .into_iter()
        .map(|m| EntityMentionInfo {
            entity_id: m.entity_id,
            entity_name: m.entity_name,
            entity_type: m.entity_type,
            positions: m.positions,
        })
        .collect())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityMentionInfo {
    pub entity_id: String,
    pub entity_name: String,
    pub entity_type: String,
    pub positions: Vec<(usize, usize)>,
}

/// Get all known entities for a book
#[tauri::command]
pub async fn get_all_entities(book_id: String) -> Result<AllEntitiesInfo, String> {
    let searcher = VectorSearcher::open(&book_id).map_err(|e| e.to_string())?;
    let entities = searcher.get_all_entities().map_err(|e| e.to_string())?;

    Ok(AllEntitiesInfo {
        characters: entities
            .characters
            .into_iter()
            .map(|e| EntityInfoItem {
                id: e.id,
                name: e.name,
                entity_type: e.entity_type,
            })
            .collect(),
        settings: entities
            .settings
            .into_iter()
            .map(|e| EntityInfoItem {
                id: e.id,
                name: e.name,
                entity_type: e.entity_type,
            })
            .collect(),
        events: entities
            .events
            .into_iter()
            .map(|e| EntityInfoItem {
                id: e.id,
                name: e.name,
                entity_type: e.entity_type,
            })
            .collect(),
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityInfoItem {
    pub id: String,
    pub name: String,
    pub entity_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllEntitiesInfo {
    pub characters: Vec<EntityInfoItem>,
    pub settings: Vec<EntityInfoItem>,
    pub events: Vec<EntityInfoItem>,
}

// ============================================================================
// Advanced Search Commands (P4.5-001~006)
// ============================================================================

// (HybridSearchResult is used in type conversions below)

/// Semantic/Hybrid search (P4.5-003)
///
/// Performs hybrid search combining vector similarity and keyword matching.
/// This provides better results than pure keyword or pure vector search.
#[tauri::command]
pub async fn semantic_search(
    book_id: String,
    query: String,
    top_k: Option<usize>,
    search_mode: Option<String>, // "hybrid", "vector", "keyword"
    chunk_type: Option<String>,
    exclude_chapter_id: Option<String>,
) -> Result<Vec<SemanticSearchResult>, String> {
    use crate::core::embedding::ChunkType;

    let searcher = VectorSearcher::open(&book_id).map_err(|e| e.to_string())?;
    let chunk_type_filter = chunk_type.and_then(|t| ChunkType::from_str(&t));
    let mode = search_mode.unwrap_or_else(|| "hybrid".to_string());
    let k = top_k.unwrap_or(10);

    match mode.as_str() {
        "vector" => {
            // Pure vector search
            let results = searcher
                .search_by_text(&query, k, chunk_type_filter, exclude_chapter_id.as_deref())
                .map_err(|e| e.to_string())?;

            Ok(results
                .into_iter()
                .map(|r| SemanticSearchResult {
                    chunk_id: r.chunk_id,
                    content: r.content,
                    chapter_id: r.chapter_id,
                    chunk_type: r.chunk_type,
                    score: r.score,
                    search_mode: "vector".to_string(),
                    vector_rank: Some(1), // Not tracked in simple search
                    keyword_rank: None,
                    char_start: r.metadata.char_start,
                    char_end: r.metadata.char_end,
                    entities_mentioned: r.metadata.entities_mentioned,
                })
                .collect())
        }
        "keyword" => {
            // Pure keyword search
            let results = searcher
                .search_by_keyword(&query, k, chunk_type_filter, exclude_chapter_id.as_deref())
                .map_err(|e| e.to_string())?;

            Ok(results
                .into_iter()
                .map(|r| SemanticSearchResult {
                    chunk_id: r.chunk_id,
                    content: r.content,
                    chapter_id: r.chapter_id,
                    chunk_type: r.chunk_type,
                    score: r.score,
                    search_mode: "keyword".to_string(),
                    vector_rank: None,
                    keyword_rank: Some(1),
                    char_start: r.metadata.char_start,
                    char_end: r.metadata.char_end,
                    entities_mentioned: r.metadata.entities_mentioned,
                })
                .collect())
        }
        _ => {
            // Hybrid search (default)
            let results = searcher
                .hybrid_search(&query, k, chunk_type_filter, exclude_chapter_id.as_deref(), None)
                .map_err(|e| e.to_string())?;

            Ok(results
                .into_iter()
                .map(|r| SemanticSearchResult {
                    chunk_id: r.chunk_id,
                    content: r.content,
                    chapter_id: r.chapter_id,
                    chunk_type: r.chunk_type,
                    score: r.rrf_score,
                    search_mode: "hybrid".to_string(),
                    vector_rank: r.vector_rank,
                    keyword_rank: r.keyword_rank,
                    char_start: r.metadata.char_start,
                    char_end: r.metadata.char_end,
                    entities_mentioned: r.metadata.entities_mentioned,
                })
                .collect())
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticSearchResult {
    pub chunk_id: String,
    pub content: String,
    pub chapter_id: String,
    pub chunk_type: String,
    pub score: f32,
    pub search_mode: String,
    pub vector_rank: Option<usize>,
    pub keyword_rank: Option<usize>,
    pub char_start: u32,
    pub char_end: u32,
    pub entities_mentioned: Vec<String>,
}
