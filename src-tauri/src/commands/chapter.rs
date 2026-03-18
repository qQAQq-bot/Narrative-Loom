use crate::core::ids::ChapterId;
use crate::storage::book_db::ChapterSummary;
use serde::{Deserialize, Serialize};

use super::common::open_book;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChapterDetail {
    pub id: String,
    pub book_id: String,
    pub index: u32,
    pub title: Option<String>,
    pub parent_title: Option<String>,
    pub char_count: u32,
    pub analyzed: bool,
    pub technique_count: u32,
    pub knowledge_count: u32,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChapterListItem {
    pub id: String,
    pub index: u32,
    pub title: Option<String>,
    pub parent_title: Option<String>,
    pub char_count: u32,
    pub analyzed: bool,
}

impl From<ChapterSummary> for ChapterListItem {
    fn from(s: ChapterSummary) -> Self {
        Self {
            id: s.id.to_string(),
            index: s.index_num,
            title: s.title,
            parent_title: s.parent_title,
            char_count: s.char_count,
            analyzed: s.analyzed,
        }
    }
}

#[tauri::command]
pub async fn get_chapters(book_id: String) -> Result<Vec<ChapterListItem>, String> {
    let opened = open_book(&book_id)?;
    let chapters = opened.db.list_chapters().map_err(|e| e.to_string())?;

    Ok(chapters.into_iter().map(ChapterListItem::from).collect())
}

#[tauri::command]
pub async fn get_chapter(book_id: String, chapter_id: String) -> Result<ChapterDetail, String> {
    let cid = ChapterId::from_string(chapter_id.clone());

    let opened = open_book(&book_id)?;

    let chapter = opened.db
        .get_chapter(&cid)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Chapter not found: {}", chapter_id))?;

    let content = opened.db
        .get_chapter_content(&cid)
        .map_err(|e| e.to_string())?
        .unwrap_or_default();

    Ok(ChapterDetail {
        id: chapter.id.to_string(),
        book_id: chapter.book_id.to_string(),
        index: chapter.index_num,
        title: chapter.title,
        parent_title: chapter.parent_title,
        char_count: chapter.char_count,
        analyzed: chapter.analyzed,
        technique_count: chapter.technique_count,
        knowledge_count: chapter.knowledge_count,
        content,
    })
}

#[tauri::command]
pub async fn get_chapter_by_index(book_id: String, index: u32) -> Result<ChapterDetail, String> {
    let opened = open_book(&book_id)?;

    let chapter = opened.db
        .get_chapter_by_index(index)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Chapter not found at index: {}", index))?;

    let content = opened.db
        .get_chapter_content(&chapter.id)
        .map_err(|e| e.to_string())?
        .unwrap_or_default();

    Ok(ChapterDetail {
        id: chapter.id.to_string(),
        book_id: chapter.book_id.to_string(),
        index: chapter.index_num,
        title: chapter.title,
        parent_title: chapter.parent_title,
        char_count: chapter.char_count,
        analyzed: chapter.analyzed,
        technique_count: chapter.technique_count,
        knowledge_count: chapter.knowledge_count,
        content,
    })
}

#[tauri::command]
pub async fn get_adjacent_chapters(
    book_id: String,
    current_index: u32,
) -> Result<AdjacentChapters, String> {
    let opened = open_book(&book_id)?;

    let prev = if current_index > 1 {
        opened.db
            .get_chapter_by_index(current_index - 1)
            .ok()
            .flatten()
            .map(|c| ChapterListItem {
                id: c.id.to_string(),
                index: c.index_num,
                title: c.title,
                parent_title: c.parent_title,
                char_count: c.char_count,
                analyzed: c.analyzed,
            })
    } else {
        None
    };

    let next = opened.db
        .get_chapter_by_index(current_index + 1)
        .ok()
        .flatten()
        .map(|c| ChapterListItem {
            id: c.id.to_string(),
            index: c.index_num,
            title: c.title,
            parent_title: c.parent_title,
            char_count: c.char_count,
            analyzed: c.analyzed,
        });

    Ok(AdjacentChapters { prev, next })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdjacentChapters {
    pub prev: Option<ChapterListItem>,
    pub next: Option<ChapterListItem>,
}
