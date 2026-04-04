//! Word and definition CRUD commands.

use super::{Db, Res, with_db};
use crate::db;
use crate::models::*;

#[tauri::command]
pub fn get_words(
    state: Db,
    q: String,
    type_filter: String,
    event_id: Option<i64>,
) -> Res<Vec<WordListItem>> {
    with_db(&state, |conn| {
        db::list_words(conn, &q, &type_filter, event_id)
    })
}

#[tauri::command]
pub fn get_word(state: Db, id: i64) -> Res<WordDetail> {
    with_db(&state, |conn| db::get_word(conn, id))
}

#[tauri::command]
pub fn save_word(state: Db, id: Option<i64>, data: SaveWord) -> Res<WordDetail> {
    let wid = with_db(&state, |conn| db::save_word(conn, id, &data))?;
    with_db(&state, |conn| db::get_word(conn, wid))
}

#[tauri::command]
pub fn delete_word(state: Db, id: i64) -> Res<()> {
    with_db(&state, |conn| {
        db::delete_word(conn, id)?;
        db::rebuild_fts(conn)?;
        Ok(())
    })
}

#[tauri::command]
pub fn save_definition(
    state: Db,
    id: Option<i64>,
    word_id: i64,
    data: SaveDefinition,
) -> Res<WordDetail> {
    with_db(&state, |conn| {
        db::save_definition(conn, id, word_id, &data)?;
        if let Some(def_id) = id {
            db::fts_update(conn, def_id, &data.body).ok();
        } else if let Ok(def_id) = conn.query_row(
            "SELECT id FROM definitions WHERE word_id=?1 ORDER BY position DESC LIMIT 1",
            [word_id],
            |r| r.get(0),
        ) {
            db::fts_update(conn, def_id, &data.body).ok();
        }
        Ok(())
    })?;
    with_db(&state, |conn| db::get_word(conn, word_id))
}

#[tauri::command]
pub fn delete_definition(state: Db, id: i64, word_id: i64) -> Res<WordDetail> {
    with_db(&state, |conn| {
        db::delete_definition(conn, id)?;
        let _ = db::fts_update(conn, id, "");
        Ok(())
    })?;
    with_db(&state, |conn| db::get_word(conn, word_id))
}
