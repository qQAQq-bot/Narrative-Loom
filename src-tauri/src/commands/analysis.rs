use crate::commands::dedup::{check_entity_dedup, is_combo_name, KnownEntity};
use crate::core::agent::{TaskType, DEFAULT_CHAPTER_ANALYSIS_TYPES};
use crate::core::card::CardStatus;
use crate::core::embedding::DEFAULT_EMBEDDING_DIMENSIONS;
use crate::core::ids::{BookId, CardId, ChapterId, EntityId};
use crate::ingestion::generate_embeddings_for_chapter;
use crate::retrieval::ContextBuilder;
use crate::sidecar::get_sidecar;
use crate::storage::book_db::{BookDb, Character, Event, KnowledgeCard, Setting, TechniqueCard};
use crate::storage::config::ConfigStore;
use crate::storage::keychain::KeychainService;
use crate::storage::library::Library;
use crate::storage::vectors::VectorDb;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use tauri::{AppHandle, Emitter};

/// Analysis progress event payload (P2-049)
#[derive(Debug, Clone, Serialize)]
pub struct AnalysisProgressEvent {
    pub book_id: String,
    pub chapter_id: String,
    pub stage: String,
    pub progress: f32,
    pub message: String,
}

/// Helper function to emit analysis progress events
fn emit_progress(
    app: &AppHandle,
    book_id: &str,
    chapter_id: &str,
    stage: &str,
    progress: f32,
    message: &str,
) {
    let event = AnalysisProgressEvent {
        book_id: book_id.to_string(),
        chapter_id: chapter_id.to_string(),
        stage: stage.to_string(),
        progress,
        message: message.to_string(),
    };
    if let Err(e) = app.emit("analysis-progress", event) {
        tracing::warn!("Failed to emit analysis progress event: {}", e);
    }
}

/// Check if a chapter has embeddings, generate if not (lazy embedding)
/// Returns the number of chunks embedded (0 if already existed)
fn ensure_chapter_embeddings(
    book_dir: &Path,
    book_id: &BookId,
    chapter_id: &ChapterId,
    chapter_content: &str,
    chapter_index: u32,
) -> Result<u32, String> {
    let vectors_path = book_dir.join("vectors.db");

    // Check if vectors.db exists and has chunks for this chapter
    let needs_embedding = if vectors_path.exists() {
        match VectorDb::open(&vectors_path, DEFAULT_EMBEDDING_DIMENSIONS) {
            Ok(db) => {
                let chunk_count = db.count_chunks_by_chapter(chapter_id.as_str())
                    .unwrap_or(0);
                chunk_count == 0
            }
            Err(_) => true, // Can't open, assume needs embedding
        }
    } else {
        true // No vectors.db, needs embedding
    };

    if !needs_embedding {
        tracing::debug!(
            "Chapter {} already has embeddings, skipping generation",
            chapter_id
        );
        return Ok(0);
    }

    tracing::info!(
        "Generating embeddings for chapter {} (lazy embedding)",
        chapter_id
    );

    // Generate embeddings for this chapter
    match generate_embeddings_for_chapter(
        book_dir,
        book_id,
        chapter_id,
        chapter_content,
        chapter_index,
    ) {
        Ok(count) => {
            tracing::info!(
                "Generated {} embeddings for chapter {}",
                count,
                chapter_id
            );
            Ok(count)
        }
        Err(e) => {
            tracing::warn!(
                "Failed to generate embeddings for chapter {}: {}",
                chapter_id,
                e
            );
            // Don't fail the analysis, just continue without embeddings
            Ok(0)
        }
    }
}

/// Update chapter's Paragraph chunks with entity IDs extracted during analysis
/// This enables RAG to find relevant entities by searching similar paragraphs
///
/// Entity association rules:
/// - Characters: Only those with first_appearance_chapter_id == this chapter
/// - Events: Only those with chapter_id == this chapter
/// - Settings: Use knowledge_cards to find settings discovered from this chapter
fn update_chapter_chunks_with_entities(
    book_dir: &Path,
    chapter_id: &ChapterId,
    book_db: &BookDb,
) -> Result<(), String> {
    let vectors_path = book_dir.join("vectors.db");

    if !vectors_path.exists() {
        tracing::debug!("No vectors.db found, skipping entity linking");
        return Ok(());
    }

    let vector_db = VectorDb::open(&vectors_path, DEFAULT_EMBEDDING_DIMENSIONS)
        .map_err(|e| format!("Failed to open vectors.db: {}", e))?;

    // Collect entity IDs that belong to THIS chapter specifically
    let mut entity_ids: Vec<String> = Vec::new();
    let mut char_count = 0;
    let mut event_count = 0;
    let mut setting_count = 0;

    // Get characters that first appeared in this chapter
    if let Ok(characters) = book_db.list_characters() {
        for c in characters {
            if c.first_appearance_chapter_id.as_ref() == Some(chapter_id) {
                entity_ids.push(c.id.to_string());
                char_count += 1;
            }
        }
    }

    // Get events that belong to this chapter (Event has chapter_id field)
    if let Ok(events) = book_db.list_events() {
        for e in events {
            if e.chapter_id.as_ref() == Some(chapter_id) {
                entity_ids.push(e.id.to_string());
                event_count += 1;
            }
        }
    }

    // Get settings discovered from this chapter
    // Settings don't have chapter_id, so we use knowledge_cards as a bridge:
    // Find accepted setting cards from this chapter, then match to setting entities by name
    if let Ok(cards) = book_db.list_knowledge_cards_by_chapter(chapter_id) {
        let setting_names: Vec<String> = cards
            .iter()
            .filter(|c| c.knowledge_type == "setting" && c.status == CardStatus::Accepted.as_str())
            .filter_map(|c| {
                c.content.get("name")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_lowercase())
            })
            .collect();

        if !setting_names.is_empty() {
            if let Ok(settings) = book_db.list_settings() {
                for s in settings {
                    if setting_names.contains(&s.name.to_lowercase()) {
                        entity_ids.push(s.id.to_string());
                        setting_count += 1;
                    }
                }
            }
        }
    }

    if entity_ids.is_empty() {
        tracing::debug!("No entities found for chapter {}", chapter_id);
        return Ok(());
    }

    // Update chunks with entity IDs
    let updated = vector_db
        .update_chapter_chunks_entities(chapter_id.as_str(), &entity_ids)
        .map_err(|e| format!("Failed to update chunks: {}", e))?;

    tracing::info!(
        "[RAG Entity Link] Updated {} chunks for chapter {} with {} entities (chars={}, events={}, settings={})",
        updated,
        chapter_id,
        entity_ids.len(),
        char_count,
        event_count,
        setting_count
    );

    Ok(())
}

/// Check if a confidence level should be auto-accepted based on threshold
/// Threshold: "off" = never, "high" = only high, "medium" = high+medium, "low" = all
fn should_auto_accept(confidence: &str, threshold: &str) -> bool {
    match threshold {
        "off" => false,
        "high" => confidence == "high",
        "medium" => confidence == "high" || confidence == "medium",
        "low" => true, // Accept all
        _ => false,
    }
}

/// Auto-accept a knowledge card, converting it to the appropriate entity type
fn auto_accept_card(book_db: &BookDb, card: &KnowledgeCard) -> Result<(), String> {
    let now = Utc::now().to_rfc3339();

    match card.knowledge_type.as_str() {
        "character" => {
            let name = card.content.get("name")
                .and_then(|v| v.as_str())
                .unwrap_or(&card.title)
                .to_string();

            if is_combo_name(&name) {
                tracing::info!("auto_accept_card: Skipping combo name: {}", name);
                book_db
                    .update_knowledge_card_status(&card.id, CardStatus::Rejected)
                    .map_err(|e| e.to_string())?;
                return Ok(());
            }

            let existing_chars = book_db.list_characters().map_err(|e| e.to_string())?;
            let known_entities: Vec<KnownEntity> = existing_chars.iter().map(|c| KnownEntity {
                id: c.id.to_string(),
                name: c.name.clone(),
                aliases: c.aliases.clone(),
            }).collect();

            let dedup_result = check_entity_dedup(&name, &known_entities, 0.85);

            if let Some(merge_id) = dedup_result.merge_with {
                if let Some(existing) = existing_chars.iter().find(|c| c.id.to_string() == merge_id) {
                    tracing::info!("auto_accept_card: Dedup merging '{}' with existing '{}': {}",
                        name, existing.name, dedup_result.reason.as_deref().unwrap_or(""));

                    let new_description = card.content.get("description")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                    let description = new_description.or(existing.description.clone());

                    let mut aliases = existing.aliases.clone();
                    if name.to_lowercase() != existing.name.to_lowercase() && !aliases.iter().any(|a| a.to_lowercase() == name.to_lowercase()) {
                        aliases.push(name.clone());
                    }
                    if let Some(new_aliases) = card.content.get("aliases").and_then(|v| v.as_array()) {
                        for alias in new_aliases.iter().filter_map(|v| v.as_str()) {
                            if !aliases.iter().any(|a| a.to_lowercase() == alias.to_lowercase()) {
                                aliases.push(alias.to_string());
                            }
                        }
                    }

                    let mut traits = existing.traits.clone();
                    if let Some(new_traits) = card.content.get("traits").and_then(|v| v.as_array()) {
                        for t in new_traits.iter().filter_map(|v| v.as_str()) {
                            if !traits.iter().any(|x| x.to_lowercase() == t.to_lowercase()) {
                                traits.push(t.to_string());
                            }
                        }
                    }

                    let role = card.content.get("role")
                        .and_then(|v| v.as_str())
                        .unwrap_or(&existing.role)
                        .to_string();

                    let mut evidence = existing.evidence.clone();
                    for e in &card.evidence {
                        if !evidence.contains(e) {
                            evidence.push(e.clone());
                        }
                    }

                    let updated = Character {
                        id: existing.id.clone(),
                        name: existing.name.clone(),
                        aliases,
                        description,
                        description_structured: existing.description_structured.clone(),
                        traits,
                        role,
                        first_appearance_chapter_id: existing.first_appearance_chapter_id.clone(),
                        relationships: existing.relationships.clone(),
                        evidence,
                        notes: existing.notes.clone(),
                        updated_at: now.clone(),
                    };

                    book_db.update_character(&updated).map_err(|e| e.to_string())?;
                    book_db
                        .update_knowledge_card_status(&card.id, CardStatus::Accepted)
                        .map_err(|e| e.to_string())?;
                    return Ok(());
                }
            }

            if let Ok(Some(existing)) = book_db.find_character_by_name(&name) {
                let new_description = card.content.get("description")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                let description = new_description.or(existing.description);

                let mut aliases = existing.aliases.clone();
                if let Some(new_aliases) = card.content.get("aliases").and_then(|v| v.as_array()) {
                    for alias in new_aliases.iter().filter_map(|v| v.as_str()) {
                        if !aliases.iter().any(|a| a.to_lowercase() == alias.to_lowercase()) {
                            aliases.push(alias.to_string());
                        }
                    }
                }

                let mut traits = existing.traits.clone();
                if let Some(new_traits) = card.content.get("traits").and_then(|v| v.as_array()) {
                    for t in new_traits.iter().filter_map(|v| v.as_str()) {
                        if !traits.iter().any(|x| x.to_lowercase() == t.to_lowercase()) {
                            traits.push(t.to_string());
                        }
                    }
                }

                let role = card.content.get("role")
                    .and_then(|v| v.as_str())
                    .unwrap_or(&existing.role)
                    .to_string();

                let mut evidence = existing.evidence.clone();
                for e in &card.evidence {
                    if !evidence.contains(e) {
                        evidence.push(e.clone());
                    }
                }

                let updated = Character {
                    id: existing.id,
                    name: existing.name,
                    aliases,
                    description,
                    description_structured: existing.description_structured,
                    traits,
                    role,
                    first_appearance_chapter_id: existing.first_appearance_chapter_id,
                    relationships: existing.relationships,
                    evidence,
                    notes: existing.notes,
                    updated_at: now.clone(),
                };

                book_db.update_character(&updated).map_err(|e| e.to_string())?;
                book_db
                    .update_knowledge_card_status(&card.id, CardStatus::Accepted)
                    .map_err(|e| e.to_string())?;
                return Ok(());
            }

            let description = card.content.get("description")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            let aliases: Vec<String> = card.content.get("aliases")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                .unwrap_or_default();

            let traits: Vec<String> = card.content.get("traits")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                .unwrap_or_default();

            let role = card.content.get("role")
                .and_then(|v| v.as_str())
                .unwrap_or("supporting")
                .to_string();

            let character = Character {
                id: EntityId::new(),
                name,
                aliases,
                description,
                description_structured: None,
                traits,
                role,
                first_appearance_chapter_id: Some(card.chapter_id.clone()),
                relationships: serde_json::json!({}),
                evidence: card.evidence.clone(),
                notes: None,
                updated_at: now.clone(),
            };

            book_db.insert_character(&character).map_err(|e| e.to_string())?;
        }
        "setting" => {
            let name = card.content.get("name")
                .and_then(|v| v.as_str())
                .unwrap_or(&card.title)
                .to_string();

            if is_combo_name(&name) {
                tracing::info!("auto_accept_card: Skipping combo setting name: {}", name);
                book_db
                    .update_knowledge_card_status(&card.id, CardStatus::Rejected)
                    .map_err(|e| e.to_string())?;
                return Ok(());
            }

            let existing_settings = book_db.list_settings().map_err(|e| e.to_string())?;
            let known_entities: Vec<KnownEntity> = existing_settings.iter().map(|s| KnownEntity {
                id: s.id.to_string(),
                name: s.name.clone(),
                aliases: vec![],
            }).collect();

            let dedup_result = check_entity_dedup(&name, &known_entities, 0.85);

            if let Some(merge_id) = dedup_result.merge_with {
                if let Some(existing) = existing_settings.iter().find(|s| s.id.to_string() == merge_id) {
                    tracing::info!("auto_accept_card: Dedup merging setting '{}' with existing '{}': {}",
                        name, existing.name, dedup_result.reason.as_deref().unwrap_or(""));

                    let new_description = card.content.get("description")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                    let description = new_description.or(existing.description.clone());

                    let setting_type = card.content.get("type")
                        .or_else(|| card.content.get("setting_type"))
                        .and_then(|v| v.as_str())
                        .unwrap_or(&existing.setting_type)
                        .to_string();

                    let mut properties = existing.properties.clone();
                    if let Some(obj) = properties.as_object_mut() {
                        if let Some(new_props) = card.content.get("properties").and_then(|v| v.as_object()) {
                            for (k, v) in new_props {
                                obj.insert(k.clone(), v.clone());
                            }
                        }
                    }

                    let mut evidence = existing.evidence.clone();
                    for e in &card.evidence {
                        if !evidence.contains(e) {
                            evidence.push(e.clone());
                        }
                    }

                    let updated = Setting {
                        id: existing.id.clone(),
                        setting_type,
                        name: existing.name.clone(),
                        description,
                        description_structured: existing.description_structured.clone(),
                        properties,
                        evidence,
                        notes: existing.notes.clone(),
                        updated_at: now.clone(),
                    };

                    book_db.update_setting(&updated).map_err(|e| e.to_string())?;
                    book_db
                        .update_knowledge_card_status(&card.id, CardStatus::Accepted)
                        .map_err(|e| e.to_string())?;
                    return Ok(());
                }
            }

            if let Ok(Some(existing)) = book_db.find_setting_by_name(&name) {
                let new_description = card.content.get("description")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                let description = new_description.or(existing.description);

                let setting_type = card.content.get("type")
                    .or_else(|| card.content.get("setting_type"))
                    .and_then(|v| v.as_str())
                    .unwrap_or(&existing.setting_type)
                    .to_string();

                // Merge properties
                let mut properties = existing.properties.clone();
                if let Some(obj) = properties.as_object_mut() {
                    if let Some(new_props) = card.content.get("properties").and_then(|v| v.as_object()) {
                        for (k, v) in new_props {
                            obj.insert(k.clone(), v.clone());
                        }
                    }
                }

                let mut evidence = existing.evidence.clone();
                for e in &card.evidence {
                    if !evidence.contains(e) {
                        evidence.push(e.clone());
                    }
                }

                let updated = Setting {
                    id: existing.id,
                    setting_type,
                    name: existing.name,
                    description,
                    description_structured: existing.description_structured,
                    properties,
                    evidence,
                    notes: existing.notes,
                    updated_at: now.clone(),
                };

                book_db.update_setting(&updated).map_err(|e| e.to_string())?;
                book_db
                    .update_knowledge_card_status(&card.id, CardStatus::Accepted)
                    .map_err(|e| e.to_string())?;
                return Ok(());
            }

            let description = card.content.get("description")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            let setting_type = card.content.get("type")
                .or_else(|| card.content.get("setting_type"))
                .and_then(|v| v.as_str())
                .unwrap_or("location")
                .to_string();

            let properties = card.content.get("properties")
                .cloned()
                .unwrap_or_else(|| serde_json::json!({}));

            let setting = Setting {
                id: EntityId::new(),
                name,
                setting_type,
                description,
                description_structured: None,
                properties,
                evidence: card.evidence.clone(),
                notes: None,
                updated_at: now.clone(),
            };

            book_db.insert_setting(&setting).map_err(|e| e.to_string())?;
        }
        "event" => {
            let title = card.content.get("title")
                .or_else(|| card.content.get("name"))
                .and_then(|v| v.as_str())
                .unwrap_or(&card.title)
                .to_string();

            let description = card.content.get("description")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            let importance = card.content.get("importance")
                .and_then(|v| v.as_str())
                .unwrap_or("normal")
                .to_string();

            // Support both the new "flattened" fields and the legacy location under content.extra.
            let extra = card.content.get("extra");

            let time_marker = card.content.get("time_marker")
                .and_then(|v| v.as_str())
                .or_else(|| extra.and_then(|e| e.get("time_marker")).and_then(|v| v.as_str()))
                .map(|s| s.to_string());

            let order_in_chapter = card.content.get("order_in_chapter")
                .or_else(|| extra.and_then(|e| e.get("order_in_chapter")))
                .and_then(|v| v.as_i64().map(|n| n as i32).or_else(|| v.as_f64().map(|n| n as i32)))
                .unwrap_or(0);

            let is_flashback = card.content.get("is_flashback")
                .and_then(|v| v.as_bool())
                .or_else(|| extra.and_then(|e| e.get("is_flashback")).and_then(|v| v.as_bool()))
                .unwrap_or(false);

            let relative_time = card.content.get("relative_time")
                .and_then(|v| v.as_str())
                .or_else(|| extra.and_then(|e| e.get("relative_time")).and_then(|v| v.as_str()))
                .map(|s| s.to_string());

            // Note: characters_involved expects EntityId, but we don't have entity IDs here
            // Leave empty for now - user can link characters manually later
            let characters_involved: Vec<EntityId> = Vec::new();

            let event = Event {
                id: EntityId::new(),
                title,
                description,
                importance,
                chapter_id: Some(card.chapter_id.clone()),
                characters_involved,
                evidence: card.evidence.clone(),
                notes: None,
                updated_at: now.clone(),
                time_marker,
                order_in_chapter,
                is_flashback,
                relative_time,
            };

            book_db.insert_event(&event).map_err(|e| e.to_string())?;
        }
        _ => {}
    }

    // Mark the card as accepted
    book_db
        .update_knowledge_card_status(&card.id, CardStatus::Accepted)
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TechniqueCardInfo {
    pub id: String,
    pub chapter_id: String,
    pub technique_type: String,
    pub title: String,
    pub description: String,
    pub mechanism: String,
    pub evidence: Vec<String>,
    pub tags: Vec<String>,
    pub collected: bool,
    pub created_at: String,
}

impl From<TechniqueCard> for TechniqueCardInfo {
    fn from(card: TechniqueCard) -> Self {
        Self {
            id: card.id.to_string(),
            chapter_id: card.chapter_id.to_string(),
            technique_type: card.technique_type,
            title: card.title,
            description: card.description,
            mechanism: card.mechanism,
            evidence: card.evidence,
            tags: card.tags,
            collected: card.collected,
            created_at: card.created_at,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeCardInfo {
    pub id: String,
    pub chapter_id: String,
    pub knowledge_type: String,
    pub title: String,
    pub content: serde_json::Value,
    pub evidence: Vec<String>,
    pub confidence: String,
    pub status: String,
    pub created_at: String,
}

impl From<KnowledgeCard> for KnowledgeCardInfo {
    fn from(card: KnowledgeCard) -> Self {
        Self {
            id: card.id.to_string(),
            chapter_id: card.chapter_id.to_string(),
            knowledge_type: card.knowledge_type,
            title: card.title,
            content: card.content,
            evidence: card.evidence,
            confidence: card.confidence,
            status: card.status,
            created_at: card.created_at,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisRequest {
    pub book_id: String,
    pub chapter_id: String,
    pub analysis_types: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisResult {
    pub chapter_id: String,
    pub technique_cards: Vec<TechniqueCardInfo>,
    pub knowledge_cards: Vec<KnowledgeCardInfo>,
    pub success: bool,
    pub error: Option<String>,
}

#[tauri::command]
pub async fn get_technique_cards(
    book_id: String,
    chapter_id: String,
) -> Result<Vec<TechniqueCardInfo>, String> {
    let library = Library::open().map_err(|e| e.to_string())?;
    let bid = BookId::from_string(book_id);
    let cid = ChapterId::from_string(chapter_id);

    let book_dir = library.book_dir(&bid);
    let book_db_path = book_dir.join("book.db");

    if !book_db_path.exists() {
        return Err("Book database not found".to_string());
    }

    let book_db = BookDb::open(&book_db_path, bid).map_err(|e| e.to_string())?;
    let cards = book_db
        .list_technique_cards_by_chapter(&cid)
        .map_err(|e| e.to_string())?;

    Ok(cards.into_iter().map(TechniqueCardInfo::from).collect())
}

#[tauri::command]
pub async fn get_knowledge_cards(
    book_id: String,
    chapter_id: String,
) -> Result<Vec<KnowledgeCardInfo>, String> {
    let library = Library::open().map_err(|e| e.to_string())?;
    let bid = BookId::from_string(book_id);
    let cid = ChapterId::from_string(chapter_id);

    let book_dir = library.book_dir(&bid);
    let book_db_path = book_dir.join("book.db");

    if !book_db_path.exists() {
        return Err("Book database not found".to_string());
    }

    let book_db = BookDb::open(&book_db_path, bid).map_err(|e| e.to_string())?;
    let cards = book_db
        .list_knowledge_cards_by_chapter(&cid)
        .map_err(|e| e.to_string())?;

    Ok(cards.into_iter().map(KnowledgeCardInfo::from).collect())
}

#[tauri::command]
pub async fn collect_technique(book_id: String, card_id: String) -> Result<bool, String> {
    let library = Library::open().map_err(|e| e.to_string())?;
    let bid = BookId::from_string(book_id);
    let cid = CardId::from_string(card_id);

    let book_dir = library.book_dir(&bid);
    let book_db_path = book_dir.join("book.db");

    if !book_db_path.exists() {
        return Err("Book database not found".to_string());
    }

    let book_db = BookDb::open(&book_db_path, bid).map_err(|e| e.to_string())?;
    book_db
        .update_technique_card_collected(&cid, true)
        .map_err(|e| e.to_string())?;

    Ok(true)
}

#[tauri::command]
pub async fn uncollect_technique(book_id: String, card_id: String) -> Result<bool, String> {
    let library = Library::open().map_err(|e| e.to_string())?;
    let bid = BookId::from_string(book_id);
    let cid = CardId::from_string(card_id);

    let book_dir = library.book_dir(&bid);
    let book_db_path = book_dir.join("book.db");

    if !book_db_path.exists() {
        return Err("Book database not found".to_string());
    }

    let book_db = BookDb::open(&book_db_path, bid).map_err(|e| e.to_string())?;
    book_db
        .update_technique_card_collected(&cid, false)
        .map_err(|e| e.to_string())?;

    Ok(true)
}

#[tauri::command]
pub async fn get_collected_techniques(
    book_id: String,
) -> Result<Vec<TechniqueCardInfo>, String> {
    let library = Library::open().map_err(|e| e.to_string())?;
    let bid = BookId::from_string(book_id);

    let book_dir = library.book_dir(&bid);
    let book_db_path = book_dir.join("book.db");

    if !book_db_path.exists() {
        return Err("Book database not found".to_string());
    }

    let book_db = BookDb::open(&book_db_path, bid).map_err(|e| e.to_string())?;
    let cards = book_db
        .list_collected_technique_cards()
        .map_err(|e| e.to_string())?;

    Ok(cards.into_iter().map(TechniqueCardInfo::from).collect())
}

#[tauri::command]
pub async fn update_knowledge_card_status(
    book_id: String,
    card_id: String,
    status: String,
) -> Result<bool, String> {
    // Backwards compatible mapping: some older UIs use "confirmed" meaning "accepted".
    let validated_status = if status == "confirmed" {
        CardStatus::Accepted
    } else {
        CardStatus::parse(status.as_str()).ok_or_else(|| {
            format!(
                "Invalid status: {}. Allowed: pending, accepted, rejected, merged",
                status
            )
        })?
    };

    let library = Library::open().map_err(|e| e.to_string())?;
    let bid = BookId::from_string(book_id);
    let cid = CardId::from_string(card_id);

    let book_dir = library.book_dir(&bid);
    let book_db_path = book_dir.join("book.db");

    if !book_db_path.exists() {
        return Err("Book database not found".to_string());
    }

    let book_db = BookDb::open(&book_db_path, bid).map_err(|e| e.to_string())?;
    book_db
        .update_knowledge_card_status(&cid, validated_status)
        .map_err(|e| e.to_string())?;

    Ok(true)
}

#[tauri::command]
pub async fn get_pending_knowledge_cards(book_id: String) -> Result<Vec<KnowledgeCardInfo>, String> {
    let library = Library::open().map_err(|e| e.to_string())?;
    let bid = BookId::from_string(book_id);

    let book_dir = library.book_dir(&bid);
    let book_db_path = book_dir.join("book.db");

    if !book_db_path.exists() {
        return Err("Book database not found".to_string());
    }

    let book_db = BookDb::open(&book_db_path, bid).map_err(|e| e.to_string())?;
    let cards = book_db
        .list_pending_knowledge_cards()
        .map_err(|e| e.to_string())?;

    Ok(cards.into_iter().map(KnowledgeCardInfo::from).collect())
}

#[tauri::command]
pub async fn mark_chapter_analyzed(
    book_id: String,
    chapter_id: String,
    technique_count: u32,
    knowledge_count: u32,
) -> Result<bool, String> {
    let library = Library::open().map_err(|e| e.to_string())?;
    let bid = BookId::from_string(book_id);
    let cid = ChapterId::from_string(chapter_id);

    let book_dir = library.book_dir(&bid);
    let book_db_path = book_dir.join("book.db");

    if !book_db_path.exists() {
        return Err("Book database not found".to_string());
    }

    let book_db = BookDb::open(&book_db_path, bid.clone()).map_err(|e| e.to_string())?;
    book_db
        .update_chapter_analyzed(&cid, true, technique_count, knowledge_count)
        .map_err(|e| e.to_string())?;

    // Update analyzed_chapters count in library
    let analyzed_count = book_db.count_analyzed_chapters().map_err(|e| e.to_string())?;
    if let Some(mut book) = library.get_book(&bid).map_err(|e| e.to_string())? {
        book.analyzed_chapters = analyzed_count;
        library.update_book(&book).map_err(|e| e.to_string())?;
    }

    Ok(true)
}

/// Custom deserializer for evidence field that handles both formats:
/// 1. `["string1", "string2"]` - array of strings
/// 2. `[{"content": "string1"}, {"content": "string2"}]` - array of objects with content field
fn deserialize_evidence<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::Error;
    
    let value: serde_json::Value = serde::Deserialize::deserialize(deserializer)?;
    
    match value {
        serde_json::Value::Array(arr) => {
            let mut result = Vec::with_capacity(arr.len());
            for item in arr {
                match item {
                    serde_json::Value::String(s) => result.push(s),
                    serde_json::Value::Object(obj) => {
                        if let Some(serde_json::Value::String(content)) = obj.get("content") {
                            result.push(content.clone());
                        }
                    }
                    _ => {}
                }
            }
            Ok(result)
        }
        serde_json::Value::Null => Ok(Vec::new()),
        _ => Err(D::Error::custom("evidence must be an array")),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SidecarAnalysisResult {
    techniques: Vec<SidecarTechnique>,
    characters: Vec<SidecarKnowledge>,
    settings: Vec<SidecarKnowledge>,
    events: Vec<SidecarKnowledge>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SidecarTechnique {
    #[serde(rename = "type")]
    technique_type: String,
    title: String,
    description: String,
    mechanism: String,
    #[serde(default, deserialize_with = "deserialize_evidence")]
    evidence: Vec<String>,
    #[serde(default)]
    tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SidecarKnowledge {
    #[serde(rename = "type")]
    knowledge_type: Option<String>,
    name: Option<String>,
    title: Option<String>,
    description: Option<String>,
    #[serde(default, deserialize_with = "deserialize_evidence")]
    evidence: Vec<String>,
    #[serde(default)]
    confidence: Option<String>,
    #[serde(flatten)]
    extra: serde_json::Value,
}

#[tauri::command]
pub async fn analyze_chapter(
    app: AppHandle,
    book_id: String,
    chapter_id: String,
    analysis_types: Option<Vec<String>>,
) -> Result<AnalysisResult, String> {
    // Stage 1: Initialization (0-10%)
    emit_progress(&app, &book_id, &chapter_id, "init", 0.0, "正在初始化分析...");

    let library = Library::open().map_err(|e| e.to_string())?;
    let bid = BookId::from_string(book_id.clone());
    let cid = ChapterId::from_string(chapter_id.clone());

    let book_dir = library.book_dir(&bid);
    let book_db_path = book_dir.join("book.db");

    if !book_db_path.exists() {
        return Err("Book database not found".to_string());
    }

    emit_progress(&app, &book_id, &chapter_id, "init", 5.0, "正在加载章节内容...");

    let book_db = BookDb::open(&book_db_path, bid.clone()).map_err(|e| e.to_string())?;

    let chapter = book_db
        .get_chapter(&cid)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Chapter not found: {}", chapter_id))?;

    let content = book_db
        .get_chapter_content(&cid)
        .map_err(|e| e.to_string())?
        .unwrap_or_default();

    let book_meta = library
        .get_book(&bid)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Book metadata not found".to_string())?;

    emit_progress(&app, &book_id, &chapter_id, "init", 10.0, "正在加载 Agent 配置...");

    let config_store = ConfigStore::new().map_err(|e| e.to_string())?;
    let keychain = KeychainService::new();

    // Load TaskBindings and Agent configurations
    let task_bindings = config_store.load_task_bindings().map_err(|e| e.to_string())?;
    let agents = config_store.load_agents().map_err(|e| e.to_string())?;
    let providers = config_store.load_providers().map_err(|e| e.to_string())?;
    let max_retries = config_store.get_request_retry_count().unwrap_or(3);
    let prompt_cards = config_store.load_prompt_cards().unwrap_or_default();

    // Map analysis types to TaskType
    let types = analysis_types.unwrap_or_else(|| {
        DEFAULT_CHAPTER_ANALYSIS_TYPES
            .iter()
            .map(|s| s.to_string())
            .collect()
    });

    // Build agent_configs for each analysis type
    let mut agent_configs: HashMap<String, serde_json::Value> = HashMap::new();

    tracing::info!(
        "Building agent configs: {} task bindings, {} agents, {} providers",
        task_bindings.bindings.len(),
        agents.len(),
        providers.len()
    );

    for agent in &agents {
        tracing::info!(
            "Loaded agent: id={}, name={}, model={}, provider={}, enabled={}",
            agent.id, agent.name, agent.model, agent.provider_id, agent.enabled
        );
    }

    for analysis_type in &types {
        let task_type = match TaskType::from_analysis_type(analysis_type.as_str()) {
            Some(t) => t,
            None => {
                tracing::warn!("Unknown analysis type requested: {}", analysis_type);
                continue;
            }
        };

        // Find the Agent ID bound to this task type
        if let Some(agent_id) = task_bindings.bindings.get(&task_type) {
            tracing::info!("Task {:?} bound to agent: {}", task_type, agent_id);

            // Find the Agent configuration
            if let Some(agent) = agents.iter().find(|a| &a.id == agent_id && a.enabled) {
                tracing::info!("Found agent {} with provider_id={}, model={}", agent.id, agent.provider_id, agent.model);

                // Find the Provider configuration (don't require enabled - use whatever is configured)
                if let Some(provider) = providers.iter().find(|p| p.id == agent.provider_id) {
                    if !provider.enabled {
                        tracing::warn!("Provider {} is disabled, but using it anyway for agent {}", provider.id, agent.id);
                    }

                    // Get API key from keychain
                    let api_key = if provider.api_key_ref.is_empty() {
                        String::new()
                    } else {
                        keychain
                            .get_key(&provider.api_key_ref)
                            .unwrap_or_default()
                    };

                    // Validate provider configuration before using
                    if provider.base_url.is_empty() || provider.base_url == "unknown" {
                        tracing::error!(
                            "Provider {} has invalid base_url: '{}'. Please configure a valid URL.",
                            provider.id, provider.base_url
                        );
                        continue;
                    }

                    if agent.model.is_empty() || agent.model == "unknown" {
                        tracing::error!(
                            "Agent {} has invalid model: '{}'. Please configure a valid model.",
                            agent.id, agent.model
                        );
                        continue;
                    }

                    let agent_config = serde_json::json!({
                        "provider_config": {
                            "name": provider.id,
                            "api_base": provider.base_url,
                            "api_key": api_key,
                            "model": agent.model,
                            "api_format": provider.api_format.to_python_format(),
                            "max_retries": max_retries,
                        },
                        "model": agent.model,
                        "temperature": agent.temperature,
                        "max_tokens": agent.max_tokens,
                        "system_prompt": agent.system_prompt,
                    });

                    agent_configs.insert(analysis_type.clone(), agent_config);
                    tracing::info!("Configured agent for {}: provider={}, model={}", analysis_type, provider.id, agent.model);
                } else {
                    tracing::warn!("Provider {} not found for agent {}", agent.provider_id, agent.id);
                }
            } else {
                tracing::warn!("Agent {} not found or disabled", agent_id);
            }
        } else {
            tracing::warn!("No task binding found for {:?}", task_type);
        }
    }

    tracing::info!("Built {} agent configs", agent_configs.len());

    // Fallback: if no agent configs found, use first enabled provider with defaults
    if agent_configs.is_empty() {
        tracing::warn!("No agent configurations found for analysis types, using fallback provider");

        if let Some(provider) = providers.iter().find(|p| p.enabled) {
            let api_key = if provider.api_key_ref.is_empty() {
                String::new()
            } else {
                keychain
                    .get_key(&provider.api_key_ref)
                    .unwrap_or_default()
            };

            let default_config = serde_json::json!({
                "provider_config": {
                    "name": provider.id,
                    "api_base": provider.base_url,
                    "api_key": api_key,
                    "model": provider.default_model,
                    "api_format": provider.api_format.to_python_format(),
                    "max_retries": max_retries,
                },
                "model": provider.default_model,
                "temperature": 0.7,
                "max_tokens": null,
                "system_prompt": null,
            });

            for analysis_type in &types {
                agent_configs.insert(analysis_type.clone(), default_config.clone());
            }
        } else {
            return Err("No enabled provider found".to_string());
        }
    }

    // Stage 2: Preparing context (10-20%)
    emit_progress(&app, &book_id, &chapter_id, "context", 15.0, "正在启动分析引擎...");

    let sidecar = get_sidecar();
    if !sidecar.is_running() {
        sidecar.start().map_err(|e| e.to_string())?;
    }

    // Ensure embeddings exist for this chapter (lazy embedding)
    emit_progress(&app, &book_id, &chapter_id, "context", 17.0, "正在检查/生成章节向量...");
    if let Err(e) = ensure_chapter_embeddings(
        &book_dir,
        &bid,
        &cid,
        &content,
        chapter.index_num,
    ) {
        tracing::warn!("Failed to ensure chapter embeddings: {}", e);
        // Continue without embeddings - analysis can still work
    }

    emit_progress(&app, &book_id, &chapter_id, "context", 20.0, "正在构建分析上下文...");

    // Try to use smart context with vector search for better entity relevance
    let vectors_path = crate::storage::paths::get_vectors_db_path(&book_id).ok();

    let analysis_context = match ContextBuilder::new(&book_db_path, bid.clone()) {
        Ok(builder) => {
            // Try to enable vector search if vectors.db exists
            let builder_with_vectors = if let Some(ref vp) = vectors_path {
                if vp.exists() {
                    match builder.with_vector_db(vp, DEFAULT_EMBEDDING_DIMENSIONS) {
                        Ok(b) => Some(b),
                        Err(e) => {
                            tracing::warn!("Failed to open vectors.db, using simple context: {}", e);
                            match ContextBuilder::new(&book_db_path, bid.clone()) {
                                Ok(simple_builder) => Some(simple_builder),
                                Err(e2) => {
                                    tracing::warn!(
                                        "Failed to create ContextBuilder fallback after vector init failure: {}",
                                        e2
                                    );
                                    None
                                }
                            }
                        }
                    }
                } else {
                    Some(builder)
                }
            } else {
                Some(builder)
            };

            if let Some(builder_with_vectors) = builder_with_vectors {
                // Use smart context if vector search is available
                match builder_with_vectors.build_smart_context(
                    &cid,
                    &content,
                    20, // max_characters
                    10, // max_settings
                    15, // max_events
                    12, // max_passages
                ) {
                    Ok(ctx) => Some(ctx),
                    Err(e) => {
                        tracing::warn!("Failed to build analysis context: {}", e);
                        None
                    }
                }
            } else {
                None
            }
        }
        Err(e) => {
            tracing::warn!("Failed to create ContextBuilder: {}", e);
            None
        }
    };

    let (known_characters, known_settings, known_events) = if let Some(ref ctx) = analysis_context {
        (
            serde_json::to_value(&ctx.known_characters).unwrap_or(serde_json::json!([])),
            serde_json::to_value(&ctx.known_settings).unwrap_or(serde_json::json!([])),
            serde_json::to_value(&ctx.known_events).unwrap_or(serde_json::json!([])),
        )
    } else {
        (serde_json::json!([]), serde_json::json!([]), serde_json::json!([]))
    };

    // Stage 3: AI Analysis (20-80%)
    emit_progress(&app, &book_id, &chapter_id, "analyzing", 25.0, "正在调用 AI 分析章节内容...");

    let params = serde_json::json!({
        "content": content,
        "chapter_index": chapter.index_num,
        "chapter_title": chapter.title,
        "book_title": book_meta.title,
        "agent_configs": agent_configs,
        "analysis_types": types,
        "known_characters": known_characters,
        "known_settings": known_settings,
        "known_events": known_events,
        "prompt_cards": prompt_cards,
    });

    emit_progress(&app, &book_id, &chapter_id, "analyzing", 30.0, "AI 正在分析章节...(这可能需要一段时间)");

    // Debug: Log the current logging state
    tracing::info!(
        "analyze_chapter: About to call sidecar with {} agent configs, is_api_logging_enabled={}",
        agent_configs.len(),
        crate::is_api_logging_enabled()
    );

    let result = sidecar
        .call("analyze_chapter", params)
        .await
        .map_err(|e| e.to_string())?;

    emit_progress(&app, &book_id, &chapter_id, "analyzing", 80.0, "AI 分析完成，正在处理结果...");

    let analysis_result: SidecarAnalysisResult =
        serde_json::from_value(result).map_err(|e| e.to_string())?;

    // Stage 4: Saving results (80-100%)
    emit_progress(&app, &book_id, &chapter_id, "saving", 85.0, "正在保存技巧卡片...");

    let now = Utc::now().to_rfc3339();
    let mut technique_cards = Vec::new();
    let mut knowledge_cards = Vec::new();

    for tech in analysis_result.techniques {
        let card = TechniqueCard {
            id: CardId::new(),
            chapter_id: cid.clone(),
            technique_type: tech.technique_type,
            title: tech.title,
            description: tech.description,
            mechanism: tech.mechanism,
            evidence: tech.evidence,
            tags: tech.tags,
            collected: false,
            created_at: now.clone(),
        };
        book_db
            .insert_technique_card(&card)
            .map_err(|e| e.to_string())?;
        // Also add to technique library
        if let Err(e) = book_db.add_technique_from_card(&card) {
            tracing::warn!("Failed to add technique to library: {}", e);
        }
        technique_cards.push(TechniqueCardInfo::from(card));
    }

    emit_progress(&app, &book_id, &chapter_id, "saving", 90.0, "正在保存人物信息...");

    for char_data in analysis_result.characters {
        let title = char_data.name.clone().or(char_data.title.clone()).unwrap_or_default();
        let content = serde_json::json!({
            "name": char_data.name,
            "description": char_data.description,
            "extra": char_data.extra,
        });
        let card = KnowledgeCard {
            id: CardId::new(),
            chapter_id: cid.clone(),
            knowledge_type: "character".to_string(),
            title,
            content,
            evidence: char_data.evidence,
            confidence: char_data.confidence.unwrap_or_else(|| "medium".to_string()),
            status: CardStatus::Pending.as_str().to_string(),
            created_at: now.clone(),
        };
        book_db
            .insert_knowledge_card(&card)
            .map_err(|e| e.to_string())?;
        knowledge_cards.push(KnowledgeCardInfo::from(card));
    }

    emit_progress(&app, &book_id, &chapter_id, "saving", 93.0, "正在保存设定信息...");

    for setting_data in analysis_result.settings {
        let title = setting_data.name.clone().or(setting_data.title.clone()).unwrap_or_default();
        let setting_type = setting_data.knowledge_type.clone().unwrap_or_else(|| "location".to_string());
        let content = serde_json::json!({
            "name": setting_data.name,
            "type": setting_type,
            "description": setting_data.description,
            "extra": setting_data.extra,
        });
        let card = KnowledgeCard {
            id: CardId::new(),
            chapter_id: cid.clone(),
            knowledge_type: "setting".to_string(),
            title,
            content,
            evidence: setting_data.evidence,
            confidence: setting_data.confidence.unwrap_or_else(|| "medium".to_string()),
            status: CardStatus::Pending.as_str().to_string(),
            created_at: now.clone(),
        };
        book_db
            .insert_knowledge_card(&card)
            .map_err(|e| e.to_string())?;
        knowledge_cards.push(KnowledgeCardInfo::from(card));
    }

    emit_progress(&app, &book_id, &chapter_id, "saving", 96.0, "正在保存事件信息...");

    for event_data in analysis_result.events {
        let title = event_data.title.clone().or(event_data.name.clone()).unwrap_or_default();
        // Extract importance from extra if present
        let importance = event_data.extra.get("importance")
            .and_then(|v| v.as_str())
            .unwrap_or("normal")
            .to_string();

        // Time-related fields returned by Python EventAgent live in `extra` because SidecarKnowledge
        // flattens unknown fields. Copy them to top-level content so acceptance paths can persist them.
        let time_marker = event_data
            .extra
            .get("time_marker")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let order_in_chapter = event_data
            .extra
            .get("order_in_chapter")
            .and_then(|v| v.as_i64().map(|n| n as i32).or_else(|| v.as_f64().map(|n| n as i32)))
            .unwrap_or(0);
        let is_flashback = event_data
            .extra
            .get("is_flashback")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let relative_time = event_data
            .extra
            .get("relative_time")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let content = serde_json::json!({
            "title": title,
            "description": event_data.description,
            "importance": importance,
            "time_marker": time_marker,
            "order_in_chapter": order_in_chapter,
            "is_flashback": is_flashback,
            "relative_time": relative_time,
            "extra": event_data.extra,
        });
        let card = KnowledgeCard {
            id: CardId::new(),
            chapter_id: cid.clone(),
            knowledge_type: "event".to_string(),
            title,
            content,
            evidence: event_data.evidence,
            confidence: event_data.confidence.unwrap_or_else(|| "medium".to_string()),
            status: CardStatus::Pending.as_str().to_string(),
            created_at: now.clone(),
        };
        book_db
            .insert_knowledge_card(&card)
            .map_err(|e| e.to_string())?;
        knowledge_cards.push(KnowledgeCardInfo::from(card));
    }

    emit_progress(&app, &book_id, &chapter_id, "saving", 98.0, "正在更新章节状态...");

    book_db
        .update_chapter_analyzed(
            &cid,
            true,
            technique_cards.len() as u32,
            knowledge_cards.len() as u32,
        )
        .map_err(|e| e.to_string())?;

    // Update chapter's Paragraph chunks with entity IDs for RAG
    // This links the chapter content to the entities found during analysis
    emit_progress(&app, &book_id, &chapter_id, "saving", 99.0, "正在更新向量索引...");
    if let Err(e) = update_chapter_chunks_with_entities(&book_dir, &cid, &book_db) {
        tracing::warn!("Failed to update chapter chunks with entity IDs: {}", e);
        // Don't fail the analysis, just log the warning
    }

    emit_progress(&app, &book_id, &chapter_id, "complete", 100.0, "分析完成！");

    Ok(AnalysisResult {
        chapter_id,
        technique_cards,
        knowledge_cards,
        success: true,
        error: None,
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CancelAnalysisResult {
    pub cancelled: bool,
    pub message: String,
}

/// Get the path to the cancel flag file (must match Python's _get_cancel_flag_path)
fn get_cancel_flag_path() -> std::path::PathBuf {
    #[cfg(target_os = "windows")]
    {
        let temp = std::env::var("TEMP")
            .or_else(|_| std::env::var("TMP"))
            .unwrap_or_else(|_| ".".to_string());
        std::path::PathBuf::from(temp).join("narrative_loom_cancel_flag")
    }
    #[cfg(not(target_os = "windows"))]
    {
        std::path::PathBuf::from("/tmp/narrative_loom_cancel_flag")
    }
}

#[tauri::command]
pub async fn cancel_analysis() -> Result<CancelAnalysisResult, String> {
    tracing::info!("cancel_analysis: Received cancel request");

    // Create the cancel flag file
    let flag_path = get_cancel_flag_path();
    tracing::info!("cancel_analysis: Creating cancel flag at {:?}", flag_path);

    match std::fs::File::create(&flag_path) {
        Ok(_) => {
            tracing::info!("cancel_analysis: Cancel flag file created successfully");
            Ok(CancelAnalysisResult {
                cancelled: true,
                message: format!("Cancel flag created at {:?}", flag_path),
            })
        }
        Err(e) => {
            tracing::error!("cancel_analysis: Failed to create cancel flag file: {}", e);
            Err(format!("Failed to create cancel flag: {}", e))
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SingleAgentResult {
    pub agent_type: String,
    pub success: bool,
    pub error: Option<String>,
    pub data: Vec<serde_json::Value>,
}

#[tauri::command]
pub async fn analyze_single_agent(
    app: AppHandle,
    book_id: String,
    chapter_id: String,
    agent_type: String,
) -> Result<SingleAgentResult, String> {
    tracing::debug!(
        "analyze_single_agent: Starting {} for chapter {}",
        agent_type,
        chapter_id
    );

    // Emit progress event
    emit_progress(
        &app,
        &book_id,
        &chapter_id,
        &format!("agent_{}", agent_type),
        0.0,
        &format!("正在运行 {} 分析...", agent_type),
    );

    let library = Library::open().map_err(|e| e.to_string())?;
    let bid = BookId::from_string(book_id.clone());
    let cid = ChapterId::from_string(chapter_id.clone());

    let book_dir = library.book_dir(&bid);
    let book_db_path = book_dir.join("book.db");

    if !book_db_path.exists() {
        return Err("Book database not found".to_string());
    }

    let book_db = BookDb::open(&book_db_path, bid.clone()).map_err(|e| e.to_string())?;

    // Clear old cards for this chapter and agent type before re-analysis
    match agent_type.as_str() {
        "technique" => {
            let deleted = book_db.delete_technique_cards_by_chapter(&cid).map_err(|e| e.to_string())?;
            if deleted > 0 {
                tracing::debug!("Cleared {} old technique cards for chapter {}", deleted, chapter_id);
            }
        }
        "character" | "setting" | "event" => {
            let deleted = book_db.delete_knowledge_cards_by_chapter_and_type(&cid, &agent_type).map_err(|e| e.to_string())?;
            if deleted > 0 {
                tracing::debug!("Cleared {} old {} cards for chapter {}", deleted, agent_type, chapter_id);
            }
        }
        "style" => {
            // Style observations are per-chapter, will be replaced on insert
            tracing::debug!("Style analysis will replace existing observation for chapter {}", chapter_id);
        }
        _ => {}
    }

    let chapter = book_db
        .get_chapter(&cid)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Chapter not found: {}", chapter_id))?;

    let content = book_db
        .get_chapter_content(&cid)
        .map_err(|e| e.to_string())?
        .unwrap_or_default();

    let book_meta = library
        .get_book(&bid)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Book metadata not found".to_string())?;

    // Load configurations
    let config_store = ConfigStore::new().map_err(|e| e.to_string())?;
    let keychain = KeychainService::new();

    let task_bindings = config_store.load_task_bindings().map_err(|e| e.to_string())?;
    let agents = config_store.load_agents().map_err(|e| e.to_string())?;
    let providers = config_store.load_providers().map_err(|e| e.to_string())?;
    let max_retries = config_store.get_request_retry_count().unwrap_or(3);
    let prompt_cards = config_store.load_prompt_cards().unwrap_or_default();

    tracing::info!(
        "analyze_single_agent: loaded {} task bindings, {} agents, {} providers",
        task_bindings.bindings.len(),
        agents.len(),
        providers.len()
    );

    // Map agent_type to TaskType
    let task_type = TaskType::from_analysis_type(agent_type.as_str())
        .ok_or_else(|| format!("Unknown agent type: {}", agent_type))?;

    // Build agent config
    let agent_config = if let Some(agent_id) = task_bindings.bindings.get(&task_type) {
        tracing::info!("analyze_single_agent: task {:?} bound to agent: {}", task_type, agent_id);
        if let Some(agent) = agents.iter().find(|a| &a.id == agent_id && a.enabled) {
            tracing::info!("analyze_single_agent: using agent {} with model={}", agent.id, agent.model);
            if let Some(provider) = providers.iter().find(|p| p.id == agent.provider_id) {
                // Validate provider configuration
                if provider.base_url.is_empty() || provider.base_url == "unknown" {
                    return Err(format!(
                        "Provider {} has invalid base_url: '{}'. Please configure a valid URL in settings.",
                        provider.id, provider.base_url
                    ));
                }

                if agent.model.is_empty() || agent.model == "unknown" {
                    return Err(format!(
                        "Agent {} has invalid model: '{}'. Please configure a valid model in settings.",
                        agent.id, agent.model
                    ));
                }

                let api_key = if provider.api_key_ref.is_empty() {
                    tracing::warn!("analyze_single_agent: provider {} has empty api_key_ref", provider.id);
                    String::new()
                } else {
                    match keychain.get_key(&provider.api_key_ref) {
                        Ok(key) => {
                            tracing::debug!("Got API key for {} (len={})", provider.id, key.len());
                            key
                        }
                        Err(e) => {
                            tracing::error!("analyze_single_agent: failed to get API key for {}: {:?}", provider.api_key_ref, e);
                            String::new()
                        }
                    }
                };

                serde_json::json!({
                    "provider_config": {
                        "name": provider.id,
                        "api_base": provider.base_url,
                        "api_key": api_key,
                        "model": agent.model,
                        "api_format": provider.api_format.to_python_format(),
                        "max_retries": max_retries,
                    },
                    "model": agent.model,
                    "temperature": agent.temperature,
                    "max_tokens": agent.max_tokens,
                    "system_prompt": agent.system_prompt,
                })
            } else {
                return Err(format!("Provider not found for agent {}", agent_id));
            }
        } else {
            return Err(format!("Agent {} not found or disabled", agent_id));
        }
    } else {
        return Err(format!("No task binding found for {}", agent_type));
    };

    // Ensure embeddings exist for this chapter (lazy embedding)
    if let Err(e) = ensure_chapter_embeddings(
        &book_dir,
        &bid,
        &cid,
        &content,
        chapter.index_num,
    ) {
        tracing::warn!("Failed to ensure chapter embeddings: {}", e);
        // Continue without embeddings - analysis can still work
    }

    // Build context (optional - for character/setting/event deduplication)
    // Use smart context with vector search for better entity relevance
    let vectors_path = crate::storage::paths::get_vectors_db_path(&book_id).ok();

    let analysis_context = match ContextBuilder::new(&book_db_path, bid.clone()) {
        Ok(builder) => {
            // Try to enable vector search if vectors.db exists
            let builder_with_vectors = if let Some(ref vp) = vectors_path {
                if vp.exists() {
                    match builder.with_vector_db(vp, DEFAULT_EMBEDDING_DIMENSIONS) {
                        Ok(b) => Some(b),
                        Err(e) => {
                            tracing::warn!("Failed to open vectors.db, using simple context: {}", e);
                            match ContextBuilder::new(&book_db_path, bid.clone()) {
                                Ok(simple_builder) => Some(simple_builder),
                                Err(e2) => {
                                    tracing::warn!(
                                        "Failed to create ContextBuilder fallback after vector init failure: {}",
                                        e2
                                    );
                                    None
                                }
                            }
                        }
                    }
                } else {
                    Some(builder)
                }
            } else {
                Some(builder)
            };

            if let Some(builder_with_vectors) = builder_with_vectors {
                match builder_with_vectors.build_smart_context(&cid, &content, 20, 10, 15, 12) {
                    Ok(ctx) => Some(ctx),
                    Err(e) => {
                        tracing::warn!("Failed to build analysis context: {}", e);
                        None
                    }
                }
            } else {
                None
            }
        }
        Err(e) => {
            tracing::warn!("Failed to create ContextBuilder: {}", e);
            None
        }
    };

    let (known_characters, known_settings, known_events) = if let Some(ref ctx) = analysis_context {
        (
            serde_json::to_value(&ctx.known_characters).unwrap_or(serde_json::json!([])),
            serde_json::to_value(&ctx.known_settings).unwrap_or(serde_json::json!([])),
            serde_json::to_value(&ctx.known_events).unwrap_or(serde_json::json!([])),
        )
    } else {
        (serde_json::json!([]), serde_json::json!([]), serde_json::json!([]))
    };

    // Start sidecar if needed
    let sidecar = get_sidecar();
    if !sidecar.is_running() {
        sidecar.start().map_err(|e| e.to_string())?;
    }

    emit_progress(
        &app,
        &book_id,
        &chapter_id,
        &format!("agent_{}", agent_type),
        30.0,
        &format!("AI 正在分析 {}...", agent_type),
    );

    // For style analysis, get the current profile for incremental refinement
    let current_profile = if agent_type == "style" {
        match book_db.get_style_profile() {
            Ok(Some(profile)) => {
                tracing::info!("Passing current style profile for incremental refinement");
                Some(profile.profile_json)
            }
            Ok(None) => {
                tracing::debug!("No existing style profile, starting fresh");
                None
            }
            Err(e) => {
                tracing::warn!("Failed to get style profile: {}", e);
                None
            }
        }
    } else {
        None
    };

    let params = serde_json::json!({
        "content": content,
        "chapter_index": chapter.index_num,
        "chapter_title": chapter.title,
        "book_title": book_meta.title,
        "agent_type": agent_type,
        "agent_config": agent_config,
        "known_characters": known_characters,
        "known_settings": known_settings,
        "known_events": known_events,
        "prompt_cards": prompt_cards,
        "current_profile": current_profile,
    });

    let result = sidecar
        .call("analyze_single_agent", params)
        .await
        .map_err(|e| e.to_string())?;

    let success = result.get("success").and_then(|v| v.as_bool()).unwrap_or(false);
    let error = result.get("error").and_then(|v| v.as_str()).map(String::from);

    // Handle different data structures from different agents:
    // - Style agent returns a single profile object (not an array)
    // - Other agents return an array
    let is_style = agent_type == "style";
    let style_data_obj = if is_style {
        result.get("data").cloned()
    } else {
        None
    };

    let data = if is_style {
        // Style agent: we don't need the vec representation for storage
        vec![]
    } else if let Some(arr) = result.get("data").and_then(|v| v.as_array()) {
        arr.clone()
    } else {
        vec![]
    };

    // Save results to database
    if success && (!data.is_empty() || is_style) {
        let now = Utc::now().to_rfc3339();

        match agent_type.as_str() {
            "technique" => {
                for item in &data {
                    if let Ok(tech) = serde_json::from_value::<SidecarTechnique>(item.clone()) {
                        let card = TechniqueCard {
                            id: CardId::new(),
                            chapter_id: cid.clone(),
                            technique_type: tech.technique_type,
                            title: tech.title,
                            description: tech.description,
                            mechanism: tech.mechanism,
                            evidence: tech.evidence,
                            tags: tech.tags,
                            collected: false,
                            created_at: now.clone(),
                        };
                        if let Err(e) = book_db.insert_technique_card(&card) {
                            tracing::error!("Failed to save technique card: {}", e);
                        } else {
                            // Also add to technique library
                            if let Err(e) = book_db.add_technique_from_card(&card) {
                                tracing::warn!("Failed to add technique to library: {}", e);
                            }
                        }
                    }
                }
            }
            "character" | "setting" | "event" => {
                // Load auto-accept threshold
                let auto_accept_threshold = config_store
                    .get_auto_accept_threshold()
                    .unwrap_or_else(|_| "off".to_string());

                let mut auto_accepted_count = 0;

                for item in &data {
                    if let Ok(knowledge) = serde_json::from_value::<SidecarKnowledge>(item.clone()) {
                        let title = knowledge.name.clone().or(knowledge.title.clone()).unwrap_or_default();

                        // Build content based on agent type
                        let content = match agent_type.as_str() {
                            "setting" => {
                                let setting_type = knowledge.knowledge_type.clone().unwrap_or_else(|| "location".to_string());
                                serde_json::json!({
                                    "name": knowledge.name,
                                    "type": setting_type,
                                    "description": knowledge.description,
                                    "extra": knowledge.extra,
                                })
                            }
                            "event" => {
                                let importance = knowledge.extra.get("importance")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("normal")
                                    .to_string();
                                serde_json::json!({
                                    "title": title,
                                    "description": knowledge.description,
                                    "importance": importance,
                                    "extra": knowledge.extra,
                                })
                            }
                            _ => {
                                serde_json::json!({
                                    "name": knowledge.name,
                                    "description": knowledge.description,
                                    "extra": knowledge.extra,
                                })
                            }
                        };

                        let confidence = knowledge.confidence.clone().unwrap_or_else(|| "medium".to_string());
                        let card = KnowledgeCard {
                            id: CardId::new(),
                            chapter_id: cid.clone(),
                            knowledge_type: agent_type.clone(),
                            title,
                            content,
                            evidence: knowledge.evidence,
                            confidence: confidence.clone(),
                            status: CardStatus::Pending.as_str().to_string(),
                            created_at: now.clone(),
                        };

                        if let Err(e) = book_db.insert_knowledge_card(&card) {
                            tracing::error!("Failed to save knowledge card: {}", e);
                            continue;
                        }

                        // Check if we should auto-accept this card
                        if should_auto_accept(&confidence, &auto_accept_threshold) {
                            if let Err(e) = auto_accept_card(&book_db, &card) {
                                tracing::warn!("Failed to auto-accept card: {}", e);
                            } else {
                                auto_accepted_count += 1;
                            }
                        }
                    }
                }

                if auto_accepted_count > 0 {
                    tracing::info!("Auto-accepted {} {} cards", auto_accepted_count, agent_type);
                }
            }
            "style" => {
                // Save style observation for this chapter
                if let Some(style_obj) = &style_data_obj {
                    if let Err(e) = book_db.insert_style_observation(&cid, style_obj) {
                        tracing::error!("Failed to save style observation: {}", e);
                    } else {
                        tracing::info!("Saved style observation for chapter {}", chapter_id);

                        // Aggregate all observations into style profile
                        if let Err(e) = aggregate_style_profile(&book_db) {
                            tracing::warn!("Failed to aggregate style profile: {}", e);
                        }
                    }
                }
            }
            _ => {}
        }

        tracing::debug!("Saved {} {} cards to database", data.len(), agent_type);

        // Update chapter's Paragraph chunks with entity IDs for RAG
        // Only update for entity-related agent types (character, setting, event)
        if matches!(agent_type.as_str(), "character" | "setting" | "event") {
            if let Err(e) = update_chapter_chunks_with_entities(&book_dir, &cid, &book_db) {
                tracing::warn!("Failed to update chapter chunks with entity IDs: {}", e);
            }
        }
    }

    emit_progress(
        &app,
        &book_id,
        &chapter_id,
        &format!("agent_{}", agent_type),
        100.0,
        &format!("{} 分析完成", agent_type),
    );

    tracing::info!(
        "analyze_single_agent: {} completed, success={}, items={}",
        agent_type,
        success,
        data.len()
    );

    Ok(SingleAgentResult {
        agent_type,
        success,
        error,
        data,
    })
}

/// Batch analyze multiple chapters
#[derive(Debug, Clone, Serialize)]
pub struct BatchAnalysisProgress {
    pub book_id: String,
    pub current_chapter_id: String,
    pub current_chapter_index: i32,
    pub total_chapters: i32,
    pub current_agent_type: String,
    pub status: String, // "running", "completed", "error", "cancelled"
    pub error: Option<String>,
}

#[tauri::command]
pub async fn batch_analyze_chapters(
    app: AppHandle,
    book_id: String,
    chapter_ids: Vec<String>,
    agent_types: Vec<String>,
) -> Result<serde_json::Value, String> {
    // Filter out unsupported agent types
    let supported_types = ["technique", "character", "setting", "event", "style"];
    let agent_types: Vec<String> = agent_types
        .into_iter()
        .filter(|t| supported_types.contains(&t.as_str()))
        .collect();

    if agent_types.is_empty() {
        return Err("No valid agent types specified".to_string());
    }

    let total_chapters = chapter_ids.len() as i32;
    let mut completed_chapters: Vec<String> = Vec::new();
    let mut failed_chapters: Vec<serde_json::Value> = Vec::new();

    tracing::info!(
        "batch_analyze_chapters: starting batch analysis of {} chapters with agents: {:?}",
        total_chapters,
        agent_types
    );

    for (chapter_index, chapter_id) in chapter_ids.iter().enumerate() {
        // Emit progress for chapter start
        let progress = BatchAnalysisProgress {
            book_id: book_id.clone(),
            current_chapter_id: chapter_id.clone(),
            current_chapter_index: chapter_index as i32,
            total_chapters,
            current_agent_type: "starting".to_string(),
            status: "running".to_string(),
            error: None,
        };
        if let Err(e) = app.emit("batch-analysis-progress", progress) {
            tracing::warn!("Failed to emit batch analysis progress: {}", e);
        }

        // Run each agent type for this chapter
        for agent_type in &agent_types {
            let agent_progress = BatchAnalysisProgress {
                book_id: book_id.clone(),
                current_chapter_id: chapter_id.clone(),
                current_chapter_index: chapter_index as i32,
                total_chapters,
                current_agent_type: agent_type.clone(),
                status: "running".to_string(),
                error: None,
            };
            if let Err(e) = app.emit("batch-analysis-progress", agent_progress) {
                tracing::warn!("Failed to emit batch analysis progress: {}", e);
            }

            // Call analyze_single_agent for this chapter and agent type
            match analyze_single_agent(
                app.clone(),
                book_id.clone(),
                chapter_id.clone(),
                agent_type.clone(),
            ).await {
                Ok(result) => {
                    if !result.success {
                        tracing::warn!(
                            "Agent {} failed for chapter {}: {:?}",
                            agent_type,
                            chapter_id,
                            result.error
                        );
                    }
                }
                Err(e) => {
                    tracing::error!(
                        "Error analyzing chapter {} with agent {}: {}",
                        chapter_id,
                        agent_type,
                        e
                    );
                    failed_chapters.push(serde_json::json!({
                        "id": chapter_id,
                        "agent_type": agent_type,
                        "error": e.to_string(),
                    }));
                }
            }
        }

        // Mark chapter as analyzed after all agents complete for this chapter
        let bid = BookId::from_string(book_id.clone());
        let cid = ChapterId::from_string(chapter_id.clone());
        let library = Library::open().map_err(|e| e.to_string())?;
        let book_dir = library.book_dir(&bid);
        let book_db_path = book_dir.join("book.db");

        if book_db_path.exists() {
            if let Ok(book_db) = BookDb::open(&book_db_path, bid) {
                // Get counts from the inbox for this chapter
                let technique_count = book_db.list_collected_technique_cards()
                    .map(|cards| cards.iter().filter(|c| c.chapter_id.as_str() == chapter_id).count() as u32)
                    .unwrap_or(0);
                let knowledge_count = book_db.list_pending_knowledge_cards()
                    .map(|cards| cards.iter().filter(|c| c.chapter_id.as_str() == chapter_id).count() as u32)
                    .unwrap_or(0);

                if let Err(e) = book_db.update_chapter_analyzed(&cid, true, technique_count, knowledge_count) {
                    tracing::warn!("Failed to mark chapter {} as analyzed: {}", chapter_id, e);
                }
            }
        }

        completed_chapters.push(chapter_id.clone());
    }

    // Emit final progress
    let final_progress = BatchAnalysisProgress {
        book_id: book_id.clone(),
        current_chapter_id: String::new(),
        current_chapter_index: total_chapters,
        total_chapters,
        current_agent_type: String::new(),
        status: "completed".to_string(),
        error: None,
    };
    if let Err(e) = app.emit("batch-analysis-progress", final_progress) {
        tracing::warn!("Failed to emit batch analysis progress: {}", e);
    }

    tracing::info!(
        "batch_analyze_chapters: completed. {} succeeded, {} failed",
        completed_chapters.len(),
        failed_chapters.len()
    );

    Ok(serde_json::json!({
        "completed_chapters": completed_chapters,
        "failed_chapters": failed_chapters,
        "total": total_chapters,
    }))
}

/// Delete a single technique card
#[tauri::command]
pub async fn delete_technique_card(book_id: String, card_id: String) -> Result<bool, String> {
    let library = Library::open().map_err(|e| e.to_string())?;
    let bid = BookId::from_string(book_id);
    let cid = CardId::from_string(card_id);

    let book_dir = library.book_dir(&bid);
    let book_db_path = book_dir.join("book.db");

    if !book_db_path.exists() {
        return Err("Book database not found".to_string());
    }

    let book_db = BookDb::open(&book_db_path, bid).map_err(|e| e.to_string())?;
    book_db.delete_technique_card(&cid).map_err(|e| e.to_string())
}

/// Delete all technique cards for a chapter
#[tauri::command]
pub async fn clear_chapter_technique_cards(
    book_id: String,
    chapter_id: String,
) -> Result<u32, String> {
    let library = Library::open().map_err(|e| e.to_string())?;
    let bid = BookId::from_string(book_id);
    let cid = ChapterId::from_string(chapter_id);

    let book_dir = library.book_dir(&bid);
    let book_db_path = book_dir.join("book.db");

    if !book_db_path.exists() {
        return Err("Book database not found".to_string());
    }

    let book_db = BookDb::open(&book_db_path, bid).map_err(|e| e.to_string())?;
    book_db.delete_technique_cards_by_chapter(&cid).map_err(|e| e.to_string())
}

/// Delete a single knowledge card
#[tauri::command]
pub async fn delete_knowledge_card(book_id: String, card_id: String) -> Result<bool, String> {
    let library = Library::open().map_err(|e| e.to_string())?;
    let bid = BookId::from_string(book_id);
    let cid = CardId::from_string(card_id);

    let book_dir = library.book_dir(&bid);
    let book_db_path = book_dir.join("book.db");

    if !book_db_path.exists() {
        return Err("Book database not found".to_string());
    }

    let book_db = BookDb::open(&book_db_path, bid).map_err(|e| e.to_string())?;
    book_db.delete_knowledge_card(&cid).map_err(|e| e.to_string())
}

/// Delete all knowledge cards for a chapter
#[tauri::command]
pub async fn clear_chapter_knowledge_cards(
    book_id: String,
    chapter_id: String,
) -> Result<u32, String> {
    let library = Library::open().map_err(|e| e.to_string())?;
    let bid = BookId::from_string(book_id);
    let cid = ChapterId::from_string(chapter_id);

    let book_dir = library.book_dir(&bid);
    let book_db_path = book_dir.join("book.db");

    if !book_db_path.exists() {
        return Err("Book database not found".to_string());
    }

    let book_db = BookDb::open(&book_db_path, bid).map_err(|e| e.to_string())?;
    book_db.delete_knowledge_cards_by_chapter(&cid).map_err(|e| e.to_string())
}

/// Delete all cards (both technique and knowledge) for a chapter
#[tauri::command]
pub async fn clear_chapter_all_cards(
    book_id: String,
    chapter_id: String,
) -> Result<serde_json::Value, String> {
    let library = Library::open().map_err(|e| e.to_string())?;
    let bid = BookId::from_string(book_id);
    let cid = ChapterId::from_string(chapter_id);

    let book_dir = library.book_dir(&bid);
    let book_db_path = book_dir.join("book.db");

    if !book_db_path.exists() {
        return Err("Book database not found".to_string());
    }

    let book_db = BookDb::open(&book_db_path, bid).map_err(|e| e.to_string())?;
    let technique_count = book_db.delete_technique_cards_by_chapter(&cid).map_err(|e| e.to_string())?;
    let knowledge_count = book_db.delete_knowledge_cards_by_chapter(&cid).map_err(|e| e.to_string())?;

    Ok(serde_json::json!({
        "technique_cards_deleted": technique_count,
        "knowledge_cards_deleted": knowledge_count,
    }))
}

/// Delete all technique cards for a book
#[tauri::command]
pub async fn clear_all_technique_cards(book_id: String) -> Result<u32, String> {
    let library = Library::open().map_err(|e| e.to_string())?;
    let bid = BookId::from_string(book_id);

    let book_dir = library.book_dir(&bid);
    let book_db_path = book_dir.join("book.db");

    if !book_db_path.exists() {
        return Err("Book database not found".to_string());
    }

    let book_db = BookDb::open(&book_db_path, bid).map_err(|e| e.to_string())?;
    book_db.delete_all_technique_cards().map_err(|e| e.to_string())
}

/// Delete all knowledge cards for a book
#[tauri::command]
pub async fn clear_all_knowledge_cards(book_id: String) -> Result<u32, String> {
    let library = Library::open().map_err(|e| e.to_string())?;
    let bid = BookId::from_string(book_id);

    let book_dir = library.book_dir(&bid);
    let book_db_path = book_dir.join("book.db");

    if !book_db_path.exists() {
        return Err("Book database not found".to_string());
    }

    let book_db = BookDb::open(&book_db_path, bid).map_err(|e| e.to_string())?;
    book_db.delete_all_knowledge_cards().map_err(|e| e.to_string())
}

// ============================================================================
// Style Profile Aggregation (Intelligent Refinement)
// ============================================================================

/// Helper struct to track enum field values with confidence
#[derive(Debug)]
struct EnumFieldStats {
    counts: std::collections::HashMap<String, u32>,
    total: u32,
}

impl EnumFieldStats {
    fn new() -> Self {
        Self {
            counts: std::collections::HashMap::new(),
            total: 0,
        }
    }

    fn add(&mut self, value: &str) {
        *self.counts.entry(value.to_string()).or_insert(0) += 1;
        self.total += 1;
    }

    /// Get the dominant value (mode)
    fn get_value(&self) -> Option<String> {
        self.counts.iter()
            .max_by_key(|(_, count)| *count)
            .map(|(v, _)| v.clone())
    }

    /// Get confidence (0.0 - 1.0)
    fn get_confidence(&self) -> f64 {
        if self.total == 0 {
            return 0.0;
        }
        let max_count = self.counts.values().max().copied().unwrap_or(0);
        (max_count as f64) / (self.total as f64)
    }
}

/// Aggregate all style observations into a single style profile with intelligent refinement
/// - Enum fields: Track confidence based on consistency across chapters
/// - List fields: Merge and deduplicate, sort by frequency
/// - Description fields: Keep the most recent (latest chapter analyzed)
/// - Add analysis_metadata with coverage and confidence scores
fn aggregate_style_profile(book_db: &BookDb) -> Result<(), String> {
    let observations = book_db.list_style_observations().map_err(|e| e.to_string())?;

    if observations.is_empty() {
        return Ok(());
    }

    let total_chapters = observations.len() as u32;

    // Initialize aggregated profile structure (8 core dimensions)
    let mut aggregated = serde_json::json!({
        "vocabulary": {},
        "sentence_structure": {},
        "narrative_voice": {},
        "dialogue_style": {},
        "description_style": {},
        "pacing": {},
        "emotional_expression": {},
        "thematic_elements": {},
        "key_observations": [],
        "notable_techniques": [],
        "sample_passages": [],
        "analysis_metadata": {}
    });

    // Track enum field frequencies for mode calculation with confidence
    let mut enum_stats: std::collections::HashMap<String, EnumFieldStats> =
        std::collections::HashMap::new();

    // Track all list items with frequency counts
    let mut list_counts: std::collections::HashMap<String, std::collections::HashMap<String, u32>> =
        std::collections::HashMap::new();

    // Track latest descriptions for each field
    let mut latest_descriptions: std::collections::HashMap<String, String> =
        std::collections::HashMap::new();

    // Track sample passages with chapter info
    let mut all_sample_passages: Vec<serde_json::Value> = Vec::new();

    // Enum fields to track (8 core dimensions)
    let enum_fields = vec![
        // vocabulary
        ("vocabulary.formality_level", "vocabulary", "formality_level"),
        // sentence_structure
        ("sentence_structure.typical_length", "sentence_structure", "typical_length"),
        ("sentence_structure.complexity_level", "sentence_structure", "complexity_level"),
        // narrative_voice
        ("narrative_voice.perspective", "narrative_voice", "perspective"),
        ("narrative_voice.tense", "narrative_voice", "tense"),
        ("narrative_voice.intrusion_level", "narrative_voice", "intrusion_level"),
        ("narrative_voice.reliability", "narrative_voice", "reliability"),
        // dialogue_style
        ("dialogue_style.dialogue_proportion", "dialogue_style", "dialogue_proportion"),
        // description_style
        ("description_style.detail_level", "description_style", "detail_level"),
        // pacing
        ("pacing.overall_tempo", "pacing", "overall_tempo"),
        // emotional_expression (upgraded from emotional_tone)
        ("emotional_expression.show_vs_tell", "emotional_expression", "show_vs_tell"),
        ("emotional_expression.emotional_restraint", "emotional_expression", "emotional_restraint"),
    ];

    // Description fields to track
    let desc_fields = vec![
        // vocabulary
        ("vocabulary.vocabulary_tendencies", "vocabulary", "vocabulary_tendencies"),
        ("vocabulary.word_choice_patterns", "vocabulary", "word_choice_patterns"),
        ("vocabulary.period_markers", "vocabulary", "period_markers"),
        // sentence_structure
        ("sentence_structure.rhythm_patterns", "sentence_structure", "rhythm_patterns"),
        ("sentence_structure.punctuation_style", "sentence_structure", "punctuation_style"),
        ("sentence_structure.paragraph_structure", "sentence_structure", "paragraph_structure"),
        // narrative_voice
        ("narrative_voice.narrator_characteristics", "narrative_voice", "narrator_characteristics"),
        // dialogue_style
        ("dialogue_style.dialogue_tags", "dialogue_style", "dialogue_tags"),
        ("dialogue_style.speech_rhythm", "dialogue_style", "speech_rhythm"),
        ("dialogue_style.character_voice_differentiation", "dialogue_style", "character_voice_differentiation"),
        ("dialogue_style.subtext_usage", "dialogue_style", "subtext_usage"),
        // description_style
        ("description_style.metaphor_usage", "description_style", "metaphor_usage"),
        ("description_style.imagery_patterns", "description_style", "imagery_patterns"),
        // pacing
        ("pacing.scene_transitions", "pacing", "scene_transitions"),
        ("pacing.tension_building", "pacing", "tension_building"),
        ("pacing.action_vs_reflection_balance", "pacing", "action_vs_reflection_balance"),
        // emotional_expression
        ("emotional_expression.dominant_mood", "emotional_expression", "dominant_mood"),
        ("emotional_expression.mood_range", "emotional_expression", "mood_range"),
        ("emotional_expression.atmosphere_techniques", "emotional_expression", "atmosphere_techniques"),
        // thematic_elements
        ("thematic_elements.symbolic_patterns", "thematic_elements", "symbolic_patterns"),
    ];

    // Process each observation
    for (obs_idx, obs) in observations.iter().enumerate() {
        let obj = &obs.observation_json;
        let chapter_num = obs_idx + 1;

        // Count enum field values
        for (key, section, field) in &enum_fields {
            if let Some(value) = obj.get(*section).and_then(|s| s.get(*field)).and_then(|v| v.as_str()) {
                enum_stats.entry(key.to_string()).or_insert_with(EnumFieldStats::new).add(value);
            }
        }

        // Track latest descriptions
        for (key, section, field) in &desc_fields {
            if let Some(value) = obj.get(*section).and_then(|s| s.get(*field)).and_then(|v| v.as_str()) {
                if !value.is_empty() {
                    latest_descriptions.insert(key.to_string(), value.to_string());
                }
            }
        }

        // Collect list items with frequency tracking
        for list_name in &["key_observations", "notable_techniques"] {
            if let Some(arr) = obj.get(*list_name).and_then(|v| v.as_array()) {
                let counts = list_counts.entry(list_name.to_string()).or_insert_with(std::collections::HashMap::new);
                for item in arr {
                    if let Some(s) = item.as_str() {
                        *counts.entry(s.to_string()).or_insert(0) += 1;
                    }
                }
            }
        }

        // Handle nested lists
        let nested_lists = vec![
            ("vocabulary.distinctive_vocabulary", "vocabulary", "distinctive_vocabulary"),
            ("description_style.sensory_preferences", "description_style", "sensory_preferences"),
            ("description_style.metaphor_examples", "description_style", "metaphor_examples"),
            ("thematic_elements.recurring_motifs", "thematic_elements", "recurring_motifs"),
            ("thematic_elements.thematic_concerns", "thematic_elements", "thematic_concerns"),
        ];

        for (key, section, field) in &nested_lists {
            if let Some(arr) = obj.get(*section).and_then(|s| s.get(*field)).and_then(|v| v.as_array()) {
                let counts = list_counts.entry(key.to_string()).or_insert_with(std::collections::HashMap::new);
                for item in arr {
                    if let Some(s) = item.as_str() {
                        *counts.entry(s.to_string()).or_insert(0) += 1;
                    }
                }
            }
        }

        // Collect sample passages with chapter info
        if let Some(arr) = obj.get("sample_passages").and_then(|v| v.as_array()) {
            for item in arr {
                let mut passage = item.clone();
                if let Some(obj) = passage.as_object_mut() {
                    obj.insert("chapter".to_string(), serde_json::json!(chapter_num));
                }
                all_sample_passages.push(passage);
            }
        }

        // Handle backward compatibility: emotional_tone -> emotional_expression
        if obj.get("emotional_expression").is_none() {
            if let Some(old_tone) = obj.get("emotional_tone") {
                if let Some(restraint) = old_tone.get("emotional_restraint").and_then(|v| v.as_str()) {
                    enum_stats.entry("emotional_expression.emotional_restraint".to_string())
                        .or_insert_with(EnumFieldStats::new).add(restraint);
                }
                if let Some(mood) = old_tone.get("dominant_mood").and_then(|v| v.as_str()) {
                    latest_descriptions.insert("emotional_expression.dominant_mood".to_string(), mood.to_string());
                }
                if let Some(range) = old_tone.get("mood_range").and_then(|v| v.as_str()) {
                    latest_descriptions.insert("emotional_expression.mood_range".to_string(), range.to_string());
                }
                if let Some(tech) = old_tone.get("atmosphere_techniques").and_then(|v| v.as_str()) {
                    latest_descriptions.insert("emotional_expression.atmosphere_techniques".to_string(), tech.to_string());
                }
            }
        }
    }

    // Track confidence for metadata
    let mut total_confidence: f64 = 0.0;
    let mut confidence_count: u32 = 0;

    // Apply enum fields
    for (key, section, field) in &enum_fields {
        if let Some(stats) = enum_stats.get(*key) {
            if let Some(value) = stats.get_value() {
                if let Some(section_obj) = aggregated.get_mut(*section) {
                    section_obj[*field] = serde_json::Value::String(value);
                }
                total_confidence += stats.get_confidence();
                confidence_count += 1;
            }
        }
    }

    // Apply latest descriptions
    for (key, section, field) in &desc_fields {
        if let Some(value) = latest_descriptions.get(*key) {
            if let Some(section_obj) = aggregated.get_mut(*section) {
                section_obj[*field] = serde_json::Value::String(value.clone());
            }
        }
    }

    // Helper: sort list items by frequency (most common first)
    fn sorted_by_frequency(counts: &std::collections::HashMap<String, u32>, limit: usize) -> Vec<String> {
        let mut items: Vec<_> = counts.iter().collect();
        items.sort_by(|a, b| b.1.cmp(a.1));
        items.into_iter().take(limit).map(|(k, _)| k.clone()).collect()
    }

    // Apply top-level lists
    if let Some(counts) = list_counts.get("key_observations") {
        aggregated["key_observations"] = serde_json::Value::Array(
            sorted_by_frequency(counts, 20).into_iter().map(serde_json::Value::String).collect()
        );
    }
    if let Some(counts) = list_counts.get("notable_techniques") {
        aggregated["notable_techniques"] = serde_json::Value::Array(
            sorted_by_frequency(counts, 15).into_iter().map(serde_json::Value::String).collect()
        );
    }

    // Apply nested lists
    let nested_list_configs = vec![
        ("vocabulary.distinctive_vocabulary", "vocabulary", "distinctive_vocabulary", 30),
        ("description_style.sensory_preferences", "description_style", "sensory_preferences", 10),
        ("description_style.metaphor_examples", "description_style", "metaphor_examples", 15),
        ("thematic_elements.recurring_motifs", "thematic_elements", "recurring_motifs", 15),
        ("thematic_elements.thematic_concerns", "thematic_elements", "thematic_concerns", 15),
    ];

    for (key, section, field, limit) in nested_list_configs {
        if let Some(counts) = list_counts.get(key) {
            if let Some(section_obj) = aggregated.get_mut(section) {
                section_obj[field] = serde_json::Value::Array(
                    sorted_by_frequency(counts, limit).into_iter().map(serde_json::Value::String).collect()
                );
            }
        }
    }

    // Apply sample passages (limit to 10)
    aggregated["sample_passages"] = serde_json::Value::Array(
        all_sample_passages.into_iter().take(10).collect()
    );

    // Calculate overall confidence score
    let overall_confidence = if confidence_count > 0 {
        (total_confidence / confidence_count as f64 * 100.0).round() / 100.0
    } else {
        0.0
    };

    // Build analysis metadata
    let now = chrono::Utc::now().to_rfc3339();
    aggregated["analysis_metadata"] = serde_json::json!({
        "chapters_analyzed": total_chapters,
        "confidence_score": overall_confidence,
        "last_updated": now
    });

    // Save aggregated profile
    book_db.upsert_style_profile(&aggregated, total_chapters).map_err(|e| e.to_string())?;

    tracing::info!(
        "Aggregated style profile from {} chapters (confidence: {:.0}%)",
        total_chapters,
        overall_confidence * 100.0
    );

    Ok(())
}

// ============================================================================
// Style Profile Commands
// ============================================================================

/// Get the aggregated style profile for a book
#[tauri::command]
pub async fn get_style_profile(book_id: String) -> Result<Option<serde_json::Value>, String> {
    let library = Library::open().map_err(|e| e.to_string())?;
    let bid = BookId::from_string(book_id);

    let book_dir = library.book_dir(&bid);
    let book_db_path = book_dir.join("book.db");

    if !book_db_path.exists() {
        return Ok(None);
    }

    let book_db = BookDb::open(&book_db_path, bid).map_err(|e| e.to_string())?;
    let profile = book_db.get_style_profile().map_err(|e| e.to_string())?;

    Ok(profile.map(|p| serde_json::json!({
        "id": p.id,
        "version": p.version,
        "profile": p.profile_json,
        "analyzed_chapters": p.analyzed_chapters,
        "created_at": p.created_at,
        "updated_at": p.updated_at,
    })))
}

/// Clear the style profile and all observations for a book
#[tauri::command]
pub async fn clear_style_profile(book_id: String) -> Result<serde_json::Value, String> {
    let library = Library::open().map_err(|e| e.to_string())?;
    let bid = BookId::from_string(book_id);

    let book_dir = library.book_dir(&bid);
    let book_db_path = book_dir.join("book.db");

    if !book_db_path.exists() {
        return Err("Book database not found".to_string());
    }

    let book_db = BookDb::open(&book_db_path, bid).map_err(|e| e.to_string())?;
    let profile_deleted = book_db.delete_style_profile().map_err(|e| e.to_string())?;
    let observations_deleted = book_db.delete_style_observations().map_err(|e| e.to_string())?;

    Ok(serde_json::json!({
        "profile_deleted": profile_deleted,
        "observations_deleted": observations_deleted,
    }))
}

/// Get style observation for a specific chapter
#[tauri::command]
pub async fn get_style_observation(
    book_id: String,
    chapter_id: String,
) -> Result<Option<serde_json::Value>, String> {
    let library = Library::open().map_err(|e| e.to_string())?;
    let bid = BookId::from_string(book_id);
    let cid = ChapterId::from_string(chapter_id);

    let book_dir = library.book_dir(&bid);
    let book_db_path = book_dir.join("book.db");

    if !book_db_path.exists() {
        return Ok(None);
    }

    let book_db = BookDb::open(&book_db_path, bid).map_err(|e| e.to_string())?;
    let observation = book_db.get_style_observation(&cid).map_err(|e| e.to_string())?;

    Ok(observation.map(|o| serde_json::json!({
        "id": o.id,
        "chapter_id": o.chapter_id.to_string(),
        "observation": o.observation_json,
        "created_at": o.created_at,
    })))
}
