// Library commands for book management

use crate::core::book::{Book, BookStatus};
use crate::core::ids::BookId;
use crate::ingestion::{import_book as do_import, preview_import, ImportOptions};
use crate::storage::book_db::BookDb;
use crate::storage::library::Library;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

// Re-export types for frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookInfo {
    pub id: String,
    pub title: String,
    pub author: Option<String>,
    pub cover_path: Option<String>,
    pub total_chapters: u32,
    pub analyzed_chapters: u32,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

impl From<Book> for BookInfo {
    fn from(book: Book) -> Self {
        Self {
            id: book.id.to_string(),
            title: book.title,
            author: book.author,
            cover_path: book.cover_path,
            total_chapters: book.total_chapters,
            analyzed_chapters: book.analyzed_chapters,
            status: format_status(&book.status),
            created_at: book.created_at.to_rfc3339(),
            updated_at: book.updated_at.to_rfc3339(),
        }
    }
}

fn format_status(status: &BookStatus) -> String {
    match status {
        BookStatus::Importing => "importing".to_string(),
        BookStatus::Ready => "ready".to_string(),
        BookStatus::Analyzing => "analyzing".to_string(),
        BookStatus::Completed => "completed".to_string(),
        BookStatus::Error(msg) => format!("error:{}", msg),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChapterInfo {
    pub id: String,
    pub index: u32,
    pub title: Option<String>,
    pub char_count: u32,
    pub analyzed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportPreviewInfo {
    pub title: String,
    pub author: Option<String>,
    pub chapter_count: u32,
    pub total_chars: u32,
    pub encoding: String,
    pub chapters: Vec<ChapterPreviewInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChapterPreviewInfo {
    pub index: u32,
    pub title: Option<String>,
    pub char_count: u32,
    pub preview: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportResultInfo {
    pub book: BookInfo,
    pub chapter_count: u32,
    pub total_chars: u32,
    pub encoding: String,
}

/// List all books in the library
#[tauri::command]
pub async fn list_books() -> Result<Vec<BookInfo>, String> {
    let library = Library::open().map_err(|e| e.to_string())?;
    let books = library.list_books().map_err(|e| e.to_string())?;
    Ok(books.into_iter().map(BookInfo::from).collect())
}

/// Get a single book by ID
#[tauri::command]
pub async fn get_book(book_id: String) -> Result<BookInfo, String> {
    let library = Library::open().map_err(|e| e.to_string())?;
    let id = BookId::from_string(book_id.clone());

    library
        .get_book(&id)
        .map_err(|e| e.to_string())?
        .map(BookInfo::from)
        .ok_or_else(|| format!("Book not found: {}", book_id))
}

/// Preview import without actually importing
#[tauri::command]
pub async fn preview_book_import(
    path: String,
    title: Option<String>,
    author: Option<String>,
    encoding: Option<String>,
) -> Result<ImportPreviewInfo, String> {
    let source_path = PathBuf::from(&path);
    let options = ImportOptions {
        title,
        author,
        encoding,
        chapter_pattern: None,
    };

    let preview = preview_import(&source_path, &options).map_err(|e| e.to_string())?;

    Ok(ImportPreviewInfo {
        title: preview.title,
        author: preview.author,
        chapter_count: preview.chapter_count,
        total_chars: preview.total_chars,
        encoding: preview.encoding_detected,
        chapters: preview
            .chapters
            .into_iter()
            .map(|c| ChapterPreviewInfo {
                index: c.index,
                title: c.title,
                char_count: c.char_count,
                preview: c.preview,
            })
            .collect(),
    })
}

/// Import a book from file
#[tauri::command]
pub async fn import_book(
    path: String,
    title: Option<String>,
    author: Option<String>,
    encoding: Option<String>,
) -> Result<ImportResultInfo, String> {
    let library = Library::open().map_err(|e| e.to_string())?;
    let source_path = PathBuf::from(&path);
    let options = ImportOptions {
        title,
        author,
        encoding,
        chapter_pattern: None,
    };

    let result = do_import(&library, &source_path, options).map_err(|e| e.to_string())?;

    Ok(ImportResultInfo {
        book: BookInfo::from(result.book),
        chapter_count: result.chapter_count,
        total_chars: result.total_chars,
        encoding: result.encoding_detected,
    })
}

/// Delete a book and all its data
#[tauri::command]
pub async fn delete_book(book_id: String) -> Result<bool, String> {
    let library = Library::open().map_err(|e| e.to_string())?;
    let id = BookId::from_string(book_id);
    library.delete_book(&id).map_err(|e| e.to_string())
}

/// Get chapters for a book
#[tauri::command]
pub async fn get_book_chapters(book_id: String) -> Result<Vec<ChapterInfo>, String> {
    let library = Library::open().map_err(|e| e.to_string())?;
    let id = BookId::from_string(book_id);

    let book_dir = library.book_dir(&id);
    let book_db_path = book_dir.join("book.db");

    if !book_db_path.exists() {
        return Err("Book database not found".to_string());
    }

    let book_db = BookDb::open(&book_db_path, id).map_err(|e| e.to_string())?;
    let chapters = book_db.list_chapters().map_err(|e| e.to_string())?;

    Ok(chapters
        .into_iter()
        .map(|c| ChapterInfo {
            id: c.id.to_string(),
            index: c.index_num,
            title: c.title,
            char_count: c.char_count,
            analyzed: c.analyzed,
        })
        .collect())
}

/// Get chapter content
#[tauri::command]
pub async fn get_chapter_content(book_id: String, chapter_id: String) -> Result<String, String> {
    let library = Library::open().map_err(|e| e.to_string())?;
    let bid = BookId::from_string(book_id);
    let cid = crate::core::ids::ChapterId::from_string(chapter_id);

    let book_dir = library.book_dir(&bid);
    let book_db_path = book_dir.join("book.db");

    if !book_db_path.exists() {
        return Err("Book database not found".to_string());
    }

    let book_db = BookDb::open(&book_db_path, bid).map_err(|e| e.to_string())?;

    book_db
        .get_chapter_content(&cid)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Chapter content not found".to_string())
}

/// Get library statistics
#[tauri::command]
pub async fn get_library_stats() -> Result<LibraryStats, String> {
    let library = Library::open().map_err(|e| e.to_string())?;
    let books = library.list_books().map_err(|e| e.to_string())?;

    let total_books = books.len() as u32;
    let total_chapters: u32 = books.iter().map(|b| b.total_chapters).sum();

    // Calculate total words by summing char_count from all chapters
    let mut total_words: u64 = 0;
    for book in &books {
        let book_dir = library.book_dir(&book.id);
        let book_db_path = book_dir.join("book.db");
        if book_db_path.exists() {
            if let Ok(book_db) = BookDb::open(&book_db_path, book.id.clone()) {
                if let Ok(chapters) = book_db.list_chapters() {
                    total_words += chapters.iter().map(|c| c.char_count as u64).sum::<u64>();
                }
            }
        }
    }

    Ok(LibraryStats {
        total_books,
        total_chapters,
        total_words: total_words as u32,
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibraryStats {
    pub total_books: u32,
    pub total_chapters: u32,
    pub total_words: u32,
}

/// Update book metadata
#[tauri::command]
pub async fn update_book_metadata(
    book_id: String,
    title: Option<String>,
    author: Option<String>,
) -> Result<BookInfo, String> {
    let library = Library::open().map_err(|e| e.to_string())?;
    let id = BookId::from_string(book_id.clone());

    let mut book = library
        .get_book(&id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Book not found: {}", book_id))?;

    if let Some(t) = title {
        book.title = t;
    }
    if let Some(a) = author {
        book.author = Some(a);
    }
    book.updated_at = chrono::Utc::now();

    library.update_book(&book).map_err(|e| e.to_string())?;

    Ok(BookInfo::from(book))
}

/// Get book cover as base64 data URL
#[tauri::command]
pub async fn get_book_cover(book_id: String) -> Result<Option<String>, String> {
    let library = Library::open().map_err(|e| e.to_string())?;
    let id = BookId::from_string(book_id.clone());

    let book = library
        .get_book(&id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Book not found: {}", book_id))?;

    if let Some(cover_path) = book.cover_path {
        let path = std::path::PathBuf::from(&cover_path);
        if path.exists() {
            let data = std::fs::read(&path).map_err(|e| e.to_string())?;
            let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("png");
            let mime = match ext {
                "jpg" | "jpeg" => "image/jpeg",
                "png" => "image/png",
                "gif" => "image/gif",
                "webp" => "image/webp",
                _ => "image/png",
            };
            use base64::{Engine as _, engine::general_purpose::STANDARD};
            let base64_str = STANDARD.encode(&data);
            Ok(Some(format!("data:{};base64,{}", mime, base64_str)))
        } else {
            Ok(None)
        }
    } else {
        Ok(None)
    }
}
