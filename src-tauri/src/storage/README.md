# Storage Module

This module handles all data persistence for Narrative Loom.

## Modules

### `book_db.rs`
SQLite database operations for individual book databases. Each book has its own `book.db` file containing:
- Book metadata
- Chapters
- Technique cards
- Knowledge cards
- Story Bible entities (characters, settings, events)

### `chapters.rs`
Text utility functions for chapter content processing:
- `extract_paragraphs()` - Split content into paragraphs
- `count_chinese_chars()` - Count CJK characters
- `estimate_reading_time_minutes()` - Estimate reading time

> **Note:** Chapter content is now stored in SQLite (`chapter_contents` table in `book.db`) instead of individual `.txt` files. The old `ChapterFiles` struct has been removed.

### `config.rs`
Application configuration storage including:
- AI provider configurations
- Agent settings
- Task bindings
- Embedding configurations

### `keychain.rs`
Secure API key storage using system keychain.

### `library.rs`
Book library management with metadata stored in `library.db`.

### `migration.rs`
Database schema versioning and migration system. Current versions:
- Library DB: v1
- Book DB: v5

### `paths.rs`
Application path management for data directories.

### `structured_description.rs`
**NEW** - Structured description types for characters and settings:
- `CharacterStructuredDescription`: Dimension-based character info (appearance, personality, background, abilities, goals, status)
- `SettingStructuredDescription`: Dimension-based setting info (physical, atmosphere, history, function, rules, inhabitants, status)
- `DescriptionEntry`: Individual description entries with chapter source tracking

Key features:
- Accumulates descriptions across multiple chapters
- Tracks source chapter for each piece of information
- Deduplication of similar entries
- Generates plain text summary from structured data

### `vectors.rs`
Vector database operations using SQLite (`vectors.db`) for semantic search.

## Database Schema

### Book Database (v5)

Tables:
- `book` - Book metadata
- `chapters` - Chapter list with analysis status
- `chapter_contents` - Chapter full text content (replaces `.txt` files)
- `chapter_fts` - FTS5 virtual table for full-text search (trigram tokenizer for CJK)
- `technique_cards` - Writing technique cards
- `technique_types` - Consolidated technique categories
- `technique_examples` - Specific technique instances
- `knowledge_cards` - Pending knowledge cards (inbox)
- `characters` - Story Bible characters (with `description_structured_json`)
- `settings` - Story Bible settings (with `description_structured_json`)
- `events` - Story Bible events

### Library Database (v1)

Tables:
- `books` - Book metadata for library listing
- `schema_version` - Migration tracking

## Recent Changes

### Chapter Content Migration to SQLite + FTS5
- **Removed**: `ChapterFiles` struct and file-based `.txt` storage
- **Added**: `chapter_contents` table for storing chapter text in `book.db`
- **Added**: `chapter_fts` FTS5 virtual table with trigram tokenizer for CJK full-text search
- **Added**: `ChapterInsert` struct for batch import with content
- **Added**: `FtsSearchResult` struct for search results with snippets
- **New methods on `BookDb`**:
  - `insert_chapters_with_content_batch()` - Atomic batch write of chapters + content + FTS
  - `insert_chapter_content()` - Single chapter content write
  - `get_chapter_content()` - Read chapter content
  - `search_chapter_fts()` - Full-text search with FTS5 MATCH + LIKE fallback
  - `delete_chapter_fts()` - Delete FTS entry
- **Architecture**: `books/{id}/` now contains `book.db` + `vectors.db` (no more `chapters/` directory)

### v3 Migration (Structured Descriptions)
- Added `description_structured_json` column to `characters` table
- Added `description_structured_json` column to `settings` table
- Enables accumulated descriptions across chapters with dimension-based storage
