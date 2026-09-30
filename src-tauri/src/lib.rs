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
mod converter;
mod db;
mod export;
mod import;
mod models;

use commands::AppState;
use std::sync::Mutex;

// Re-export converter functions for easier access
pub use converter::converter::convert_text_to_sqlite;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .setup(|app| {
            #[cfg(not(desktop))]
            let _ = app;
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
            commands::import::convert_text_files,
            commands::search::search_english,
            commands::search::rebuild_fts,
            commands::search::compact_db,
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
async fn debug_update_check(app: tauri::AppHandle) -> commands::Res<String> {
    use tauri_plugin_updater::UpdaterExt;
    eprintln!("[Updater] debug_update_check called");

    match app.updater() {
        Ok(updater) => {
            eprintln!("[Updater] Updater obtained, calling check()");
            let result = updater.check().await;
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
            "INSERT INTO types (type, \"group\") VALUES (?1, ?2)",
            ("test_type", "test_group"),
        )
        .unwrap();

        let type_id: i64 = conn.last_insert_rowid();

        conn.execute(
            "INSERT INTO words (name, type, year, rank, \"match\", origin, notes, id_old, event_start) 
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 1, 1)",
            (
                "testword",
                type_id,
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
            "INSERT INTO definitions (word_id, position, grammar_code, usage, body, case_tags)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            (word_id, 0, "grammar1", "usage1", "body1", "tags1"),
        )
        .unwrap();

        conn.execute(
            "INSERT INTO definitions (word_id, position, grammar_code, usage, body, case_tags)
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

        assert!(duration.as_millis() < 1000, "get_word should be very fast");

        let result = db::get_word(&conn, 999_999);
        assert!(result.is_err(), "Invalid word ID should return error");
    }

    #[test]
    fn test_fts_update_incremental() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        db::init_schema(&conn).unwrap();
        db::init_fts(&conn).unwrap();

        conn.execute("INSERT INTO types (type) VALUES ('gismu')", [])
            .unwrap();
        let type_id: i64 = conn.last_insert_rowid();
        conn.execute(
            "INSERT INTO words (name, type, id_old, event_start) VALUES ('testword', ?1, 1, 1)",
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

        conn.execute("INSERT INTO types (type) VALUES ('gismu')", [])
            .unwrap();
        let type_id: i64 = conn.last_insert_rowid();

        conn.execute(
            "INSERT INTO words (name, type, id_old, event_start) VALUES ('abc', ?1, 1, 1)",
            [type_id],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO words (name, type, id_old, event_start) VALUES ('xyz', ?1, 2, 1)",
            [type_id],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO words (name, type, id_old, event_start) VALUES ('def', ?1, 3, 1)",
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

        let words = db::list_words(&conn, "", "", Some(0)).unwrap();
        assert_eq!(words.len(), 0);
    }

    #[test]
    fn test_fts_search_basic() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        db::init_schema(&conn).unwrap();
        db::init_fts(&conn).unwrap();

        conn.execute("INSERT INTO types (type) VALUES ('gismu')", [])
            .unwrap();
        let type_id: i64 = conn.last_insert_rowid();
        conn.execute(
            "INSERT INTO words (name, type, id_old, event_start) VALUES ('camgu', ?1, 1, 1)",
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
            "INSERT INTO types (type, \"group\") VALUES ('gismu', 'core')",
            [],
        )
        .unwrap();
        let type_id: i64 = conn.last_insert_rowid();

        conn.execute(
            "INSERT INTO words (name, type, year, rank, \"match\", origin, notes, id_old, event_start)
             VALUES ('testword', ?1, '2024', 'A', 'exact', 'test_origin', 'test_notes', 1, 1)",
            [type_id],
        ).unwrap();
        let word_id: i64 = conn.last_insert_rowid();
        assert!(word_id > 0);

        let word = db::get_word(&conn, word_id).unwrap();
        assert_eq!(word.name, "testword");
        assert_eq!(word.definitions.len(), 0);
        assert_eq!(word.affixes.len(), 0);

        conn.execute(
            "INSERT INTO definitions (word_id, position, grammar_code, usage, body, case_tags) VALUES (?1, 0, 'GU', 'test', 'first definition', 'tag1')",
            [word_id],
        ).unwrap();
        conn.execute(
            "INSERT INTO definitions (word_id, position, grammar_code, usage, body, case_tags) VALUES (?1, 1, 'N', 'test', 'second definition', 'tag2')",
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

        conn.execute("INSERT INTO types (type) VALUES ('gismu')", [])
            .unwrap();
        let type_id: i64 = conn.last_insert_rowid();
        conn.execute(
            "INSERT INTO words (name, type, id_old, event_start) VALUES ('testword', ?1, 1, 1)",
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

        let type_id = db::save_type(
            &conn,
            None,
            &models::SaveType {
                name: "lujvo".to_string(),
                type_x: Some("Lujvo".to_string()),
                group_: Some("derived".to_string()),
                parentable: Some(true),
                description: Some("Derived word".to_string()),
            },
        )
        .unwrap();
        assert!(type_id > 0);

        let types = db::list_types(&conn).unwrap();
        let found = types.iter().find(|t| t.name == "lujvo");
        assert!(found.is_some());
        assert_eq!(found.unwrap().group_.as_deref(), Some("derived"));

        db::save_type(
            &conn,
            Some(type_id),
            &models::SaveType {
                name: "lujvo".to_string(),
                type_x: Some("Lujvo".to_string()),
                group_: Some("modified".to_string()),
                parentable: Some(true),
                description: None,
            },
        )
        .unwrap();

        let types = db::list_types(&conn).unwrap();
        let found = types.iter().find(|t| t.name == "lujvo");
        assert_eq!(found.unwrap().group_.as_deref(), Some("modified"));

        conn.execute(
            "INSERT INTO words (name, type, id_old, event_start) VALUES ('testlujvo', ?1, 1, 1)",
            [type_id],
        )
        .unwrap();
        let result = db::delete_type(&conn, type_id);
        assert!(result.is_err(), "Cannot delete type with dependent words");
    }

    #[test]
    fn test_event_crud_operations() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        db::init_schema(&conn).unwrap();

        let event_id = db::save_event(
            &conn,
            None,
            &models::SaveEvent {
                name: "NewEvent".to_string(),
                date: Some("2024-06-15".to_string()),
                annotation: Some("test annotation".to_string()),
                suffix: Some("NE".to_string()),
                notes: Some("test notes".to_string()),
            },
        )
        .unwrap();
        assert!(event_id > 0);

        let events = db::list_events(&conn).unwrap();
        let found = events.iter().find(|e| e.name == "NewEvent");
        assert!(found.is_some());
        assert_eq!(found.unwrap().date.as_deref(), Some("2024-06-15"));

        db::save_event(
            &conn,
            Some(event_id),
            &models::SaveEvent {
                name: "NewEvent".to_string(),
                date: Some("2024-07-01".to_string()),
                annotation: Some("updated".to_string()),
                suffix: Some("NE".to_string()),
                notes: Some("test notes".to_string()),
            },
        )
        .unwrap();

        let events = db::list_events(&conn).unwrap();
        let found = events.iter().find(|e| e.name == "NewEvent");
        assert_eq!(found.unwrap().date.as_deref(), Some("2024-07-01"));
        assert!(events.len() >= 2);
    }

    #[test]
    fn test_author_crud_operations() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        db::init_schema(&conn).unwrap();

        let author_id = db::save_author(
            &conn,
            None,
            &models::SaveAuthor {
                initials: "JD".to_string(),
                full_name: Some("John Doe".to_string()),
                notes: Some("test author".to_string()),
            },
        )
        .unwrap();
        assert!(author_id > 0);

        let authors = db::list_authors(&conn).unwrap();
        let found = authors.iter().find(|a| a.initials == "JD");
        assert!(found.is_some());
        assert_eq!(found.unwrap().full_name.as_deref(), Some("John Doe"));

        db::save_author(
            &conn,
            Some(author_id),
            &models::SaveAuthor {
                initials: "JD".to_string(),
                full_name: Some("Jane Doe".to_string()),
                notes: Some("test author".to_string()),
            },
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

        conn.execute("INSERT INTO types (type) VALUES ('gismu')", [])
            .unwrap();
        let type_id: i64 = conn.last_insert_rowid();
        conn.execute(
            "INSERT INTO words (name, type, id_old, event_start) VALUES ('testword', ?1, 1, 1)",
            [type_id],
        )
        .unwrap();
        let word_id: i64 = conn.last_insert_rowid();

        conn.execute(
            "INSERT INTO word_affixes (word_id, affix) VALUES (?1, 'test'), (?1, 'affix')",
            [word_id],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO word_spellings (word_id, spelling) VALUES (?1, 'testword2'), (?1, 'testword3')",
            [word_id],
        )
        .unwrap();

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
    fn test_p0_1_definition_slots_and_grammar_code() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        db::init_schema(&conn).unwrap();
        db::init_fts(&conn).unwrap();

        conn.execute("INSERT INTO types (type) VALUES ('C-Prim')", [])
            .unwrap();
        let type_id: i64 = conn.last_insert_rowid();
        conn.execute(
            "INSERT INTO words (name, type, id_old, event_start) VALUES ('ekti', ?1, 100, 1)",
            [type_id],
        )
        .unwrap();
        let word_id: i64 = conn.last_insert_rowid();

        // Simulate frontend IPC payload {"grammar": "2a", "usage": "%", "body": "«act» on something", "tags": "B-K"}
        let ipc_json = r#"{"grammar":"2a","usage":"%","body":"«act» on something","tags":"B-K"}"#;
        let save_def: models::SaveDefinition = serde_json::from_str(ipc_json).unwrap();
        assert_eq!(save_def.grammar.as_deref(), Some("2a"));
        assert_eq!(save_def.tags.as_deref(), Some("B-K"));

        db::save_definition(&conn, None, word_id, &save_def).unwrap();

        // Verify slots and grammar_code are split in SQLite table (loglan_core parity)
        let (slots, gcode, ctags): (Option<i64>, Option<String>, Option<String>) = conn
            .query_row(
                "SELECT slots, grammar_code, case_tags FROM definitions WHERE word_id = ?1",
                [word_id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .unwrap();
        assert_eq!(slots, Some(2));
        assert_eq!(gcode.as_deref(), Some("a"));
        assert_eq!(ctags.as_deref(), Some("B-K"));

        // Verify get_word combines slots + grammar_code into "2a" and serializes "grammar" & "tags"
        let word = db::get_word(&conn, word_id).unwrap();
        assert_eq!(word.definitions.len(), 1);
        assert_eq!(word.definitions[0].grammar.as_deref(), Some("2a"));
        assert_eq!(word.definitions[0].tags.as_deref(), Some("B-K"));
        let serialized = serde_json::to_value(&word.definitions[0]).unwrap();
        assert_eq!(serialized["grammar"], "2a");
        assert_eq!(serialized["tags"], "B-K");

        // Verify FTS and LIKE searches and HTML export also return combined "2a"
        db::rebuild_fts(&conn).unwrap();
        let fts_res = db::search_english_fts(&conn, "act", 10).unwrap();
        assert_eq!(fts_res[0].grammar.as_deref(), Some("2a"));
        let kw_res = db::search_english_keywords_fts(&conn, "act", 10).unwrap();
        assert_eq!(kw_res[0].grammar.as_deref(), Some("2a"));
        let like_res = db::search_english_like(&conn, "act", 10).unwrap();
        assert_eq!(like_res[0].grammar.as_deref(), Some("2a"));
        let html = crate::export::generate_html(&conn, None).unwrap();
        assert!(html.contains("(2a)"));
    }

    #[test]
    fn test_p0_3_get_event_words() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        db::init_schema(&conn).unwrap();

        conn.execute("INSERT INTO types (type) VALUES ('C-Prim')", [])
            .unwrap();
        let type_id: i64 = conn.last_insert_rowid();
        conn.execute(
            "INSERT INTO events (event_id, name, date) VALUES (2, 'EventTwo', '2020-01-01')",
            [],
        )
        .unwrap();
        let ev2_pk: i64 = conn.last_insert_rowid();

        conn.execute(
            "INSERT INTO words (name, type, id_old, event_start, event_end) VALUES ('oldword', ?1, 1, 1, 2)",
            [type_id],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO words (name, type, id_old, event_start, event_end) VALUES ('newword', ?1, 2, 2, NULL)",
            [type_id],
        )
        .unwrap();

        let (added, removed) = db::get_event_words(&conn, ev2_pk).unwrap();
        assert_eq!(added, vec!["newword".to_string()]);
        assert_eq!(removed, vec!["oldword".to_string()]);
    }

    #[test]
    fn test_p0_4_save_word_and_p3_4_rename_cascade() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        db::init_schema(&conn).unwrap();

        conn.execute("INSERT INTO types (type) VALUES ('C-Prim')", [])
            .unwrap();

        let wid = db::save_word(
            &conn,
            None,
            &models::SaveWord {
                name: "oldname".to_string(),
                type_name: Some("C-Prim".to_string()),
                source: Some("JCB".to_string()),
                year: Some("1975".to_string()),
                rank: Some("1.0".to_string()),
                match_: Some("99%".to_string()),
                origin: Some("origin".to_string()),
                origin_x: Some("origin_x".to_string()),
                notes: Some("custom note".to_string()),
                event_start: Some("Start".to_string()),
                event_end: None,
                affixes: vec!["old".to_string()],
                spellings: vec![],
                id_old: Some(42),
                event_start_id: None,
                event_end_id: None,
            },
        )
        .unwrap();

        // Add a word_usage entry referencing "oldname"
        conn.execute(
            "INSERT INTO word_usage (word_id, used_in_word) VALUES (?1, 'oldname')",
            [wid],
        )
        .unwrap();

        // Rename "oldname" -> "newname" via save_word
        db::save_word(
            &conn,
            Some(wid),
            &models::SaveWord {
                name: "newname".to_string(),
                type_name: Some("C-Prim".to_string()),
                source: Some("JCB".to_string()),
                year: Some("1975".to_string()),
                rank: Some("1.0".to_string()),
                match_: Some("99%".to_string()),
                origin: Some("origin".to_string()),
                origin_x: Some("origin_x".to_string()),
                notes: Some("custom note".to_string()),
                event_start: Some("Start".to_string()),
                event_end: None,
                affixes: vec!["new".to_string()],
                spellings: vec![],
                id_old: Some(42),
                event_start_id: None,
                event_end_id: None,
            },
        )
        .unwrap();

        let detail = db::get_word(&conn, wid).unwrap();
        assert_eq!(detail.name, "newname");
        assert_eq!(detail.source.as_deref(), Some("JCB"));
        assert_eq!(detail.used_in, vec!["newname".to_string()]);
    }

    #[test]
    fn test_p0_5_duplicate_word_name_and_type_across_events() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        db::init_schema(&conn).unwrap();
        db::add_missing_indexes(&conn).unwrap();

        conn.execute("INSERT INTO types (type) VALUES ('C-Prim')", [])
            .unwrap();
        let tid: i64 = conn.last_insert_rowid();
        conn.execute(
            "INSERT INTO events (event_id, name) VALUES (6, 'Event6')",
            [],
        )
        .unwrap();

        // Insert two historical versions of 'cenja' with same (name, type) across different event intervals
        conn.execute(
            "INSERT INTO words (name, type, id_old, event_start, event_end) VALUES ('cenja', ?1, 802, 1, 6)",
            [tid],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO words (name, type, id_old, event_start, event_end) VALUES ('cenja', ?1, 10133, 6, NULL)",
            [tid],
        )
        .unwrap();

        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM words WHERE name='cenja'", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(
            count, 2,
            "Both historical and active rows of cenja must coexist"
        );
    }

    #[test]
    fn test_p0_6_settings_loglan_core_schema() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        db::init_schema(&conn).unwrap();

        conn.execute(
            "INSERT INTO settings (date, db_version, last_word_id, db_release) VALUES ('2024-08-22', 1, 10150, '4.5.9')",
            [],
        )
        .unwrap();

        let stats = db::get_db_stats(&conn).unwrap();
        assert!(
            stats
                .settings
                .iter()
                .any(|s| s.key == "db_release" && s.value == "4.5.9")
        );
        assert!(
            stats
                .settings
                .iter()
                .any(|s| s.key == "last_word_id" && s.value == "10150")
        );

        db::upsert_setting(&conn, "db_release", "4.6.0").unwrap();
        let settings = db::list_settings(&conn).unwrap();
        assert!(
            settings
                .iter()
                .any(|s| s.key == "db_release" && s.value == "4.6.0")
        );
    }

    #[test]
    fn test_p0_7_connect_words_authors_year_and_json_notes() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        db::init_schema(&conn).unwrap();

        conn.execute(
            "INSERT INTO types (type, type_x, \"group\") VALUES ('C-Prim', 'Composite Primitive', 'Prim')",
            [],
        )
        .unwrap();
        let prim_tid: i64 = conn.last_insert_rowid();

        conn.execute(
            "INSERT INTO types (type, type_x, \"group\") VALUES ('Afx', 'Affix', 'Affix')",
            [],
        )
        .unwrap();
        let afx_tid: i64 = conn.last_insert_rowid();

        conn.execute(
            "INSERT INTO types (type, type_x, \"group\") VALUES ('2-Cpx', 'Two-Term Complex', 'Cpx')",
            [],
        )
        .unwrap();
        let cpx_tid: i64 = conn.last_insert_rowid();

        conn.execute(
            "INSERT INTO authors (abbreviation, full_name) VALUES ('JCB', 'James Cooke Brown')",
            [],
        )
        .unwrap();
        let aid: i64 = conn.last_insert_rowid();

        // Word with DATE year '1975-01-01' and JSON notes
        conn.execute(
            "INSERT INTO words (name, type, year, rank, notes, id_old, event_start)
             VALUES ('humni', ?1, '1975-01-01', '1.0', '{\"year\":\"(changed ''16)\",\"author\":\"L4\",\"rank\":\"top\"}', 10, 1)",
            [prim_tid],
        )
        .unwrap();
        let humni_id: i64 = conn.last_insert_rowid();

        // Affix with hyphen 'hei-' and JSON 'null' notes
        conn.execute(
            "INSERT INTO words (name, type, notes, id_old, event_start)
             VALUES ('hei-', ?1, 'null', 11, 1)",
            [afx_tid],
        )
        .unwrap();
        let afx_id: i64 = conn.last_insert_rowid();

        // Complex word derived from humni
        conn.execute(
            "INSERT INTO words (name, type, notes, id_old, event_start)
             VALUES ('humcpx', ?1, 'null', 12, 1)",
            [cpx_tid],
        )
        .unwrap();
        let cpx_id: i64 = conn.last_insert_rowid();

        // Derived non-affix, non-complex child word from humni
        conn.execute(
            "INSERT INTO words (name, type, notes, id_old, event_start)
             VALUES ('humda', ?1, 'null', 13, 1)",
            [prim_tid],
        )
        .unwrap();
        let child_id: i64 = conn.last_insert_rowid();

        conn.execute(
            "INSERT INTO connect_words (parent_id, child_id) VALUES (?1, ?2), (?1, ?3), (?1, ?4)",
            (humni_id, afx_id, cpx_id, child_id),
        )
        .unwrap();
        conn.execute(
            "INSERT INTO connect_authors (\"AID\", \"WID\") VALUES (?1, ?2)",
            (aid, humni_id),
        )
        .unwrap();

        let humni = db::get_word(&conn, humni_id).unwrap();
        assert_eq!(humni.affixes, vec!["hei".to_string()]);
        assert_eq!(humni.used_in, vec!["humcpx".to_string()]);
        assert_eq!(humni.children, vec!["humda".to_string()]);
        assert!(humni.parents.is_empty());
        assert_eq!(humni.source.as_deref(), Some("JCB L4"));
        assert_eq!(humni.year.as_deref(), Some("1975 (changed '16)"));
        assert_eq!(humni.rank.as_deref(), Some("1.0 top"));

        let cpx = db::get_word(&conn, cpx_id).unwrap();
        assert_eq!(cpx.notes, None, "JSON 'null' string must become None");
        assert_eq!(
            cpx.parents,
            vec!["humni".to_string()],
            "Complex word should show parent words in parents"
        );
        assert!(
            cpx.children.is_empty(),
            "Complex word must not show parent words in children"
        );

        let humda = db::get_word(&conn, child_id).unwrap();
        assert_eq!(humda.parents, vec!["humni".to_string()]);
        assert!(humda.children.is_empty());

        let authors = db::list_authors(&conn).unwrap();
        assert_eq!(authors[0].word_count, 1);
    }

    #[test]
    fn test_p0_8_delete_without_on_delete_cascade() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        // Create schema mimicking export.db where foreign keys do NOT have ON DELETE CASCADE
        conn.execute_batch(
            "PRAGMA foreign_keys=ON;
             CREATE TABLE types (id INTEGER PRIMARY KEY, type TEXT NOT NULL, type_x TEXT, \"group\" TEXT, parentable BOOLEAN, description TEXT);
             CREATE TABLE events (id INTEGER PRIMARY KEY, event_id INTEGER NOT NULL UNIQUE, name TEXT NOT NULL, date TEXT, definition TEXT, annotation TEXT, suffix TEXT);
             CREATE TABLE authors (id INTEGER PRIMARY KEY, abbreviation TEXT NOT NULL UNIQUE, full_name TEXT, notes TEXT);
             CREATE TABLE words (id INTEGER PRIMARY KEY, name TEXT NOT NULL, type INTEGER NOT NULL REFERENCES types(id), origin TEXT, origin_x TEXT, \"match\" TEXT, rank TEXT, year TEXT, notes TEXT, id_old INTEGER NOT NULL, \"TID_old\" INTEGER, event_start INTEGER NOT NULL REFERENCES events(event_id), event_end INTEGER REFERENCES events(event_id));
             CREATE TABLE definitions (id INTEGER PRIMARY KEY, word_id INTEGER NOT NULL REFERENCES words(id), position INTEGER NOT NULL, body TEXT NOT NULL, usage TEXT, grammar_code TEXT, slots INTEGER, case_tags TEXT, language TEXT, notes TEXT);
             CREATE TABLE keys (id INTEGER PRIMARY KEY, word TEXT NOT NULL, language TEXT);
             CREATE TABLE connect_keys (\"KID\" INTEGER NOT NULL REFERENCES keys(id), \"DID\" INTEGER NOT NULL REFERENCES definitions(id), PRIMARY KEY (\"KID\", \"DID\"));
             CREATE TABLE connect_words (parent_id INTEGER NOT NULL REFERENCES words(id), child_id INTEGER NOT NULL REFERENCES words(id), PRIMARY KEY (parent_id, child_id));
             CREATE TABLE connect_authors (\"AID\" INTEGER NOT NULL REFERENCES authors(id), \"WID\" INTEGER NOT NULL REFERENCES words(id), PRIMARY KEY (\"AID\", \"WID\"));
             INSERT INTO types (id, type) VALUES (1, 'C-Prim');
             INSERT INTO events (id, event_id, name) VALUES (1, 1, 'Start');
             INSERT INTO authors (id, abbreviation) VALUES (1, 'JCB');
             INSERT INTO words (id, name, type, id_old, event_start) VALUES (1, 'w1', 1, 1, 1), (2, 'w2', 1, 2, 1);
             INSERT INTO definitions (id, word_id, position, body) VALUES (10, 1, 1, 'def1'), (20, 2, 1, 'def2');
             INSERT INTO keys (id, word) VALUES (100, 'key1');
             INSERT INTO connect_keys (\"KID\", \"DID\") VALUES (100, 10), (100, 20);
             INSERT INTO connect_words (parent_id, child_id) VALUES (1, 2);
             INSERT INTO connect_authors (\"AID\", \"WID\") VALUES (1, 1), (1, 2);",
        )
        .unwrap();

        db::delete_definition(&conn, 10).unwrap();
        db::delete_author(&conn, 1).unwrap();
        db::delete_word(&conn, 2).unwrap();
    }

    #[test]
    fn test_export_html_empty() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        db::init_schema(&conn).unwrap();

        let html = crate::export::generate_html(&conn, None).unwrap();
        assert!(html.contains("No words found"));
    }

    #[test]
    fn test_export_html_with_data() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        db::init_schema(&conn).unwrap();

        conn.execute("INSERT INTO types (type) VALUES ('gismu')", [])
            .unwrap();
        let type_id: i64 = conn.last_insert_rowid();
        conn.execute(
            "INSERT INTO words (name, type, id_old, event_start) VALUES ('camgu', ?1, 1, 1)",
            [type_id],
        )
        .unwrap();
        let word_id: i64 = conn.last_insert_rowid();
        conn.execute(
            "INSERT INTO definitions (word_id, position, body) VALUES (?1, 0, 'to want to desire')",
            [word_id],
        )
        .unwrap();

        let html = crate::export::generate_html(&conn, None).unwrap();
        assert!(html.contains("camgu"));
        assert!(html.contains("to want to desire"));
        assert!(html.contains("gismu"));
    }

    #[test]
    fn test_fts_rebuild_and_search() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        db::init_schema(&conn).unwrap();
        db::init_fts(&conn).unwrap();

        conn.execute("INSERT INTO types (type) VALUES ('gismu')", [])
            .unwrap();
        let type_id: i64 = conn.last_insert_rowid();
        conn.execute(
            "INSERT INTO words (name, type, id_old, event_start) VALUES ('testword', ?1, 1, 1)",
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

        conn.execute("INSERT INTO types (type) VALUES ('gismu')", [])
            .unwrap();
        let type_id: i64 = conn.last_insert_rowid();
        conn.execute(
            "INSERT INTO words (name, type, id_old, event_start) VALUES ('word1', ?1, 1, 1)",
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

        conn.execute("INSERT INTO types (type) VALUES ('gismu')", [])
            .unwrap();
        let type_id: i64 = conn.last_insert_rowid();
        conn.execute(
            "INSERT INTO words (name, type, id_old, event_start) VALUES ('kwtest', ?1, 1, 1)",
            [type_id],
        )
        .unwrap();
        let word_id: i64 = conn.last_insert_rowid();
        conn.execute(
            "INSERT INTO definitions (word_id, position, body) VALUES (?1, 0, 'text with \u{AB}keyword\u{BB} marker')",
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

        let files = vec![
            (
                "types.txt".to_string(),
                "C-Prim@Composite Primitive@Prim@False@Composite desc\n".to_string(),
            ),
            (
                "lexevent.txt".to_string(),
                "1@Initial@01.01.1975@The initial vocabulary@INIT@init\n".to_string(),
            ),
            (
                "author.txt".to_string(),
                "JCB@James Cooke Brown@Founder\n".to_string(),
            ),
            (
                "words.txt".to_string(),
                "75@C-Prim@@alk@50%@JCB@1975@1.0@3/6E alcohol@alcohol@@10\n".to_string(),
            ),
            (
                "wordspell.txt".to_string(),
                "75@alkooli@@1@1@2\n75@alkoholi@@2@1@9999\n".to_string(),
            ),
            (
                "worddefinition.txt".to_string(),
                "75@1@%@2a@«alcohol» drink@\n".to_string(),
            ),
        ];
        let result = import::import_contents(&mut conn, &files).unwrap();
        assert_eq!(result.types, 1);
        assert_eq!(result.events, 1);
        assert_eq!(result.authors, 1);
        assert_eq!(result.words, 2);
        assert_eq!(
            result.definitions, 2,
            "Both spellings of old_id=75 must get the definition"
        );
        assert_eq!(result.skipped_rows, 0);

        // Verify event annotation vs suffix order and event_id=1 update
        let events = db::list_events(&conn).unwrap();
        let ev1 = events
            .iter()
            .find(|e| e.name == "Initial")
            .expect("event_id 1 should be updated to Initial");
        assert_eq!(ev1.annotation.as_deref(), Some("INIT"));
        assert_eq!(ev1.suffix.as_deref(), Some("init"));

        // Verify types parentable ("False") and description
        let (parentable, desc): (bool, Option<String>) = conn
            .query_row(
                "SELECT parentable, description FROM types WHERE type='C-Prim'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert!(!parentable);
        assert_eq!(desc.as_deref(), Some("Composite desc"));

        // Verify origin_x, TID_old, and event_end=9999 -> NULL
        let (ox, tid_old, ev_end): (Option<String>, Option<i64>, Option<i64>) = conn
            .query_row(
                "SELECT origin_x, \"TID_old\", event_end FROM words WHERE name='alkoholi'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .unwrap();
        assert_eq!(ox.as_deref(), Some("alcohol"));
        assert_eq!(tid_old, Some(10));
        assert_eq!(ev_end, None);
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

    #[test]
    fn test_export_db_real_compatibility() {
        let export_path = std::path::Path::new("../export.db");
        if !export_path.exists() {
            return;
        }
        let conn = rusqlite::Connection::open_with_flags(
            export_path,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .unwrap();

        let stats = db::get_db_stats(&conn).unwrap();
        assert_eq!(stats.word_count, 10_173);
        assert_eq!(stats.definition_count, 18_766);
        assert!(
            stats
                .settings
                .iter()
                .any(|s| s.key == "db_release" && !s.value.is_empty()),
            "Should read db_release from export.db settings, got: {:?}",
            stats.settings
        );

        let types = db::list_types(&conn).unwrap();
        assert_eq!(types.len(), 17);
        assert!(types.iter().any(|t| t.group_.as_deref() == Some("Cpx")));

        let authors = db::list_authors(&conn).unwrap();
        assert_eq!(authors.len(), 47);
        assert!(
            authors
                .iter()
                .any(|a| a.initials == "JCB" && a.word_count > 1000)
        );

        // Check a word with affixes, used_in, authors, slots+grammar_code, and 'null' notes
        let humni_id: i64 = conn
            .query_row("SELECT id FROM words WHERE name='humni'", [], |r| r.get(0))
            .unwrap();
        let humni = db::get_word(&conn, humni_id).unwrap();
        assert_eq!(humni.notes, None, "'null' JSON in export.db should be None");
        assert!(
            !humni.affixes.is_empty(),
            "humni should have affixes from connect_words"
        );
        assert!(
            !humni.used_in.is_empty(),
            "humni should have used_in complexes from connect_words"
        );
        assert!(
            humni.source.is_some(),
            "humni should have source from connect_authors"
        );
        assert_eq!(humni.year.as_deref(), Some("1975"));
        assert!(
            humni
                .definitions
                .iter()
                .any(|d| d.grammar.as_deref() == Some("2n") && d.tags.as_deref() == Some("P-S")),
            "humni definition should combine slots=2 and grammar_code='n' into '2n' with tags='P-S', got: {:?}",
            humni.definitions
        );

        // Verify complex word 'farlai' has parents ['fanra', 'landi'] in parents, NOT in children
        let farlai_id: i64 = conn
            .query_row("SELECT id FROM words WHERE name='farlai'", [], |r| r.get(0))
            .unwrap();
        let farlai = db::get_word(&conn, farlai_id).unwrap();
        assert_eq!(
            farlai.parents,
            vec!["fanra".to_string(), "landi".to_string()],
            "farlai must list fanra and landi in parents"
        );
        assert!(
            farlai.children.is_empty(),
            "farlai must have empty children, got: {:?}",
            farlai.children
        );

        // Verify 'hekri' has affix 'hei' and event 'Torrua Dictionary Repair', and 'hei-' (Afx) has parent 'hekri'
        let hekri_id: i64 = conn
            .query_row("SELECT id FROM words WHERE name='hekri'", [], |r| r.get(0))
            .unwrap();
        let hekri = db::get_word(&conn, hekri_id).unwrap();
        assert_eq!(hekri.affixes, vec!["hei".to_string()]);
        assert_eq!(
            hekri.event_start_name.as_deref(),
            Some("Torrua Dictionary Repair")
        );

        let hei_afx_id: i64 = conn
            .query_row("SELECT id FROM words WHERE name='hei-'", [], |r| r.get(0))
            .unwrap();
        let hei_afx = db::get_word(&conn, hei_afx_id).unwrap();
        assert_eq!(hei_afx.type_name.as_deref(), Some("Afx"));
        assert_eq!(hei_afx.parents, vec!["hekri".to_string()]);
        assert!(hei_afx.children.is_empty());

        // Verify morphological ordering of parents for non-alphabetical compounds (e.g. heicli = hekri + clika, heirslicui = hekri + sliti + cutri)
        let heicli_id: i64 = conn
            .query_row("SELECT id FROM words WHERE name='heicli'", [], |r| r.get(0))
            .unwrap();
        let heicli = db::get_word(&conn, heicli_id).unwrap();
        assert_eq!(
            heicli.parents,
            vec!["hekri".to_string(), "clika".to_string()],
            "heicli parents must follow morphological order from origin"
        );

        let heirslicui_id: i64 = conn
            .query_row("SELECT id FROM words WHERE name='heirslicui'", [], |r| {
                r.get(0)
            })
            .unwrap();
        let heirslicui = db::get_word(&conn, heirslicui_id).unwrap();
        assert_eq!(
            heirslicui.parents,
            vec![
                "hekri".to_string(),
                "sliti".to_string(),
                "cutri".to_string()
            ],
            "heirslicui parents must follow morphological order from origin"
        );

        // Verify list_words orders active words (event_end IS NULL) before retired homonyms when event_id is None
        let clika_matches = db::list_words(&conn, "clika", "", None).unwrap();
        assert!(clika_matches.len() >= 2);
        let first_clika = db::get_word(&conn, clika_matches[0].id).unwrap();
        assert_eq!(
            first_clika.event_end_name, None,
            "Active word (event_end IS NULL) must precede retired homonym in list_words"
        );
    }

    #[test]
    fn test_export_db_roundtrip_save_word_preserves_metadata_and_id_old() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        db::init_schema(&conn).unwrap();

        conn.execute(
            "INSERT INTO types (type, type_x, \"group\") VALUES ('C-Prim', 'Composite Primitive', 'Prim'), ('Afx', 'Affix', 'Affix')",
            [],
        )
        .unwrap();
        let prim_tid: i64 = conn
            .query_row("SELECT id FROM types WHERE type='C-Prim'", [], |r| r.get(0))
            .unwrap();
        let afx_tid: i64 = conn
            .query_row("SELECT id FROM types WHERE type='Afx'", [], |r| r.get(0))
            .unwrap();

        conn.execute(
            "INSERT INTO authors (abbreviation, full_name) VALUES ('JCB', 'James Cooke Brown')",
            [],
        )
        .unwrap();
        let aid: i64 = conn.last_insert_rowid();

        conn.execute(
            "INSERT INTO words (name, type, year, rank, notes, id_old, event_start)
             VALUES ('humni', ?1, '1975-01-01', '1.0', '{\"author\":\"(?)\",\"year\":\"(changed ''16)\",\"rank\":\"(grammar vocab project)\"}', 1388, 1)",
            [prim_tid],
        )
        .unwrap();
        let wid: i64 = conn.last_insert_rowid();

        conn.execute(
            "INSERT INTO words (name, type, notes, id_old, event_start) VALUES ('hum', ?1, 'null', 1389, 1), ('hmu-', ?1, 'null', 1390, 1)",
            [afx_tid],
        )
        .unwrap();
        let hum_id: i64 = conn
            .query_row("SELECT id FROM words WHERE name='hum'", [], |r| r.get(0))
            .unwrap();
        let hmu_id: i64 = conn
            .query_row("SELECT id FROM words WHERE name='hmu-'", [], |r| r.get(0))
            .unwrap();

        conn.execute(
            "INSERT INTO connect_authors (\"AID\", \"WID\") VALUES (?1, ?2)",
            (aid, wid),
        )
        .unwrap();
        conn.execute(
            "INSERT INTO connect_words (parent_id, child_id) VALUES (?1, ?2), (?1, ?3)",
            (wid, hum_id, hmu_id),
        )
        .unwrap();

        let loaded = db::get_word(&conn, wid).unwrap();
        assert_eq!(loaded.source.as_deref(), Some("JCB (?)"));
        assert_eq!(loaded.year.as_deref(), Some("1975 (changed '16)"));
        assert_eq!(loaded.rank.as_deref(), Some("1.0 (grammar vocab project)"));
        assert_eq!(loaded.notes, None);
        assert_eq!(loaded.affixes, vec!["hmu".to_string(), "hum".to_string()]);

        // Simulate WordForm.svelte submitting the loaded word (with id_old = None, removing 'hmu', adding custom note)
        db::save_word(
            &conn,
            Some(wid),
            &models::SaveWord {
                name: loaded.name,
                type_name: loaded.type_name,
                source: loaded.source,
                year: loaded.year,
                rank: loaded.rank,
                match_: loaded.match_,
                origin: loaded.origin,
                origin_x: loaded.origin_x,
                notes: Some("custom note".to_string()),
                event_start: loaded.event_start_name,
                event_end: loaded.event_end_name,
                affixes: vec!["hum".to_string()],
                spellings: loaded.spellings,
                id_old: None,
                event_start_id: None,
                event_end_id: None,
            },
        )
        .unwrap();

        // 1. id_old must still be 1388
        let (db_id_old, db_year, db_rank, db_notes): (
            i64,
            Option<String>,
            Option<String>,
            Option<String>,
        ) = conn
            .query_row(
                "SELECT id_old, year, rank, notes FROM words WHERE id=?1",
                [wid],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )
            .unwrap();
        assert_eq!(
            db_id_old, 1388,
            "save_word must preserve existing id_old when data.id_old is None"
        );
        assert_eq!(
            db_year.as_deref(),
            Some("1975-01-01"),
            "year must be stored in canonical DATE format"
        );
        assert_eq!(
            db_rank.as_deref(),
            Some("1.0"),
            "base rank must be separated from rank note"
        );
        assert!(
            db_notes.as_deref().unwrap_or("").starts_with('{'),
            "notes must preserve JSON structure"
        );

        // 2. Bogus author "JCB (?)" must NOT be created in authors table
        let bogus_author_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM authors WHERE abbreviation LIKE '%(%'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(
            bogus_author_count, 0,
            "Note suffix must not be inserted as an author abbreviation"
        );

        // 3. Reload via get_word and verify full round-trip
        let reloaded = db::get_word(&conn, wid).unwrap();
        assert_eq!(reloaded.source.as_deref(), Some("JCB (?)"));
        assert_eq!(reloaded.year.as_deref(), Some("1975 (changed '16)"));
        assert_eq!(
            reloaded.rank.as_deref(),
            Some("1.0 (grammar vocab project)")
        );
        assert_eq!(reloaded.notes.as_deref(), Some("custom note"));
        assert_eq!(
            reloaded.affixes,
            vec!["hum".to_string()],
            "Removed affix 'hmu' must not reappear from connect_words"
        );
    }

    #[test]
    fn test_export_db_ddl_save_event_and_type_with_none_fields() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "PRAGMA foreign_keys=ON;
             CREATE TABLE types (
                 type VARCHAR(16) NOT NULL,
                 type_x VARCHAR(16) NOT NULL,
                 \"group\" VARCHAR(16) NOT NULL,
                 parentable BOOLEAN NOT NULL,
                 description VARCHAR(255),
                 id INTEGER NOT NULL,
                 created DATETIME NOT NULL,
                 updated DATETIME,
                 PRIMARY KEY (id)
             );
             CREATE TABLE events (
                 event_id INTEGER NOT NULL,
                 name VARCHAR(64) NOT NULL,
                 date DATE NOT NULL,
                 definition TEXT NOT NULL,
                 annotation VARCHAR(16) NOT NULL,
                 suffix VARCHAR(16) NOT NULL,
                 id INTEGER NOT NULL,
                 created DATETIME NOT NULL,
                 updated DATETIME,
                 PRIMARY KEY (id),
                 UNIQUE (event_id)
             );
             CREATE TABLE words (
                 id INTEGER NOT NULL PRIMARY KEY,
                 name VARCHAR(64) NOT NULL,
                 type INTEGER NOT NULL REFERENCES types(id),
                 id_old INTEGER NOT NULL,
                 event_start INTEGER NOT NULL REFERENCES events(event_id),
                 event_end INTEGER REFERENCES events(event_id),
                 created DATETIME NOT NULL
             );",
        )
        .unwrap();

        let eid = db::save_event(
            &conn,
            None,
            &models::SaveEvent {
                name: "TestEvent".to_string(),
                date: None,
                annotation: None,
                suffix: None,
                notes: None,
            },
        )
        .unwrap();
        db::save_event(
            &conn,
            Some(eid),
            &models::SaveEvent {
                name: "TestEventUpdated".to_string(),
                date: None,
                annotation: None,
                suffix: None,
                notes: None,
            },
        )
        .unwrap();

        let tid = db::save_type(
            &conn,
            None,
            &models::SaveType {
                name: "TestType".to_string(),
                type_x: None,
                group_: None,
                parentable: None,
                description: None,
            },
        )
        .unwrap();
        db::save_type(
            &conn,
            Some(tid),
            &models::SaveType {
                name: "TestTypeUpdated".to_string(),
                type_x: None,
                group_: None,
                parentable: None,
                description: None,
            },
        )
        .unwrap();

        conn.execute(
            "INSERT INTO words (id, name, type, id_old, event_start, created) VALUES (1, 'w1', ?1, 1, 1, datetime('now'))",
            [tid],
        )
        .unwrap();

        assert!(
            db::delete_event(&conn, eid).is_err(),
            "Deleting an event referenced by words must return an error"
        );
    }

    #[test]
    fn test_export_db_ddl_import_contents_and_dedup_definitions() {
        let mut conn = rusqlite::Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "PRAGMA foreign_keys=ON;
             CREATE TABLE authors (abbreviation VARCHAR(64) NOT NULL, full_name VARCHAR(64), notes VARCHAR(128), id INTEGER NOT NULL, created DATETIME NOT NULL, updated DATETIME, PRIMARY KEY (id), UNIQUE (abbreviation));
             CREATE TABLE events (event_id INTEGER NOT NULL, name VARCHAR(64) NOT NULL, date DATE NOT NULL, definition TEXT NOT NULL, annotation VARCHAR(16) NOT NULL, suffix VARCHAR(16) NOT NULL, id INTEGER NOT NULL, created DATETIME NOT NULL, updated DATETIME, PRIMARY KEY (id), UNIQUE (event_id));
             CREATE TABLE settings (date DATETIME NOT NULL, db_version INTEGER NOT NULL, last_word_id INTEGER NOT NULL, db_release VARCHAR(16) NOT NULL, id INTEGER NOT NULL, created DATETIME NOT NULL, updated DATETIME, PRIMARY KEY (id), UNIQUE (date));
             CREATE TABLE syllables (name VARCHAR(8) NOT NULL, type VARCHAR(32) NOT NULL, allowed BOOLEAN NOT NULL, id INTEGER NOT NULL, created DATETIME NOT NULL, updated DATETIME, PRIMARY KEY (id));
             CREATE TABLE types (type VARCHAR(16) NOT NULL, type_x VARCHAR(16) NOT NULL, \"group\" VARCHAR(16) NOT NULL, parentable BOOLEAN NOT NULL, description VARCHAR(255), id INTEGER NOT NULL, created DATETIME NOT NULL, updated DATETIME, PRIMARY KEY (id));
             CREATE TABLE words (id INTEGER NOT NULL, name VARCHAR(64) NOT NULL, origin VARCHAR(128), origin_x VARCHAR(64), \"match\" VARCHAR(8), rank VARCHAR(8), year DATE, notes JSON, id_old INTEGER NOT NULL, \"TID_old\" INTEGER, type INTEGER NOT NULL, event_start INTEGER NOT NULL, event_end INTEGER, created DATETIME NOT NULL, updated DATETIME, PRIMARY KEY (id), FOREIGN KEY(type) REFERENCES types (id), FOREIGN KEY(event_start) REFERENCES events (event_id), FOREIGN KEY(event_end) REFERENCES events (event_id));
             CREATE TABLE connect_authors (\"AID\" INTEGER NOT NULL, \"WID\" INTEGER NOT NULL, PRIMARY KEY (\"AID\", \"WID\"), FOREIGN KEY(\"AID\") REFERENCES authors (id), FOREIGN KEY(\"WID\") REFERENCES words (id));
             CREATE TABLE connect_words (parent_id INTEGER NOT NULL, child_id INTEGER NOT NULL, PRIMARY KEY (parent_id, child_id), FOREIGN KEY(parent_id) REFERENCES words (id), FOREIGN KEY(child_id) REFERENCES words (id));
             CREATE TABLE definitions (word_id INTEGER NOT NULL, position INTEGER NOT NULL, body TEXT NOT NULL, usage VARCHAR(64), grammar_code VARCHAR(8), slots INTEGER, case_tags VARCHAR(16), language VARCHAR(16), notes VARCHAR(255), id INTEGER NOT NULL, created DATETIME NOT NULL, updated DATETIME, PRIMARY KEY (id), FOREIGN KEY(word_id) REFERENCES words (id));",
        )
        .unwrap();

        let files = vec![
            (
                "types.txt".to_string(),
                "C-Prim@Composite Primitive@Prim@False@Composite desc\n".to_string(),
            ),
            (
                "lexevent.txt".to_string(),
                "1@Initial@01.01.1975@The initial vocabulary@INIT@\n".to_string(),
            ),
            (
                "author.txt".to_string(),
                "JCB@James Cooke Brown@Founder\n".to_string(),
            ),
            ("syllables.txt".to_string(), "ba@CV@True\n".to_string()),
            (
                "words.txt".to_string(),
                "75@C-Prim@@alk@50%@JCB (?)@1988 (Keugru Proposal 2)@7+@3/6E alcohol@alcohol@@10\n"
                    .to_string(),
            ),
            (
                "wordspell.txt".to_string(),
                "75@alkooli@@1@1@9999\n75@alkoholi@@2@1@9999\n".to_string(),
            ),
            (
                "worddefinition.txt".to_string(),
                "75@1@%@2a@«alcohol» drink@\n75@1@%@2a@«alcohol» drink@\n".to_string(),
            ),
            (
                "setting.txt".to_string(),
                "22.08.2024 05:08:00@1@10150@4.5.9\n".to_string(),
            ),
        ];

        let res = import::import_contents(&mut conn, &files).unwrap();
        assert_eq!(res.types, 1);
        assert_eq!(res.events, 1);
        assert_eq!(res.authors, 1);
        assert_eq!(res.words, 2);
        assert_eq!(
            res.definitions, 2,
            "Duplicate (WID=75, position=1) rows in WordDefinition.txt must be deduplicated to 1 per spelling (2 total)"
        );
        assert_eq!(res.settings, 1);
    }
}
