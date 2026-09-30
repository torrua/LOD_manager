//! LOD (Loglan Online Dictionary) file import.
//!
//! Parses `@`-delimited LOD text files and imports them into the `SQLite` database.
//! File types are matched by filename: types, authors, events, syllables, words,
//! spellings, definitions, and settings.
//!
//! # Import pipeline
//! 1. Types → 2. Authors → 3. Events → 4. Syllables → 5. Words & Spellings → 6. Definitions → 7. Settings
//!
//! All imports run in a single transaction for atomicity.

use crate::db;
use crate::models::ImportResult;
use rusqlite::{Connection, params};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

const SEP: char = '@';

#[derive(Clone, Default)]
struct WordStaging {
    type_name: String,
    affixes: Vec<String>,
    match_: Option<String>,
    source: Option<String>,
    year: Option<String>,
    rank: Option<String>,
    origin: Option<String>,
    origin_x: Option<String>,
    usedin: Option<String>,
    tid_old: Option<i64>,
}

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

fn parse_bool_literal(s: &str) -> Option<bool> {
    let t = s.trim();
    if t.eq_ignore_ascii_case("true") || t == "1" || t.eq_ignore_ascii_case("yes") {
        Some(true)
    } else if t.eq_ignore_ascii_case("false") || t == "0" || t.eq_ignore_ascii_case("no") {
        Some(false)
    } else {
        None
    }
}

/// Import from `(filename, text_content)` pairs directly in memory.
/// Used on Android (where `content://` URIs are read via `plugin-fs`) and by `import_files`.
pub fn import_contents(
    conn: &mut Connection,
    files: &[(String, String)],
) -> Result<ImportResult, String> {
    let mut result = ImportResult {
        words: 0,
        definitions: 0,
        events: 0,
        types: 0,
        authors: 0,
        settings: 0,
        errors: 0,
        skipped_rows: 0,
        messages: vec![],
    };

    if files.is_empty() {
        return Ok(result);
    }

    let mut type_content: Option<&str> = None;
    let mut author_content: Option<&str> = None;
    let mut event_content: Option<&str> = None;
    let mut syllable_content: Option<&str> = None;
    let mut word_content: Option<&str> = None;
    let mut spell_content: Option<&str> = None;
    let mut def_content: Option<&str> = None;
    let mut settings_content: Option<&str> = None;

    for (name, content) in files {
        let lower = Path::new(name)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(name.as_str())
            .to_lowercase();
        match lower.as_str() {
            s if s.contains("setting") => settings_content = Some(content.as_str()),
            s if s.contains("type") => type_content = Some(content.as_str()),
            s if s.contains("author") => author_content = Some(content.as_str()),
            s if s.contains("lexevent") || s.contains("event") => {
                event_content = Some(content.as_str());
            }
            s if s.contains("syllable") => syllable_content = Some(content.as_str()),
            s if s.contains("wordspell") || s.contains("spell") => {
                spell_content = Some(content.as_str());
            }
            s if (s.contains("worddef") || s.contains("definition")) && !s.contains("word.txt") => {
                def_content = Some(content.as_str());
            }
            s if s == "word.txt" || s == "words.txt" => word_content = Some(content.as_str()),
            _ => {}
        }
    }

    let tx = conn
        .transaction()
        .map_err(|e| format!("Failed to start import transaction: {e}"))?;

    // 1. Types
    if let Some(content) = type_content {
        import_types(&tx, content, &mut result);
    }

    // 2. Authors
    if let Some(content) = author_content {
        import_authors(&tx, content, &mut result);
    }

    // 3. Events
    if let Some(content) = event_content {
        import_events(&tx, content, &mut result);
    }

    // 4. Syllables
    if let Some(content) = syllable_content {
        import_syllables(&tx, content, &mut result);
    }

    // 5. Words + Spellings
    let old_id_to_db_ids = import_words_and_spells(&tx, word_content, spell_content, &mut result);

    // 6. Definitions
    if let Some(content) = def_content {
        import_definitions(&tx, content, &old_id_to_db_ids, &mut result);
    }

    // 7. Settings
    if let Some(content) = settings_content
        && let Ok(n) = import_settings_content(&tx, content)
    {
        result.settings += n;
        result
            .messages
            .push(format!("Settings: {}", result.settings));
    }

    if result.skipped_rows > 0 {
        result
            .messages
            .push(format!("Skipped rows: {}", result.skipped_rows));
    }

    tx.commit()
        .map_err(|e| format!("Failed to commit import transaction: {e}"))?;
    Ok(result)
}

/// Import from file paths on disk.
pub fn import_files(conn: &mut Connection, paths: &[String]) -> Result<ImportResult, String> {
    if paths.is_empty() {
        return import_contents(conn, &[]);
    }
    let mut files = Vec::with_capacity(paths.len());
    for path in paths {
        let content =
            fs::read_to_string(path).map_err(|e| format!("Failed to read file {path}: {e}"))?;
        files.push((path.clone(), content));
    }
    import_contents(conn, &files)
}

fn import_types(tx: &Connection, content: &str, result: &mut ImportResult) {
    for r in rows(content) {
        if let Some(name) = r.first().filter(|s| !s.is_empty()) {
            let type_x = r.get(1).and_then(|s| opt(s)).unwrap_or_default();
            let group = r.get(2).and_then(|s| opt(s)).unwrap_or_default();
            let (parentable, description) =
                if let Some(b) = r.get(3).and_then(|s| parse_bool_literal(s)) {
                    (b, r.get(4).and_then(|s| opt(s)))
                } else if r.len() >= 5 {
                    (true, r.get(4).and_then(|s| opt(s)))
                } else {
                    (true, r.get(3).and_then(|s| opt(s)))
                };
            let existing_id: Option<i64> = tx
                .query_row(
                    "SELECT id FROM types WHERE type = ?1",
                    params![name],
                    |row| row.get(0),
                )
                .ok();
            let ok = if let Some(tid) = existing_id {
                tx.execute(
                    "UPDATE types SET type_x = ?1, \"group\" = ?2, parentable = ?3, description = ?4, updated = datetime('now')
                     WHERE id = ?5",
                    params![type_x, group, parentable, description, tid],
                )
                .is_ok()
            } else {
                tx.execute(
                    "INSERT INTO types (type, type_x, \"group\", parentable, description, id, created, updated)
                     VALUES (?1, ?2, ?3, ?4, ?5, (SELECT COALESCE(MAX(id), 0) + 1 FROM types), datetime('now'), datetime('now'))",
                    params![name, type_x, group, parentable, description],
                )
                .is_ok()
            };
            if ok && tx.changes() > 0 {
                result.types += 1;
            }
        } else {
            result.skipped_rows += 1;
        }
    }
    result.messages.push(format!("Types: {}", result.types));
}

fn import_authors(tx: &Connection, content: &str, result: &mut ImportResult) {
    for r in rows(content) {
        if let Some(initials) = r.first().filter(|s| !s.is_empty()) {
            let full_name = r.get(1).and_then(|s| opt(s));
            let notes = r.get(2).and_then(|s| opt(s));
            if tx
                .execute(
                    "INSERT INTO authors (abbreviation, full_name, notes, id, created, updated)
                     VALUES (?1, ?2, ?3, (SELECT COALESCE(MAX(id), 0) + 1 FROM authors), datetime('now'), datetime('now'))
                     ON CONFLICT(abbreviation) DO UPDATE SET
                         full_name = excluded.full_name,
                         notes = excluded.notes,
                         updated = datetime('now')",
                    params![initials, full_name, notes],
                )
                .is_ok()
                && tx.changes() > 0
            {
                result.authors += 1;
            }
        } else {
            result.skipped_rows += 1;
        }
    }
    result.messages.push(format!("Authors: {}", result.authors));
}

fn import_events(tx: &Connection, content: &str, result: &mut ImportResult) {
    for r in rows(content) {
        if r.len() < 2 {
            result.skipped_rows += 1;
            continue;
        }
        let Ok(event_id) = r[0].parse::<i64>() else {
            result.skipped_rows += 1;
            continue;
        };
        let name = &r[1];
        if name.is_empty() {
            result.skipped_rows += 1;
            continue;
        }
        let date = r.get(2).and_then(|s| opt(s)).unwrap_or_default();
        let definition = r.get(3).and_then(|s| opt(s)).unwrap_or_default();
        let annotation = r.get(4).and_then(|s| opt(s)).unwrap_or_default();
        let suffix = r.get(5).and_then(|s| opt(s)).unwrap_or_default();
        if tx
            .execute(
                "INSERT INTO events (event_id, name, date, definition, annotation, suffix, id, created, updated)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, (SELECT COALESCE(MAX(id), 0) + 1 FROM events), datetime('now'), datetime('now'))
                 ON CONFLICT(event_id) DO UPDATE SET
                     name = excluded.name,
                     date = excluded.date,
                     definition = excluded.definition,
                     annotation = excluded.annotation,
                     suffix = excluded.suffix,
                     updated = datetime('now')",
                params![event_id, name, date, definition, annotation, suffix],
            )
            .is_ok()
            && tx.changes() > 0
        {
            result.events += 1;
        }
    }
    result.messages.push(format!("Events: {}", result.events));
}

fn import_syllables(tx: &Connection, content: &str, result: &mut ImportResult) {
    for r in rows(content) {
        if r.len() < 2 || r[0].is_empty() {
            result.skipped_rows += 1;
            continue;
        }
        let name = &r[0];
        let type_ = r.get(1).and_then(|s| opt(s)).unwrap_or_default();
        let allowed = r.get(2).and_then(|s| parse_bool_literal(s)).unwrap_or(true);
        let exists: bool = tx
            .query_row(
                "SELECT 1 FROM syllables WHERE name = ?1 LIMIT 1",
                params![name],
                |_| Ok(true),
            )
            .unwrap_or(false);
        if !exists {
            let _ = tx.execute(
                "INSERT INTO syllables (name, type, allowed, id, created, updated)
                 VALUES (?1, ?2, ?3, (SELECT COALESCE(MAX(id), 0) + 1 FROM syllables), datetime('now'), datetime('now'))",
                params![name, type_, allowed],
            );
        }
    }
}

fn import_words_and_spells(
    tx: &Connection,
    word_content: Option<&str>,
    spell_content: Option<&str>,
    result: &mut ImportResult,
) -> HashMap<String, Vec<i64>> {
    let mut word_staging: HashMap<String, WordStaging> = HashMap::new();
    let mut old_id_to_db_ids: HashMap<String, Vec<i64>> = HashMap::new();

    if let Some(content) = word_content {
        for r in rows(content) {
            if r.len() < 2 || r[0].is_empty() {
                result.skipped_rows += 1;
                continue;
            }
            let old_id = r[0].clone();
            let affixes: Vec<String> = r
                .get(3)
                .map_or("", String::as_str)
                .split_whitespace()
                .filter(|s| !s.is_empty())
                .map(String::from)
                .collect();
            word_staging.insert(
                old_id,
                WordStaging {
                    type_name: r.get(1).map_or("", String::as_str).to_string(),
                    affixes,
                    match_: r.get(4).and_then(|s| opt(s)),
                    source: r.get(5).and_then(|s| opt(s)),
                    year: r.get(6).and_then(|s| opt(s)),
                    rank: r.get(7).and_then(|s| opt(s)),
                    origin: r.get(8).and_then(|s| opt(s)),
                    origin_x: r.get(9).and_then(|s| opt(s)),
                    usedin: r.get(10).and_then(|s| opt(s)),
                    tid_old: r.get(11).and_then(|s| s.trim().parse::<i64>().ok()),
                },
            );
        }
    }

    if let Some(content) = spell_content {
        let fk_on: bool = tx
            .query_row("PRAGMA foreign_keys", [], |r| r.get::<_, i64>(0))
            .unwrap_or(0)
            != 0;
        let has_word_affixes = db::table_exists(tx, "word_affixes");
        let has_word_usage = db::table_exists(tx, "word_usage");

        for r in rows(content) {
            if r.len() < 2 {
                result.skipped_rows += 1;
                continue;
            }
            let old_id = &r[0];
            let name = &r[1];
            if old_id.is_empty() || name.is_empty() {
                result.skipped_rows += 1;
                continue;
            }

            let id_old_num: i64 = old_id.parse().unwrap_or(0);
            let ev_start: i64 = r
                .get(4)
                .and_then(|s| s.trim().parse::<i64>().ok())
                .unwrap_or(1);
            let raw_ev_end: Option<i64> = r
                .get(5)
                .and_then(|s| s.trim().parse::<i64>().ok())
                .filter(|&eid| eid < 9999);
            let ev_end: Option<i64> = if fk_on {
                raw_ev_end.and_then(|eid| {
                    tx.query_row(
                        "SELECT event_id FROM events WHERE event_id=?1",
                        params![eid],
                        |row| row.get(0),
                    )
                    .ok()
                })
            } else {
                raw_ev_end
            };

            let st = word_staging.get(old_id).cloned().unwrap_or_default();
            let type_id: Option<i64> = if st.type_name.is_empty() {
                None
            } else {
                tx.query_row(
                    "SELECT id FROM types WHERE type=?1",
                    params![&st.type_name],
                    |row| row.get(0),
                )
                .ok()
            };

            let (base_source, db_year, db_rank, db_notes) = db::denormalize_word_fields(
                st.source.as_deref(),
                st.year.as_deref(),
                st.rank.as_deref(),
                None,
            );

            if tx
                .execute(
                    "INSERT INTO words (name, type, \"match\", rank, year, origin, origin_x, notes, id_old, \"TID_old\", event_start, event_end, id, created, updated)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, (SELECT COALESCE(MAX(id), 0) + 1 FROM words), datetime('now'), datetime('now'))",
                    params![
                        name,
                        type_id,
                        st.match_,
                        db_rank,
                        db_year,
                        st.origin,
                        st.origin_x,
                        db_notes,
                        id_old_num,
                        st.tid_old,
                        ev_start,
                        ev_end
                    ],
                )
                .is_ok()
                && tx.changes() > 0
            {
                let wid = tx.last_insert_rowid();
                result.words += 1;
                old_id_to_db_ids
                    .entry(old_id.clone())
                    .or_default()
                    .push(wid);

                if has_word_affixes {
                    for a in &st.affixes {
                        let _ = tx.execute(
                            "INSERT INTO word_affixes (word_id, affix) VALUES (?1, ?2)",
                            params![wid, a],
                        );
                    }
                }

                if has_word_usage
                    && let Some(ref usedin_data) = st.usedin
                {
                    for used_word in usedin_data.split(|c: char| c == '|' || c.is_whitespace()) {
                        let used_word = used_word.trim();
                        if !used_word.is_empty() {
                            let _ = tx.execute(
                                "INSERT INTO word_usage (word_id, used_in_word) VALUES (?1, ?2)",
                                params![wid, used_word],
                            );
                        }
                    }
                }

                if let Some(ref src) = base_source {
                    for abbr in src.split(|c: char| c == '/' || c.is_whitespace()) {
                        let abbr = abbr.trim();
                        if !abbr.is_empty()
                            && let Ok(aid) = tx.query_row(
                                "SELECT id FROM authors WHERE abbreviation=?1",
                                params![abbr],
                                |row| row.get::<_, i64>(0),
                            )
                        {
                            let _ = tx.execute(
                                "INSERT OR IGNORE INTO connect_authors (\"AID\", \"WID\") VALUES (?1, ?2)",
                                params![aid, wid],
                            );
                        }
                    }
                }
            }
        }
        result.messages.push(format!("Words: {}", result.words));
    }

    old_id_to_db_ids
}

fn import_definitions(
    tx: &Connection,
    content: &str,
    old_id_to_db_ids: &HashMap<String, Vec<i64>>,
    result: &mut ImportResult,
) {
    let mut def_count = 0usize;
    let mut seen_wid_pos = std::collections::HashSet::<(i64, i64)>::new();

    for r in rows(content) {
        if r.len() < 5 {
            result.skipped_rows += 1;
            continue;
        }
        let old_word_id = &r[0];
        let position: i64 = r[1].parse().unwrap_or(0);
        let usage = r.get(2).and_then(|s| opt(s));
        let raw_grammar = r.get(3).and_then(|s| opt(s));
        let (slots, grammar_code) = db::split_grammar(raw_grammar.as_deref());
        let body = r.get(4).map_or("", String::as_str);
        if body.is_empty() {
            result.skipped_rows += 1;
            continue;
        }
        let tags = r
            .get(6)
            .and_then(|s| opt(s))
            .or_else(|| r.get(5).and_then(|s| opt(s)));

        let target_wids: Vec<i64> = if let Some(wids) = old_id_to_db_ids.get(old_word_id) {
            wids.clone()
        } else if let Ok(id_old_num) = old_word_id.parse::<i64>()
            && let Ok(mut stmt) = tx.prepare("SELECT id FROM words WHERE id_old=?1 ORDER BY id")
            && let Ok(mapped) = stmt.query_map(params![id_old_num], |row| row.get::<_, i64>(0))
        {
            mapped.filter_map(std::result::Result::ok).collect()
        } else {
            Vec::new()
        };

        if target_wids.is_empty() {
            result.skipped_rows += 1;
            continue;
        }

        for wid in target_wids {
            if !seen_wid_pos.insert((wid, position)) {
                continue;
            }
            if tx
                .execute(
                    "INSERT INTO definitions (word_id, position, body, usage, grammar_code, slots, case_tags, id, created, updated)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, (SELECT COALESCE(MAX(id), 0) + 1 FROM definitions), datetime('now'), datetime('now'))",
                    params![wid, position, body, usage, grammar_code, slots, tags],
                )
                .is_ok()
                && tx.changes() > 0
            {
                def_count += 1;
            }
        }
    }
    result.definitions = def_count;
    result
        .messages
        .push(format!("Definitions: {}", result.definitions));
}

fn import_settings_content(
    conn: &Connection,
    content: &str,
) -> Result<usize, Box<dyn std::error::Error>> {
    let mut count = 0usize;
    for line in content
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#') && !l.starts_with("//"))
    {
        if line.contains('@') && line.chars().filter(|&c| c == '@').count() >= 3 {
            let parts: Vec<&str> = line.split('@').map(str::trim).collect();
            if parts.len() >= 4 {
                let db_version: i64 = parts[1].parse().unwrap_or(1);
                let last_word_id: i64 = parts[2].parse().unwrap_or(0);
                conn.execute(
                    "INSERT INTO settings (date, db_version, last_word_id, db_release, id, created, updated)
                     VALUES (?1, ?2, ?3, ?4, (SELECT COALESCE(MAX(id), 0) + 1 FROM settings), datetime('now'), datetime('now'))
                     ON CONFLICT(date) DO UPDATE SET
                         db_version = excluded.db_version,
                         last_word_id = excluded.last_word_id,
                         db_release = excluded.db_release,
                         updated = datetime('now')",
                    params![parts[0], db_version, last_word_id, parts[3]],
                )?;
                count += 1;
            }
        } else if let Some((k, v)) = line.split_once('=').or_else(|| line.split_once('\t')) {
            let (k, v) = (k.trim(), v.trim());
            if !k.is_empty() {
                db::upsert_setting(conn, k, v)?;
                count += 1;
            }
        }
    }
    Ok(count)
}
