//! Text to `SQLite` converter module.
//!
//! Converts `@`-delimited text files in a directory to `SQLite` database format
//! compatible with the `export.db` (`loglan_core`) schema.

use crate::import;
use crate::models::ImportResult;
use rusqlite::Connection;
use std::fs;
use std::path::Path;

/// Convert `@`-delimited text files in a directory to `SQLite` database.
pub fn convert_text_to_sqlite(
    conn: &mut Connection,
    text_dir: &str,
) -> Result<ImportResult, String> {
    if !Path::new(text_dir).exists() {
        return Err("Text directory does not exist".to_string());
    }

    let mut text_files = Vec::new();
    for entry in fs::read_dir(text_dir).map_err(|e| format!("Failed to read directory: {e}"))? {
        let entry = entry.map_err(|e| format!("Failed to read directory entry: {e}"))?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("txt") {
            text_files.push(path.to_string_lossy().into_owned());
        }
    }

    if text_files.is_empty() {
        return Err("No text files found in directory".to_string());
    }

    import::import_files(conn, &text_files)
}
