// Technique Library commands
// Provides API for the new technique library system

use crate::core::ids::EntityId;
use crate::storage::book_db::{TechniqueType, TechniqueTypeWithExamples};
use serde::{Deserialize, Serialize};
use super::common::open_book_db;

#[derive(Debug, Serialize)]
pub struct TechniqueLibraryStats {
    pub type_count: u32,
    pub example_count: u32,
    pub featured_count: u32,
}

#[derive(Debug, Deserialize)]
pub struct UpdateTechniqueTypeRequest {
    pub id: String,
    pub name: String,
    pub category: String,
    pub description: Option<String>,
    pub principle: Option<String>,
}

/// Get all technique types for a book
#[tauri::command]
pub fn get_technique_types(book_id: String) -> Result<Vec<TechniqueType>, String> {
    let db = open_book_db(&book_id)?;
    db.list_technique_types().map_err(|e| e.to_string())
}

/// Get all technique types with their examples
#[tauri::command]
pub fn get_technique_library(book_id: String) -> Result<Vec<TechniqueTypeWithExamples>, String> {
    let db = open_book_db(&book_id)?;
    db.list_technique_types_with_examples()
        .map_err(|e| e.to_string())
}

/// Get a single technique type with its examples
#[tauri::command]
pub fn get_technique_type_with_examples(
    book_id: String,
    type_id: String,
) -> Result<Option<TechniqueTypeWithExamples>, String> {
    let db = open_book_db(&book_id)?;

    let type_entity_id = EntityId::from_string(type_id);
    let tt = db
        .get_technique_type(&type_entity_id)
        .map_err(|e| e.to_string())?;

    if let Some(tt) = tt {
        let examples = db
            .list_technique_examples(&type_entity_id)
            .map_err(|e| e.to_string())?;
        Ok(Some(TechniqueTypeWithExamples {
            technique_type: tt,
            examples,
        }))
    } else {
        Ok(None)
    }
}

/// Update a technique type
#[tauri::command]
pub fn update_technique_type(
    book_id: String,
    request: UpdateTechniqueTypeRequest,
) -> Result<(), String> {
    let db = open_book_db(&book_id)?;

    let type_entity_id = EntityId::from_string(request.id);
    let existing = db
        .get_technique_type(&type_entity_id)
        .map_err(|e| e.to_string())?
        .ok_or("Technique type not found")?;

    let updated = TechniqueType {
        id: type_entity_id,
        name: request.name,
        category: request.category,
        description: request.description,
        principle: request.principle,
        example_count: existing.example_count,
        created_at: existing.created_at,
        updated_at: chrono::Utc::now().to_rfc3339(),
    };

    db.update_technique_type(&updated)
        .map_err(|e| e.to_string())
}

/// Delete a technique type
#[tauri::command]
pub fn delete_technique_type(book_id: String, type_id: String) -> Result<bool, String> {
    let db = open_book_db(&book_id)?;

    let type_entity_id = EntityId::from_string(type_id);
    db.delete_technique_type(&type_entity_id)
        .map_err(|e| e.to_string())
}

/// Toggle featured status of a technique example
#[tauri::command]
pub fn toggle_example_featured(
    book_id: String,
    example_id: String,
    featured: bool,
) -> Result<(), String> {
    let db = open_book_db(&book_id)?;

    let example_entity_id = EntityId::from_string(example_id);
    db.toggle_technique_example_featured(&example_entity_id, featured)
        .map_err(|e| e.to_string())
}

/// Delete a technique example
#[tauri::command]
pub fn delete_technique_example(book_id: String, example_id: String) -> Result<bool, String> {
    let db = open_book_db(&book_id)?;

    let example_entity_id = EntityId::from_string(example_id);
    db.delete_technique_example(&example_entity_id)
        .map_err(|e| e.to_string())
}

/// Get technique library statistics
#[tauri::command]
pub fn get_technique_library_stats(book_id: String) -> Result<TechniqueLibraryStats, String> {
    let db = open_book_db(&book_id)?;

    let types = db.list_technique_types().map_err(|e| e.to_string())?;

    let mut example_count = 0u32;
    let mut featured_count = 0u32;

    for tt in &types {
        let examples = db
            .list_technique_examples(&tt.id)
            .map_err(|e| e.to_string())?;
        example_count += examples.len() as u32;
        featured_count += examples.iter().filter(|e| e.example.is_featured).count() as u32;
    }

    Ok(TechniqueLibraryStats {
        type_count: types.len() as u32,
        example_count,
        featured_count,
    })
}

/// Clear entire technique library (all types and examples)
#[tauri::command]
pub fn clear_technique_library(book_id: String) -> Result<u32, String> {
    let db = open_book_db(&book_id)?;
    db.delete_all_technique_library().map_err(|e| e.to_string())
}

