use crate::core::ids::BookId;
use crate::storage::book_db::BookDb;
use crate::storage::library::Library;
use std::path::PathBuf;

pub fn open_book_db(book_id: &str) -> Result<BookDb, String> {
    Ok(open_book(book_id)?.db)
}

pub struct OpenBook {
    pub bid: BookId,
    pub book_dir: PathBuf,
    pub book_db_path: PathBuf,
    pub db: BookDb,
}

pub fn open_book(book_id: &str) -> Result<OpenBook, String> {
    let library = Library::open().map_err(|e| e.to_string())?;
    open_book_from_library(&library, book_id)
}

pub fn open_book_from_library(library: &Library, book_id: &str) -> Result<OpenBook, String> {
    let bid = BookId::from_string(book_id.to_string());
    let book_dir = library.book_dir(&bid);
    let book_db_path = book_dir.join("book.db");

    if !book_db_path.exists() {
        return Err("Book database not found".to_string());
    }

    let db = BookDb::open(&book_db_path, bid.clone()).map_err(|e| e.to_string())?;

    Ok(OpenBook {
        bid,
        book_dir,
        book_db_path,
        db,
    })
}
