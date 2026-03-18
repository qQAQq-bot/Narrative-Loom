# Commands Module

This module contains all Tauri commands exposed to the frontend.

## Modules

### `analysis.rs`
Chapter analysis commands for extracting technique cards and knowledge cards from novel text using AI agents.

### `bible.rs`
Story Bible management commands for characters, settings, and events.

### `chapter.rs`
Chapter navigation and content retrieval commands.

### `dedup.rs`
Entity deduplication module that provides intelligent name matching:
- Combo name detection (names with "&", "和", "与", "、")
- Containment detection (one name is substring of another)
- Similarity detection using Jaro-Winkler algorithm (threshold: 0.8 for manual, 0.85 for auto)
- List marker stripping (e.g., "~1", "#2", "(3)")

Key functions:
- `is_combo_name()` - Check if a name represents multiple entities
- `check_entity_dedup()` - Check a new name against existing entities
- `filter_entity_names()` - Filter a batch of names for duplicates
- `normalize_name()` - Normalize names for comparison (strips suffixes, markers, punctuation)

### `embedding.rs`
Embedding generation and vector search commands for semantic search functionality.

### `export.rs`
Export commands for story bible and style prompt generation.

### `inbox.rs`
Knowledge card inbox management with accept/reject/merge workflows. Includes:
- **Dedup integration**: Intelligent card acceptance with automatic duplicate detection
- **Structured description parsing**: Extracts dimension-based descriptions from AI analysis
- **Description accumulation**: Merges new descriptions into existing entities across chapters

Key helper functions:
- `parse_character_structured_description()` - Parse structured description from card content
- `parse_setting_structured_description()` - Parse structured description from card content
- `merge_character_structured_description()` - Merge new descriptions with existing ones
- `merge_setting_structured_description()` - Merge new descriptions with existing ones
- `get_chapter_index()` - Get chapter index for source tracking

### `library.rs`
Book library management commands (list, import, delete books).

### `search.rs`
Search commands including text search, entity search, and semantic search.
- Chapter text search now uses FTS5 full-text index (with LIKE fallback) instead of file system scanning.

### `settings.rs`
Application settings commands for AI providers, agents, and configuration.

### `technique_library.rs`
Technique library management for collected writing techniques.

## Recent Changes

### v0.3.1 - Unified Deduplication Across All Acceptance Paths
- **`accept_card_with_edits`**: Now runs `check_entity_dedup()` before exact-name matching for character/setting
- **`auto_accept_card`**: Now runs `check_entity_dedup()` with conservative threshold (0.85) for character/setting
- **`normalize_name`**: Extended to strip list markers like "~1", "#2", "(3)", "（1）"
- Variants like "费舍尔" vs "费舍尔·贝纳维德斯" and "龙人" vs "龙人种" now merge correctly across all paths

### v0.3.0 - Entity Deduplication & Structured Descriptions
- Added `dedup.rs` module for intelligent entity name deduplication
- Updated `inbox.rs` to use dedup when accepting cards
- Added support for `description_structured_json` field for characters and settings
- Database schema updated to version 3
- **Full structured description flow**: Parse AI output → Merge with existing → Store in database

### Key Features
1. **Combo Name Rejection**: Cards with names like "张三 & 李四" are auto-rejected
2. **Name Containment Merge**: "费舍尔" merges with existing "费舍尔·贝纳维德斯"
3. **Similar Name Merge**: Names with >80% Jaro-Winkler similarity are merged
4. **Structured Descriptions**: Character/Setting descriptions now support dimension-based storage for accumulated details across chapters

### Structured Description Dimensions

**Character dimensions:**
- appearance (外貌)
- personality (性格)
- background (背景)
- abilities (能力)
- goals (目标)
- status (状态)

**Setting dimensions:**
- physical (物理特征)
- atmosphere (氛围)
- history (历史)
- function (功能)
- rules (规则)
- inhabitants (居民)
- status (状态)
