//! Tauri commands for the LOD Manager.
//!
//! Each submodule groups related commands by domain.

pub mod authors;
pub mod database;
pub mod events;
pub mod export;
pub mod import;
pub mod search;
pub mod types;
pub mod words;

use rusqlite::Connection;
use std::sync::Mutex;
use tauri::State;

pub struct AppState {
    pub db: Mutex<Option<Connection>>,
    pub db_path: Mutex<String>,
}

pub type Db<'a> = State<'a, AppState>;
pub type Res<T> = Result<T, String>;

pub fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}

pub fn with_db<T, F: FnOnce(&Connection) -> rusqlite::Result<T>>(state: &AppState, f: F) -> Res<T> {
    let guard = state.db.lock().map_err(err)?;
    let conn = guard.as_ref().ok_or("No database open.")?;
    f(conn).map_err(err)
}

pub fn with_db_mut<T, F: FnOnce(&mut Connection) -> rusqlite::Result<T>>(
    state: &AppState,
    f: F,
) -> Res<T> {
    let mut guard = state.db.lock().map_err(err)?;
    let conn = guard.as_mut().ok_or("No database open.")?;
    f(conn).map_err(err)
}
