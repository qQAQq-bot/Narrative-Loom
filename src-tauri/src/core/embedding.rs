use serde::{Deserialize, Serialize};

/// Default embedding dimensions used throughout the app.
///
/// Note: This is a project-level assumption today. Some embedding providers/models
/// can return other dimensions; callers should treat mismatches as a configuration
/// issue (and ideally warn) rather than silently degrading retrieval quality.
pub const DEFAULT_EMBEDDING_DIMENSIONS: u32 = 1024;
pub const DEFAULT_EMBEDDING_DIMENSIONS_USIZE: usize = DEFAULT_EMBEDDING_DIMENSIONS as usize;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ChunkId(pub String);

impl ChunkId {
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }

    pub fn from_string(s: String) -> Self {
        Self(s)
    }
}

impl Default for ChunkId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for ChunkId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChunkType {
    Paragraph,
    EntityDescription,
    EventSummary,
    RelationshipNote,
}

impl ChunkType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ChunkType::Paragraph => "paragraph",
            ChunkType::EntityDescription => "entity",
            ChunkType::EventSummary => "event",
            ChunkType::RelationshipNote => "relationship",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "paragraph" => Some(ChunkType::Paragraph),
            "entity" => Some(ChunkType::EntityDescription),
            "event" => Some(ChunkType::EventSummary),
            "relationship" => Some(ChunkType::RelationshipNote),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkMetadata {
    pub entities_mentioned: Vec<String>,
    pub timestamp_marker: Option<String>,
}

impl Default for ChunkMetadata {
    fn default() -> Self {
        Self {
            entities_mentioned: Vec::new(),
            timestamp_marker: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chunk {
    pub id: ChunkId,
    pub chapter_id: String,
    pub chunk_index: u32,
    pub chunk_type: ChunkType,
    pub content: String,
    pub char_start: u32,
    pub char_end: u32,
    pub metadata: ChunkMetadata,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl Chunk {
    pub fn new_paragraph(
        chapter_id: String,
        chunk_index: u32,
        content: String,
        char_start: u32,
        char_end: u32,
    ) -> Self {
        let now = chrono::Utc::now();
        Self {
            id: ChunkId::new(),
            chapter_id,
            chunk_index,
            chunk_type: ChunkType::Paragraph,
            content,
            char_start,
            char_end,
            metadata: ChunkMetadata::default(),
            created_at: now,
            updated_at: now,
        }
    }

    pub fn new_entity(
        chapter_id: String,
        entity_id: String,
        content: String,
    ) -> Self {
        let now = chrono::Utc::now();
        Self {
            id: ChunkId::new(),
            chapter_id,
            chunk_index: 0,
            chunk_type: ChunkType::EntityDescription,
            content,
            char_start: 0,
            char_end: 0,
            metadata: ChunkMetadata {
                entities_mentioned: vec![entity_id],
                timestamp_marker: None,
            },
            created_at: now,
            updated_at: now,
        }
    }

    /// Set the entities mentioned in this chunk
    pub fn with_entities(mut self, entity_ids: Vec<String>) -> Self {
        self.metadata.entities_mentioned = entity_ids;
        self
    }
}

#[derive(Debug, Clone)]
pub struct VectorEntry {
    pub chunk_id: ChunkId,
    pub embedding: Vec<f32>,
}

impl VectorEntry {
    pub fn new(chunk_id: ChunkId, embedding: Vec<f32>) -> Self {
        Self { chunk_id, embedding }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimilarChunk {
    pub chunk: Chunk,
    pub score: f32,
}
