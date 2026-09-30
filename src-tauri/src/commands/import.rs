//! Import commands.

use super::{Db, Res, with_db, with_db_mut};
use crate::converter::converter;
use crate::db;
use crate::import;
use crate::models::ImportResult;

/// Android variant: receives file contents directly (content:// URIs can't be
/// read by `std::fs`, so the frontend reads them via plugin-fs and sends content).
/// `files` is a list of (filename, `utf8_content`) pairs.
#[tauri::command]
pub fn import_lod_contents(state: Db, files: Vec<(String, String)>) -> Res<ImportResult> {
    // Prevent OOM on Android by limiting total import size to 100MB
    let total_size: usize = files.iter().map(|(_, c)| c.len()).sum();
    if total_size > 100_000_000 {
        return Err(format!(
            "Import too large: {:.1}MB (max 100MB)",
            total_size as f64 / 1_000_000.0
        ));
    }
    let result = with_db_mut(&state, |conn| import::import_contents(conn, &files))?;
    let _ = with_db(&state, db::rebuild_fts);
    Ok(result)
}

#[tauri::command]
pub fn import_lod_files(state: Db, paths: Vec<String>) -> Res<ImportResult> {
    let result = with_db_mut(&state, |conn| import::import_files(conn, &paths))?;
    let _ = with_db(&state, db::rebuild_fts);
    Ok(result)
}

/// Convert `@`-delimited text files to `SQLite` database format
/// Compatible with the Python `loglan_converter` output format
#[tauri::command]
pub fn convert_text_files(state: Db, text_dir: String) -> Res<ImportResult> {
    let result = with_db_mut(&state, |conn| {
        converter::convert_text_to_sqlite(conn, &text_dir)
    })?;
    let _ = with_db(&state, db::rebuild_fts);
    Ok(result)
}
