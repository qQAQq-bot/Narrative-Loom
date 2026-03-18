use crate::core::book::{Book, BookStatus};
use crate::core::ids::BookId;
use crate::storage::migration;
use crate::storage::paths;
use rusqlite::{params, Connection, OptionalExtension};
use std::fs;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum LibraryError {
    #[error("Failed to find data directory")]
    NoDataDir,

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Database error: {0}")]
    DatabaseError(#[from] rusqlite::Error),

    #[error("Migration error: {0}")]
    MigrationError(#[from] migration::MigrationError),

    #[error("Book not found: {0}")]
    BookNotFound(String),

    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("Path error: {0}")]
    PathError(#[from] paths::PathError),
}

pub struct Library {
    data_dir: PathBuf,
    index_conn: Connection,
}

impl Library {
    pub fn open() -> Result<Self, LibraryError> {
        let data_dir = paths::get_library_dir()?;
        fs::create_dir_all(&data_dir)?;

        let books_dir = data_dir.join("books");
        fs::create_dir_all(&books_dir)?;

        let index_path = data_dir.join("library.db");
        let index_conn = Connection::open(&index_path)?;

        let library = Self {
            data_dir,
            index_conn,
        };
        library.init()?;

        Ok(library)
    }

    fn init(&self) -> Result<(), LibraryError> {
        self.index_conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS books (
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

            CREATE INDEX IF NOT EXISTS idx_books_title ON books(title);
            CREATE INDEX IF NOT EXISTS idx_books_status ON books(status);
            CREATE INDEX IF NOT EXISTS idx_books_updated ON books(updated_at);
            "#,
        )?;

        // Run migrations (P1-014)
        migration::migrate_library(&self.index_conn)?;

        Ok(())
    }

    pub fn books_dir(&self) -> PathBuf {
        self.data_dir.join("books")
    }

    pub fn book_dir(&self, book_id: &BookId) -> PathBuf {
        self.books_dir().join(book_id.as_str())
    }

    pub fn list_books(&self) -> Result<Vec<Book>, LibraryError> {
        let mut stmt = self.index_conn.prepare(
            r#"
            SELECT id, title, author, cover_path, total_chapters, analyzed_chapters, 
                   status, created_at, updated_at
            FROM books
            ORDER BY updated_at DESC
            "#,
        )?;

        let books = stmt
            .query_map([], |row| {
                let id: String = row.get(0)?;
                let title: String = row.get(1)?;
                let author: Option<String> = row.get(2)?;
                let cover_path: Option<String> = row.get(3)?;
                let total_chapters: u32 = row.get(4)?;
                let analyzed_chapters: u32 = row.get(5)?;
                let status_str: String = row.get(6)?;
                let created_at_str: String = row.get(7)?;
                let updated_at_str: String = row.get(8)?;

                Ok((
                    id,
                    title,
                    author,
                    cover_path,
                    total_chapters,
                    analyzed_chapters,
                    status_str,
                    created_at_str,
                    updated_at_str,
                ))
            })?
            .filter_map(|r| r.ok())
            .map(
                |(
                    id,
                    title,
                    author,
                    cover_path,
                    total_chapters,
                    analyzed_chapters,
                    status_str,
                    created_at_str,
                    updated_at_str,
                )| {
                    let status = parse_book_status(&status_str);
                    let created_at = parse_datetime(&created_at_str);
                    let updated_at = parse_datetime(&updated_at_str);

                    Book {
                        id: BookId::from_string(id),
                        title,
                        author,
                        cover_path,
                        total_chapters,
                        analyzed_chapters,
                        status,
                        created_at,
                        updated_at,
                    }
                },
            )
            .collect();

        Ok(books)
    }

    pub fn get_book(&self, book_id: &BookId) -> Result<Option<Book>, LibraryError> {
        let mut stmt = self.index_conn.prepare(
            r#"
            SELECT id, title, author, cover_path, total_chapters, analyzed_chapters, 
                   status, created_at, updated_at
            FROM books WHERE id = ?1
            "#,
        )?;

        let book = stmt
            .query_row(params![book_id.as_str()], |row| {
                let id: String = row.get(0)?;
                let title: String = row.get(1)?;
                let author: Option<String> = row.get(2)?;
                let cover_path: Option<String> = row.get(3)?;
                let total_chapters: u32 = row.get(4)?;
                let analyzed_chapters: u32 = row.get(5)?;
                let status_str: String = row.get(6)?;
                let created_at_str: String = row.get(7)?;
                let updated_at_str: String = row.get(8)?;

                Ok((
                    id,
                    title,
                    author,
                    cover_path,
                    total_chapters,
                    analyzed_chapters,
                    status_str,
                    created_at_str,
                    updated_at_str,
                ))
            })
            .optional()?;

        match book {
            Some((
                id,
                title,
                author,
                cover_path,
                total_chapters,
                analyzed_chapters,
                status_str,
                created_at_str,
                updated_at_str,
            )) => {
                let status = parse_book_status(&status_str);
                let created_at = parse_datetime(&created_at_str);
                let updated_at = parse_datetime(&updated_at_str);

                Ok(Some(Book {
                    id: BookId::from_string(id),
                    title,
                    author,
                    cover_path,
                    total_chapters,
                    analyzed_chapters,
                    status,
                    created_at,
                    updated_at,
                }))
            }
            None => Ok(None),
        }
    }

    pub fn insert_book(&self, book: &Book) -> Result<(), LibraryError> {
        let status_str = format_book_status(&book.status);

        self.index_conn.execute(
            r#"
            INSERT INTO books (id, title, author, cover_path, total_chapters, analyzed_chapters,
                              status, created_at, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
            "#,
            params![
                book.id.as_str(),
                book.title,
                book.author,
                book.cover_path,
                book.total_chapters,
                book.analyzed_chapters,
                status_str,
                book.created_at.to_rfc3339(),
                book.updated_at.to_rfc3339(),
            ],
        )?;

        let book_dir = self.book_dir(&book.id);
        fs::create_dir_all(&book_dir)?;
        fs::create_dir_all(book_dir.join("chapters"))?;

        Ok(())
    }

    pub fn update_book(&self, book: &Book) -> Result<(), LibraryError> {
        let status_str = format_book_status(&book.status);

        let rows_affected = self.index_conn.execute(
            r#"
            UPDATE books SET
                title = ?2,
                author = ?3,
                cover_path = ?4,
                total_chapters = ?5,
                analyzed_chapters = ?6,
                status = ?7,
                updated_at = ?8
            WHERE id = ?1
            "#,
            params![
                book.id.as_str(),
                book.title,
                book.author,
                book.cover_path,
                book.total_chapters,
                book.analyzed_chapters,
                status_str,
                book.updated_at.to_rfc3339(),
            ],
        )?;

        if rows_affected == 0 {
            return Err(LibraryError::BookNotFound(book.id.to_string()));
        }

        Ok(())
    }

    pub fn delete_book(&self, book_id: &BookId) -> Result<bool, LibraryError> {
        let book_dir = self.book_dir(book_id);
        if book_dir.exists() {
            fs::remove_dir_all(&book_dir)?;
        }

        let rows_affected = self.index_conn.execute(
            "DELETE FROM books WHERE id = ?1",
            params![book_id.as_str()],
        )?;

        Ok(rows_affected > 0)
    }

    pub fn book_exists(&self, book_id: &BookId) -> Result<bool, LibraryError> {
        let count: i64 = self.index_conn.query_row(
            "SELECT COUNT(*) FROM books WHERE id = ?1",
            params![book_id.as_str()],
            |row| row.get(0),
        )?;

        Ok(count > 0)
    }

    pub fn count_books(&self) -> Result<usize, LibraryError> {
        let count: i64 = self
            .index_conn
            .query_row("SELECT COUNT(*) FROM books", [], |row| row.get(0))?;
        Ok(count as usize)
    }
}

fn parse_book_status(s: &str) -> BookStatus {
    match s {
        "importing" => BookStatus::Importing,
        "ready" => BookStatus::Ready,
        "analyzing" => BookStatus::Analyzing,
        "completed" => BookStatus::Completed,
        _ if s.starts_with("error:") => BookStatus::Error(s[6..].to_string()),
        _ => BookStatus::Ready,
    }
}

fn format_book_status(status: &BookStatus) -> String {
    match status {
        BookStatus::Importing => "importing".to_string(),
        BookStatus::Ready => "ready".to_string(),
        BookStatus::Analyzing => "analyzing".to_string(),
        BookStatus::Completed => "completed".to_string(),
        BookStatus::Error(msg) => format!("error:{}", msg),
    }
}

fn parse_datetime(s: &str) -> chrono::DateTime<chrono::Utc> {
    chrono::DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.with_timezone(&chrono::Utc))
        .unwrap_or_else(|_| chrono::Utc::now())
}
