//! Event CRUD commands.

use super::{Db, Res, with_db};
use crate::db;
use crate::models::*;
use rusqlite::params;

#[tauri::command]
pub fn get_events(state: Db) -> Res<Vec<EventItem>> {
    with_db(&state, db::list_events)
}

#[tauri::command]
pub fn save_event(state: Db, id: Option<i64>, data: SaveEvent) -> Res<EventItem> {
    let eid = with_db(&state, |conn| db::save_event(conn, id, &data))?;
    with_db(&state, |conn| {
        conn.query_row(
            "SELECT id,name,date,annotation,suffix,notes FROM events WHERE id=?1",
            params![eid],
            |r| {
                Ok(EventItem {
                    id: r.get(0)?,
                    name: r.get(1)?,
                    date: r.get(2)?,
                    annotation: r.get(3)?,
                    suffix: r.get(4)?,
                    notes: r.get(5)?,
                })
            },
        )
    })
}

#[tauri::command]
pub fn delete_event(state: Db, id: i64) -> Res<()> {
    with_db(&state, |conn| db::delete_event(conn, id))
}

#[tauri::command]
pub fn get_event_words(state: Db, event_id: i64) -> Res<(Vec<String>, Vec<String>)> {
    with_db(&state, |conn| db::get_event_words(conn, event_id))
}
