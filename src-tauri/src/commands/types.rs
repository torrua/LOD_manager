//! Type CRUD commands.

use super::{Db, Res, with_db};
use crate::db;
use crate::models::*;

#[tauri::command]
pub fn get_types(state: Db) -> Res<Vec<TypeItem>> {
    with_db(&state, db::list_types)
}

#[tauri::command]
pub fn save_type(state: Db, id: Option<i64>, data: SaveType) -> Res<Vec<TypeItem>> {
    with_db(&state, |conn| db::save_type(conn, id, &data))?;
    with_db(&state, db::list_types)
}

#[tauri::command]
pub fn delete_type(state: Db, id: i64) -> Res<Vec<TypeItem>> {
    with_db(&state, |conn| db::delete_type(conn, id))?;
    with_db(&state, db::list_types)
}
