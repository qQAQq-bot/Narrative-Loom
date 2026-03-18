use crate::core::ids::{BookId, ChapterId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Book {
    pub id: BookId,
    pub title: String,
    pub author: Option<String>,
    pub cover_path: Option<String>,
    pub total_chapters: u32,
    pub analyzed_chapters: u32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub status: BookStatus,
}

impl Book {
    pub fn new(title: String, author: Option<String>) -> Self {
        let now = Utc::now();
        Self {
            id: BookId::new(),
            title,
            author,
            cover_path: None,
            total_chapters: 0,
            analyzed_chapters: 0,
            created_at: now,
            updated_at: now,
            status: BookStatus::Importing,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BookStatus {
    Importing,
    Ready,
    Analyzing,
    Completed,
    Error(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chapter {
    pub id: ChapterId,
    pub book_id: BookId,
    pub index: u32,
    pub title: Option<String>,
    pub parent_title: Option<String>,
    pub char_count: u32,
    pub analyzed: bool,
    pub technique_count: u32,
    pub knowledge_count: u32,
}

impl Chapter {
    pub fn new(book_id: BookId, index: u32, title: Option<String>, parent_title: Option<String>, char_count: u32) -> Self {
        Self {
            id: ChapterId::new(),
            book_id,
            index,
            title,
            parent_title,
            char_count,
            analyzed: false,
            technique_count: 0,
            knowledge_count: 0,
        }
    }
}
