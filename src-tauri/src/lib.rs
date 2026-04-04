//! LOD Manager — Tauri backend.
//!
//! # Architecture
//! - Tauri commands live in `commands/` submodules grouped by domain
//! - `AppState` holds the shared database connection (`Mutex<Option<Connection>>`)
//! - Database operations live in `db.rs`, import/export in their own modules
//!
//! # Error handling
//! All commands return `Result<T, String>` using the `Res<T>` type alias.
//! Errors are converted via the `err()` helper which implements `Display`.
// Clippy configuration — applied project-wide
#![warn(clippy::all)]
#![warn(clippy::pedantic)]
#![allow(clippy::needless_pass_by_value)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::wildcard_imports)]
#![allow(clippy::too_many_lines)]
#![allow(clippy::cast_precision_loss)]

mod commands;
mod db;
mod export;
mod import;
mod models;

use commands::AppState;
use std::sync::Mutex;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .setup(|app| {
            #[cfg(desktop)]
            {
                let handle = app.handle();
                handle.plugin(tauri_plugin_updater::Builder::new().build())?;
                eprintln!("[Updater] Plugin initialized with endpoints from config");
            }
            Ok(())
        })
        .manage(AppState {
            db: Mutex::new(None),
            db_path: Mutex::new(String::new()),
        })
        .invoke_handler(tauri::generate_handler![
            commands::database::open_database,
            commands::database::create_database,
            commands::database::get_db_stats,
            commands::database::get_default_db_path,
            commands::words::get_words,
            commands::words::get_word,
            commands::words::save_word,
            commands::words::delete_word,
            commands::words::save_definition,
            commands::words::delete_definition,
            commands::events::get_events,
            commands::events::save_event,
            commands::events::delete_event,
            commands::events::get_event_words,
            commands::types::get_types,
            commands::types::save_type,
            commands::types::delete_type,
            commands::authors::get_authors,
            commands::authors::save_author,
            commands::authors::delete_author,
            commands::import::import_lod_contents,
            commands::import::import_lod_files,
            commands::search::search_english,
            commands::search::rebuild_fts,
            commands::search::fts_is_ready,
            commands::export::export_html,
            commands::export::export_html_to_file,
            #[cfg(desktop)]
            debug_update_check
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// Desktop-only update check command.
#[cfg(desktop)]
#[tauri::command]
fn debug_update_check(app: tauri::AppHandle) -> commands::Res<String> {
    use tauri::async_runtime::block_on;
    use tauri_plugin_updater::UpdaterExt;
    eprintln!("[Updater] debug_update_check called");

    match app.updater() {
        Ok(updater) => {
            eprintln!("[Updater] Updater obtained, calling check()");
            let result = block_on(updater.check());
            match result {
                Ok(Some(update)) => {
                    eprintln!("[Updater] Update found: {}", update.version);
                    Ok(format!("Update available: {}", update.version))
                }
                Ok(None) => {
                    eprintln!("[Updater] No update available");
                    Ok("No update available".to_string())
                }
                Err(e) => {
                    eprintln!("[Updater] Check error: {e:?}");
                    Err(format!("Check failed: {e:?}"))
                }
            }
        }
        Err(e) => {
            eprintln!("[Updater] Failed to get updater: {e:?}");
            Err(format!("Updater error: {e:?}"))
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::db;
    use crate::import;
    use crate::models;
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
        db::add_missing_indexes(&conn).unwrap();

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

        conn.execute(
            "INSERT INTO word_affixes (word_id, affix) VALUES (?1, ?2), (?1, ?3)",
            (word_id, "test", "affix"),
        )
        .unwrap();

        conn.execute(
            "INSERT INTO word_spellings (word_id, spelling) VALUES (?1, ?2), (?1, ?3)",
            (word_id, "spelling1", "spelling2"),
        )
        .unwrap();

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

        assert!(duration.as_millis() < 1000, "get_word should be very fast");

        let result = db::get_word(&conn, 999_999);
        assert!(result.is_err(), "Invalid word ID should return error");
    }

    #[test]
    fn test_fts_update_incremental() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        db::init_schema(&conn).unwrap();
        db::init_fts(&conn).unwrap();

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

        db::rebuild_fts(&conn).unwrap();

        assert!(db::fts_is_ready(&conn), "FTS should be ready after rebuild");

        let results = db::search_english_fts(&conn, "test", 10).unwrap();
        assert!(!results.is_empty(), "Should find 'testword' by 'test'");

        db::fts_update(
            &conn,
            def_id,
            "This is an updated definition about something else",
        )
        .ok();

        let results = db::search_english_fts(&conn, "updated", 10).unwrap();
        assert!(
            !results.is_empty(),
            "New term 'updated' should match after FTS update"
        );
    }

    #[test]
    fn test_list_words_basic() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        db::init_schema(&conn).unwrap();
        db::init_fts(&conn).unwrap();

        conn.execute("INSERT INTO types (name) VALUES ('gismu')", [])
            .unwrap();
        let type_id: i64 = conn.last_insert_rowid();

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

        let words = db::list_words(&conn, "", "", None).unwrap();
        assert_eq!(words.len(), 3);

        let words = db::list_words(&conn, "a", "", None).unwrap();
        assert_eq!(words.len(), 1);
        assert_eq!(words[0].name, "abc");

        let words = db::list_words(&conn, "x*", "", None).unwrap();
        assert_eq!(words.len(), 1);
        assert_eq!(words[0].name, "xyz");

        let words = db::list_words(&conn, "", "gismu", None).unwrap();
        assert_eq!(words.len(), 3);

        let words = db::list_words(&conn, "", "", Some(999)).unwrap();
        assert_eq!(words.len(), 0);
    }

    #[test]
    fn test_fts_search_basic() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        db::init_schema(&conn).unwrap();
        db::init_fts(&conn).unwrap();

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

        db::rebuild_fts(&conn).unwrap();

        let results = db::search_english_fts(&conn, "desire", 10).unwrap();
        assert!(!results.is_empty());
    }

    #[test]
    fn test_word_crud_operations() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        db::init_schema(&conn).unwrap();

        conn.execute(
            "INSERT INTO types (name, group_) VALUES ('gismu', 'core')",
            [],
        )
        .unwrap();
        let type_id: i64 = conn.last_insert_rowid();

        conn.execute(
            "INSERT INTO words (name, type_id, source, year, rank, match_, origin, notes)
             VALUES ('testword', ?1, 'test_source', '2024', 'A', 'exact', 'test_origin', 'test_notes')",
            [type_id],
        ).unwrap();
        let word_id: i64 = conn.last_insert_rowid();
        assert!(word_id > 0);

        let word = db::get_word(&conn, word_id).unwrap();
        assert_eq!(word.name, "testword");
        assert_eq!(word.definitions.len(), 0);
        assert_eq!(word.affixes.len(), 0);

        conn.execute(
            "INSERT INTO definitions (word_id, position, grammar, usage, body, tags) VALUES (?1, 0, 'GU', 'test', 'first definition', 'tag1')",
            [word_id],
        ).unwrap();
        conn.execute(
            "INSERT INTO definitions (word_id, position, grammar, usage, body, tags) VALUES (?1, 1, 'N', 'test', 'second definition', 'tag2')",
            [word_id],
        ).unwrap();

        let updated_word = db::get_word(&conn, word_id).unwrap();
        assert_eq!(updated_word.definitions.len(), 2);

        db::delete_word(&conn, word_id).unwrap();
        let result = db::get_word(&conn, word_id);
        assert!(result.is_err(), "Word should be deleted");
    }

    #[test]
    fn test_definition_crud_operations() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        db::init_schema(&conn).unwrap();

        conn.execute("INSERT INTO types (name) VALUES ('gismu')", [])
            .unwrap();
        let type_id: i64 = conn.last_insert_rowid();
        conn.execute(
            "INSERT INTO words (name, type_id) VALUES ('testword', ?1)",
            [type_id],
        )
        .unwrap();
        let word_id: i64 = conn.last_insert_rowid();

        let def_data = models::SaveDefinition {
            grammar: Some("GU".to_string()),
            usage: Some("verb".to_string()),
            body: "to want".to_string(),
            tags: Some("main".to_string()),
        };
        db::save_definition(&conn, None, word_id, &def_data).unwrap();

        let word = db::get_word(&conn, word_id).unwrap();
        assert_eq!(word.definitions.len(), 1);
        let def_id = word.definitions[0].id;

        db::delete_definition(&conn, def_id).unwrap();

        let updated_def = models::SaveDefinition {
            grammar: Some("GU".to_string()),
            usage: Some("verb".to_string()),
            body: "to strongly want".to_string(),
            tags: Some("updated".to_string()),
        };
        db::save_definition(&conn, None, word_id, &updated_def).unwrap();

        let updated_word = db::get_word(&conn, word_id).unwrap();
        assert!(updated_word.definitions[0].body.contains("strongly"));

        let new_def_id = updated_word.definitions[0].id;
        db::delete_definition(&conn, new_def_id).unwrap();
        let final_word = db::get_word(&conn, word_id).unwrap();
        assert_eq!(final_word.definitions.len(), 0);
    }

    #[test]
    fn test_type_crud_operations() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        db::init_schema(&conn).unwrap();

        conn.execute(
            "INSERT INTO types (name, group_) VALUES ('lujvo', 'derived')",
            [],
        )
        .unwrap();
        let type_id: i64 = conn.last_insert_rowid();
        assert!(type_id > 0);

        let types = db::list_types(&conn).unwrap();
        let found = types.iter().find(|t| t.name == "lujvo");
        assert!(found.is_some());
        assert_eq!(found.unwrap().group_.as_deref(), Some("derived"));

        conn.execute(
            "UPDATE types SET group_ = 'modified' WHERE id = ?1",
            [type_id],
        )
        .unwrap();

        let types = db::list_types(&conn).unwrap();
        let found = types.iter().find(|t| t.name == "lujvo");
        assert_eq!(found.unwrap().group_.as_deref(), Some("modified"));

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

        conn.execute(
            "INSERT INTO events (name, date, annotation, suffix, notes) VALUES ('NewEvent', '2024-06-15', 'test annotation', 'NE', 'test notes')",
            [],
        ).unwrap();
        let event_id: i64 = conn.last_insert_rowid();
        assert!(event_id > 0);

        let events = db::list_events(&conn).unwrap();
        let found = events.iter().find(|e| e.name == "NewEvent");
        assert!(found.is_some());
        assert_eq!(found.unwrap().date.as_deref(), Some("2024-06-15"));

        conn.execute(
            "UPDATE events SET date = '2024-07-01', annotation = 'updated' WHERE id = ?1",
            [event_id],
        )
        .unwrap();

        let events = db::list_events(&conn).unwrap();
        let found = events.iter().find(|e| e.name == "NewEvent");
        assert_eq!(found.unwrap().date.as_deref(), Some("2024-07-01"));

        let events = db::list_events(&conn).unwrap();
        assert!(events.len() >= 2);
    }

    #[test]
    fn test_author_crud_operations() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        db::init_schema(&conn).unwrap();

        conn.execute("INSERT INTO authors (initials, full_name, notes) VALUES ('JD', 'John Doe', 'test author')", []).unwrap();
        let author_id: i64 = conn.last_insert_rowid();
        assert!(author_id > 0);

        let authors = db::list_authors(&conn).unwrap();
        let found = authors.iter().find(|a| a.initials == "JD");
        assert!(found.is_some());
        assert_eq!(found.unwrap().full_name.as_deref(), Some("John Doe"));

        conn.execute(
            "UPDATE authors SET full_name = 'Jane Doe' WHERE id = ?1",
            [author_id],
        )
        .unwrap();

        let authors = db::list_authors(&conn).unwrap();
        let found = authors.iter().find(|a| a.initials == "JD");
        assert_eq!(found.unwrap().full_name.as_deref(), Some("Jane Doe"));

        db::delete_author(&conn, author_id).unwrap();
        let authors = db::list_authors(&conn).unwrap();
        assert!(!authors.iter().any(|a| a.initials == "JD"));
    }

    #[test]
    fn test_word_affixes_and_spellings() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        db::init_schema(&conn).unwrap();

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
            "INSERT INTO word_affixes (word_id, affix) VALUES (?1, 'test'), (?1, 'affix')",
            [word_id],
        )
        .unwrap();

        conn.execute("INSERT INTO word_spellings (word_id, spelling) VALUES (?1, 'testword2'), (?1, 'testword3')", [word_id]).unwrap();

        let word = db::get_word(&conn, word_id).unwrap();
        assert_eq!(word.affixes.len(), 2);
        assert_eq!(word.spellings.len(), 2);

        conn.execute(
            "DELETE FROM word_affixes WHERE word_id = ?1 AND affix = 'test'",
            [word_id],
        )
        .unwrap();

        let word = db::get_word(&conn, word_id).unwrap();
        assert_eq!(word.affixes.len(), 1);
    }

    #[test]
    fn test_migrate_words_unique_if_needed() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        db::init_schema(&conn).unwrap();

        db::migrate_words_unique_if_needed(&conn).unwrap();
        let flag: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM settings WHERE key='words_unique_migrated'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(flag, 1);

        db::migrate_words_unique_if_needed(&conn).unwrap();
    }

    #[test]
    fn test_migrate_event_columns_if_needed() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        db::init_schema(&conn).unwrap();

        conn.execute(
            "INSERT INTO events (name, annotation, notes) VALUES ('TestEvent', 'ann', 'notes')",
            [],
        )
        .unwrap();

        db::migrate_event_columns_if_needed(&conn).unwrap();

        let flag: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM settings WHERE key='ev_col_migrated'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(flag, 1);

        let (ann, notes): (String, String) = conn
            .query_row(
                "SELECT annotation, notes FROM events WHERE name='TestEvent'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(ann, "notes");
        assert_eq!(notes, "ann");

        db::migrate_event_columns_if_needed(&conn).unwrap();
        let (ann2, notes2): (String, String) = conn
            .query_row(
                "SELECT annotation, notes FROM events WHERE name='TestEvent'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(ann2, "notes");
        assert_eq!(notes2, "ann");
    }

    #[test]
    fn test_export_html_empty() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        db::init_schema(&conn).unwrap();

        let html = crate::export::generate_html(&conn, None, false).unwrap();
        assert!(html.contains("No words found"));
    }

    #[test]
    fn test_export_html_with_data() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        db::init_schema(&conn).unwrap();

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
            "INSERT INTO definitions (word_id, position, body) VALUES (?1, 0, 'to want to desire')",
            [word_id],
        )
        .unwrap();

        let html = crate::export::generate_html(&conn, None, false).unwrap();
        assert!(html.contains("camgu"));
        assert!(html.contains("to want to desire"));
        assert!(html.contains("gismu"));
    }

    #[test]
    fn test_fts_rebuild_and_search() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        db::init_schema(&conn).unwrap();
        db::init_fts(&conn).unwrap();

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
            "INSERT INTO definitions (word_id, position, body) VALUES (?1, 0, 'this is a test definition about something')",
            [word_id],
        ).unwrap();

        db::rebuild_fts(&conn).unwrap();

        assert!(db::fts_is_ready(&conn), "FTS should be ready after rebuild");

        let results = db::search_english_fts(&conn, "test", 10).unwrap();
        assert!(!results.is_empty(), "Should find 'testword' by 'test'");
        assert_eq!(results[0].word_name, "testword");

        let results = db::search_english_like(&conn, "definition", 10).unwrap();
        assert!(
            !results.is_empty(),
            "LIKE fallback should find 'definition'"
        );
    }

    #[test]
    fn test_fts_update_after_save() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        db::init_schema(&conn).unwrap();
        db::init_fts(&conn).unwrap();

        conn.execute("INSERT INTO types (name) VALUES ('gismu')", [])
            .unwrap();
        let type_id: i64 = conn.last_insert_rowid();
        conn.execute(
            "INSERT INTO words (name, type_id) VALUES ('word1', ?1)",
            [type_id],
        )
        .unwrap();
        let word_id: i64 = conn.last_insert_rowid();

        let def_data = models::SaveDefinition {
            grammar: Some("GU".to_string()),
            usage: None,
            body: "original text".to_string(),
            tags: None,
        };
        db::save_definition(&conn, None, word_id, &def_data).unwrap();
        let def_id: i64 = conn
            .query_row(
                "SELECT id FROM definitions ORDER BY id DESC LIMIT 1",
                [],
                |r| r.get(0),
            )
            .unwrap();

        db::rebuild_fts(&conn).unwrap();

        db::fts_update(&conn, def_id, "updated text with newterm").unwrap();

        let results = db::search_english_fts(&conn, "newterm", 10).unwrap();
        assert!(
            !results.is_empty(),
            "Should find updated definition by 'newterm'"
        );
    }

    #[test]
    fn test_search_english_strategies() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        db::init_schema(&conn).unwrap();
        db::init_fts(&conn).unwrap();

        conn.execute("INSERT INTO types (name) VALUES ('gismu')", [])
            .unwrap();
        let type_id: i64 = conn.last_insert_rowid();
        conn.execute(
            "INSERT INTO words (name, type_id) VALUES ('kwtest', ?1)",
            [type_id],
        )
        .unwrap();
        let word_id: i64 = conn.last_insert_rowid();
        conn.execute(
            "INSERT INTO definitions (word_id, position, body) VALUES (?1, 0, 'text with \\u{AB}keyword\\u{BB} marker')",
            [word_id],
        ).unwrap();

        db::rebuild_fts(&conn).unwrap();

        let results = db::search_english_fts(&conn, "keyword", 10).unwrap();
        assert!(!results.is_empty(), "Should find keyword in definition");
    }

    #[test]
    fn test_import_files_empty_paths() {
        let mut conn = rusqlite::Connection::open_in_memory().unwrap();
        db::init_schema(&conn).unwrap();

        let result = import::import_files(&mut conn, &[]).unwrap();
        assert_eq!(result.words, 0);
        assert_eq!(result.definitions, 0);
        assert_eq!(result.errors, 0);
    }

    #[test]
    fn test_import_files_malformed_data() {
        let mut conn = rusqlite::Connection::open_in_memory().unwrap();
        db::init_schema(&conn).unwrap();

        let tmp_dir = std::env::temp_dir().join("lod_test_malformed");
        let _ = std::fs::remove_dir_all(&tmp_dir);
        std::fs::create_dir_all(&tmp_dir).unwrap();

        let types_path = tmp_dir.join("types.txt");
        std::fs::write(&types_path, "gismu@core\n\n@bad\ncmavo@particle\n").unwrap();

        let paths = vec![types_path.to_string_lossy().into_owned()];
        let result = import::import_files(&mut conn, &paths).unwrap();
        assert_eq!(result.types, 2);

        let _ = std::fs::remove_dir_all(&tmp_dir);
    }

    #[test]
    fn test_import_contents_android() {
        let mut conn = rusqlite::Connection::open_in_memory().unwrap();
        db::init_schema(&conn).unwrap();

        let files = vec![("types.txt".to_string(), "gismu@core\n".to_string())];
        let result = import::import_contents(&mut conn, &files).unwrap();
        assert_eq!(result.types, 1);
        assert_eq!(result.skipped_rows, 0);
    }

    #[test]
    fn test_import_skipped_rows_counted() {
        let mut conn = rusqlite::Connection::open_in_memory().unwrap();
        db::init_schema(&conn).unwrap();

        let tmp_dir = std::env::temp_dir().join("lod_test_skipped");
        let _ = std::fs::remove_dir_all(&tmp_dir);
        std::fs::create_dir_all(&tmp_dir).unwrap();

        let types_path = tmp_dir.join("types.txt");
        std::fs::write(&types_path, "gismu@core\n\n@\ncmavo@particle\n\n").unwrap();

        let authors_path = tmp_dir.join("author.txt");
        std::fs::write(&authors_path, "JD@John Doe\n\n@NoName\n").unwrap();

        let paths = vec![
            types_path.to_string_lossy().into_owned(),
            authors_path.to_string_lossy().into_owned(),
        ];
        let result = import::import_files(&mut conn, &paths).unwrap();

        assert_eq!(result.types, 2);
        assert_eq!(result.authors, 1);

        assert!(
            result.skipped_rows > 0,
            "Should have skipped some rows, got {}",
            result.skipped_rows
        );

        let has_skipped_msg: bool = result
            .messages
            .iter()
            .any(|m: &String| m.contains("Skipped"));
        assert!(
            has_skipped_msg,
            "Should have skipped rows message, got: {:?}",
            result.messages
        );

        let _ = std::fs::remove_dir_all(&tmp_dir);
    }

    #[test]
    fn test_import_no_skipped_rows_clean_data() {
        let mut conn = rusqlite::Connection::open_in_memory().unwrap();
        db::init_schema(&conn).unwrap();

        let tmp_dir = std::env::temp_dir().join("lod_test_clean");
        let _ = std::fs::remove_dir_all(&tmp_dir);
        std::fs::create_dir_all(&tmp_dir).unwrap();

        let types_path = tmp_dir.join("types.txt");
        std::fs::write(&types_path, "gismu@core\ncmavo@particle\n").unwrap();

        let paths = vec![types_path.to_string_lossy().into_owned()];
        let result = import::import_files(&mut conn, &paths).unwrap();

        assert_eq!(result.types, 2);
        assert_eq!(result.skipped_rows, 0);

        let has_skipped_msg: bool = result
            .messages
            .iter()
            .any(|m: &String| m.contains("Skipped"));
        assert!(
            !has_skipped_msg,
            "Should not have skipped rows message for clean data"
        );

        let _ = std::fs::remove_dir_all(&tmp_dir);
    }

    #[test]
    fn test_import_skipped_rows_empty_file() {
        let mut conn = rusqlite::Connection::open_in_memory().unwrap();
        db::init_schema(&conn).unwrap();

        let tmp_dir = std::env::temp_dir().join("lod_test_empty");
        let _ = std::fs::remove_dir_all(&tmp_dir);
        std::fs::create_dir_all(&tmp_dir).unwrap();

        let types_path = tmp_dir.join("types.txt");
        std::fs::write(&types_path, "").unwrap();

        let paths = vec![types_path.to_string_lossy().into_owned()];
        let result = import::import_files(&mut conn, &paths).unwrap();

        assert_eq!(result.types, 0);
        assert_eq!(result.skipped_rows, 0);

        let _ = std::fs::remove_dir_all(&tmp_dir);
    }
}
