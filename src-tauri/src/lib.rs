// Clippy configuration — applied project-wide
#![warn(clippy::all)]
#![warn(clippy::pedantic)]
#![allow(clippy::needless_pass_by_value)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::wildcard_imports)]
#![allow(clippy::too_many_lines)]

mod db;
mod export;
mod import;
mod models;
use models::*;
use rusqlite::Connection;
use std::sync::Mutex;
use tauri::{Manager, State};

pub struct AppState {
    pub db: Mutex<Option<Connection>>,
    pub db_path: Mutex<String>,
}

type Db<'a> = State<'a, AppState>;
type Res<T> = Result<T, String>;

fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}

fn with_db<T, F: FnOnce(&Connection) -> rusqlite::Result<T>>(state: &AppState, f: F) -> Res<T> {
    let guard = state.db.lock().map_err(err)?;
    let conn = guard.as_ref().ok_or("No database open.")?;
    f(conn).map_err(err)
}

fn with_db_mut<T, F: FnOnce(&mut Connection) -> rusqlite::Result<T>>(
    state: &AppState,
    f: F,
) -> Res<T> {
    let mut guard = state.db.lock().map_err(err)?;
    let conn = guard.as_mut().ok_or("No database open.")?;
    f(conn).map_err(err)
}

#[tauri::command]
fn open_database(state: Db, path: String) -> Res<AppInfo> {
    let conn = Connection::open(&path).map_err(err)?;
    conn.execute_batch("PRAGMA foreign_keys=ON;").map_err(err)?;
    db::init_schema(&conn).map_err(err)?;
    db::init_fts(&conn).map_err(err)?;
    let _ = db::migrate_words_unique_if_needed(&conn);
    let _ = db::migrate_event_columns_if_needed(&conn);
    let _ = db::add_missing_indexes(&conn); // Add missing indexes for performance
    let mut info = db::get_stats(&conn).map_err(err)?;
    info.db_path.clone_from(&path);
    *state.db.lock().map_err(err)? = Some(conn);
    *state.db_path.lock().map_err(err)? = path;
    Ok(info)
}

#[tauri::command]
fn create_database(state: Db, path: String) -> Res<AppInfo> {
    let _ = std::fs::remove_file(&path);
    open_database(state, path)
}

#[tauri::command]
fn get_db_stats(state: Db) -> Res<DbStats> {
    let path = state.db_path.lock().map_err(err)?.clone();
    with_db(&state, |conn| {
        let mut s = db::get_db_stats(conn)?;
        s.db_path = path;
        Ok(s)
    })
}

#[tauri::command]
fn get_words(
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
fn get_word(state: Db, id: i64) -> Res<WordDetail> {
    with_db(&state, |conn| db::get_word(conn, id))
}

#[tauri::command]
fn save_word(state: Db, id: Option<i64>, data: SaveWord) -> Res<WordDetail> {
    let wid = with_db(&state, |conn| db::save_word(conn, id, &data))?;
    with_db(&state, |conn| db::get_word(conn, wid))
}

#[tauri::command]
fn delete_word(state: Db, id: i64) -> Res<()> {
    with_db(&state, |conn| db::delete_word(conn, id))
}

#[tauri::command]
fn save_definition(
    state: Db,
    id: Option<i64>,
    word_id: i64,
    data: SaveDefinition,
) -> Res<WordDetail> {
    with_db(&state, |conn| db::save_definition(conn, id, word_id, &data))?;
    with_db(&state, |conn| db::get_word(conn, word_id))
}

#[tauri::command]
fn delete_definition(state: Db, id: i64, word_id: i64) -> Res<WordDetail> {
    with_db(&state, |conn| db::delete_definition(conn, id))?;
    with_db(&state, |conn| db::get_word(conn, word_id))
}

#[tauri::command]
fn get_events(state: Db) -> Res<Vec<EventItem>> {
    with_db(&state, db::list_events)
}

#[tauri::command]
fn save_event(state: Db, id: Option<i64>, data: SaveEvent) -> Res<EventItem> {
    let eid = with_db(&state, |conn| db::save_event(conn, id, &data))?;
    with_db(&state, |conn| {
        conn.query_row(
            "SELECT id,name,date,annotation,suffix,notes FROM events WHERE id=?1",
            rusqlite::params![eid],
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
fn delete_event(state: Db, id: i64) -> Res<()> {
    with_db(&state, |conn| db::delete_event(conn, id))
}

#[tauri::command]
fn get_types(state: Db) -> Res<Vec<TypeItem>> {
    with_db(&state, db::list_types)
}

#[tauri::command]
fn save_type(state: Db, id: Option<i64>, data: SaveType) -> Res<Vec<TypeItem>> {
    with_db(&state, |conn| db::save_type(conn, id, &data))?;
    with_db(&state, db::list_types)
}

#[tauri::command]
fn delete_type(state: Db, id: i64) -> Res<Vec<TypeItem>> {
    with_db(&state, |conn| db::delete_type(conn, id))?;
    with_db(&state, db::list_types)
}

#[tauri::command]
fn get_authors(state: Db) -> Res<Vec<AuthorItem>> {
    with_db(&state, db::list_authors)
}

#[tauri::command]
fn save_author(state: Db, id: Option<i64>, data: SaveAuthor) -> Res<Vec<AuthorItem>> {
    with_db(&state, |conn| db::save_author(conn, id, &data))?;
    with_db(&state, db::list_authors)
}

#[tauri::command]
fn delete_author(state: Db, id: i64) -> Res<Vec<AuthorItem>> {
    with_db(&state, |conn| db::delete_author(conn, id))?;
    with_db(&state, db::list_authors)
}

/// Android variant: receives file contents directly (content:// URIs can't be
/// read by `std::fs`, so the frontend reads them via plugin-fs and sends content).
/// `files` is a list of (filename, `utf8_content`) pairs.
#[tauri::command]
fn import_lod_contents(state: Db, files: Vec<(String, String)>) -> Res<ImportResult> {
    let result = with_db_mut(&state, |conn| Ok(import::import_contents(conn, &files)))?;
    Ok(result)
}

#[tauri::command]
fn import_lod_files(state: Db, paths: Vec<String>) -> Res<ImportResult> {
    let result = with_db_mut(&state, |conn| Ok(import::import_files(conn, &paths)))?;
    let _ = with_db(&state, db::rebuild_fts);
    Ok(result)
}

#[tauri::command]
fn get_event_words(state: Db, event_id: i64) -> Res<(Vec<String>, Vec<String>)> {
    with_db(&state, |conn| db::get_event_words(conn, event_id))
}

#[tauri::command]
fn search_english(state: Db, params: ELSearchParams) -> Res<Vec<ELResult>> {
    if params.query.trim().is_empty() {
        return Ok(vec![]);
    }

    if params.use_keywords_only {
        // ── Keyword-only mode: search «...» markers ───────────────────────
        if params.use_like {
            return with_db(&state, |conn| {
                db::search_english_keywords_like(conn, &params.query, params.limit)
            });
        }
        return with_db(&state, |conn| {
            match db::search_english_keywords_fts(conn, &params.query, params.limit) {
                Ok(r) if !r.is_empty() => Ok(r),
                // FTS may be empty or query may be malformed — fall back to LIKE
                _ => db::search_english_keywords_like(conn, &params.query, params.limit),
            }
        });
    }

    // ── Full-body mode ────────────────────────────────────────────────────
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
fn rebuild_fts(state: Db) -> Res<i64> {
    // Open a *separate* connection for the rebuild so the shared Mutex is NOT
    // held during the potentially long operation.  Other commands (fts_is_ready,
    // list_words, etc.) can proceed normally while the rebuild runs.
    let path = state.db_path.lock().map_err(err)?.clone();
    if path.is_empty() {
        return Err("No database open".to_string());
    }
    let conn = Connection::open(&path).map_err(err)?;
    // WAL mode so the rebuild writer doesn't block readers on the main connection.
    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;")
        .map_err(err)?;
    db::rebuild_fts(&conn).map_err(err)?;
    let body_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM def_fts", [], |r| r.get(0))
        .map_err(err)?;
    let kw_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM def_kw_fts", [], |r| r.get(0))
        .map_err(err)?;
    Ok(body_count + kw_count)
}

#[tauri::command]
fn fts_is_ready(state: Db) -> bool {
    state
        .db
        .lock()
        .ok()
        .and_then(|g| g.as_ref().map(db::fts_is_ready))
        .unwrap_or(false)
}

#[tauri::command]
fn export_html(state: Db, event_name: Option<String>) -> Res<String> {
    with_db(&state, |conn| {
        export::generate_html(conn, event_name.as_deref())
    })
}

#[tauri::command]
fn export_html_to_file(state: Db, path: String, event_name: Option<String>) -> Res<()> {
    with_db(&state, |conn| {
        export::write_html_to_file(conn, &path, event_name.as_deref())
    })
}

/// Returns the canonical default database path in the app's data directory.
/// This is the reliable cross-platform (especially Android) way to get
/// a writable, persistent path that survives app restarts.
#[tauri::command]
fn get_default_db_path(app: tauri::AppHandle) -> Res<String> {
    let dir = app.path().app_data_dir().map_err(err)?;
    std::fs::create_dir_all(&dir).map_err(err)?;
    Ok(dir.join("lod.db").to_string_lossy().into_owned())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .manage(AppState {
            db: Mutex::new(None),
            db_path: Mutex::new(String::new()),
        })
        .invoke_handler(tauri::generate_handler![
            open_database,
            create_database,
            get_db_stats,
            get_words,
            get_word,
            save_word,
            delete_word,
            save_definition,
            delete_definition,
            get_events,
            save_event,
            delete_event,
            get_types,
            save_type,
            delete_type,
            get_authors,
            save_author,
            delete_author,
            import_lod_contents,
            import_lod_files,
            get_event_words,
            search_english,
            rebuild_fts,
            fts_is_ready,
            export_html,
            export_html_to_file,
            get_default_db_path
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn test_in_memory_db_init() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        db::init_schema(&conn).unwrap();
        db::init_fts(&conn).unwrap();
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM events WHERE name='Start'", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_get_word_performance_optimal() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        db::init_schema(&conn).unwrap();
        db::init_fts(&conn).unwrap();
        db::add_missing_indexes(&conn).unwrap(); // Add the new indexes

        // Create test data
        conn.execute(
            "INSERT INTO types (name, group_) VALUES (?1, ?2)",
            ("test_type", "test_group"),
        )
        .unwrap();

        let type_id: i64 = conn.last_insert_rowid();

        conn.execute(
            "INSERT INTO words (name, type_id, source, year, rank, match_, origin, notes) 
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            (
                "testword",
                type_id,
                "test_source",
                "2023",
                "A",
                "exact",
                "test_origin",
                "test_notes",
            ),
        )
        .unwrap();

        let word_id: i64 = conn.last_insert_rowid();

        // Add affixes
        conn.execute(
            "INSERT INTO word_affixes (word_id, affix) VALUES (?1, ?2), (?1, ?3)",
            (word_id, "test", "affix"),
        )
        .unwrap();

        // Add spellings
        conn.execute(
            "INSERT INTO word_spellings (word_id, spelling) VALUES (?1, ?2), (?1, ?3)",
            (word_id, "spelling1", "spelling2"),
        )
        .unwrap();

        // Add definitions
        conn.execute(
            "INSERT INTO definitions (word_id, position, grammar, usage, body, tags) 
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            (word_id, 0, "grammar1", "usage1", "body1", "tags1"),
        )
        .unwrap();

        conn.execute(
            "INSERT INTO definitions (word_id, position, grammar, usage, body, tags) 
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            (word_id, 1, "grammar2", "usage2", "body2", "tags2"),
        )
        .unwrap();

        // Test performance with optimal 3-query approach
        let start = Instant::now();
        for _ in 0..100 {
            let word = db::get_word(&conn, word_id).unwrap();
            assert_eq!(word.name, "testword");
            assert_eq!(word.affixes.len(), 2);
            assert_eq!(word.spellings.len(), 2);
            assert_eq!(word.definitions.len(), 2);
        }
        let duration = start.elapsed();

        println!(
            "100 get_word calls with optimal 3-query approach took: {:?}",
            duration
        );
        println!("Average per call: {:?}", duration / 100);

        // Should be very fast
        assert!(duration.as_millis() < 1000, "get_word should be very fast");

        // Test that invalid ID returns error properly
        let result = db::get_word(&conn, 999_999);
        assert!(result.is_err(), "Invalid word ID should return error");
    }
}
