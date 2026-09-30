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

#[derive(Debug)]
pub enum AppError {
    DbNotOpen,
    Database(rusqlite::Error),
    Io(std::io::Error),
    Custom(String),
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DbNotOpen => write!(f, "No database open."),
            Self::Database(e) => write!(f, "{e}"),
            Self::Io(e) => write!(f, "{e}"),
            Self::Custom(s) => write!(f, "{s}"),
        }
    }
}

impl std::error::Error for AppError {}

impl From<rusqlite::Error> for AppError {
    fn from(e: rusqlite::Error) -> Self {
        Self::Database(e)
    }
}

impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

impl From<String> for AppError {
    fn from(s: String) -> Self {
        Self::Custom(s)
    }
}

impl From<AppError> for String {
    fn from(e: AppError) -> Self {
        e.to_string()
    }
}

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
    let conn = guard
        .as_ref()
        .ok_or_else(|| AppError::DbNotOpen.to_string())?;
    f(conn).map_err(|e| AppError::from(e).into())
}

pub fn with_db_mut<T, E: Into<AppError>, F: FnOnce(&mut Connection) -> Result<T, E>>(
    state: &AppState,
    f: F,
) -> Res<T> {
    let mut guard = state.db.lock().map_err(err)?;
    let conn = guard
        .as_mut()
        .ok_or_else(|| AppError::DbNotOpen.to_string())?;
    f(conn).map_err(|e| e.into().into())
}
