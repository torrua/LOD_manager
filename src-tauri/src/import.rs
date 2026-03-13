use crate::models::ImportResult;
use rusqlite::{Connection, params};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

const SEP: char = '@';

type WordData = (
    String,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Vec<String>,
);

fn rows(content: &str) -> Vec<Vec<String>> {
    content
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| l.split(SEP).map(|c| c.trim().to_string()).collect())
        .collect()
}

fn opt(s: &str) -> Option<String> {
    let t = s.trim();
    if t.is_empty() {
        None
    } else {
        Some(t.to_string())
    }
}

/// Import from (filename, `text_content`) pairs.
/// Used on Android where paths are `content://` URIs that `std::fs` cannot read.
pub fn import_contents(conn: &mut Connection, files: &[(String, String)]) -> ImportResult {
    let mut result = ImportResult {
        words: 0,
        definitions: 0,
        events: 0,
        types: 0,
        authors: 0,
        settings: 0,
        errors: 0,
        messages: vec![],
    };

    // Write each file's content to a temp file with its original name,
    // then reuse import_files which matches by filename.
    let tmp_dir = std::env::temp_dir().join(format!("lod_import_{}", std::process::id()));
    if std::fs::create_dir_all(&tmp_dir).is_err() {
        result.errors += 1;
        result
            .messages
            .push("Could not create temp dir for import".into());
        return result;
    }
    let mut tmp_paths: Vec<String> = Vec::new();
    for (name, content) in files {
        let dest = tmp_dir.join(name);
        if std::fs::write(&dest, content.as_bytes()).is_ok() {
            tmp_paths.push(dest.to_string_lossy().into_owned());
        }
    }
    let r = import_files(conn, &tmp_paths);
    let _ = std::fs::remove_dir_all(&tmp_dir);
    r
}

pub fn import_files(conn: &mut Connection, paths: &[String]) -> ImportResult {
    let mut result = ImportResult {
        words: 0,
        definitions: 0,
        events: 0,
        types: 0,
        authors: 0,
        settings: 0,
        errors: 0,
        messages: vec![],
    };

    let mut type_file = None;
    let mut author_file = None;
    let mut event_file = None;
    let mut word_file = None;
    let mut spell_file = None;
    let mut def_file = None;
    let mut settings_file = None;

    for path in paths {
        let lower = Path::new(path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_lowercase();
        match lower.as_str() {
            s if s.contains("setting") => settings_file = Some(path.clone()),
            s if s.contains("type") => type_file = Some(path.clone()),
            s if s.contains("author") => author_file = Some(path.clone()),
            s if s.contains("lexevent") || s.contains("event") => event_file = Some(path.clone()),
            s if s.contains("wordspell") || s.contains("spell") => spell_file = Some(path.clone()),
            s if (s.contains("worddef") || s.contains("definition")) && !s.contains("word.txt") => {
                def_file = Some(path.clone());
            }
            s if s == "word.txt" || s == "words.txt" => word_file = Some(path.clone()),
            _ => {}
        }
    }

    let tx = conn.transaction().unwrap();

    // 1. Types
    if let Some(p) = type_file
        && let Ok(content) = fs::read_to_string(&p)
    {
        for r in rows(&content) {
            if let Some(name) = r.first().filter(|s| !s.is_empty()) {
                let type_x = r.get(1).and_then(|s| opt(s));
                let group = r.get(2).and_then(|s| opt(s));
                if tx
                    .execute(
                        "INSERT OR IGNORE INTO types (name, type_x, group_) VALUES (?1,?2,?3)",
                        params![name, type_x, group],
                    )
                    .is_ok()
                    && tx.changes() > 0
                {
                    result.types += 1;
                }
            }
        }
        result.messages.push(format!("Types: {}", result.types));
    }

    // 2. Authors
    if let Some(p) = author_file
        && let Ok(content) = fs::read_to_string(&p)
    {
        for r in rows(&content) {
            if let Some(initials) = r.first().filter(|s| !s.is_empty()) {
                let full_name = r.get(1).and_then(|s| opt(s));
                if tx
                    .execute(
                        "INSERT OR IGNORE INTO authors (initials, full_name) VALUES (?1,?2)",
                        params![initials, full_name],
                    )
                    .is_ok()
                    && tx.changes() > 0
                {
                    result.authors += 1;
                }
            }
        }
        result.messages.push(format!("Authors: {}", result.authors));
    }

    // 3. Events
    let mut event_id_map: HashMap<String, i64> = HashMap::new();
    if let Some(p) = event_file
        && let Ok(content) = fs::read_to_string(&p)
    {
        for r in rows(&content) {
            if r.len() < 2 {
                continue;
            }
            let old_id = &r[0];
            let name = &r[1];
            if name.is_empty() {
                continue;
            }
            let date = r.get(2).and_then(|s| opt(s));
            let notes = r.get(3).and_then(|s| opt(s));
            let suffix = r.get(4).and_then(|s| opt(s));
            let annotation = r.get(5).and_then(|s| opt(s));
            if tx.execute("INSERT OR IGNORE INTO events (name, date, annotation, suffix, notes) VALUES (?1,?2,?3,?4,?5)", params![name, date, annotation, suffix, notes]).is_ok()
                    && let Ok(eid) = tx.query_row("SELECT id FROM events WHERE name=?1", params![name], |row| row.get::<_, i64>(0)) {
                        event_id_map.insert(old_id.clone(), eid);
                        if tx.changes() > 0 { result.events += 1; }
                    }
        }
        result.messages.push(format!("Events: {}", result.events));
    }

    // 4. Words
    let mut word_staging: HashMap<String, WordData> = HashMap::new();
    let mut old_id_to_db_id: HashMap<String, i64> = HashMap::new();

    if let Some(p) = word_file
        && let Ok(content) = fs::read_to_string(&p)
    {
        for r in rows(&content) {
            if r.len() < 2 {
                continue;
            }
            let old_id = r[0].clone();
            let type_name = r.get(1).map_or("", String::as_str).to_string();
            let affixes = r
                .get(3)
                .map_or("", String::as_str)
                .split_whitespace()
                .map(String::from)
                .collect();
            word_staging.insert(
                old_id,
                (
                    type_name,
                    r.get(4).and_then(|s| opt(s)),
                    r.get(5).and_then(|s| opt(s)),
                    r.get(6).and_then(|s| opt(s)),
                    r.get(7).and_then(|s| opt(s)),
                    r.get(8).and_then(|s| opt(s)),
                    r.get(9).and_then(|s| opt(s)),
                    affixes,
                ),
            );
        }
    }

    if let Some(p) = spell_file
        && let Ok(content) = fs::read_to_string(&p)
    {
        let start_id = tx
            .query_row("SELECT id FROM events WHERE name='Start'", [], |r| r.get(0))
            .unwrap_or(1);
        for r in rows(&content) {
            if r.len() < 2 {
                continue;
            }
            let old_id = &r[0];
            let name = &r[1];
            if name.is_empty() {
                continue;
            }

            let ev_start = event_id_map
                .get(r.get(4).map_or("1", String::as_str))
                .copied()
                .unwrap_or(start_id);
            let ev_end = r
                .get(5)
                .and_then(|s| opt(s))
                .and_then(|s| event_id_map.get(&s).copied());

            let (type_name, match_, source, year, rank, origin, origin_x, affixes) =
                word_staging.get(old_id).cloned().unwrap_or_default();
            let type_id: Option<i64> = if type_name.is_empty() {
                None
            } else {
                tx.query_row(
                    "SELECT id FROM types WHERE name=?1",
                    params![type_name],
                    |r| r.get(0),
                )
                .ok()
            };

            if tx.execute("INSERT OR IGNORE INTO words (name, type_id, match_, source, year, rank, origin, origin_x, event_start_id, event_end_id) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)", params![name, type_id, match_, source, year, rank, origin, origin_x, ev_start, ev_end]).is_ok() {
                    let db_id = if tx.changes() > 0 {
                        let wid = tx.last_insert_rowid();
                        result.words += 1;
                        for a in &affixes { let _ = tx.execute("INSERT INTO word_affixes (word_id, affix) VALUES (?1,?2)", params![wid, a]); }
                        wid
                    } else {
                        tx.query_row("SELECT id FROM words WHERE name=?1 AND (type_id=?2 OR (type_id IS NULL AND ?2 IS NULL))", params![name, type_id], |r| r.get(0)).unwrap_or(0)
                    };
                    if db_id > 0 { old_id_to_db_id.insert(old_id.clone(), db_id); }
                }
        }
        result.messages.push(format!("Words: {}", result.words));
    }

    // 5. Definitions
    if let Some(p) = def_file
        && let Ok(content) = fs::read_to_string(&p)
    {
        let mut def_count = 0usize;
        for r in rows(&content) {
            if r.len() < 5 {
                continue;
            }
            let old_word_id = &r[0];
            let position: i64 = r[1].parse().unwrap_or(0);
            let usage = r.get(2).and_then(|s| opt(s));
            let grammar = r.get(3).and_then(|s| opt(s));
            let body = r.get(4).map_or("", String::as_str);
            if body.is_empty() {
                continue;
            }
            let tags = r.get(6).and_then(|s| opt(s));

            if let Some(wid) = old_id_to_db_id
                    .get(old_word_id)
                    .copied()
                    .or_else(|| old_word_id.parse().ok())
                    && tx.execute("INSERT OR IGNORE INTO definitions (word_id, position, grammar, usage, body, tags) VALUES (?1,?2,?3,?4,?5,?6)", params![wid, position, grammar, usage, body, tags]).is_ok() && tx.changes() > 0 {
                        def_count += 1;
                    }
        }
        result.definitions = def_count;
        result
            .messages
            .push(format!("Definitions: {}", result.definitions));
    }

    // 6. Settings - import before commit
    if let Some(p) = &settings_file
        && let Ok(n) = import_settings(&tx, p)
    {
        result.settings += n;
        result
            .messages
            .push(format!("Settings: {}", result.settings));
    }

    let _ = tx.commit();
    result
}

fn import_settings(conn: &Connection, path: &str) -> Result<usize, Box<dyn std::error::Error>> {
    let content = fs::read_to_string(path)?;
    let mut count = 0usize;
    for line in content
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#') && !l.starts_with("//"))
    {
        // Handle both key=value format and special database_info format
        if let Some((k, v)) = line.split_once('=').or_else(|| line.split_once('\t')) {
            let (k, v) = (k.trim(), v.trim());
            if !k.is_empty() {
                conn.execute("INSERT INTO settings(key,value) VALUES(?1,?2) ON CONFLICT(key) DO UPDATE SET value=excluded.value", params![k, v])?;
                count += 1;
            }
        } else if line.contains('@') && line.chars().filter(|&c| c == '@').count() >= 3 {
            // Special format like "07.10.2020 07:10:20@2@10141@4.5.8"
            // Store as database_info key
            conn.execute("INSERT INTO settings(key,value) VALUES(?1,?2) ON CONFLICT(key) DO UPDATE SET value=excluded.value", params!["database_info", line])?;
            count += 1;
        }
    }
    Ok(count)
}
