// Book ingestion module
// Handles importing novels from TXT and EPUB files

pub mod embedding_task;
pub mod epub_parser;
pub mod parser;
pub mod segmentation;

use crate::core::book::{Book, BookStatus};
use crate::core::ids::{BookId, ChapterId};
use crate::storage::book_db::{BookDb, Chapter, ChapterInsert};
use crate::storage::library::Library;
use chrono::Utc;
use std::path::Path;
use thiserror::Error;

pub use embedding_task::{generate_embeddings_for_book, generate_embeddings_for_chapter};
pub use epub_parser::{extract_cover, parse_epub, EpubMetadata};
pub use parser::{detect_encoding, read_file_with_encoding};
pub use segmentation::{segment_chapters, ChapterSegment};

#[derive(Debug, Error)]
pub enum ImportError {
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Library error: {0}")]
    LibraryError(#[from] crate::storage::library::LibraryError),

    #[error("Book database error: {0}")]
    BookDbError(#[from] crate::storage::book_db::BookDbError),

    #[error("File not found: {0}")]
    FileNotFound(String),

    #[error("Invalid file format: {0}")]
    InvalidFormat(String),

    #[error("EPUB parsing error: {0}")]
    EpubError(String),

    #[error("Encoding detection failed")]
    EncodingError,

    #[error("No chapters found in file")]
    NoChaptersFound,
}

/// Import options
#[derive(Debug, Clone, Default)]
pub struct ImportOptions {
    /// Override detected title
    pub title: Option<String>,
    /// Override detected author
    pub author: Option<String>,
    /// Force specific encoding (e.g., "utf-8", "gbk")
    pub encoding: Option<String>,
    /// Custom chapter pattern regex
    pub chapter_pattern: Option<String>,
}

/// Import result with statistics
#[derive(Debug, Clone)]
pub struct ImportResult {
    pub book: Book,
    pub chapter_count: u32,
    pub total_chars: u32,
    pub encoding_detected: String,
}

/// Import a novel from a TXT or EPUB file
pub fn import_book(
    library: &Library,
    source_path: &Path,
    options: ImportOptions,
) -> Result<ImportResult, ImportError> {
    // 1. Validate source file
    if !source_path.exists() {
        return Err(ImportError::FileNotFound(
            source_path.display().to_string(),
        ));
    }

    let extension = source_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    // Dispatch based on file extension
    match extension.as_str() {
        "txt" => import_txt_book(library, source_path, options),
        "epub" => import_epub_book(library, source_path, options),
        _ => Err(ImportError::InvalidFormat(format!(
            "Unsupported file format: .{}. Supported formats: .txt, .epub",
            extension
        ))),
    }
}

/// Import a TXT file
fn import_txt_book(
    library: &Library,
    source_path: &Path,
    options: ImportOptions,
) -> Result<ImportResult, ImportError> {
    // Detect encoding and read file
    let (content, encoding) = if let Some(enc) = &options.encoding {
        let content = read_file_with_encoding(source_path, enc)?;
        (content, enc.clone())
    } else {
        let encoding = detect_encoding(source_path)?;
        let content = read_file_with_encoding(source_path, &encoding)?;
        (content, encoding)
    };

    // Extract title from filename if not provided
    let title = options.title.unwrap_or_else(|| {
        source_path
            .file_stem()
            .and_then(|s| s.to_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| "未命名".to_string())
    });

    // Segment into chapters
    let segments = if let Some(pattern) = &options.chapter_pattern {
        segmentation::segment_chapters_with_pattern(&content, pattern)
    } else {
        segmentation::segment_chapters(&content)
    };

    if segments.is_empty() {
        return Err(ImportError::NoChaptersFound);
    }

    // Create and store book
    store_book(library, title, options.author, None, segments, encoding)
}

/// Import an EPUB file
fn import_epub_book(
    library: &Library,
    source_path: &Path,
    options: ImportOptions,
) -> Result<ImportResult, ImportError> {
    // Parse EPUB file
    tracing::debug!("Importing EPUB file: {:?}", source_path);
    let (metadata, segments) = epub_parser::parse_epub(source_path)
        .map_err(|e| ImportError::EpubError(e.to_string()))?;

    tracing::debug!("EPUB parsed: {} chapters found", segments.len());
    for (i, seg) in segments.iter().enumerate() {
        tracing::debug!(
            "  Chapter {}: title={:?}, parent={:?}, chars={}",
            i,
            seg.title,
            seg.parent_title,
            seg.char_count
        );
    }

    if segments.is_empty() {
        return Err(ImportError::NoChaptersFound);
    }

    // Use provided options or fall back to EPUB metadata
    let title = options.title.or(metadata.title).unwrap_or_else(|| {
        source_path
            .file_stem()
            .and_then(|s| s.to_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| "未命名".to_string())
    });

    let author = options.author.or(metadata.author);

    // Try to extract cover
    let cover_data = match epub_parser::extract_cover(source_path) {
        Ok(Some(data)) => {
            tracing::debug!("Extracted EPUB cover");
            Some(data)
        }
        Ok(None) => {
            tracing::debug!("No cover found in EPUB");
            None
        }
        Err(e) => {
            tracing::debug!("Failed to extract EPUB cover: {}", e);
            None
        }
    };

    // Create and store book
    store_book(library, title, author, cover_data, segments, "EPUB".to_string())
}

/// Common book storage logic
fn store_book(
    library: &Library,
    title: String,
    author: Option<String>,
    cover_data: Option<(Vec<u8>, String)>,
    segments: Vec<ChapterSegment>,
    encoding: String,
) -> Result<ImportResult, ImportError> {
    let book_id = BookId::new();
    let now = Utc::now();

    let total_chars: u32 = segments.iter().map(|s| s.char_count).sum();

    // Save cover if provided
    let cover_path = if let Some((data, mime)) = cover_data {
        let ext = match mime.as_str() {
            "image/png" => "png",
            "image/jpeg" => "jpg",
            "image/gif" => "gif",
            "image/webp" => "webp",
            _ => "bin",
        };
        let book_dir = library.book_dir(&book_id);
        std::fs::create_dir_all(&book_dir)?;
        let cover_file = book_dir.join(format!("cover.{}", ext));
        std::fs::write(&cover_file, &data)?;
        Some(cover_file.to_string_lossy().to_string())
    } else {
        None
    };

    let book = Book {
        id: book_id.clone(),
        title: title.clone(),
        author: author.clone(),
        cover_path,
        total_chapters: segments.len() as u32,
        analyzed_chapters: 0,
        status: BookStatus::Importing,
        created_at: now,
        updated_at: now,
    };

    // Insert into library index
    library.insert_book(&book)?;

    // Create book directory and initialize book database
    let book_dir = library.book_dir(&book_id);
    let book_db_path = book_dir.join("book.db");
    let book_db = BookDb::open(&book_db_path, book_id.clone())?;

    // Insert book record into book.db (required for foreign key constraints)
    book_db.insert_book(&book)?;

    // Build chapter inserts with content for atomic batch write
    let chapter_inserts: Vec<ChapterInsert> = segments
        .iter()
        .map(|segment| {
            let chapter_id = ChapterId::new();
            ChapterInsert {
                chapter: Chapter {
                    id: chapter_id,
                    book_id: book_id.clone(),
                    index_num: segment.index,
                    title: segment.title.clone(),
                    parent_title: segment.parent_title.clone(),
                    char_count: segment.char_count,
                    analyzed: false,
                    technique_count: 0,
                    knowledge_count: 0,
                },
                content: segment.content.clone(),
            }
        })
        .collect();

    // Store chapters + content + FTS in a single transaction
    book_db.insert_chapters_with_content_batch(&chapter_inserts)?;

    // NOTE: Embedding generation is now deferred to analysis time (lazy embedding)
    // This significantly speeds up import and avoids API rate limits
    // Embeddings will be generated on-demand when a chapter is analyzed

    // Update book status to ready
    let mut ready_book = book.clone();
    ready_book.status = BookStatus::Ready;
    ready_book.updated_at = Utc::now();
    library.update_book(&ready_book)?;

    Ok(ImportResult {
        book: ready_book,
        chapter_count: segments.len() as u32,
        total_chars,
        encoding_detected: encoding,
    })
}

/// Get import preview without actually importing
pub fn preview_import(
    source_path: &Path,
    options: &ImportOptions,
) -> Result<ImportPreview, ImportError> {
    if !source_path.exists() {
        return Err(ImportError::FileNotFound(
            source_path.display().to_string(),
        ));
    }

    let extension = source_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    match extension.as_str() {
        "txt" => preview_txt_import(source_path, options),
        "epub" => preview_epub_import(source_path, options),
        _ => Err(ImportError::InvalidFormat(format!(
            "Unsupported file format: .{}. Supported formats: .txt, .epub",
            extension
        ))),
    }
}

/// Preview TXT import
fn preview_txt_import(
    source_path: &Path,
    options: &ImportOptions,
) -> Result<ImportPreview, ImportError> {
    // Detect encoding and read file
    let (content, encoding) = if let Some(enc) = &options.encoding {
        let content = read_file_with_encoding(source_path, enc)?;
        (content, enc.clone())
    } else {
        let encoding = detect_encoding(source_path)?;
        let content = read_file_with_encoding(source_path, &encoding)?;
        (content, encoding)
    };

    // Extract title
    let title = options.title.clone().unwrap_or_else(|| {
        source_path
            .file_stem()
            .and_then(|s| s.to_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| "未命名".to_string())
    });

    // Segment chapters
    let segments = if let Some(pattern) = &options.chapter_pattern {
        segmentation::segment_chapters_with_pattern(&content, pattern)
    } else {
        segmentation::segment_chapters(&content)
    };

    let chapter_previews: Vec<ChapterPreview> = segments
        .iter()
        .map(|s| ChapterPreview {
            index: s.index,
            title: s.title.clone(),
            char_count: s.char_count,
            preview: s.content.chars().take(100).collect(),
        })
        .collect();

    let total_chars: u32 = segments.iter().map(|s| s.char_count).sum();

    Ok(ImportPreview {
        title,
        author: options.author.clone(),
        chapter_count: segments.len() as u32,
        total_chars,
        encoding_detected: encoding,
        chapters: chapter_previews,
    })
}

/// Preview EPUB import
fn preview_epub_import(
    source_path: &Path,
    options: &ImportOptions,
) -> Result<ImportPreview, ImportError> {
    // Parse EPUB file
    tracing::debug!("Previewing EPUB file: {:?}", source_path);
    let (metadata, segments) = epub_parser::parse_epub(source_path)
        .map_err(|e| ImportError::EpubError(e.to_string()))?;

    tracing::debug!("EPUB preview parsed: {} chapters found", segments.len());
    for (i, seg) in segments.iter().enumerate() {
        tracing::debug!(
            "  Preview Chapter {}: title={:?}, parent={:?}, chars={}",
            i,
            seg.title,
            seg.parent_title,
            seg.char_count
        );
    }

    // Use provided options or fall back to EPUB metadata
    let title = options.title.clone().or(metadata.title).unwrap_or_else(|| {
        source_path
            .file_stem()
            .and_then(|s| s.to_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| "未命名".to_string())
    });

    let author = options.author.clone().or(metadata.author);

    let chapter_previews: Vec<ChapterPreview> = segments
        .iter()
        .map(|s| ChapterPreview {
            index: s.index,
            title: s.title.clone(),
            char_count: s.char_count,
            preview: s.content.chars().take(100).collect(),
        })
        .collect();

    let total_chars: u32 = segments.iter().map(|s| s.char_count).sum();

    Ok(ImportPreview {
        title,
        author,
        chapter_count: segments.len() as u32,
        total_chars,
        encoding_detected: "EPUB".to_string(),
        chapters: chapter_previews,
    })
}

#[derive(Debug, Clone)]
pub struct ImportPreview {
    pub title: String,
    pub author: Option<String>,
    pub chapter_count: u32,
    pub total_chars: u32,
    pub encoding_detected: String,
    pub chapters: Vec<ChapterPreview>,
}

#[derive(Debug, Clone)]
pub struct ChapterPreview {
    pub index: u32,
    pub title: Option<String>,
    pub char_count: u32,
    pub preview: String,
}
