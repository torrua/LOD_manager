use rusqlite::{Connection, params};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use crate::models::ImportResult;

const SEP: char = '@';

fn rows(content: &str) -> Vec<Vec<String>> {
    content.lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| l.split(SEP).map(|c| c.trim().to_string()).collect())
        .collect()
}

fn opt(s: &str) -> Option<String> {
    let t = s.trim();
    if t.is_empty() { None } else { Some(t.to_string()) }
}

pub fn import_files(conn: &mut Connection, paths: &[String]) -> ImportResult {
    let mut result = ImportResult {
        words: 0, definitions: 0, events: 0,
        types: 0, authors: 0, settings: 0, errors: 0, messages: vec![],
    };

    // Detect and categorise files
    let mut type_file:    Option<String> = None;
    let mut author_file:  Option<String> = None;
    let mut event_file:   Option<String> = None;
    let mut word_file:    Option<String> = None;
    let mut spell_file:   Option<String> = None;
    let mut def_file:     Option<String> = None;
    let mut settings_file: Option<String> = None;

    for path in paths {
        let lower = Path::new(path).file_name()
            .and_then(|n| n.to_str()).unwrap_or("").to_lowercase();
        match lower.as_str() {
            s if s.contains("setting")    => settings_file = Some(path.clone()),
            s if s.contains("type")       => type_file   = Some(path.clone()),
            s if s.contains("author")     => author_file = Some(path.clone()),
            s if s.contains("lexevent") || s.contains("event") => event_file = Some(path.clone()),
            s if s.contains("wordspell") || s.contains("spell") => spell_file = Some(path.clone()),
            s if (s.contains("worddef") || s.contains("definition")) && !s.contains("word.txt") => def_file = Some(path.clone()),
            s if s == "word.txt" || s == "words.txt" => word_file = Some(path.clone()),
            _ => {}
        }
    }

    let tx = conn.transaction().unwrap();

    // 1. Types
    if let Some(p) = type_file {
        match fs::read_to_string(&p) {
            Ok(content) => {
                for r in rows(&content) {
                    if r.is_empty() { continue; }
                    let name = r[0].as_str();
                    if name.is_empty() { continue; }
                    let type_x = r.get(1).and_then(|s| opt(s));
                    let group  = r.get(2).and_then(|s| opt(s));
                    let res = tx.execute(
                        "INSERT OR IGNORE INTO types (name, type_x, group_) VALUES (?1,?2,?3)",
                        params![name, type_x, group]);
                    if res.is_ok() && tx.changes() > 0 { result.types += 1; }
                }
                result.messages.push(format!("Types: {}", result.types));
            }
            Err(e) => { result.errors += 1; result.messages.push(format!("Types error: {e}")); }
        }
    }

    // 2. Authors
    if let Some(p) = author_file {
        match fs::read_to_string(&p) {
            Ok(content) => {
                for r in rows(&content) {
                    if r.is_empty() { continue; }
                    let initials = r[0].as_str();
                    if initials.is_empty() { continue; }
                    let full_name = r.get(1).and_then(|s| opt(s));
                    let res = tx.execute(
                        "INSERT OR IGNORE INTO authors (initials, full_name) VALUES (?1,?2)",
                        params![initials, full_name]);
                    if res.is_ok() && tx.changes() > 0 { result.authors += 1; }
                }
                result.messages.push(format!("Authors: {}", result.authors));
            }
            Err(e) => { result.errors += 1; result.messages.push(format!("Authors error: {e}")); }
        }
    }

    // 3. Events (LexEvent.txt): [0]id [1]name [2]date [3]notes [4]suffix [5]annotation
    let mut event_id_map: HashMap<String, i64> = HashMap::new();
    if let Some(p) = event_file {
        match fs::read_to_string(&p) {
            Ok(content) => {
                for r in rows(&content) {
                    if r.len() < 2 { continue; }
                    let old_id = r[0].as_str();
                    let name   = r[1].as_str();
                    if name.is_empty() { continue; }
                    let date       = r.get(2).and_then(|s| opt(s));
                    // LexEvent.txt: [3]=notes(why/what), [4]=suffix, [5]=annotation(short code)
                    let notes      = r.get(3).and_then(|s| opt(s));
                    let suffix     = r.get(4).and_then(|s| opt(s));
                    let annotation = r.get(5).and_then(|s| opt(s));
                    let res = tx.execute(
                        "INSERT OR IGNORE INTO events (name, date, annotation, suffix, notes) VALUES (?1,?2,?3,?4,?5)",
                        params![name, date, annotation, suffix, notes]);
                    if res.is_ok() {
                        let eid: i64 = tx.query_row("SELECT id FROM events WHERE name=?1", params![name], |r| r.get(0)).unwrap_or(0);
                        event_id_map.insert(old_id.to_string(), eid);
                        if tx.changes() > 0 { result.events += 1; }
                    }
                }
                // also map by name for Start
                let _ = tx.query_row("SELECT id FROM events WHERE name='Start'", [], |r| r.get::<_,i64>(0))
                    .map(|id| event_id_map.insert("1".to_string(), id));
                result.messages.push(format!("Events: {}", result.events));
            }
            Err(e) => { result.errors += 1; result.messages.push(format!("Events error: {e}")); }
        }
    }

    // 4. Words: Word.txt [0]old_id [1]type_name [4]match [5]source [6]year [7]rank [8]origin [9]origin_x [10]usedin
    //    WordSpell.txt: [0]old_id [1]name [4]event_start_old_id [5]event_end_old_id
    let mut word_staging: HashMap<String, (String, Option<String>, Option<String>, Option<String>, Option<String>, Option<String>, Option<String>, Vec<String>)> = HashMap::new();
    // (type_name, match, source, year, rank, origin, origin_x, affixes)
    let mut old_id_to_name: HashMap<String, String> = HashMap::new(); // old_id -> word name
    let mut old_id_to_db_id: HashMap<String, i64> = HashMap::new();  // old_id -> DB row id

    if let Some(p) = word_file {
        match fs::read_to_string(&p) {
            Ok(content) => {
                for r in rows(&content) {
                    if r.len() < 2 { continue; }
                    let old_id    = r[0].clone();
                    let type_name = r.get(1).map(|s| s.as_str()).unwrap_or("");
                    let affixes_s = r.get(3).map(|s| s.as_str()).unwrap_or("");
                    let match_    = r.get(4).and_then(|s| opt(s));
                    let source    = r.get(5).and_then(|s| opt(s));
                    let year      = r.get(6).and_then(|s| opt(s));
                    let rank      = r.get(7).and_then(|s| opt(s));
                    let origin    = r.get(8).and_then(|s| opt(s));
                    let origin_x  = r.get(9).and_then(|s| opt(s));
                    let affixes: Vec<String> = affixes_s.split_whitespace()
                        .filter(|s| !s.is_empty()).map(|s| s.to_string()).collect();
                    word_staging.insert(old_id, (
                        type_name.to_string(),
                        match_, source, year, rank, origin, origin_x, affixes
                    ));
                }
            }
            Err(e) => { result.errors += 1; result.messages.push(format!("Word.txt error: {e}")); }
        }
    }

    if let Some(p) = spell_file {
        match fs::read_to_string(&p) {
            Ok(content) => {
                let start_id = tx.query_row("SELECT id FROM events WHERE name='Start'", [], |r| r.get::<_,i64>(0)).unwrap_or(1);
                for r in rows(&content) {
                    if r.len() < 2 { continue; }
                    let old_id   = &r[0];
                    let name     = &r[1];
                    if name.is_empty() { continue; }
                    let ev_start_old = r.get(4).map(|s| s.as_str()).unwrap_or("1");
                    let ev_end_old   = r.get(5).and_then(|s| opt(s));

                    let ev_start: i64 = event_id_map.get(ev_start_old).copied().unwrap_or(start_id);
                    let ev_end: Option<i64> = ev_end_old.as_deref().and_then(|s| event_id_map.get(s).copied());

                    old_id_to_name.insert(old_id.to_string(), name.to_string());
                    let (type_name, match_, source, year, rank, origin, origin_x, affixes) =
                        if let Some(st) = word_staging.get(old_id) { st.clone() }
                        else { (String::new(), None, None, None, None, None, None, vec![]) };

                    let type_id: Option<i64> = if !type_name.is_empty() {
                        tx.query_row("SELECT id FROM types WHERE name=?1", params![type_name], |r| r.get(0)).ok()
                    } else { None };

                    let res = tx.execute(
                        "INSERT OR IGNORE INTO words (name, type_id, match_, source, year, rank, origin, origin_x, event_start_id, event_end_id)
                         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
                        params![name, type_id, match_, source, year, rank, origin, origin_x, ev_start, ev_end]);

                    if res.is_ok() {
                        let db_id: i64 = if tx.changes() > 0 {
                            // freshly inserted — use last rowid
                            let wid = tx.last_insert_rowid();
                            result.words += 1;
                            for a in &affixes {
                                let _ = tx.execute("INSERT INTO word_affixes (word_id, affix) VALUES (?1,?2)", params![wid, a]);
                            }
                            wid
                        } else {
                            // already existed — find by (name, type_id) to get the right row
                            // when type_id is Some we match exactly; when None, match by name only
                            if let Some(tid) = type_id {
                                tx.query_row(
                                    "SELECT id FROM words WHERE name=?1 AND type_id=?2",
                                    params![name, tid], |r| r.get(0)
                                ).unwrap_or(0)
                            } else {
                                tx.query_row(
                                    "SELECT id FROM words WHERE name=?1 AND type_id IS NULL",
                                    params![name], |r| r.get(0)
                                ).unwrap_or(0)
                            }
                        };
                        if db_id > 0 {
                            old_id_to_db_id.insert(old_id.to_string(), db_id);
                        }
                    }
                }
                result.messages.push(format!("Words: {}", result.words));
            }
            Err(e) => { result.errors += 1; result.messages.push(format!("WordSpell error: {e}")); }
        }
    }

    // 5. Definitions: [0]word_old_id [1]position [2]usage [3]grammar [4]body [5]keys [6]tags
    if let Some(p) = def_file {
        match fs::read_to_string(&p) {
            Ok(content) => {
                // old_id_to_db_id was populated during WordSpell import:
                // each old_id maps to the exact DB row, even when multiple words share a name.
                let old_to_new: &HashMap<String, i64> = &old_id_to_db_id;
                let mut def_count = 0usize;
                for r in rows(&content) {
                    if r.len() < 5 { continue; }
                    let old_word_id = &r[0];
                    let position: i64 = r[1].parse().unwrap_or(0);
                    let usage   = r.get(2).and_then(|s| opt(s));
                    let grammar = r.get(3).and_then(|s| opt(s));
                    let body    = r.get(4).map(|s| s.as_str()).unwrap_or("");
                    if body.is_empty() { continue; }
                    let tags    = r.get(6).and_then(|s| opt(s));

                    // Find word_id from old_word_id
                    let word_id: Option<i64> = old_to_new.get(old_word_id).copied()
                        .or_else(|| old_word_id.parse::<i64>().ok());

                    if let Some(wid) = word_id {
                        let res = tx.execute(
                            "INSERT OR IGNORE INTO definitions (word_id, position, grammar, usage, body, tags)
                             VALUES (?1,?2,?3,?4,?5,?6)",
                            params![wid, position, grammar, usage, body, tags]);
                        if res.is_ok() && tx.changes() > 0 { def_count += 1; }
                    }
                }
                result.definitions = def_count;
                result.messages.push(format!("Definitions: {}", result.definitions));
            }
            Err(e) => { result.errors += 1; result.messages.push(format!("Definitions error: {e}")); }
        }
    }

    match tx.commit() {
        Ok(_) => {}
        Err(e) => { result.errors += 1; result.messages.push(format!("Commit error: {e}")); }
    }

    // Import settings
    if let Some(p) = &settings_file {
        match import_settings(conn, p) {
            Ok(n) => result.settings += n,
            Err(e) => { result.errors += 1; result.messages.push(format!("settings: {}", e)); }
        }
    }

    result
}

fn import_settings(conn: &Connection, path: &str) -> Result<usize, Box<dyn std::error::Error>> {
    let content = std::fs::read_to_string(path)?;
    let mut count = 0usize;
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with("//") { continue; }
        if let Some((k, v)) = line.split_once('=') {
            let k = k.trim(); let v = v.trim();
            if !k.is_empty() {
                conn.execute(
                    "INSERT INTO settings(key,value) VALUES(?1,?2)
                     ON CONFLICT(key) DO UPDATE SET value=excluded.value",
                    rusqlite::params![k, v])?;
                count += 1;
            }
        } else if let Some((k, v)) = line.split_once('\t') {
            let k = k.trim(); let v = v.trim();
            if !k.is_empty() {
                conn.execute(
                    "INSERT INTO settings(key,value) VALUES(?1,?2)
                     ON CONFLICT(key) DO UPDATE SET value=excluded.value",
                    rusqlite::params![k, v])?;
                count += 1;
            }
        }
    }
    Ok(count)
}
