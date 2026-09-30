//! FTS search commands.

use super::{Db, Res, with_db};
use crate::db;
use crate::models::*;

#[tauri::command]
pub fn search_english(state: Db, params: ELSearchParams) -> Res<Vec<ELResult>> {
    if params.query.trim().is_empty() {
        return Ok(vec![]);
    }

    if params.use_keywords_only {
        if params.use_like {
            return with_db(&state, |conn| {
                db::search_english_keywords_like(conn, &params.query, params.limit)
            });
        }
        return with_db(&state, |conn| {
            match db::search_english_keywords_fts(conn, &params.query, params.limit) {
                Ok(r) if !r.is_empty() => Ok(r),
                _ => db::search_english_keywords_like(conn, &params.query, params.limit),
            }
        });
    }

    if params.use_like {
        return with_db(&state, |conn| {
            db::search_english_like(conn, &params.query, params.limit)
        });
    }
    with_db(&state, |conn| {
        match db::search_english_fts(conn, &params.query, params.limit) {
            Ok(r) if !r.is_empty() => Ok(r),
            _ => db::search_english_like(conn, &params.query, params.limit),
        }
    })
}

#[tauri::command]
pub fn rebuild_fts(state: Db) -> Res<i64> {
    with_db(&state, |conn| {
        db::rebuild_fts(conn)?;
        let body_count: i64 = conn.query_row("SELECT COUNT(*) FROM def_fts", [], |r| r.get(0))?;
        let kw_count: i64 = conn.query_row("SELECT COUNT(*) FROM def_kw_fts", [], |r| r.get(0))?;
        Ok(body_count + kw_count)
    })
}

#[tauri::command]
pub fn compact_db(state: Db) -> Res<String> {
    with_db(&state, |conn| {
        db::vacuum_db(conn)?;
        let size: i64 = conn.query_row(
            "SELECT page_count * page_size FROM pragma_page_count, pragma_page_size",
            [],
            |r| r.get(0),
        )?;
        Ok(format!("{:.1} MB", size as f64 / 1_048_576.0))
    })
}

#[tauri::command]
pub fn fts_is_ready(state: Db) -> bool {
    state
        .db
        .lock()
        .ok()
        .and_then(|g| g.as_ref().map(db::fts_is_ready))
        .unwrap_or(false)
}
