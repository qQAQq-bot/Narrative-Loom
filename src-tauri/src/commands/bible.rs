use crate::core::ids::{BookId, ChapterId, EntityId};
use crate::storage::book_db::{BookDb, Character, Event, Setting};
use crate::storage::library::Library;
use crate::commands::embedding::{
    update_entity_embedding_internal, delete_entity_embedding_internal, EntityType,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterInfo {
    pub id: String,
    pub name: String,
    pub aliases: Vec<String>,
    pub description: Option<String>,
    pub traits: Vec<String>,
    pub role: String,
    pub first_appearance_chapter_id: Option<String>,
    pub relationships: serde_json::Value,
    pub evidence: Vec<String>,
    pub notes: Option<String>,
    pub updated_at: String,
}

impl From<Character> for CharacterInfo {
    fn from(c: Character) -> Self {
        Self {
            id: c.id.to_string(),
            name: c.name,
            aliases: c.aliases,
            description: c.description,
            traits: c.traits,
            role: c.role,
            first_appearance_chapter_id: c.first_appearance_chapter_id.map(|id| id.to_string()),
            relationships: c.relationships,
            evidence: c.evidence,
            notes: c.notes,
            updated_at: c.updated_at,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettingInfo {
    pub id: String,
    pub setting_type: String,
    pub name: String,
    pub description: Option<String>,
    pub properties: serde_json::Value,
    pub evidence: Vec<String>,
    pub notes: Option<String>,
    pub updated_at: String,
}

impl From<Setting> for SettingInfo {
    fn from(s: Setting) -> Self {
        Self {
            id: s.id.to_string(),
            setting_type: s.setting_type,
            name: s.name,
            description: s.description,
            properties: s.properties,
            evidence: s.evidence,
            notes: s.notes,
            updated_at: s.updated_at,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventInfo {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub chapter_id: Option<String>,
    pub characters_involved: Vec<String>,
    pub importance: String,
    pub evidence: Vec<String>,
    pub notes: Option<String>,
    pub updated_at: String,
    // Time-related fields
    pub time_marker: Option<String>,
    pub order_in_chapter: i32,
    pub is_flashback: bool,
    pub relative_time: Option<String>,
}

impl From<Event> for EventInfo {
    fn from(e: Event) -> Self {
        Self {
            id: e.id.to_string(),
            title: e.title,
            description: e.description,
            chapter_id: e.chapter_id.map(|id| id.to_string()),
            characters_involved: e.characters_involved.into_iter().map(|id| id.to_string()).collect(),
            importance: e.importance,
            evidence: e.evidence,
            notes: e.notes,
            updated_at: e.updated_at,
            time_marker: e.time_marker,
            order_in_chapter: e.order_in_chapter,
            is_flashback: e.is_flashback,
            relative_time: e.relative_time,
        }
    }
}

use super::common::open_book_db;

#[tauri::command]
pub async fn get_characters(book_id: String) -> Result<Vec<CharacterInfo>, String> {
    let book_db = open_book_db(&book_id)?;
    let characters = book_db.list_characters().map_err(|e| e.to_string())?;
    Ok(characters.into_iter().map(CharacterInfo::from).collect())
}

#[tauri::command]
pub async fn get_character(book_id: String, character_id: String) -> Result<Option<CharacterInfo>, String> {
    let book_db = open_book_db(&book_id)?;
    let eid = EntityId::from_string(character_id);
    let character = book_db.get_character(&eid).map_err(|e| e.to_string())?;
    Ok(character.map(CharacterInfo::from))
}

#[tauri::command]
pub async fn update_character(book_id: String, character: CharacterInfo) -> Result<bool, String> {
    let book_db = open_book_db(&book_id)?;
    let now = Utc::now().to_rfc3339();

    // Get existing character to preserve description_structured
    let entity_id = EntityId::from_string(character.id.clone());
    let existing_description_structured = book_db
        .get_character(&entity_id)
        .ok()
        .flatten()
        .and_then(|c| c.description_structured);

    let entity = Character {
        id: entity_id,
        name: character.name.clone(),
        aliases: character.aliases.clone(),
        description: character.description.clone(),
        description_structured: existing_description_structured,
        traits: character.traits.clone(),
        role: character.role.clone(),
        first_appearance_chapter_id: character.first_appearance_chapter_id.map(ChapterId::from_string),
        relationships: character.relationships,
        evidence: character.evidence,
        notes: character.notes,
        updated_at: now,
    };

    book_db.update_character(&entity).map_err(|e| e.to_string())?;

    // P3.5-001: Update embedding for the character
    let traits_str = if character.traits.is_empty() {
        None
    } else {
        Some(character.traits.join(", "))
    };

    if let Err(e) = update_entity_embedding_internal(
        &book_id,
        &character.id,
        &EntityType::Character,
        &character.name,
        character.description.as_deref(),
        traits_str.as_deref(),
    ) {
        tracing::warn!("Failed to update character embedding: {}", e);
        // Continue anyway - embedding update failure shouldn't block the main update
    }

    Ok(true)
}

#[tauri::command]
pub async fn get_settings(book_id: String) -> Result<Vec<SettingInfo>, String> {
    let book_db = open_book_db(&book_id)?;
    let settings = book_db.list_settings().map_err(|e| e.to_string())?;
    Ok(settings.into_iter().map(SettingInfo::from).collect())
}

#[tauri::command]
pub async fn update_setting(book_id: String, setting: SettingInfo) -> Result<bool, String> {
    let book_db = open_book_db(&book_id)?;
    let now = Utc::now().to_rfc3339();

    // Get existing setting to preserve description_structured
    let entity_id = EntityId::from_string(setting.id.clone());
    let existing_description_structured = book_db
        .get_setting(&entity_id)
        .ok()
        .flatten()
        .and_then(|s| s.description_structured);

    let entity = Setting {
        id: entity_id,
        setting_type: setting.setting_type.clone(),
        name: setting.name.clone(),
        description: setting.description.clone(),
        description_structured: existing_description_structured,
        properties: setting.properties,
        evidence: setting.evidence,
        notes: setting.notes,
        updated_at: now,
    };

    book_db.update_setting(&entity).map_err(|e| e.to_string())?;

    // P3.5-002: Update embedding for the setting
    let type_info = Some(format!("类型: {}", setting.setting_type));

    if let Err(e) = update_entity_embedding_internal(
        &book_id,
        &setting.id,
        &EntityType::Setting,
        &setting.name,
        setting.description.as_deref(),
        type_info.as_deref(),
    ) {
        tracing::warn!("Failed to update setting embedding: {}", e);
    }

    Ok(true)
}

#[tauri::command]
pub async fn get_events(book_id: String) -> Result<Vec<EventInfo>, String> {
    let book_db = open_book_db(&book_id)?;
    let events = book_db.list_events().map_err(|e| e.to_string())?;
    Ok(events.into_iter().map(EventInfo::from).collect())
}

#[tauri::command]
pub async fn update_event(book_id: String, event: EventInfo) -> Result<bool, String> {
    let book_db = open_book_db(&book_id)?;
    let now = Utc::now().to_rfc3339();

    let entity = Event {
        id: EntityId::from_string(event.id.clone()),
        title: event.title.clone(),
        description: event.description.clone(),
        chapter_id: event.chapter_id.map(ChapterId::from_string),
        characters_involved: event.characters_involved.iter().map(|s| EntityId::from_string(s.clone())).collect(),
        importance: event.importance.clone(),
        evidence: event.evidence,
        notes: event.notes,
        updated_at: now,
        time_marker: event.time_marker,
        order_in_chapter: event.order_in_chapter,
        is_flashback: event.is_flashback,
        relative_time: event.relative_time,
    };

    book_db.update_event(&entity).map_err(|e| e.to_string())?;

    // P3.5-003: Update embedding for the event
    let importance_info = Some(format!("重要性: {}", event.importance));

    if let Err(e) = update_entity_embedding_internal(
        &book_id,
        &event.id,
        &EntityType::Event,
        &event.title,
        event.description.as_deref(),
        importance_info.as_deref(),
    ) {
        tracing::warn!("Failed to update event embedding: {}", e);
    }

    Ok(true)
}

#[tauri::command]
pub async fn get_bible_stats(book_id: String) -> Result<serde_json::Value, String> {
    let book_db = open_book_db(&book_id)?;

    let characters = book_db.list_characters().map_err(|e| e.to_string())?;
    let settings = book_db.list_settings().map_err(|e| e.to_string())?;
    let events = book_db.list_events().map_err(|e| e.to_string())?;

    Ok(serde_json::json!({
        "characters": characters.len(),
        "settings": settings.len(),
        "events": events.len(),
    }))
}

#[tauri::command]
pub async fn delete_character(book_id: String, character_id: String) -> Result<bool, String> {
    let book_db = open_book_db(&book_id)?;
    let eid = EntityId::from_string(character_id.clone());
    let result = book_db.delete_character(&eid).map_err(|e| e.to_string())?;

    // Delete corresponding embedding
    if let Err(e) = delete_entity_embedding_internal(&book_id, &character_id) {
        tracing::warn!("Failed to delete character embedding: {}", e);
    }

    Ok(result)
}

#[tauri::command]
pub async fn delete_setting(book_id: String, setting_id: String) -> Result<bool, String> {
    let book_db = open_book_db(&book_id)?;
    let eid = EntityId::from_string(setting_id.clone());
    let result = book_db.delete_setting(&eid).map_err(|e| e.to_string())?;

    // Delete corresponding embedding
    if let Err(e) = delete_entity_embedding_internal(&book_id, &setting_id) {
        tracing::warn!("Failed to delete setting embedding: {}", e);
    }

    Ok(result)
}

#[tauri::command]
pub async fn delete_event(book_id: String, event_id: String) -> Result<bool, String> {
    let book_db = open_book_db(&book_id)?;
    let eid = EntityId::from_string(event_id.clone());
    let result = book_db.delete_event(&eid).map_err(|e| e.to_string())?;

    // Delete corresponding embedding
    if let Err(e) = delete_entity_embedding_internal(&book_id, &event_id) {
        tracing::warn!("Failed to delete event embedding: {}", e);
    }

    Ok(result)
}

#[tauri::command]
pub async fn clear_all_characters(book_id: String) -> Result<u32, String> {
    let book_db = open_book_db(&book_id)?;
    book_db.delete_all_characters().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn clear_all_settings(book_id: String) -> Result<u32, String> {
    let book_db = open_book_db(&book_id)?;
    book_db.delete_all_settings().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn clear_all_events(book_id: String) -> Result<u32, String> {
    let book_db = open_book_db(&book_id)?;
    book_db.delete_all_events().map_err(|e| e.to_string())
}

// ============================================================================
// Story Blueprint (时间线视图)
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlueprintEvent {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub importance: String,
    pub is_turning_point: bool,
    pub characters_involved: Vec<String>,
    pub time_marker: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChapterBlueprint {
    pub id: String,
    pub index: u32,
    pub title: Option<String>,
    pub analyzed: bool,
    pub events: Vec<BlueprintEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlueprintData {
    pub book_id: String,
    pub total_chapters: u32,
    pub analyzed_chapters: u32,
    pub chapters: Vec<ChapterBlueprint>,
}

#[tauri::command]
pub async fn get_story_blueprint(book_id: String) -> Result<BlueprintData, String> {
    let library = crate::storage::library::Library::open().map_err(|e| e.to_string())?;
    let bid = BookId::from_string(book_id.clone());

    let book_dir = library.book_dir(&bid);
    let book_db_path = book_dir.join("book.db");

    if !book_db_path.exists() {
        return Err("Book database not found".to_string());
    }

    let book_db = crate::storage::book_db::BookDb::open(&book_db_path, bid.clone())
        .map_err(|e| e.to_string())?;

    // Get all chapters
    let chapters_summary = book_db.list_chapters().map_err(|e| e.to_string())?;

    // Get all events
    let all_events = book_db.list_events().map_err(|e| e.to_string())?;

    // Get all characters for name lookup
    let all_characters = book_db.list_characters().map_err(|e| e.to_string())?;

    // Build character ID to name map
    let char_map: std::collections::HashMap<String, String> = all_characters
        .iter()
        .map(|c| (c.id.to_string(), c.name.clone()))
        .collect();

    // Group events by chapter (with a sortable key for stable ordering in the UI)
    let mut events_by_chapter: std::collections::HashMap<String, Vec<(i32, BlueprintEvent)>> =
        std::collections::HashMap::new();

    for event in all_events {
        if let Some(chapter_id) = &event.chapter_id {
            let chapter_id_str = chapter_id.to_string();

            // Determine if it's a turning point based on importance
            let is_turning_point = event.importance == "critical";

            // Get character names
            let character_names: Vec<String> = event.characters_involved
                .iter()
                .filter_map(|cid| char_map.get(&cid.to_string()).cloned())
                .collect();

            let time_marker = event.time_marker.clone();

            let order_in_chapter = event.order_in_chapter;

            let blueprint_event = BlueprintEvent {
                id: event.id.to_string(),
                title: event.title,
                description: event.description,
                importance: event.importance,
                is_turning_point,
                characters_involved: character_names,
                time_marker,
            };

            events_by_chapter
                .entry(chapter_id_str)
                .or_insert_with(Vec::new)
                .push((order_in_chapter, blueprint_event));
        }
    }

    // Build chapter blueprints
    let mut chapter_blueprints: Vec<ChapterBlueprint> = Vec::new();
    let mut analyzed_count = 0u32;

    for chapter in &chapters_summary {
        if chapter.analyzed {
            analyzed_count += 1;
        }

        let mut chapter_events = events_by_chapter
            .remove(&chapter.id.to_string())
            .unwrap_or_default();

        // Sort events by order_in_chapter (0/unknown goes last)
        chapter_events.sort_by_key(|(order, _)| (*order <= 0, *order));
        let chapter_events: Vec<BlueprintEvent> = chapter_events.into_iter().map(|(_, e)| e).collect();

        chapter_blueprints.push(ChapterBlueprint {
            id: chapter.id.to_string(),
            index: chapter.index_num,
            title: chapter.title.clone(),
            analyzed: chapter.analyzed,
            events: chapter_events,
        });
    }

    // Sort chapters by index
    chapter_blueprints.sort_by_key(|c| c.index);

    Ok(BlueprintData {
        book_id,
        total_chapters: chapters_summary.len() as u32,
        analyzed_chapters: analyzed_count,
        chapters: chapter_blueprints,
    })
}

/// Merge duplicate characters (same name, case-insensitive)
#[tauri::command]
pub async fn merge_duplicate_characters(book_id: String) -> Result<u32, String> {
    let library = Library::open().map_err(|e| e.to_string())?;
    let bid = BookId::from_string(book_id);

    let book_dir = library.book_dir(&bid);
    let book_db_path = book_dir.join("book.db");

    if !book_db_path.exists() {
        return Err("Book database not found".to_string());
    }

    let book_db = BookDb::open(&book_db_path, bid).map_err(|e| e.to_string())?;
    let characters = book_db.list_characters().map_err(|e| e.to_string())?;

    // Group by normalized name
    let mut groups: std::collections::HashMap<String, Vec<Character>> = std::collections::HashMap::new();
    for c in characters {
        let key = c.name.trim().to_lowercase();
        groups.entry(key).or_insert_with(Vec::new).push(c);
    }

    let mut merged_count = 0u32;
    let now = Utc::now().to_rfc3339();

    for (_name, mut chars) in groups {
        if chars.len() <= 1 {
            continue;
        }

        // Sort by updated_at to keep the earliest as primary
        chars.sort_by(|a, b| a.updated_at.cmp(&b.updated_at));

        let primary = chars.remove(0);
        let mut merged = primary.clone();

        // Merge all duplicates into primary
        for dup in &chars {
            // Merge aliases
            for alias in &dup.aliases {
                if !merged.aliases.iter().any(|a| a.to_lowercase() == alias.to_lowercase()) {
                    merged.aliases.push(alias.clone());
                }
            }

            // Merge traits
            for t in &dup.traits {
                if !merged.traits.iter().any(|x| x.to_lowercase() == t.to_lowercase()) {
                    merged.traits.push(t.clone());
                }
            }

            // Merge evidence
            for e in &dup.evidence {
                if !merged.evidence.contains(e) {
                    merged.evidence.push(e.clone());
                }
            }

            // Use longer description
            if let Some(ref dup_desc) = dup.description {
                match &merged.description {
                    Some(existing) if existing.len() < dup_desc.len() => {
                        merged.description = Some(dup_desc.clone());
                    }
                    None => {
                        merged.description = Some(dup_desc.clone());
                    }
                    _ => {}
                }
            }
        }

        merged.updated_at = now.clone();
        book_db.update_character(&merged).map_err(|e| e.to_string())?;

        // Delete duplicates
        for dup in &chars {
            book_db.delete_character(&dup.id).map_err(|e| e.to_string())?;
            merged_count += 1;
        }
    }

    Ok(merged_count)
}

/// Merge duplicate settings (same name, case-insensitive)
#[tauri::command]
pub async fn merge_duplicate_settings(book_id: String) -> Result<u32, String> {
    let library = Library::open().map_err(|e| e.to_string())?;
    let bid = BookId::from_string(book_id);

    let book_dir = library.book_dir(&bid);
    let book_db_path = book_dir.join("book.db");

    if !book_db_path.exists() {
        return Err("Book database not found".to_string());
    }

    let book_db = BookDb::open(&book_db_path, bid).map_err(|e| e.to_string())?;
    let settings = book_db.list_settings().map_err(|e| e.to_string())?;

    // Group by normalized name
    let mut groups: std::collections::HashMap<String, Vec<Setting>> = std::collections::HashMap::new();
    for s in settings {
        let key = s.name.trim().to_lowercase();
        groups.entry(key).or_insert_with(Vec::new).push(s);
    }

    let mut merged_count = 0u32;
    let now = Utc::now().to_rfc3339();

    for (_name, mut items) in groups {
        if items.len() <= 1 {
            continue;
        }

        // Sort by updated_at to keep the earliest as primary
        items.sort_by(|a, b| a.updated_at.cmp(&b.updated_at));

        let primary = items.remove(0);
        let mut merged = primary.clone();

        // Merge all duplicates into primary
        for dup in &items {
            // Merge properties
            if let Some(obj) = merged.properties.as_object_mut() {
                if let Some(dup_props) = dup.properties.as_object() {
                    for (k, v) in dup_props {
                        if !obj.contains_key(k) {
                            obj.insert(k.clone(), v.clone());
                        }
                    }
                }
            }

            // Merge evidence
            for e in &dup.evidence {
                if !merged.evidence.contains(e) {
                    merged.evidence.push(e.clone());
                }
            }

            // Use longer description
            if let Some(ref dup_desc) = dup.description {
                match &merged.description {
                    Some(existing) if existing.len() < dup_desc.len() => {
                        merged.description = Some(dup_desc.clone());
                    }
                    None => {
                        merged.description = Some(dup_desc.clone());
                    }
                    _ => {}
                }
            }
        }

        merged.updated_at = now.clone();
        book_db.update_setting(&merged).map_err(|e| e.to_string())?;

        // Delete duplicates
        for dup in &items {
            book_db.delete_setting(&dup.id).map_err(|e| e.to_string())?;
            merged_count += 1;
        }
    }

    Ok(merged_count)
}

// ============================================================================
// Character Role Management
// ============================================================================

/// Result of auto-updating character roles
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleUpdateResult {
    pub updated_count: u32,
    pub protagonist_count: u32,
    pub major_count: u32,
    pub supporting_count: u32,
    pub minor_count: u32,
}

/// Auto-update character roles based on chapter appearance frequency
///
/// Role assignment logic:
/// - protagonist: appears in >= 50% of analyzed chapters AND is in top 2 by appearance
/// - major: appears in >= 20% of analyzed chapters OR is in top 3-5 by appearance
/// - supporting: appears in 2+ chapters
/// - minor: appears in only 1 chapter
#[tauri::command]
pub async fn auto_update_character_roles(book_id: String) -> Result<RoleUpdateResult, String> {
    let library = Library::open().map_err(|e| e.to_string())?;
    let bid = BookId::from_string(book_id);

    let book_dir = library.book_dir(&bid);
    let book_db_path = book_dir.join("book.db");

    if !book_db_path.exists() {
        return Err("Book database not found".to_string());
    }

    let book_db = BookDb::open(&book_db_path, bid).map_err(|e| e.to_string())?;

    // Get total analyzed chapters count
    let chapters = book_db.list_chapters().map_err(|e| e.to_string())?;
    let analyzed_count = chapters.iter().filter(|c| c.analyzed).count() as u32;

    if analyzed_count == 0 {
        return Err("No analyzed chapters found".to_string());
    }

    // Get character appearance counts
    let appearance_counts = book_db.count_character_chapter_appearances().map_err(|e| e.to_string())?;

    // Get all characters
    let characters = book_db.list_characters().map_err(|e| e.to_string())?;

    // Build sorted list by appearance count
    let mut char_appearances: Vec<(String, u32)> = characters
        .iter()
        .map(|c| {
            let count = appearance_counts.get(&c.id.to_string()).copied().unwrap_or(0);
            (c.id.to_string(), count)
        })
        .collect();

    // Sort by appearance count descending
    char_appearances.sort_by(|a, b| b.1.cmp(&a.1));

    // Calculate thresholds
    let protagonist_threshold = (analyzed_count as f32 * 0.5).ceil() as u32;
    let major_threshold = (analyzed_count as f32 * 0.2).ceil() as u32;

    // Build rank map
    let mut rank_map: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for (rank, (char_id, _)) in char_appearances.iter().enumerate() {
        rank_map.insert(char_id.clone(), rank + 1);
    }

    let mut updated_count = 0u32;
    let mut protagonist_count = 0u32;
    let mut major_count = 0u32;
    let mut supporting_count = 0u32;
    let mut minor_count = 0u32;

    for character in &characters {
        let char_id = character.id.to_string();
        let appearances = appearance_counts.get(&char_id).copied().unwrap_or(0);
        let rank = rank_map.get(&char_id).copied().unwrap_or(999);

        // Determine new role
        let new_role = if appearances >= protagonist_threshold && rank <= 2 {
            "protagonist"
        } else if appearances >= major_threshold || (rank >= 3 && rank <= 5 && appearances > 0) {
            "major"
        } else if appearances >= 2 {
            "supporting"
        } else {
            "minor"
        };

        // Update if role changed
        if character.role != new_role {
            book_db
                .update_character_role(&character.id, new_role)
                .map_err(|e| e.to_string())?;
            updated_count += 1;
        }

        // Count by role
        match new_role {
            "protagonist" => protagonist_count += 1,
            "major" => major_count += 1,
            "supporting" => supporting_count += 1,
            _ => minor_count += 1,
        }
    }

    tracing::info!(
        "Auto-updated character roles: {} updated, {} protagonist, {} major, {} supporting, {} minor",
        updated_count, protagonist_count, major_count, supporting_count, minor_count
    );

    Ok(RoleUpdateResult {
        updated_count,
        protagonist_count,
        major_count,
        supporting_count,
        minor_count,
    })
}

/// Manually update a single character's role
#[tauri::command]
pub async fn update_character_role(
    book_id: String,
    character_id: String,
    role: String,
) -> Result<bool, String> {
    // Validate role
    let valid_roles = ["protagonist", "major", "supporting", "minor"];
    if !valid_roles.contains(&role.as_str()) {
        return Err(format!(
            "Invalid role '{}'. Valid roles: {:?}",
            role, valid_roles
        ));
    }

    let book_db = open_book_db(&book_id)?;
    let eid = EntityId::from_string(character_id);

    book_db
        .update_character_role(&eid, &role)
        .map_err(|e| e.to_string())?;

    Ok(true)
}
