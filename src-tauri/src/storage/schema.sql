-- Main book database schema
-- Each book has its own book.db file

-- Book metadata
CREATE TABLE IF NOT EXISTS book (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    author TEXT,
    cover_path TEXT,
    total_chapters INTEGER NOT NULL DEFAULT 0,
    analyzed_chapters INTEGER NOT NULL DEFAULT 0,
    status TEXT NOT NULL DEFAULT 'ready',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- Chapters
CREATE TABLE IF NOT EXISTS chapters (
    id TEXT PRIMARY KEY,
    book_id TEXT NOT NULL,
    index_num INTEGER NOT NULL,
    title TEXT,
    parent_title TEXT,
    char_count INTEGER NOT NULL DEFAULT 0,
    analyzed INTEGER NOT NULL DEFAULT 0,
    technique_count INTEGER NOT NULL DEFAULT 0,
    knowledge_count INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY (book_id) REFERENCES book(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_chapters_book ON chapters(book_id);
CREATE INDEX IF NOT EXISTS idx_chapters_index ON chapters(book_id, index_num);

-- Technique cards
CREATE TABLE IF NOT EXISTS technique_cards (
    id TEXT PRIMARY KEY,
    chapter_id TEXT NOT NULL,
    technique_type TEXT NOT NULL,
    title TEXT NOT NULL,
    description TEXT NOT NULL,
    mechanism TEXT NOT NULL,
    evidence_json TEXT NOT NULL,
    tags_json TEXT,
    collected INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    FOREIGN KEY (chapter_id) REFERENCES chapters(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_technique_cards_chapter ON technique_cards(chapter_id);
CREATE INDEX IF NOT EXISTS idx_technique_cards_type ON technique_cards(technique_type);
CREATE INDEX IF NOT EXISTS idx_technique_cards_collected ON technique_cards(collected);

-- Technique Library: Technique Types (consolidated technique categories)
CREATE TABLE IF NOT EXISTS technique_types (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    category TEXT NOT NULL,
    description TEXT,
    principle TEXT,
    example_count INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_technique_types_category ON technique_types(category);
CREATE INDEX IF NOT EXISTS idx_technique_types_name ON technique_types(name);

-- Technique Library: Technique Examples (specific instances from chapters)
CREATE TABLE IF NOT EXISTS technique_examples (
    id TEXT PRIMARY KEY,
    technique_type_id TEXT NOT NULL,
    chapter_id TEXT NOT NULL,
    title TEXT NOT NULL,
    description TEXT NOT NULL,
    mechanism TEXT,
    evidence_json TEXT NOT NULL,
    is_featured INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    FOREIGN KEY (technique_type_id) REFERENCES technique_types(id) ON DELETE CASCADE,
    FOREIGN KEY (chapter_id) REFERENCES chapters(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_technique_examples_type ON technique_examples(technique_type_id);
CREATE INDEX IF NOT EXISTS idx_technique_examples_chapter ON technique_examples(chapter_id);
CREATE INDEX IF NOT EXISTS idx_technique_examples_featured ON technique_examples(is_featured);

-- Knowledge cards (pending review)
CREATE TABLE IF NOT EXISTS knowledge_cards (
    id TEXT PRIMARY KEY,
    chapter_id TEXT NOT NULL,
    knowledge_type TEXT NOT NULL,
    title TEXT NOT NULL,
    content_json TEXT NOT NULL,
    evidence_json TEXT NOT NULL,
    confidence TEXT NOT NULL DEFAULT 'medium',
    status TEXT NOT NULL DEFAULT 'pending',
    created_at TEXT NOT NULL,
    FOREIGN KEY (chapter_id) REFERENCES chapters(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_knowledge_cards_chapter ON knowledge_cards(chapter_id);
CREATE INDEX IF NOT EXISTS idx_knowledge_cards_type ON knowledge_cards(knowledge_type);
CREATE INDEX IF NOT EXISTS idx_knowledge_cards_status ON knowledge_cards(status);

-- Story Bible: Characters
CREATE TABLE IF NOT EXISTS characters (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    aliases_json TEXT,
    description TEXT,
    description_structured_json TEXT,
    traits_json TEXT,
    role TEXT NOT NULL DEFAULT 'minor',
    first_appearance_chapter_id TEXT,
    relationships_json TEXT,
    evidence_json TEXT,
    notes TEXT,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (first_appearance_chapter_id) REFERENCES chapters(id)
);

CREATE INDEX IF NOT EXISTS idx_characters_name ON characters(name);
CREATE INDEX IF NOT EXISTS idx_characters_role ON characters(role);

-- Story Bible: Settings
CREATE TABLE IF NOT EXISTS settings (
    id TEXT PRIMARY KEY,
    setting_type TEXT NOT NULL,
    name TEXT NOT NULL,
    description TEXT,
    description_structured_json TEXT,
    properties_json TEXT,
    evidence_json TEXT,
    notes TEXT,
    updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_settings_type ON settings(setting_type);
CREATE INDEX IF NOT EXISTS idx_settings_name ON settings(name);

-- Story Bible: Events
CREATE TABLE IF NOT EXISTS events (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    description TEXT,
    chapter_id TEXT,
    characters_involved_json TEXT,
    importance TEXT NOT NULL DEFAULT 'normal',
    evidence_json TEXT,
    notes TEXT,
    updated_at TEXT NOT NULL,
    -- Time-related fields (merged from timeline)
    time_marker TEXT,
    order_in_chapter INTEGER DEFAULT 0,
    is_flashback INTEGER NOT NULL DEFAULT 0,
    relative_time TEXT,
    FOREIGN KEY (chapter_id) REFERENCES chapters(id)
);

CREATE INDEX IF NOT EXISTS idx_events_chapter ON events(chapter_id);
CREATE INDEX IF NOT EXISTS idx_events_importance ON events(importance);
CREATE INDEX IF NOT EXISTS idx_events_order ON events(order_in_chapter);
CREATE INDEX IF NOT EXISTS idx_events_title ON events(title);

-- Chapter contents (stored in DB instead of .txt files)
CREATE TABLE IF NOT EXISTS chapter_contents (
    chapter_id TEXT PRIMARY KEY,
    content TEXT NOT NULL,
    FOREIGN KEY (chapter_id) REFERENCES chapters(id) ON DELETE CASCADE
);

-- Full-text search index for chapter content (trigram for CJK substring matching)
CREATE VIRTUAL TABLE IF NOT EXISTS chapter_fts USING fts5(
    chapter_id UNINDEXED,
    content,
    tokenize='trigram'
);
