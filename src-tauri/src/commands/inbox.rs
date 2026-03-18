use crate::commands::dedup::{check_entity_dedup, is_combo_name, KnownEntity};
use crate::core::card::CardStatus;
use crate::core::ids::{BookId, CardId, ChapterId, EntityId};
use crate::storage::book_db::{BookDb, Character, Setting, Event, KnowledgeCard};
use crate::storage::library::Library;
use crate::storage::structured_description::{
    CharacterStructuredDescription, SettingStructuredDescription,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};

use super::common::open_book_db;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InboxItem {
    pub id: String,
    pub chapter_id: String,
    pub knowledge_type: String,
    pub title: String,
    pub content: serde_json::Value,
    pub evidence: Vec<String>,
    pub confidence: String,
    pub created_at: String,
}

impl From<KnowledgeCard> for InboxItem {
    fn from(card: KnowledgeCard) -> Self {
        Self {
            id: card.id.to_string(),
            chapter_id: card.chapter_id.to_string(),
            knowledge_type: card.knowledge_type,
            title: card.title,
            content: card.content,
            evidence: card.evidence,
            confidence: card.confidence,
            created_at: card.created_at,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InboxStats {
    pub total: u32,
    pub characters: u32,
    pub settings: u32,
    pub events: u32,
}

#[tauri::command]
pub async fn get_inbox(
    book_id: String,
    filter_type: Option<String>,
) -> Result<Vec<InboxItem>, String> {
    let book_db = open_book_db(&book_id)?;
    let cards = book_db
        .list_pending_knowledge_cards()
        .map_err(|e| e.to_string())?;

    let items: Vec<InboxItem> = cards
        .into_iter()
        .filter(|card| {
            if let Some(ref ft) = filter_type {
                &card.knowledge_type == ft
            } else {
                true
            }
        })
        .map(InboxItem::from)
        .collect();

    Ok(items)
}

#[tauri::command]
pub async fn get_inbox_stats(book_id: String) -> Result<InboxStats, String> {
    let book_db = open_book_db(&book_id)?;
    let cards = book_db
        .list_pending_knowledge_cards()
        .map_err(|e| e.to_string())?;

    let mut stats = InboxStats {
        total: cards.len() as u32,
        characters: 0,
        settings: 0,
        events: 0,
    };

    for card in cards {
        match card.knowledge_type.as_str() {
            "character" => stats.characters += 1,
            "setting" => stats.settings += 1,
            "event" => stats.events += 1,
            _ => {}
        }
    }

    Ok(stats)
}

/// Parse character structured description from card content
fn parse_character_structured_description(
    content: &serde_json::Value,
    chapter_id: &str,
    chapter_index: u32,
) -> Option<CharacterStructuredDescription> {
    let desc_structured = content.get("description_structured")?;

    let mut result = CharacterStructuredDescription::new();

    if let Some(text) = desc_structured.get("appearance").and_then(|v| v.as_str()) {
        if !text.is_empty() {
            result.add_appearance(text.to_string(), chapter_id.to_string(), chapter_index, None);
        }
    }
    if let Some(text) = desc_structured.get("personality").and_then(|v| v.as_str()) {
        if !text.is_empty() {
            result.add_personality(text.to_string(), chapter_id.to_string(), chapter_index, None);
        }
    }
    if let Some(text) = desc_structured.get("background").and_then(|v| v.as_str()) {
        if !text.is_empty() {
            result.add_background(text.to_string(), chapter_id.to_string(), chapter_index, None);
        }
    }
    if let Some(text) = desc_structured.get("abilities").and_then(|v| v.as_str()) {
        if !text.is_empty() {
            result.add_abilities(text.to_string(), chapter_id.to_string(), chapter_index, None);
        }
    }
    if let Some(text) = desc_structured.get("goals").and_then(|v| v.as_str()) {
        if !text.is_empty() {
            result.add_goals(text.to_string(), chapter_id.to_string(), chapter_index, None);
        }
    }
    if let Some(text) = desc_structured.get("status").and_then(|v| v.as_str()) {
        if !text.is_empty() {
            result.add_status(text.to_string(), chapter_id.to_string(), chapter_index, None);
        }
    }

    Some(result)
}

/// Parse setting structured description from card content
fn parse_setting_structured_description(
    content: &serde_json::Value,
    chapter_id: &str,
    chapter_index: u32,
) -> Option<SettingStructuredDescription> {
    let desc_structured = content.get("description_structured")?;

    let mut result = SettingStructuredDescription::new();

    if let Some(text) = desc_structured.get("physical").and_then(|v| v.as_str()) {
        if !text.is_empty() {
            result.add_physical(text.to_string(), chapter_id.to_string(), chapter_index, None);
        }
    }
    if let Some(text) = desc_structured.get("atmosphere").and_then(|v| v.as_str()) {
        if !text.is_empty() {
            result.add_atmosphere(text.to_string(), chapter_id.to_string(), chapter_index, None);
        }
    }
    if let Some(text) = desc_structured.get("history").and_then(|v| v.as_str()) {
        if !text.is_empty() {
            result.add_history(text.to_string(), chapter_id.to_string(), chapter_index, None);
        }
    }
    if let Some(text) = desc_structured.get("function").and_then(|v| v.as_str()) {
        if !text.is_empty() {
            result.add_function(text.to_string(), chapter_id.to_string(), chapter_index, None);
        }
    }
    if let Some(text) = desc_structured.get("rules").and_then(|v| v.as_str()) {
        if !text.is_empty() {
            result.add_rules(text.to_string(), chapter_id.to_string(), chapter_index, None);
        }
    }
    if let Some(text) = desc_structured.get("inhabitants").and_then(|v| v.as_str()) {
        if !text.is_empty() {
            result.add_inhabitants(text.to_string(), chapter_id.to_string(), chapter_index, None);
        }
    }
    if let Some(text) = desc_structured.get("status").and_then(|v| v.as_str()) {
        if !text.is_empty() {
            result.add_status(text.to_string(), chapter_id.to_string(), chapter_index, None);
        }
    }

    Some(result)
}

/// Merge new structured description into existing one for characters
fn merge_character_structured_description(
    existing: Option<CharacterStructuredDescription>,
    new_desc: Option<CharacterStructuredDescription>,
) -> Option<CharacterStructuredDescription> {
    match (existing, new_desc) {
        (Some(mut existing), Some(new_desc)) => {
            existing.merge(&new_desc);
            Some(existing)
        }
        (None, Some(new_desc)) => Some(new_desc),
        (Some(existing), None) => Some(existing),
        (None, None) => None,
    }
}

/// Merge new structured description into existing one for settings
fn merge_setting_structured_description(
    existing: Option<SettingStructuredDescription>,
    new_desc: Option<SettingStructuredDescription>,
) -> Option<SettingStructuredDescription> {
    match (existing, new_desc) {
        (Some(mut existing), Some(new_desc)) => {
            existing.merge(&new_desc);
            Some(existing)
        }
        (None, Some(new_desc)) => Some(new_desc),
        (Some(existing), None) => Some(existing),
        (None, None) => None,
    }
}

/// Get chapter index from chapter_id
fn get_chapter_index(book_db: &BookDb, chapter_id: &ChapterId) -> u32 {
    book_db.get_chapter(chapter_id)
        .ok()
        .flatten()
        .map(|c| c.index_num as u32)
        .unwrap_or(0)
}

#[tauri::command]
pub async fn accept_card(book_id: String, card_id: String) -> Result<bool, String> {
    let cid = CardId::from_string(card_id.clone());
    let book_db = open_book_db(&book_id)?;

    let cards = book_db
        .list_pending_knowledge_cards()
        .map_err(|e| e.to_string())?;

    let card = cards
        .into_iter()
        .find(|c| c.id.to_string() == card_id)
        .ok_or_else(|| format!("Card not found: {}", card_id))?;

    let now = Utc::now().to_rfc3339();

    match card.knowledge_type.as_str() {
        "character" => {
            let name = card.content.get("name")
                .and_then(|v| v.as_str())
                .unwrap_or(&card.title)
                .to_string();

            // Check for combo name (e.g., "张三 & 李四")
            if is_combo_name(&name) {
                tracing::info!("Skipping combo name: {}", name);
                book_db
                    .update_knowledge_card_status(&cid, CardStatus::Rejected)
                    .map_err(|e| e.to_string())?;
                return Ok(false);
            }

            // Build list of known entities for dedup
            let existing_chars = book_db.list_characters().map_err(|e| e.to_string())?;
            let known_entities: Vec<KnownEntity> = existing_chars.iter().map(|c| KnownEntity {
                id: c.id.to_string(),
                name: c.name.clone(),
                aliases: c.aliases.clone(),
            }).collect();

            // Check for dedup
            let dedup_result = check_entity_dedup(&name, &known_entities, 0.8);

            if dedup_result.should_skip {
                tracing::info!("Dedup: skipping card '{}': {}", name, dedup_result.reason.as_deref().unwrap_or("unknown"));
                book_db
                    .update_knowledge_card_status(&cid, CardStatus::Rejected)
                    .map_err(|e| e.to_string())?;
                return Ok(false);
            }

            // If merge_with is set, merge with existing entity
            if let Some(merge_id) = dedup_result.merge_with {
                if let Some(existing) = existing_chars.iter().find(|c| c.id.to_string() == merge_id) {
                    tracing::info!("Dedup: merging '{}' with existing '{}': {}", name, existing.name, dedup_result.reason.as_deref().unwrap_or(""));

                    // Merge with existing character
                    let new_description = card.content.get("description")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                    let description = new_description.or(existing.description.clone());

                    let mut aliases = existing.aliases.clone();
                    // Add the new name as an alias if different
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

                    let mut evidence = existing.evidence.clone();
                    for e in &card.evidence {
                        if !evidence.contains(e) {
                            evidence.push(e.clone());
                        }
                    }

                    // Merge structured description
                    let chapter_index = get_chapter_index(&book_db, &card.chapter_id);
                    let new_structured = parse_character_structured_description(
                        &card.content,
                        &card.chapter_id.to_string(),
                        chapter_index,
                    );
                    let merged_structured = merge_character_structured_description(
                        existing.description_structured.clone(),
                        new_structured,
                    );

                    let updated = Character {
                        id: existing.id.clone(),
                        name: existing.name.clone(),
                        aliases,
                        description,
                        description_structured: merged_structured,
                        traits,
                        role: existing.role.clone(),
                        first_appearance_chapter_id: existing.first_appearance_chapter_id.clone(),
                        relationships: existing.relationships.clone(),
                        evidence,
                        notes: existing.notes.clone(),
                        updated_at: now.clone(),
                    };

                    book_db.update_character(&updated).map_err(|e| e.to_string())?;
                    book_db
                        .update_knowledge_card_status(&cid, CardStatus::Accepted)
                        .map_err(|e| e.to_string())?;
                    return Ok(true);
                }
            }

            // Check if character already exists by exact name match
            if let Ok(Some(existing)) = book_db.find_character_by_name(&name) {
                // Merge with existing character
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

                // Merge structured description
                let chapter_index = get_chapter_index(&book_db, &card.chapter_id);
                let new_structured = parse_character_structured_description(
                    &card.content,
                    &card.chapter_id.to_string(),
                    chapter_index,
                );
                let merged_structured = merge_character_structured_description(
                    existing.description_structured,
                    new_structured,
                );

                let updated = Character {
                    id: existing.id,
                    name: existing.name,
                    aliases,
                    description,
                    description_structured: merged_structured,
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
                    .update_knowledge_card_status(&cid, CardStatus::Accepted)
                    .map_err(|e| e.to_string())?;
                return Ok(true);
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

            // Parse structured description for new character
            let chapter_index = get_chapter_index(&book_db, &card.chapter_id);
            let description_structured = parse_character_structured_description(
                &card.content,
                &card.chapter_id.to_string(),
                chapter_index,
            );

            let character = Character {
                id: EntityId::new(),
                name,
                aliases,
                description,
                description_structured,
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

            // Check for combo name
            if is_combo_name(&name) {
                tracing::info!("Skipping combo name: {}", name);
                book_db
                    .update_knowledge_card_status(&cid, CardStatus::Rejected)
                    .map_err(|e| e.to_string())?;
                return Ok(false);
            }

            // Build list of known entities for dedup
            let existing_settings = book_db.list_settings().map_err(|e| e.to_string())?;
            let known_entities: Vec<KnownEntity> = existing_settings.iter().map(|s| KnownEntity {
                id: s.id.to_string(),
                name: s.name.clone(),
                aliases: vec![],
            }).collect();

            // Check for dedup
            let dedup_result = check_entity_dedup(&name, &known_entities, 0.8);

            if dedup_result.should_skip {
                tracing::info!("Dedup: skipping setting '{}': {}", name, dedup_result.reason.as_deref().unwrap_or("unknown"));
                book_db
                    .update_knowledge_card_status(&cid, CardStatus::Rejected)
                    .map_err(|e| e.to_string())?;
                return Ok(false);
            }

            // If merge_with is set, merge with existing entity
            if let Some(merge_id) = dedup_result.merge_with {
                if let Some(existing) = existing_settings.iter().find(|s| s.id.to_string() == merge_id) {
                    tracing::info!("Dedup: merging setting '{}' with existing '{}': {}", name, existing.name, dedup_result.reason.as_deref().unwrap_or(""));

                    let new_description = card.content.get("description")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                    let description = new_description.or(existing.description.clone());

                    let setting_type = card.content.get("type")
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

                    // Merge structured description
                    let chapter_index = get_chapter_index(&book_db, &card.chapter_id);
                    let new_structured = parse_setting_structured_description(
                        &card.content,
                        &card.chapter_id.to_string(),
                        chapter_index,
                    );
                    let merged_structured = merge_setting_structured_description(
                        existing.description_structured.clone(),
                        new_structured,
                    );

                    let updated = Setting {
                        id: existing.id.clone(),
                        setting_type,
                        name: existing.name.clone(),
                        description,
                        description_structured: merged_structured,
                        properties,
                        evidence,
                        notes: existing.notes.clone(),
                        updated_at: now.clone(),
                    };

                    book_db.update_setting(&updated).map_err(|e| e.to_string())?;
                    book_db
                        .update_knowledge_card_status(&cid, CardStatus::Accepted)
                        .map_err(|e| e.to_string())?;
                    return Ok(true);
                }
            }

            // Check if setting already exists by exact name match
            if let Ok(Some(existing)) = book_db.find_setting_by_name(&name) {
                // Merge with existing setting
                let new_description = card.content.get("description")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                let description = new_description.or(existing.description);

                let setting_type = card.content.get("type")
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

                // Merge structured description
                let chapter_index = get_chapter_index(&book_db, &card.chapter_id);
                let new_structured = parse_setting_structured_description(
                    &card.content,
                    &card.chapter_id.to_string(),
                    chapter_index,
                );
                let merged_structured = merge_setting_structured_description(
                    existing.description_structured,
                    new_structured,
                );

                let updated = Setting {
                    id: existing.id,
                    setting_type,
                    name: existing.name,
                    description,
                    description_structured: merged_structured,
                    properties,
                    evidence,
                    notes: existing.notes,
                    updated_at: now.clone(),
                };

                book_db.update_setting(&updated).map_err(|e| e.to_string())?;
                book_db
                    .update_knowledge_card_status(&cid, CardStatus::Accepted)
                    .map_err(|e| e.to_string())?;
                return Ok(true);
            }

            let description = card.content.get("description")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            let setting_type = card.content.get("type")
                .and_then(|v| v.as_str())
                .unwrap_or("location")
                .to_string();

            let properties = card.content.get("properties")
                .cloned()
                .unwrap_or(serde_json::json!({}));

            // Parse structured description for new setting
            let chapter_index = get_chapter_index(&book_db, &card.chapter_id);
            let description_structured = parse_setting_structured_description(
                &card.content,
                &card.chapter_id.to_string(),
                chapter_index,
            );

            let setting = Setting {
                id: EntityId::new(),
                setting_type,
                name,
                description,
                description_structured,
                properties,
                evidence: card.evidence.clone(),
                notes: None,
                updated_at: now.clone(),
            };

            book_db.insert_setting(&setting).map_err(|e| e.to_string())?;
        }
        "event" => {
            let title = card.title.clone();

            let description = card.content.get("description")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            let importance = card.content.get("importance")
                .and_then(|v| v.as_str())
                .unwrap_or("normal")
                .to_string();

            let characters_involved: Vec<EntityId> = card.content.get("characters_involved")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| EntityId::from_string(s.to_string()))).collect())
                .unwrap_or_default();

            // Time-related fields
            // Support both the new flattened fields and legacy data stored under content.extra.
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

            let event = Event {
                id: EntityId::new(),
                title,
                description,
                chapter_id: Some(card.chapter_id.clone()),
                characters_involved,
                importance,
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
        _ => {
            return Err(format!("Unknown knowledge type: {}", card.knowledge_type));
        }
    }

    book_db
        .update_knowledge_card_status(&cid, CardStatus::Accepted)
        .map_err(|e| e.to_string())?;

    Ok(true)
}

#[tauri::command]
pub async fn reject_card(book_id: String, card_id: String) -> Result<bool, String> {
    let cid = CardId::from_string(card_id);
    let book_db = open_book_db(&book_id)?;
    book_db
        .update_knowledge_card_status(&cid, CardStatus::Rejected)
        .map_err(|e| e.to_string())?;

    Ok(true)
}

#[tauri::command]
pub async fn batch_accept_cards(book_id: String, card_ids: Vec<String>) -> Result<u32, String> {
    let mut accepted = 0;
    for card_id in card_ids {
        match accept_card(book_id.clone(), card_id).await {
            Ok(true) => accepted += 1,
            Ok(false) => {}
            Err(e) => {
                tracing::warn!("Failed to accept card: {}", e);
            }
        }
    }
    Ok(accepted)
}

#[tauri::command]
pub async fn batch_reject_cards(book_id: String, card_ids: Vec<String>) -> Result<u32, String> {
    let mut rejected = 0;
    for card_id in card_ids {
        match reject_card(book_id.clone(), card_id).await {
            Ok(true) => rejected += 1,
            Ok(false) => {}
            Err(e) => {
                tracing::warn!("Failed to reject card: {}", e);
            }
        }
    }
    Ok(rejected)
}

/// Accept a card with modified content (edit before accept)
#[tauri::command]
pub async fn accept_card_with_edits(
    book_id: String,
    card_id: String,
    edited_content: serde_json::Value,
) -> Result<bool, String> {
    let cid = CardId::from_string(card_id.clone());
    let book_db = open_book_db(&book_id)?;

    let cards = book_db
        .list_pending_knowledge_cards()
        .map_err(|e| e.to_string())?;

    let card = cards
        .into_iter()
        .find(|c| c.id.to_string() == card_id)
        .ok_or_else(|| format!("Card not found: {}", card_id))?;

    let now = Utc::now().to_rfc3339();

    // Use edited_content instead of card.content
    match card.knowledge_type.as_str() {
        "character" => {
            let name = edited_content.get("name")
                .and_then(|v| v.as_str())
                .unwrap_or(&card.title)
                .to_string();

            // Check for combo name (e.g., "张三 & 李四") - reject these
            if is_combo_name(&name) {
                tracing::info!("accept_card_with_edits: Skipping combo name: {}", name);
                book_db
                    .update_knowledge_card_status(&cid, CardStatus::Rejected)
                    .map_err(|e| e.to_string())?;
                return Ok(false);
            }

            // Build list of known entities for dedup check
            let existing_chars = book_db.list_characters().map_err(|e| e.to_string())?;
            let known_entities: Vec<KnownEntity> = existing_chars.iter().map(|c| KnownEntity {
                id: c.id.to_string(),
                name: c.name.clone(),
                aliases: c.aliases.clone(),
            }).collect();

            // Check for dedup (containment, similarity, alias match)
            let dedup_result = check_entity_dedup(&name, &known_entities, 0.8);

            // If dedup suggests merging with an existing entity, do that
            if let Some(merge_id) = dedup_result.merge_with {
                if let Some(existing) = existing_chars.iter().find(|c| c.id.to_string() == merge_id) {
                    tracing::info!("accept_card_with_edits: Dedup merging '{}' with existing '{}': {}",
                        name, existing.name, dedup_result.reason.as_deref().unwrap_or(""));

                    let description = edited_content.get("description")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string())
                        .or(existing.description.clone());

                    let mut aliases = existing.aliases.clone();
                    // Add the new name as an alias if different from existing name
                    if name.to_lowercase() != existing.name.to_lowercase() && !aliases.iter().any(|a| a.to_lowercase() == name.to_lowercase()) {
                        aliases.push(name.clone());
                    }
                    if let Some(new_aliases) = edited_content.get("aliases").and_then(|v| v.as_array()) {
                        for alias in new_aliases.iter().filter_map(|v| v.as_str()) {
                            if !aliases.iter().any(|a| a.to_lowercase() == alias.to_lowercase()) {
                                aliases.push(alias.to_string());
                            }
                        }
                    }

                    let mut traits = existing.traits.clone();
                    if let Some(new_traits) = edited_content.get("traits").and_then(|v| v.as_array()) {
                        for t in new_traits.iter().filter_map(|v| v.as_str()) {
                            if !traits.iter().any(|x| x.to_lowercase() == t.to_lowercase()) {
                                traits.push(t.to_string());
                            }
                        }
                    }

                    let role = edited_content.get("role")
                        .and_then(|v| v.as_str())
                        .unwrap_or(&existing.role)
                        .to_string();

                    let mut evidence = existing.evidence.clone();
                    for e in &card.evidence {
                        if !evidence.contains(e) {
                            evidence.push(e.clone());
                        }
                    }

                    // Merge structured description
                    let chapter_index = get_chapter_index(&book_db, &card.chapter_id);
                    let new_structured = parse_character_structured_description(
                        &edited_content,
                        &card.chapter_id.to_string(),
                        chapter_index,
                    );
                    let merged_structured = merge_character_structured_description(
                        existing.description_structured.clone(),
                        new_structured,
                    );

                    let updated = Character {
                        id: existing.id.clone(),
                        name: existing.name.clone(),
                        aliases,
                        description,
                        description_structured: merged_structured,
                        traits,
                        role,
                        first_appearance_chapter_id: existing.first_appearance_chapter_id.clone(),
                        relationships: existing.relationships.clone(),
                        evidence,
                        notes: edited_content.get("notes").and_then(|v| v.as_str()).map(String::from).or(existing.notes.clone()),
                        updated_at: now.clone(),
                    };

                    book_db.update_character(&updated).map_err(|e| e.to_string())?;
                    book_db
                        .update_knowledge_card_status(&cid, CardStatus::Accepted)
                        .map_err(|e| e.to_string())?;
                    return Ok(true);
                }
            }

            // Fallback: Check if character already exists by exact name match
            if let Ok(Some(existing)) = book_db.find_character_by_name(&name) {
                // Update existing character instead of creating new one
                let description = edited_content.get("description")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
                    .or(existing.description);

                let mut aliases: Vec<String> = edited_content.get("aliases")
                    .and_then(|v| v.as_array())
                    .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                    .unwrap_or(existing.aliases.clone());
                // Merge aliases
                for alias in existing.aliases {
                    if !aliases.contains(&alias) {
                        aliases.push(alias);
                    }
                }

                let mut traits: Vec<String> = edited_content.get("traits")
                    .and_then(|v| v.as_array())
                    .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                    .unwrap_or(existing.traits.clone());
                // Merge traits
                for t in existing.traits {
                    if !traits.contains(&t) {
                        traits.push(t);
                    }
                }

                let role = edited_content.get("role")
                    .and_then(|v| v.as_str())
                    .unwrap_or(&existing.role)
                    .to_string();

                // Merge evidence
                let mut evidence = existing.evidence.clone();
                for e in &card.evidence {
                    if !evidence.contains(e) {
                        evidence.push(e.clone());
                    }
                }

                // Merge structured description
                let chapter_index = get_chapter_index(&book_db, &card.chapter_id);
                let new_structured = parse_character_structured_description(
                    &edited_content,
                    &card.chapter_id.to_string(),
                    chapter_index,
                );
                let merged_structured = merge_character_structured_description(
                    existing.description_structured,
                    new_structured,
                );

                let updated = Character {
                    id: existing.id,
                    name: existing.name,
                    aliases,
                    description,
                    description_structured: merged_structured,
                    traits,
                    role,
                    first_appearance_chapter_id: existing.first_appearance_chapter_id,
                    relationships: existing.relationships,
                    evidence,
                    notes: edited_content.get("notes").and_then(|v| v.as_str()).map(String::from).or(existing.notes),
                    updated_at: now.clone(),
                };

                book_db.update_character(&updated).map_err(|e| e.to_string())?;
                book_db
                    .update_knowledge_card_status(&cid, CardStatus::Accepted)
                    .map_err(|e| e.to_string())?;
                return Ok(true);
            }

            let description = edited_content.get("description")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            let aliases: Vec<String> = edited_content.get("aliases")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                .unwrap_or_default();

            let traits: Vec<String> = edited_content.get("traits")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                .unwrap_or_default();

            let role = edited_content.get("role")
                .and_then(|v| v.as_str())
                .unwrap_or("supporting")
                .to_string();

            // Parse structured description for new character
            let chapter_index = get_chapter_index(&book_db, &card.chapter_id);
            let description_structured = parse_character_structured_description(
                &edited_content,
                &card.chapter_id.to_string(),
                chapter_index,
            );

            let character = Character {
                id: EntityId::new(),
                name,
                aliases,
                description,
                description_structured,
                traits,
                role,
                first_appearance_chapter_id: Some(card.chapter_id.clone()),
                relationships: serde_json::json!({}),
                evidence: card.evidence.clone(),
                notes: edited_content.get("notes").and_then(|v| v.as_str()).map(String::from),
                updated_at: now.clone(),
            };

            book_db.insert_character(&character).map_err(|e| e.to_string())?;
        }
        "setting" => {
            let name = edited_content.get("name")
                .and_then(|v| v.as_str())
                .unwrap_or(&card.title)
                .to_string();

            if is_combo_name(&name) {
                tracing::info!("accept_card_with_edits: Skipping combo setting name: {}", name);
                book_db
                    .update_knowledge_card_status(&cid, CardStatus::Rejected)
                    .map_err(|e| e.to_string())?;
                return Ok(false);
            }

            let existing_settings = book_db.list_settings().map_err(|e| e.to_string())?;
            let known_entities: Vec<KnownEntity> = existing_settings.iter().map(|s| KnownEntity {
                id: s.id.to_string(),
                name: s.name.clone(),
                aliases: vec![],
            }).collect();

            let dedup_result = check_entity_dedup(&name, &known_entities, 0.8);

            if let Some(merge_id) = dedup_result.merge_with {
                if let Some(existing) = existing_settings.iter().find(|s| s.id.to_string() == merge_id) {
                    tracing::info!("accept_card_with_edits: Dedup merging setting '{}' with existing '{}': {}",
                        name, existing.name, dedup_result.reason.as_deref().unwrap_or(""));

                    let description = edited_content.get("description")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string())
                        .or(existing.description.clone());

                    let setting_type = edited_content.get("type")
                        .or_else(|| edited_content.get("setting_type"))
                        .and_then(|v| v.as_str())
                        .unwrap_or(&existing.setting_type)
                        .to_string();

                    let mut properties = existing.properties.clone();
                    if let Some(obj) = properties.as_object_mut() {
                        if let Some(new_props) = edited_content.get("properties").and_then(|v| v.as_object()) {
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

                    let chapter_index = get_chapter_index(&book_db, &card.chapter_id);
                    let new_structured = parse_setting_structured_description(
                        &edited_content,
                        &card.chapter_id.to_string(),
                        chapter_index,
                    );
                    let merged_structured = merge_setting_structured_description(
                        existing.description_structured.clone(),
                        new_structured,
                    );

                    let updated = Setting {
                        id: existing.id.clone(),
                        setting_type,
                        name: existing.name.clone(),
                        description,
                        description_structured: merged_structured,
                        properties,
                        evidence,
                        notes: edited_content.get("notes").and_then(|v| v.as_str()).map(String::from).or(existing.notes.clone()),
                        updated_at: now.clone(),
                    };

                    book_db.update_setting(&updated).map_err(|e| e.to_string())?;
                    book_db
                        .update_knowledge_card_status(&cid, CardStatus::Accepted)
                        .map_err(|e| e.to_string())?;
                    return Ok(true);
                }
            }

            if let Ok(Some(existing)) = book_db.find_setting_by_name(&name) {
                // Update existing setting instead of creating new one
                let description = edited_content.get("description")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
                    .or(existing.description);

                let setting_type = edited_content.get("type")
                    .or_else(|| edited_content.get("setting_type"))
                    .and_then(|v| v.as_str())
                    .unwrap_or(&existing.setting_type)
                    .to_string();

                // Merge properties
                let mut properties = existing.properties.clone();
                if let Some(obj) = properties.as_object_mut() {
                    if let Some(new_props) = edited_content.get("properties").and_then(|v| v.as_object()) {
                        for (k, v) in new_props {
                            obj.insert(k.clone(), v.clone());
                        }
                    }
                }

                // Merge evidence
                let mut evidence = existing.evidence.clone();
                for e in &card.evidence {
                    if !evidence.contains(e) {
                        evidence.push(e.clone());
                    }
                }

                // Merge structured description
                let chapter_index = get_chapter_index(&book_db, &card.chapter_id);
                let new_structured = parse_setting_structured_description(
                    &edited_content,
                    &card.chapter_id.to_string(),
                    chapter_index,
                );
                let merged_structured = merge_setting_structured_description(
                    existing.description_structured,
                    new_structured,
                );

                let updated = Setting {
                    id: existing.id,
                    setting_type,
                    name: existing.name,
                    description,
                    description_structured: merged_structured,
                    properties,
                    evidence,
                    notes: edited_content.get("notes").and_then(|v| v.as_str()).map(String::from).or(existing.notes),
                    updated_at: now.clone(),
                };

                book_db.update_setting(&updated).map_err(|e| e.to_string())?;
                book_db
                    .update_knowledge_card_status(&cid, CardStatus::Accepted)
                    .map_err(|e| e.to_string())?;
                return Ok(true);
            }

            let description = edited_content.get("description")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            let setting_type = edited_content.get("type")
                .or_else(|| edited_content.get("setting_type"))
                .and_then(|v| v.as_str())
                .unwrap_or("location")
                .to_string();

            let properties = edited_content.get("properties")
                .cloned()
                .unwrap_or(serde_json::json!({}));

            // Parse structured description for new setting
            let chapter_index = get_chapter_index(&book_db, &card.chapter_id);
            let description_structured = parse_setting_structured_description(
                &edited_content,
                &card.chapter_id.to_string(),
                chapter_index,
            );

            let setting = Setting {
                id: EntityId::new(),
                setting_type,
                name,
                description,
                description_structured,
                properties,
                evidence: card.evidence.clone(),
                notes: edited_content.get("notes").and_then(|v| v.as_str()).map(String::from),
                updated_at: now.clone(),
            };

            book_db.insert_setting(&setting).map_err(|e| e.to_string())?;
        }
        "event" => {
            let title = edited_content.get("title")
                .and_then(|v| v.as_str())
                .unwrap_or(&card.title)
                .to_string();

            let description = edited_content.get("description")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            let importance = edited_content.get("importance")
                .and_then(|v| v.as_str())
                .unwrap_or("normal")
                .to_string();

            let characters_involved: Vec<EntityId> = edited_content.get("characters_involved")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| EntityId::from_string(s.to_string()))).collect())
                .unwrap_or_default();

            // Time-related fields
            let time_marker = edited_content.get("time_marker")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let order_in_chapter = edited_content.get("order_in_chapter")
                .and_then(|v| v.as_i64())
                .unwrap_or(0) as i32;
            let is_flashback = edited_content.get("is_flashback")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let relative_time = edited_content.get("relative_time")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            let event = Event {
                id: EntityId::new(),
                title,
                description,
                chapter_id: Some(card.chapter_id.clone()),
                characters_involved,
                importance,
                evidence: card.evidence.clone(),
                notes: edited_content.get("notes").and_then(|v| v.as_str()).map(String::from),
                updated_at: now.clone(),
                time_marker,
                order_in_chapter,
                is_flashback,
                relative_time,
            };

            book_db.insert_event(&event).map_err(|e| e.to_string())?;
        }
        _ => {
            return Err(format!("Unknown knowledge type: {}", card.knowledge_type));
        }
    }

    book_db
        .update_knowledge_card_status(&cid, CardStatus::Accepted)
        .map_err(|e| e.to_string())?;

    Ok(true)
}

/// Merge a card into an existing entity
#[tauri::command]
pub async fn merge_card(
    book_id: String,
    card_id: String,
    target_entity_id: String,
    merge_strategy: Option<String>,
) -> Result<bool, String> {
    let library = Library::open().map_err(|e| e.to_string())?;
    let bid = BookId::from_string(book_id);
    let cid = CardId::from_string(card_id.clone());
    let target_id = EntityId::from_string(target_entity_id.clone());

    let book_dir = library.book_dir(&bid);
    let book_db_path = book_dir.join("book.db");

    if !book_db_path.exists() {
        return Err("Book database not found".to_string());
    }

    let book_db = BookDb::open(&book_db_path, bid).map_err(|e| e.to_string())?;

    let cards = book_db
        .list_pending_knowledge_cards()
        .map_err(|e| e.to_string())?;

    let card = cards
        .into_iter()
        .find(|c| c.id.to_string() == card_id)
        .ok_or_else(|| format!("Card not found: {}", card_id))?;

    let now = Utc::now().to_rfc3339();
    let strategy = merge_strategy.unwrap_or_else(|| "append".to_string());

    match card.knowledge_type.as_str() {
        "character" => {
            let existing = book_db
                .get_character(&target_id)
                .map_err(|e| e.to_string())?
                .ok_or_else(|| format!("Target character not found: {}", target_entity_id))?;

            // Merge aliases
            let mut aliases = existing.aliases.clone();
            if let Some(new_aliases) = card.content.get("aliases").and_then(|v| v.as_array()) {
                for alias in new_aliases.iter().filter_map(|v| v.as_str()) {
                    if !aliases.contains(&alias.to_string()) {
                        aliases.push(alias.to_string());
                    }
                }
            }

            // Merge traits
            let mut traits = existing.traits.clone();
            if let Some(new_traits) = card.content.get("traits").and_then(|v| v.as_array()) {
                for t in new_traits.iter().filter_map(|v| v.as_str()) {
                    if !traits.contains(&t.to_string()) {
                        traits.push(t.to_string());
                    }
                }
            }

            // Merge evidence
            let mut evidence = existing.evidence.clone();
            for ev in &card.evidence {
                if !evidence.contains(ev) {
                    evidence.push(ev.clone());
                }
            }

            // Merge description based on strategy
            let description = match strategy.as_str() {
                "replace" => card.content.get("description").and_then(|v| v.as_str()).map(String::from),
                "append" | _ => {
                    let new_desc = card.content.get("description").and_then(|v| v.as_str());
                    match (existing.description.as_ref(), new_desc) {
                        (Some(old), Some(new)) if !old.contains(new) => Some(format!("{}\n\n{}", old, new)),
                        (None, Some(new)) => Some(new.to_string()),
                        (old, _) => old.cloned(),
                    }
                }
            };

            // Merge structured description
            let chapter_index = get_chapter_index(&book_db, &card.chapter_id);
            let new_structured = parse_character_structured_description(
                &card.content,
                &card.chapter_id.to_string(),
                chapter_index,
            );
            let merged_structured = merge_character_structured_description(
                existing.description_structured,
                new_structured,
            );

            let updated = Character {
                id: existing.id,
                name: existing.name,
                aliases,
                description,
                description_structured: merged_structured,
                traits,
                role: existing.role,
                first_appearance_chapter_id: existing.first_appearance_chapter_id,
                relationships: existing.relationships,
                evidence,
                notes: existing.notes,
                updated_at: now,
            };

            book_db.update_character(&updated).map_err(|e| e.to_string())?;
        }
        "setting" => {
            let existing = book_db
                .list_settings()
                .map_err(|e| e.to_string())?
                .into_iter()
                .find(|s| s.id.to_string() == target_entity_id)
                .ok_or_else(|| format!("Target setting not found: {}", target_entity_id))?;

            // Merge evidence
            let mut evidence = existing.evidence.clone();
            for ev in &card.evidence {
                if !evidence.contains(ev) {
                    evidence.push(ev.clone());
                }
            }

            // Merge properties
            let mut properties = existing.properties.clone();
            if let Some(new_props) = card.content.get("properties").and_then(|v| v.as_object()) {
                if let Some(obj) = properties.as_object_mut() {
                    for (k, v) in new_props {
                        if !obj.contains_key(k) {
                            obj.insert(k.clone(), v.clone());
                        }
                    }
                }
            }

            // Merge description
            let description = match strategy.as_str() {
                "replace" => card.content.get("description").and_then(|v| v.as_str()).map(String::from),
                "append" | _ => {
                    let new_desc = card.content.get("description").and_then(|v| v.as_str());
                    match (existing.description.as_ref(), new_desc) {
                        (Some(old), Some(new)) if !old.contains(new) => Some(format!("{}\n\n{}", old, new)),
                        (None, Some(new)) => Some(new.to_string()),
                        (old, _) => old.cloned(),
                    }
                }
            };

            // Merge structured description
            let chapter_index = get_chapter_index(&book_db, &card.chapter_id);
            let new_structured = parse_setting_structured_description(
                &card.content,
                &card.chapter_id.to_string(),
                chapter_index,
            );
            let merged_structured = merge_setting_structured_description(
                existing.description_structured,
                new_structured,
            );

            let updated = Setting {
                id: existing.id,
                setting_type: existing.setting_type,
                name: existing.name,
                description,
                description_structured: merged_structured,
                properties,
                evidence,
                notes: existing.notes,
                updated_at: now,
            };

            book_db.update_setting(&updated).map_err(|e| e.to_string())?;
        }
        "event" => {
            let existing = book_db
                .list_events()
                .map_err(|e| e.to_string())?
                .into_iter()
                .find(|e| e.id.to_string() == target_entity_id)
                .ok_or_else(|| format!("Target event not found: {}", target_entity_id))?;

            // Merge evidence
            let mut evidence = existing.evidence.clone();
            for ev in &card.evidence {
                if !evidence.contains(ev) {
                    evidence.push(ev.clone());
                }
            }

            // Merge characters_involved
            let mut characters_involved = existing.characters_involved.clone();
            if let Some(new_chars) = card.content.get("characters_involved").and_then(|v| v.as_array()) {
                for c in new_chars.iter().filter_map(|v| v.as_str()) {
                    let char_id = EntityId::from_string(c.to_string());
                    if !characters_involved.iter().any(|x| x.to_string() == char_id.to_string()) {
                        characters_involved.push(char_id);
                    }
                }
            }

            // Merge description
            let description = match strategy.as_str() {
                "replace" => card.content.get("description").and_then(|v| v.as_str()).map(String::from),
                "append" | _ => {
                    let new_desc = card.content.get("description").and_then(|v| v.as_str());
                    match (existing.description.as_ref(), new_desc) {
                        (Some(old), Some(new)) if !old.contains(new) => Some(format!("{}\n\n{}", old, new)),
                        (None, Some(new)) => Some(new.to_string()),
                        (old, _) => old.cloned(),
                    }
                }
            };

            let updated = Event {
                id: existing.id,
                title: existing.title,
                description,
                chapter_id: existing.chapter_id,
                characters_involved,
                importance: existing.importance,
                evidence,
                notes: existing.notes,
                updated_at: now,
                time_marker: existing.time_marker,
                order_in_chapter: existing.order_in_chapter,
                is_flashback: existing.is_flashback,
                relative_time: existing.relative_time,
            };

            book_db.update_event(&updated).map_err(|e| e.to_string())?;
        }
        _ => {
            return Err(format!("Unknown knowledge type: {}", card.knowledge_type));
        }
    }

    book_db
        .update_knowledge_card_status(&cid, CardStatus::Merged)
        .map_err(|e| e.to_string())?;

    Ok(true)
}

/// Get entities that could be merge targets for a card
#[tauri::command]
pub async fn get_merge_candidates(
    book_id: String,
    card_id: String,
) -> Result<Vec<serde_json::Value>, String> {
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

    let card = cards
        .into_iter()
        .find(|c| c.id.to_string() == card_id)
        .ok_or_else(|| format!("Card not found: {}", card_id))?;

    let candidates: Vec<serde_json::Value> = match card.knowledge_type.as_str() {
        "character" => {
            book_db
                .list_characters()
                .map_err(|e| e.to_string())?
                .into_iter()
                .map(|c| serde_json::json!({
                    "id": c.id.to_string(),
                    "name": c.name,
                    "type": "character",
                    "description": c.description,
                }))
                .collect()
        }
        "setting" => {
            book_db
                .list_settings()
                .map_err(|e| e.to_string())?
                .into_iter()
                .map(|s| serde_json::json!({
                    "id": s.id.to_string(),
                    "name": s.name,
                    "type": "setting",
                    "setting_type": s.setting_type,
                    "description": s.description,
                }))
                .collect()
        }
        "event" => {
            book_db
                .list_events()
                .map_err(|e| e.to_string())?
                .into_iter()
                .map(|e| serde_json::json!({
                    "id": e.id.to_string(),
                    "name": e.title,
                    "type": "event",
                    "description": e.description,
                }))
                .collect()
        }
        _ => vec![],
    };

    Ok(candidates)
}
