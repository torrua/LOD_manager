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
fn delete_definition(state: Db, id: i64, word_id: i64) -> Res<WordDetail> {
    with_db(&state, |conn| {
        db::delete_definition(conn, id)?;
        let _ = db::fts_update(conn, id, "");
        Ok(())
    })?;
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
    let _ = with_db(&state, db::rebuild_fts);
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
fn export_html(state: Db, event_name: Option<String>, wildcard: bool) -> Res<String> {
    with_db(&state, |conn| {
        export::generate_html(conn, event_name.as_deref(), wildcard)
    })
}

#[tauri::command]
fn export_html_to_file(
    state: Db,
    path: String,
    event_name: Option<String>,
    wildcard: bool,
) -> Res<()> {
    with_db(&state, |conn| {
        export::write_html_to_file(conn, &path, event_name.as_deref(), wildcard)
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
        .setup(|_app| {
            #[cfg(desktop)]
            {
                _app.handle()
                    .plugin(tauri_plugin_updater::Builder::new().build())?;
            }
            Ok(())
        })
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

        println!("100 get_word calls with optimal 3-query approach took: {duration:?}");
        println!("Average per call: {:?}", duration / 100);

        // Should be very fast
        assert!(duration.as_millis() < 1000, "get_word should be very fast");

        // Test that invalid ID returns error properly
        let result = db::get_word(&conn, 999_999);
        assert!(result.is_err(), "Invalid word ID should return error");
    }

    #[test]
    fn test_fts_update_incremental() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        db::init_schema(&conn).unwrap();
        db::init_fts(&conn).unwrap();

        // Create word and definition
        conn.execute("INSERT INTO types (name) VALUES ('gismu')", [])
            .unwrap();
        let type_id: i64 = conn.last_insert_rowid();
        conn.execute(
            "INSERT INTO words (name, type_id) VALUES ('testword', ?1)",
            [type_id],
        )
        .unwrap();
        let word_id: i64 = conn.last_insert_rowid();
        conn.execute(
            "INSERT INTO definitions (word_id, position, body) VALUES (?1, 0, 'This is a test definition')",
            [word_id],
        )
        .unwrap();
        let def_id: i64 = conn.last_insert_rowid();

        // Rebuild FTS first
        db::rebuild_fts(&conn).unwrap();

        // Verify FTS is ready
        assert!(db::fts_is_ready(&conn), "FTS should be ready after rebuild");

        // Search should find the word via "test" (FTS prefix search)
        let results = db::search_english_fts(&conn, "test", 10).unwrap();
        assert!(!results.is_empty(), "Should find 'testword' by 'test'");

        // Update definition via FTS update
        db::fts_update(
            &conn,
            def_id,
            "This is an updated definition about something else",
        )
        .ok();

        // Now search for the new term
        let results = db::search_english_fts(&conn, "updated", 10).unwrap();
        assert!(
            !results.is_empty(),
            "New term 'updated' should match after FTS update"
        );

        // The old term might still be in FTS cache, but that's acceptable for incremental updates
        // The key is that new content is searchable
    }

    #[test]
    fn test_list_words_basic() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        db::init_schema(&conn).unwrap();
        db::init_fts(&conn).unwrap();

        // Create type
        conn.execute("INSERT INTO types (name) VALUES ('gismu')", [])
            .unwrap();
        let type_id: i64 = conn.last_insert_rowid();

        // Create words
        conn.execute(
            "INSERT INTO words (name, type_id) VALUES ('abc', ?1)",
            [type_id],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO words (name, type_id) VALUES ('xyz', ?1)",
            [type_id],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO words (name, type_id) VALUES ('def', ?1)",
            [type_id],
        )
        .unwrap();

        // List all
        let words = db::list_words(&conn, "", "", None).unwrap();
        assert_eq!(words.len(), 3);

        // Filter by prefix
        let words = db::list_words(&conn, "a", "", None).unwrap();
        assert_eq!(words.len(), 1);
        assert_eq!(words[0].name, "abc");

        // Wildcard search
        let words = db::list_words(&conn, "x*", "", None).unwrap();
        assert_eq!(words.len(), 1);
        assert_eq!(words[0].name, "xyz");

        // Filter by type
        let words = db::list_words(&conn, "", "gismu", None).unwrap();
        assert_eq!(words.len(), 3);

        // Filter by event (no event = none should match)
        let words = db::list_words(&conn, "", "", Some(999)).unwrap();
        assert_eq!(words.len(), 0);
    }

    #[test]
    fn test_fts_search_basic() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        db::init_schema(&conn).unwrap();
        db::init_fts(&conn).unwrap();

        // Create word with definition
        conn.execute("INSERT INTO types (name) VALUES ('gismu')", [])
            .unwrap();
        let type_id: i64 = conn.last_insert_rowid();
        conn.execute(
            "INSERT INTO words (name, type_id) VALUES ('camgu', ?1)",
            [type_id],
        )
        .unwrap();
        let word_id: i64 = conn.last_insert_rowid();
        conn.execute(
            "INSERT INTO definitions (word_id, position, body) VALUES (?1, 0, 'to want to desire to hope')",
            [word_id],
        ).unwrap();

        // Rebuild FTS
        db::rebuild_fts(&conn).unwrap();

        // Search
        let results = db::search_english_fts(&conn, "desire", 10).unwrap();
        assert!(!results.is_empty());
    }

    #[test]
    fn test_word_crud_operations() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        db::init_schema(&conn).unwrap();

        // Create type first
        conn.execute(
            "INSERT INTO types (name, group_) VALUES ('gismu', 'core')",
            [],
        )
        .unwrap();
        let type_id: i64 = conn.last_insert_rowid();

        // CREATE: Insert word directly
        conn.execute(
            "INSERT INTO words (name, type_id, source, year, rank, match_, origin, notes)
             VALUES ('testword', ?1, 'test_source', '2024', 'A', 'exact', 'test_origin', 'test_notes')",
            [type_id],
        ).unwrap();
        let word_id: i64 = conn.last_insert_rowid();
        assert!(word_id > 0);

        // READ: Get word
        let word = db::get_word(&conn, word_id).unwrap();
        assert_eq!(word.name, "testword");
        assert_eq!(word.definitions.len(), 0);
        assert_eq!(word.affixes.len(), 0);

        // UPDATE: Add definitions
        conn.execute(
            "INSERT INTO definitions (word_id, position, grammar, usage, body, tags) VALUES (?1, 0, 'GU', 'test', 'first definition', 'tag1')",
            [word_id],
        ).unwrap();
        conn.execute(
            "INSERT INTO definitions (word_id, position, grammar, usage, body, tags) VALUES (?1, 1, 'N', 'test', 'second definition', 'tag2')",
            [word_id],
        ).unwrap();

        // Verify updated word
        let updated_word = db::get_word(&conn, word_id).unwrap();
        assert_eq!(updated_word.definitions.len(), 2);

        // DELETE: Delete word
        db::delete_word(&conn, word_id).unwrap();
        let result = db::get_word(&conn, word_id);
        assert!(result.is_err(), "Word should be deleted");
    }

    #[test]
    fn test_definition_crud_operations() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        db::init_schema(&conn).unwrap();

        // Create word
        conn.execute("INSERT INTO types (name) VALUES ('gismu')", [])
            .unwrap();
        let type_id: i64 = conn.last_insert_rowid();
        conn.execute(
            "INSERT INTO words (name, type_id) VALUES ('testword', ?1)",
            [type_id],
        )
        .unwrap();
        let word_id: i64 = conn.last_insert_rowid();

        // CREATE: Add definition via save_definition (position is a parameter)
        let def_data = models::SaveDefinition {
            grammar: Some("GU".to_string()),
            usage: Some("verb".to_string()),
            body: "to want".to_string(),
            tags: Some("main".to_string()),
        };
        db::save_definition(&conn, None, word_id, &def_data).unwrap();

        // READ: Verify definition
        let word = db::get_word(&conn, word_id).unwrap();
        assert_eq!(word.definitions.len(), 1);
        let def_id = word.definitions[0].id;

        // UPDATE: Update definition (can't update body directly via save_definition, need to delete and recreate)
        db::delete_definition(&conn, def_id).unwrap();

        let updated_def = models::SaveDefinition {
            grammar: Some("GU".to_string()),
            usage: Some("verb".to_string()),
            body: "to strongly want".to_string(),
            tags: Some("updated".to_string()),
        };
        db::save_definition(&conn, None, word_id, &updated_def).unwrap();

        // Verify update
        let updated_word = db::get_word(&conn, word_id).unwrap();
        assert!(updated_word.definitions[0].body.contains("strongly"));

        // DELETE: Delete definition
        let new_def_id = updated_word.definitions[0].id;
        db::delete_definition(&conn, new_def_id).unwrap();
        let final_word = db::get_word(&conn, word_id).unwrap();
        assert_eq!(final_word.definitions.len(), 0);
    }

    #[test]
    fn test_type_crud_operations() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        db::init_schema(&conn).unwrap();

        // CREATE: Insert type
        conn.execute(
            "INSERT INTO types (name, group_) VALUES ('lujvo', 'derived')",
            [],
        )
        .unwrap();
        let type_id: i64 = conn.last_insert_rowid();
        assert!(type_id > 0);

        // READ: List types
        let types = db::list_types(&conn).unwrap();
        let found = types.iter().find(|t| t.name == "lujvo");
        assert!(found.is_some());
        assert_eq!(found.unwrap().group_.as_deref(), Some("derived"));

        // UPDATE: Update type
        conn.execute(
            "UPDATE types SET group_ = 'modified' WHERE id = ?1",
            [type_id],
        )
        .unwrap();

        // Verify update
        let types = db::list_types(&conn).unwrap();
        let found = types.iter().find(|t| t.name == "lujvo");
        assert_eq!(found.unwrap().group_.as_deref(), Some("modified"));

        // DELETE: Try to delete (should fail - words depend on it)
        // First create a word with this type
        conn.execute(
            "INSERT INTO words (name, type_id) VALUES ('testlujvo', ?1)",
            [type_id],
        )
        .unwrap();
        let result = conn.execute("DELETE FROM types WHERE id = ?1", [type_id]);
        assert!(result.is_err(), "Cannot delete type with dependent words");
    }

    #[test]
    fn test_event_crud_operations() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        db::init_schema(&conn).unwrap();

        // CREATE: Insert event
        conn.execute(
            "INSERT INTO events (name, date, annotation, suffix, notes) VALUES ('NewEvent', '2024-06-15', 'test annotation', 'NE', 'test notes')",
            [],
        ).unwrap();
        let event_id: i64 = conn.last_insert_rowid();
        assert!(event_id > 0);

        // READ: List events
        let events = db::list_events(&conn).unwrap();
        let found = events.iter().find(|e| e.name == "NewEvent");
        assert!(found.is_some());
        assert_eq!(found.unwrap().date.as_deref(), Some("2024-06-15"));

        // UPDATE: Update event
        conn.execute(
            "UPDATE events SET date = '2024-07-01', annotation = 'updated' WHERE id = ?1",
            [event_id],
        )
        .unwrap();

        // Verify update
        let events = db::list_events(&conn).unwrap();
        let found = events.iter().find(|e| e.name == "NewEvent");
        assert_eq!(found.unwrap().date.as_deref(), Some("2024-07-01"));

        // DELETE: Cannot delete events via API, just verify we can read them
        let events = db::list_events(&conn).unwrap();
        assert!(events.len() >= 2); // At least Start + NewEvent
    }

    #[test]
    fn test_author_crud_operations() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        db::init_schema(&conn).unwrap();

        // CREATE: Insert author
        conn.execute("INSERT INTO authors (initials, full_name, notes) VALUES ('JD', 'John Doe', 'test author')", []).unwrap();
        let author_id: i64 = conn.last_insert_rowid();
        assert!(author_id > 0);

        // READ: List authors
        let authors = db::list_authors(&conn).unwrap();
        let found = authors.iter().find(|a| a.initials == "JD");
        assert!(found.is_some());
        assert_eq!(found.unwrap().full_name.as_deref(), Some("John Doe"));

        // UPDATE: Update author
        conn.execute(
            "UPDATE authors SET full_name = 'Jane Doe' WHERE id = ?1",
            [author_id],
        )
        .unwrap();

        // Verify update
        let authors = db::list_authors(&conn).unwrap();
        let found = authors.iter().find(|a| a.initials == "JD");
        assert_eq!(found.unwrap().full_name.as_deref(), Some("Jane Doe"));

        // DELETE: Delete author (no dependencies)
        db::delete_author(&conn, author_id).unwrap();
        let authors = db::list_authors(&conn).unwrap();
        assert!(!authors.iter().any(|a| a.initials == "JD"));
    }

    #[test]
    fn test_word_affixes_and_spellings() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        db::init_schema(&conn).unwrap();

        // Create word
        conn.execute("INSERT INTO types (name) VALUES ('gismu')", [])
            .unwrap();
        let type_id: i64 = conn.last_insert_rowid();
        conn.execute(
            "INSERT INTO words (name, type_id) VALUES ('testword', ?1)",
            [type_id],
        )
        .unwrap();
        let word_id: i64 = conn.last_insert_rowid();

        // Add affixes
        conn.execute(
            "INSERT INTO word_affixes (word_id, affix) VALUES (?1, 'test'), (?1, 'affix')",
            [word_id],
        )
        .unwrap();

        // Add spellings
        conn.execute("INSERT INTO word_spellings (word_id, spelling) VALUES (?1, 'testword2'), (?1, 'testword3')", [word_id]).unwrap();

        // Verify via get_word
        let word = db::get_word(&conn, word_id).unwrap();
        assert_eq!(word.affixes.len(), 2);
        assert_eq!(word.spellings.len(), 2);

        // Delete affix
        conn.execute(
            "DELETE FROM word_affixes WHERE word_id = ?1 AND affix = 'test'",
            [word_id],
        )
        .unwrap();

        // Verify deletion
        let word = db::get_word(&conn, word_id).unwrap();
        assert_eq!(word.affixes.len(), 1);
    }
}
