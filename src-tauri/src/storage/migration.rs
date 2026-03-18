// Database migration system (P1-014)
// Handles schema versioning and migrations for both library.db and book.db

use rusqlite::{Connection, params, OptionalExtension};
use thiserror::Error;

/// Current schema version for library.db
pub const LIBRARY_SCHEMA_VERSION: u32 = 1;

/// Current schema version for book.db
pub const BOOK_SCHEMA_VERSION: u32 = 5;

#[derive(Debug, Error)]
pub enum MigrationError {
    #[error("Database error: {0}")]
    DatabaseError(#[from] rusqlite::Error),

    #[error("Migration failed: {0}")]
    MigrationFailed(String),

    #[error("Unknown schema version: {0}")]
    UnknownVersion(u32),

    #[error("Cannot downgrade from version {0} to {1}")]
    CannotDowngrade(u32, u32),
}

/// Get the current schema version from a database
pub fn get_schema_version(conn: &Connection) -> Result<u32, MigrationError> {
    // First, ensure the schema_version table exists
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS schema_version (
            version INTEGER NOT NULL,
            applied_at TEXT NOT NULL
        );
        "#
    )?;

    // Get the current version
    let version: Option<u32> = conn.query_row(
        "SELECT version FROM schema_version ORDER BY version DESC LIMIT 1",
        [],
        |row| row.get(0)
    ).optional()?;

    Ok(version.unwrap_or(0))
}

/// Set the schema version in the database
fn set_schema_version(conn: &Connection, version: u32) -> Result<(), MigrationError> {
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO schema_version (version, applied_at) VALUES (?1, ?2)",
        params![version, now]
    )?;
    Ok(())
}

/// Migrate the library database to the current version
pub fn migrate_library(conn: &Connection) -> Result<(), MigrationError> {
    let current = get_schema_version(conn)?;

    if current > LIBRARY_SCHEMA_VERSION {
        return Err(MigrationError::CannotDowngrade(current, LIBRARY_SCHEMA_VERSION));
    }

    if current == LIBRARY_SCHEMA_VERSION {
        return Ok(());
    }

    tracing::info!(
        "Migrating library.db from version {} to {}",
        current,
        LIBRARY_SCHEMA_VERSION
    );

    // Run migrations sequentially
    for version in (current + 1)..=LIBRARY_SCHEMA_VERSION {
        migrate_library_to_version(conn, version)?;
        set_schema_version(conn, version)?;
        tracing::info!("Library migration to version {} complete", version);
    }

    Ok(())
}

/// Migrate the book database to the current version
pub fn migrate_book(conn: &Connection) -> Result<(), MigrationError> {
    let current = get_schema_version(conn)?;

    if current > BOOK_SCHEMA_VERSION {
        return Err(MigrationError::CannotDowngrade(current, BOOK_SCHEMA_VERSION));
    }

    if current == BOOK_SCHEMA_VERSION {
        return Ok(());
    }

    tracing::info!(
        "Migrating book.db from version {} to {}",
        current,
        BOOK_SCHEMA_VERSION
    );

    // Run migrations sequentially
    for version in (current + 1)..=BOOK_SCHEMA_VERSION {
        migrate_book_to_version(conn, version)?;
        set_schema_version(conn, version)?;
        tracing::info!("Book migration to version {} complete", version);
    }

    Ok(())
}

/// Run a specific library migration
fn migrate_library_to_version(_conn: &Connection, version: u32) -> Result<(), MigrationError> {
    match version {
        1 => {
            // Initial schema - nothing to migrate, just mark as version 1
            // The CREATE TABLE IF NOT EXISTS statements handle initial creation
            Ok(())
        }
        // Add future migrations here:
        // 2 => migrate_library_v2(conn),
        // 3 => migrate_library_v3(conn),
        _ => Err(MigrationError::UnknownVersion(version)),
    }
}

/// Run a specific book migration
fn migrate_book_to_version(conn: &Connection, version: u32) -> Result<(), MigrationError> {
    match version {
        1 => {
            // Initial schema - nothing to migrate, just mark as version 1
            Ok(())
        }
        2 => {
            // Add parent_title column for volume/chapter hierarchy support
            migrate_book_v2(conn)
        }
        3 => {
            // Add structured description fields for characters and settings
            migrate_book_v3(conn)
        }
        4 => {
            // Normalize knowledge_cards.status: 'confirmed' -> 'accepted'
            migrate_book_v4(conn)
        }
        5 => {
            // Add style_profile and style_observations tables
            migrate_book_v5(conn)
        }
        _ => Err(MigrationError::UnknownVersion(version)),
    }
}

/// Migration v2: Add parent_title column to chapters table
fn migrate_book_v2(conn: &Connection) -> Result<(), MigrationError> {
    // Check if parent_title column already exists (for databases created with new schema)
    let has_column: bool = conn
        .query_row(
            "SELECT COUNT(*) > 0 FROM pragma_table_info('chapters') WHERE name = 'parent_title'",
            [],
            |row| row.get(0),
        )
        .unwrap_or(false);

    if !has_column {
        // Add parent_title column for volume/chapter hierarchy
        conn.execute_batch(r#"
            ALTER TABLE chapters ADD COLUMN parent_title TEXT;
        "#)?;
        tracing::info!("Added parent_title column to chapters table");
    } else {
        tracing::info!("parent_title column already exists, skipping ALTER TABLE");
    }
    Ok(())
}

/// Migration v3: Add structured description fields for characters and settings
/// This enables accumulated descriptions across chapters with dimension-based storage
fn migrate_book_v3(conn: &Connection) -> Result<(), MigrationError> {
    // Check if description_structured_json column already exists in characters
    let has_char_column: bool = conn
        .query_row(
            "SELECT COUNT(*) > 0 FROM pragma_table_info('characters') WHERE name = 'description_structured_json'",
            [],
            |row| row.get(0),
        )
        .unwrap_or(false);

    if !has_char_column {
        // Add structured description field to characters table
        conn.execute_batch(r#"
            ALTER TABLE characters ADD COLUMN description_structured_json TEXT;
        "#)?;
        tracing::info!("Added description_structured_json column to characters table");
    } else {
        tracing::info!("characters.description_structured_json already exists, skipping ALTER TABLE");
    }

    // Check if description_structured_json column already exists in settings
    let has_setting_column: bool = conn
        .query_row(
            "SELECT COUNT(*) > 0 FROM pragma_table_info('settings') WHERE name = 'description_structured_json'",
            [],
            |row| row.get(0),
        )
        .unwrap_or(false);

    if !has_setting_column {
        // Add structured description field to settings table
        conn.execute_batch(r#"
            ALTER TABLE settings ADD COLUMN description_structured_json TEXT;
        "#)?;
        tracing::info!("Added description_structured_json column to settings table");
    } else {
        tracing::info!("settings.description_structured_json already exists, skipping ALTER TABLE");
    }

    Ok(())
}

fn migrate_book_v4(conn: &Connection) -> Result<(), MigrationError> {
    let updated = conn.execute(
        "UPDATE knowledge_cards SET status = 'accepted' WHERE status = 'confirmed'",
        [],
    )?;
    if updated > 0 {
        tracing::info!("Migrated {} knowledge_cards from 'confirmed' to 'accepted'", updated);
    }
    Ok(())
}

/// Migration v5: Add style_profile and style_observations tables
/// For storing writing style fingerprint data
fn migrate_book_v5(conn: &Connection) -> Result<(), MigrationError> {
    // Check if style_profile table already exists
    let has_style_profile: bool = conn
        .query_row(
            "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name='style_profile'",
            [],
            |row| row.get(0),
        )
        .unwrap_or(false);

    if !has_style_profile {
        conn.execute_batch(r#"
            CREATE TABLE style_profile (
                id TEXT PRIMARY KEY DEFAULT 'main',
                version TEXT NOT NULL DEFAULT '1.0',
                profile_json TEXT NOT NULL,
                analyzed_chapters INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
        "#)?;
        tracing::info!("Created style_profile table");
    } else {
        tracing::info!("style_profile table already exists, skipping");
    }

    // Check if style_observations table already exists
    let has_style_observations: bool = conn
        .query_row(
            "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name='style_observations'",
            [],
            |row| row.get(0),
        )
        .unwrap_or(false);

    if !has_style_observations {
        conn.execute_batch(r#"
            CREATE TABLE style_observations (
                id TEXT PRIMARY KEY,
                chapter_id TEXT NOT NULL,
                observation_json TEXT NOT NULL,
                created_at TEXT NOT NULL,
                FOREIGN KEY (chapter_id) REFERENCES chapters(id) ON DELETE CASCADE
            );

            CREATE INDEX IF NOT EXISTS idx_style_observations_chapter
                ON style_observations(chapter_id);
        "#)?;
        tracing::info!("Created style_observations table and index");
    } else {
        tracing::info!("style_observations table already exists, skipping");
    }

    Ok(())
}

// Example future migration functions (commented out for reference):
/*
fn migrate_library_v2(conn: &Connection) -> Result<(), MigrationError> {
    // Example: Add a new column to the books table
    conn.execute_batch(r#"
        ALTER TABLE books ADD COLUMN language TEXT;
        ALTER TABLE books ADD COLUMN genre TEXT;
    "#)?;
    Ok(())
}

fn migrate_book_v2(conn: &Connection) -> Result<(), MigrationError> {
    // Example: Add a new table or modify existing ones
    conn.execute_batch(r#"
        CREATE TABLE IF NOT EXISTS reading_progress (
            chapter_id TEXT PRIMARY KEY,
            position INTEGER NOT NULL DEFAULT 0,
            last_read_at TEXT NOT NULL,
            FOREIGN KEY (chapter_id) REFERENCES chapters(id) ON DELETE CASCADE
        );
    "#)?;
    Ok(())
}
*/

/// Check if database needs migration
pub fn needs_library_migration(conn: &Connection) -> Result<bool, MigrationError> {
    let current = get_schema_version(conn)?;
    Ok(current < LIBRARY_SCHEMA_VERSION)
}

/// Check if book database needs migration
pub fn needs_book_migration(conn: &Connection) -> Result<bool, MigrationError> {
    let current = get_schema_version(conn)?;
    Ok(current < BOOK_SCHEMA_VERSION)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    #[test]
    fn test_get_schema_version_new_db() {
        let conn = Connection::open_in_memory().unwrap();
        let version = get_schema_version(&conn).unwrap();
        assert_eq!(version, 0);
    }

    #[test]
    fn test_set_and_get_schema_version() {
        let conn = Connection::open_in_memory().unwrap();
        get_schema_version(&conn).unwrap(); // Initialize table
        set_schema_version(&conn, 1).unwrap();
        let version = get_schema_version(&conn).unwrap();
        assert_eq!(version, 1);
    }

    #[test]
    fn test_migrate_library() {
        let conn = Connection::open_in_memory().unwrap();
        migrate_library(&conn).unwrap();
        let version = get_schema_version(&conn).unwrap();
        assert_eq!(version, LIBRARY_SCHEMA_VERSION);
    }

    #[test]
    fn test_migrate_book() {
        let conn = Connection::open_in_memory().unwrap();
        migrate_book(&conn).unwrap();
        let version = get_schema_version(&conn).unwrap();
        assert_eq!(version, BOOK_SCHEMA_VERSION);
    }

    #[test]
    fn test_idempotent_migration() {
        let conn = Connection::open_in_memory().unwrap();
        migrate_library(&conn).unwrap();
        migrate_library(&conn).unwrap(); // Should not fail on second run
        let version = get_schema_version(&conn).unwrap();
        assert_eq!(version, LIBRARY_SCHEMA_VERSION);
    }
}
