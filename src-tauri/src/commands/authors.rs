//! Author CRUD commands.

use super::{Db, Res, with_db};
use crate::db;
use crate::models::*;

#[tauri::command]
pub fn get_authors(state: Db) -> Res<Vec<AuthorItem>> {
    with_db(&state, db::list_authors)
}

#[tauri::command]
pub fn save_author(state: Db, id: Option<i64>, data: SaveAuthor) -> Res<Vec<AuthorItem>> {
    with_db(&state, |conn| db::save_author(conn, id, &data))?;
    with_db(&state, db::list_authors)
}

#[tauri::command]
pub fn delete_author(state: Db, id: i64) -> Res<Vec<AuthorItem>> {
    with_db(&state, |conn| db::delete_author(conn, id))?;
    with_db(&state, db::list_authors)
}
