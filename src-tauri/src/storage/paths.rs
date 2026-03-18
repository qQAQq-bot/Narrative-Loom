//! Path Management Module
//!
//! Centralizes all path management for the application.
//! All data (config, database, cache) is stored in the project directory
//! under a `.narrative-loom` folder.

use std::env;
use std::fs;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PathError {
    #[error("Failed to get current directory: {0}")]
    CurrentDirError(std::io::Error),

    #[error("Failed to create directory: {0}")]
    CreateDirError(std::io::Error),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

/// Get the base data directory for the application.
/// This is `.narrative-loom` in the current working directory.
pub fn get_data_dir() -> Result<PathBuf, PathError> {
    let cwd = env::current_dir().map_err(PathError::CurrentDirError)?;
    let data_dir = cwd.join(".narrative-loom");
    fs::create_dir_all(&data_dir).map_err(PathError::CreateDirError)?;
    Ok(data_dir)
}

/// Get the config directory.
/// This is `.narrative-loom/config`
pub fn get_config_dir() -> Result<PathBuf, PathError> {
    let config_dir = get_data_dir()?.join("config");
    fs::create_dir_all(&config_dir).map_err(PathError::CreateDirError)?;
    Ok(config_dir)
}

/// Get the library (books) data directory.
/// This is `.narrative-loom/library`
pub fn get_library_dir() -> Result<PathBuf, PathError> {
    let library_dir = get_data_dir()?.join("library");
    fs::create_dir_all(&library_dir).map_err(PathError::CreateDirError)?;
    Ok(library_dir)
}

/// Get the books directory within the library.
/// This is `.narrative-loom/library/books`
pub fn get_books_dir() -> Result<PathBuf, PathError> {
    let books_dir = get_library_dir()?.join("books");
    fs::create_dir_all(&books_dir).map_err(PathError::CreateDirError)?;
    Ok(books_dir)
}

/// Get the directory for a specific book.
/// This is `.narrative-loom/library/books/{book_id}`
pub fn get_book_dir(book_id: &str) -> Result<PathBuf, PathError> {
    let book_dir = get_books_dir()?.join(book_id);
    fs::create_dir_all(&book_dir).map_err(PathError::CreateDirError)?;
    Ok(book_dir)
}

/// Get the library index database path.
/// This is `.narrative-loom/library/library.db`
pub fn get_library_db_path() -> Result<PathBuf, PathError> {
    Ok(get_library_dir()?.join("library.db"))
}

/// Get the vectors database path for a specific book.
/// This is `.narrative-loom/library/books/{book_id}/vectors.db`
pub fn get_vectors_db_path(book_id: &str) -> Result<PathBuf, PathError> {
    Ok(get_book_dir(book_id)?.join("vectors.db"))
}

/// Get the book database path for a specific book.
/// This is `.narrative-loom/library/books/{book_id}/book.db`
pub fn get_book_db_path(book_id: &str) -> Result<PathBuf, PathError> {
    Ok(get_book_dir(book_id)?.join("book.db"))
}

/// Get the cache directory.
/// This is `.narrative-loom/cache`
pub fn get_cache_dir() -> Result<PathBuf, PathError> {
    let cache_dir = get_data_dir()?.join("cache");
    fs::create_dir_all(&cache_dir).map_err(PathError::CreateDirError)?;
    Ok(cache_dir)
}

/// Get the keychain data directory.
/// This is `.narrative-loom/keychain`
pub fn get_keychain_dir() -> Result<PathBuf, PathError> {
    let keychain_dir = get_data_dir()?.join("keychain");
    fs::create_dir_all(&keychain_dir).map_err(PathError::CreateDirError)?;
    Ok(keychain_dir)
}

/// Get the logs directory.
/// This is `.narrative-loom/logs`
pub fn get_logs_dir() -> Result<PathBuf, PathError> {
    let logs_dir = get_data_dir()?.join("logs");
    fs::create_dir_all(&logs_dir).map_err(PathError::CreateDirError)?;
    Ok(logs_dir)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_data_dir() {
        let result = get_data_dir();
        assert!(result.is_ok());
        let path = result.unwrap();
        assert!(path.ends_with(".narrative-loom"));
    }
}
