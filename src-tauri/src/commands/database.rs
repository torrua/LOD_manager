//! Database management commands: open, create, stats, default path.

use super::{Db, Res, err, with_db};
use crate::db;
use crate::models::*;
use rusqlite::Connection;
use tauri::Manager;

#[tauri::command]
pub fn open_database(state: Db, path: String) -> Res<AppInfo> {
    let conn = Connection::open(&path).map_err(err)?;
    conn.execute_batch("PRAGMA foreign_keys=ON; PRAGMA journal_mode=WAL;")
        .map_err(err)?;
    db::init_schema(&conn).map_err(err)?;
    db::init_fts(&conn).map_err(err)?;
    let _ = db::migrate_words_unique_if_needed(&conn);
    let _ = db::migrate_event_columns_if_needed(&conn);
    let _ = db::add_missing_indexes(&conn);
    let mut info = db::get_stats(&conn).map_err(err)?;
    info.db_path.clone_from(&path);
    *state.db.lock().map_err(err)? = Some(conn);
    *state.db_path.lock().map_err(err)? = path;
    Ok(info)
}

#[tauri::command]
pub fn create_database(state: Db, path: String) -> Res<AppInfo> {
    let _ = std::fs::remove_file(&path);
    open_database(state, path)
}

#[tauri::command]
pub fn get_db_stats(state: Db) -> Res<DbStats> {
    let path = state.db_path.lock().map_err(err)?.clone();
    with_db(&state, |conn| {
        let mut s = db::get_db_stats(conn)?;
        s.db_path = path;
        Ok(s)
    })
}

#[tauri::command]
pub fn get_default_db_path(app: tauri::AppHandle) -> Res<String> {
    let dir = app.path().app_data_dir().map_err(err)?;
    std::fs::create_dir_all(&dir).map_err(err)?;
    Ok(dir.join("lod.db").to_string_lossy().into_owned())
}
