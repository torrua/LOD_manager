//! Database operations for the Loglan Online Dictionary.
//!
//! This module provides all data access functions: schema management,
//! CRUD operations for words/definitions/events/types/authors, FTS5 search,
//! and migrations compatible with `torrua/loglan_core` (`export.db`).

use crate::models::*;
use rusqlite::{Connection, params};
use std::convert::TryInto;

pub(crate) fn table_exists(conn: &Connection, name: &str) -> bool {
    conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?1",
        params![name],
        |r| r.get::<_, i64>(0),
    )
    .unwrap_or(0)
        > 0
}

fn column_exists(conn: &Connection, table: &str, column: &str) -> bool {
    let sql = format!("PRAGMA table_info({table})");
    let Ok(mut stmt) = conn.prepare(&sql) else {
        return false;
    };
    let Ok(rows) = stmt.query_map([], |r| r.get::<_, String>(1)) else {
        return false;
    };
    rows.filter_map(std::result::Result::ok)
        .any(|col| col == column)
}

/// Split a combined grammar string like `"2a"` into `(Some(2), Some("a"))`
/// to match `torrua/loglan_core` (`definitions.slots` + `definitions.grammar_code`).
pub fn split_grammar(g: Option<&str>) -> (Option<i64>, Option<String>) {
    let Some(raw) = g.map(str::trim).filter(|s| !s.is_empty()) else {
        return (None, None);
    };
    let digit_len = raw.chars().take_while(char::is_ascii_digit).count();
    let slots = if digit_len > 0 {
        raw[..digit_len].parse::<i64>().ok()
    } else {
        None
    };
    let rest = raw[digit_len..].trim();
    let code = if rest.is_empty() {
        None
    } else {
        Some(rest.to_string())
    };
    (slots, code)
}

/// Normalize `source`, `year`, `rank`, and `notes` from `connect_authors` and `words.notes` JSON.
pub(crate) fn normalize_word_fields(
    authors_csv: Option<&str>,
    raw_year: Option<String>,
    raw_rank: Option<String>,
    raw_notes: Option<String>,
) -> (
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
) {
    let mut note_author: Option<String> = None;
    let mut note_year: Option<String> = None;
    let mut note_rank: Option<String> = None;
    let mut clean_notes: Option<String> = None;

    if let Some(rn) = raw_notes.as_deref().map(str::trim)
        && !rn.is_empty()
        && !rn.eq_ignore_ascii_case("null")
    {
        if rn.starts_with('{')
            && let Ok(serde_json::Value::Object(map)) = serde_json::from_str(rn)
        {
            note_author = map
                .get("author")
                .and_then(serde_json::Value::as_str)
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string);
            note_year = map
                .get("year")
                .and_then(serde_json::Value::as_str)
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string);
            note_rank = map
                .get("rank")
                .and_then(serde_json::Value::as_str)
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string);

            let mut extra_parts = Vec::new();
            for (k, v) in &map {
                if k != "author"
                    && k != "year"
                    && k != "rank"
                    && let Some(s) = v.as_str().map(str::trim).filter(|s| !s.is_empty())
                {
                    if k == "notes" {
                        extra_parts.push(s.to_string());
                    } else {
                        extra_parts.push(format!("{k}: {s}"));
                    }
                }
            }
            if !extra_parts.is_empty() {
                clean_notes = Some(extra_parts.join("; "));
            }
        } else {
            clean_notes = Some(rn.to_string());
        }
    }

    let base_authors = authors_csv.map(str::trim).filter(|s| !s.is_empty());
    let source = match (base_authors, note_author.as_deref()) {
        (Some(a), Some(na)) => Some(format!("{a} {na}")),
        (Some(a), None) => Some(a.to_string()),
        (None, Some(na)) => Some(na.to_string()),
        (None, None) => None,
    };

    let base_year = raw_year
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|y| y.strip_suffix("-01-01").unwrap_or(y));
    let year = match (base_year, note_year.as_deref()) {
        (Some(y), Some(ny)) => Some(format!("{y} {ny}")),
        (Some(y), None) => Some(y.to_string()),
        (None, Some(ny)) => Some(ny.to_string()),
        (None, None) => None,
    };

    let base_rank = raw_rank
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty() && *s != "None");
    let rank = match (base_rank, note_rank.as_deref()) {
        (Some(r), Some(nr)) => Some(format!("{r} {nr}")),
        (Some(r), None) => Some(r.to_string()),
        (None, Some(nr)) => Some(nr.to_string()),
        (None, None) => None,
    };

    (source, year, rank, clean_notes)
}

fn split_base_and_note(val: Option<&str>) -> (Option<String>, Option<String>) {
    let Some(trimmed) = val.map(str::trim).filter(|s| !s.is_empty()) else {
        return (None, None);
    };
    if trimmed.starts_with('(') {
        return (None, Some(trimmed.to_string()));
    }
    if let Some(idx) = trimmed.find(" (") {
        let base = trimmed[..idx].trim();
        let note = trimmed[idx + 1..].trim();
        let b = (!base.is_empty()).then(|| base.to_string());
        let n = (!note.is_empty()).then(|| note.to_string());
        return (b, n);
    }
    (Some(trimmed.to_string()), None)
}

/// Inverse of [`normalize_word_fields`]: splits combined `source`, `year`, `rank`, and `notes`
/// into `(base_source, canonical_year, base_rank, json_or_plain_notes)` for `words` storage.
pub(crate) fn denormalize_word_fields(
    source: Option<&str>,
    year: Option<&str>,
    rank: Option<&str>,
    notes: Option<&str>,
) -> (
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
) {
    let (base_source, author_note) = split_base_and_note(source);
    let (raw_year, year_note) = split_base_and_note(year);
    let (base_rank, rank_note) = split_base_and_note(rank);

    let canonical_year = raw_year.map(|y| {
        if y.len() == 4 && y.chars().all(|c| c.is_ascii_digit()) {
            format!("{y}-01-01")
        } else if y.len() == 2
            && y.chars().all(|c| c.is_ascii_digit())
            && let Ok(n) = y.parse::<u16>()
        {
            let full = if n >= 50 { 1900 + n } else { 2000 + n };
            format!("{full}-01-01")
        } else {
            y
        }
    });

    let clean_notes = notes
        .map(str::trim)
        .filter(|s| !s.is_empty() && !s.eq_ignore_ascii_case("null"));

    let db_notes = if author_note.is_some() || year_note.is_some() || rank_note.is_some() {
        let mut map = serde_json::Map::new();
        if let Some(n) = clean_notes {
            if n.starts_with('{')
                && let Ok(serde_json::Value::Object(existing)) = serde_json::from_str(n)
            {
                map = existing;
            } else {
                map.insert(
                    "notes".to_string(),
                    serde_json::Value::String(n.to_string()),
                );
            }
        }
        if let Some(an) = author_note {
            map.insert("author".to_string(), serde_json::Value::String(an));
        }
        if let Some(yn) = year_note {
            map.insert("year".to_string(), serde_json::Value::String(yn));
        }
        if let Some(rn) = rank_note {
            map.insert("rank".to_string(), serde_json::Value::String(rn));
        }
        Some(serde_json::Value::Object(map).to_string())
    } else {
        clean_notes.map(str::to_string)
    };

    (base_source, canonical_year, base_rank, db_notes)
}

pub fn init_schema(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        "PRAGMA foreign_keys=ON;

        CREATE TABLE IF NOT EXISTS types (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            type        TEXT NOT NULL UNIQUE,
            type_x      TEXT,
            \"group\"   TEXT,
            parentable  BOOLEAN DEFAULT TRUE,
            description TEXT,
            created     DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated     DATETIME DEFAULT CURRENT_TIMESTAMP
        );

        CREATE TABLE IF NOT EXISTS authors (
            id            INTEGER PRIMARY KEY AUTOINCREMENT,
            abbreviation  TEXT NOT NULL UNIQUE,
            full_name     TEXT,
            notes         TEXT,
            created       DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated       DATETIME DEFAULT CURRENT_TIMESTAMP
        );

        CREATE TABLE IF NOT EXISTS events (
            event_id   INTEGER NOT NULL UNIQUE,
            name       TEXT NOT NULL,
            date       TEXT,
            definition TEXT,
            annotation TEXT,
            suffix     TEXT,
            id         INTEGER PRIMARY KEY AUTOINCREMENT,
            created    DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated    DATETIME DEFAULT CURRENT_TIMESTAMP
        );

        CREATE TABLE IF NOT EXISTS syllables (
            id      INTEGER PRIMARY KEY AUTOINCREMENT,
            name    TEXT NOT NULL UNIQUE,
            type    TEXT NOT NULL,
            allowed BOOLEAN NOT NULL DEFAULT TRUE,
            created DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated DATETIME DEFAULT CURRENT_TIMESTAMP
        );

        CREATE TABLE IF NOT EXISTS words (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            name        TEXT NOT NULL,
            type        INTEGER NOT NULL REFERENCES types(id),
            origin      TEXT,
            origin_x    TEXT,
            \"match\"   TEXT,
            rank        TEXT,
            year        TEXT,
            notes       TEXT,
            id_old      INTEGER NOT NULL DEFAULT 0,
            \"TID_old\" INTEGER,
            event_start INTEGER NOT NULL DEFAULT 1 REFERENCES events(event_id),
            event_end   INTEGER REFERENCES events(event_id),
            created     DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated     DATETIME DEFAULT CURRENT_TIMESTAMP
        );
        CREATE INDEX IF NOT EXISTS idx_words_name       ON words(name);
        CREATE INDEX IF NOT EXISTS idx_words_name_lower ON words(LOWER(name));
        CREATE INDEX IF NOT EXISTS idx_words_type       ON words(type);
        CREATE INDEX IF NOT EXISTS idx_words_ev_start   ON words(event_start);
        CREATE INDEX IF NOT EXISTS idx_words_ev_end     ON words(event_end);

        CREATE TABLE IF NOT EXISTS word_spellings (
            id       INTEGER PRIMARY KEY AUTOINCREMENT,
            word_id  INTEGER NOT NULL REFERENCES words(id) ON DELETE CASCADE,
            spelling TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_word_spellings_word_id ON word_spellings(word_id);

        CREATE TABLE IF NOT EXISTS word_affixes (
            id      INTEGER PRIMARY KEY AUTOINCREMENT,
            word_id INTEGER NOT NULL REFERENCES words(id) ON DELETE CASCADE,
            affix   TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_word_affixes_word_id ON word_affixes(word_id);
        CREATE INDEX IF NOT EXISTS idx_word_affixes_affix   ON word_affixes(affix);

        CREATE TABLE IF NOT EXISTS word_usage (
            id           INTEGER PRIMARY KEY AUTOINCREMENT,
            word_id      INTEGER NOT NULL REFERENCES words(id) ON DELETE CASCADE,
            used_in_word TEXT NOT NULL,
            created      DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
        );
        CREATE INDEX IF NOT EXISTS idx_word_usage_word_id ON word_usage(word_id);
        CREATE INDEX IF NOT EXISTS idx_word_usage_used_in ON word_usage(used_in_word);

        CREATE TABLE IF NOT EXISTS settings (
            date         DATETIME NOT NULL,
            db_version   INTEGER NOT NULL,
            last_word_id INTEGER NOT NULL,
            db_release   TEXT NOT NULL,
            id           INTEGER PRIMARY KEY AUTOINCREMENT,
            created      DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated      DATETIME DEFAULT CURRENT_TIMESTAMP,
            UNIQUE(date)
        );

        CREATE TABLE IF NOT EXISTS definitions (
            id           INTEGER PRIMARY KEY AUTOINCREMENT,
            word_id      INTEGER NOT NULL REFERENCES words(id) ON DELETE CASCADE,
            position     INTEGER NOT NULL DEFAULT 0,
            body         TEXT NOT NULL DEFAULT '',
            usage        TEXT,
            grammar_code TEXT,
            slots        INTEGER,
            case_tags    TEXT,
            language     TEXT,
            notes        TEXT,
            created      DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated      DATETIME DEFAULT CURRENT_TIMESTAMP,
            UNIQUE(word_id, position)
        );
        CREATE INDEX IF NOT EXISTS idx_def_word_pos ON definitions(word_id, position);

        CREATE TABLE IF NOT EXISTS keys (
            id       INTEGER PRIMARY KEY AUTOINCREMENT,
            word     TEXT NOT NULL,
            language TEXT NOT NULL DEFAULT 'en',
            created  DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated  DATETIME DEFAULT CURRENT_TIMESTAMP,
            UNIQUE(word, language)
        );

        CREATE TABLE IF NOT EXISTS connect_words (
            parent_id INTEGER NOT NULL REFERENCES words(id) ON DELETE CASCADE,
            child_id  INTEGER NOT NULL REFERENCES words(id) ON DELETE CASCADE,
            PRIMARY KEY (parent_id, child_id)
        );
        CREATE INDEX IF NOT EXISTS index_parent_id ON connect_words (parent_id);
        CREATE INDEX IF NOT EXISTS index_child_id  ON connect_words (child_id);

        CREATE TABLE IF NOT EXISTS connect_authors (
            \"AID\" INTEGER NOT NULL REFERENCES authors(id) ON DELETE CASCADE,
            \"WID\" INTEGER NOT NULL REFERENCES words(id) ON DELETE CASCADE,
            PRIMARY KEY (\"AID\", \"WID\")
        );
        CREATE INDEX IF NOT EXISTS index_aid ON connect_authors (\"AID\");
        CREATE INDEX IF NOT EXISTS index_wid ON connect_authors (\"WID\");

        CREATE TABLE IF NOT EXISTS connect_keys (
            \"KID\" INTEGER NOT NULL REFERENCES keys(id) ON DELETE CASCADE,
            \"DID\" INTEGER NOT NULL REFERENCES definitions(id) ON DELETE CASCADE,
            PRIMARY KEY (\"KID\", \"DID\")
        );
        CREATE INDEX IF NOT EXISTS index_kid ON connect_keys (\"KID\");
        CREATE INDEX IF NOT EXISTS index_did ON connect_keys (\"DID\");

        INSERT OR IGNORE INTO events (event_id, name, date, definition, annotation, suffix)
        VALUES (1, 'Start', '', '', '', '');",
    )
}

/// Add any indexes or tables that may be missing in databases created before they were
/// added to `init_schema`, and migrate intermediate `group_` / `match_` column names to
/// canonical `loglan_core` `"group"` / `"match"`.
pub fn add_missing_indexes(conn: &Connection) -> rusqlite::Result<()> {
    if column_exists(conn, "types", "group_") && !column_exists(conn, "types", "group") {
        conn.execute_batch("ALTER TABLE types RENAME COLUMN group_ TO \"group\";")?;
    }
    if column_exists(conn, "words", "match_") && !column_exists(conn, "words", "match") {
        conn.execute_batch("ALTER TABLE words RENAME COLUMN match_ TO \"match\";")?;
    }

    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS word_spellings (
            id       INTEGER PRIMARY KEY AUTOINCREMENT,
            word_id  INTEGER NOT NULL REFERENCES words(id) ON DELETE CASCADE,
            spelling TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS word_affixes (
            id      INTEGER PRIMARY KEY AUTOINCREMENT,
            word_id INTEGER NOT NULL REFERENCES words(id) ON DELETE CASCADE,
            affix   TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS word_usage (
            id           INTEGER PRIMARY KEY AUTOINCREMENT,
            word_id      INTEGER NOT NULL REFERENCES words(id) ON DELETE CASCADE,
            used_in_word TEXT NOT NULL,
            created      DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
        );
        CREATE TABLE IF NOT EXISTS syllables (
            id      INTEGER PRIMARY KEY AUTOINCREMENT,
            name    TEXT NOT NULL UNIQUE,
            type    TEXT NOT NULL,
            allowed BOOLEAN NOT NULL DEFAULT TRUE,
            created DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated DATETIME DEFAULT CURRENT_TIMESTAMP
        );
        CREATE TABLE IF NOT EXISTS keys (
            id       INTEGER PRIMARY KEY AUTOINCREMENT,
            word     TEXT NOT NULL,
            language TEXT NOT NULL DEFAULT 'en',
            created  DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated  DATETIME DEFAULT CURRENT_TIMESTAMP,
            UNIQUE(word, language)
        );
        CREATE TABLE IF NOT EXISTS connect_words (
            parent_id INTEGER NOT NULL REFERENCES words(id) ON DELETE CASCADE,
            child_id  INTEGER NOT NULL REFERENCES words(id) ON DELETE CASCADE,
            PRIMARY KEY (parent_id, child_id)
        );
        CREATE TABLE IF NOT EXISTS connect_authors (
            \"AID\" INTEGER NOT NULL REFERENCES authors(id) ON DELETE CASCADE,
            \"WID\" INTEGER NOT NULL REFERENCES words(id) ON DELETE CASCADE,
            PRIMARY KEY (\"AID\", \"WID\")
        );
        CREATE TABLE IF NOT EXISTS connect_keys (
            \"KID\" INTEGER NOT NULL REFERENCES keys(id) ON DELETE CASCADE,
            \"DID\" INTEGER NOT NULL REFERENCES definitions(id) ON DELETE CASCADE,
            PRIMARY KEY (\"KID\", \"DID\")
        );

        CREATE INDEX IF NOT EXISTS idx_word_spellings_word_id ON word_spellings(word_id);
        CREATE INDEX IF NOT EXISTS idx_word_affixes_word_id   ON word_affixes(word_id);
        CREATE INDEX IF NOT EXISTS idx_word_affixes_affix     ON word_affixes(affix);
        CREATE INDEX IF NOT EXISTS idx_word_usage_word_id     ON word_usage(word_id);
        CREATE INDEX IF NOT EXISTS idx_word_usage_used_in     ON word_usage(used_in_word);
        CREATE INDEX IF NOT EXISTS idx_words_name             ON words(name);
        CREATE INDEX IF NOT EXISTS idx_words_name_lower       ON words(LOWER(name));
        CREATE INDEX IF NOT EXISTS idx_words_type             ON words(type);
        CREATE INDEX IF NOT EXISTS idx_words_ev_start         ON words(event_start);
        CREATE INDEX IF NOT EXISTS idx_words_ev_end           ON words(event_end);
        CREATE INDEX IF NOT EXISTS idx_def_word_pos           ON definitions(word_id, position);
        CREATE INDEX IF NOT EXISTS index_parent_id            ON connect_words (parent_id);
        CREATE INDEX IF NOT EXISTS index_child_id             ON connect_words (child_id);
        CREATE INDEX IF NOT EXISTS index_aid                  ON connect_authors (\"AID\");
        CREATE INDEX IF NOT EXISTS index_wid                  ON connect_authors (\"WID\");
        CREATE INDEX IF NOT EXISTS index_kid                  ON connect_keys (\"KID\");
        CREATE INDEX IF NOT EXISTS index_did                  ON connect_keys (\"DID\");

        DELETE FROM word_usage WHERE used_in_word = '|';
        ",
    )
}

// ─── Words ────────────────────────────────────────────────────────────────────

#[inline]
fn map_wli(r: &rusqlite::Row<'_>) -> rusqlite::Result<WordListItem> {
    Ok(WordListItem {
        id: r.get(0)?,
        name: r.get(1)?,
        type_name: r.get(2)?,
        def_count: r.get(3)?,
    })
}

/// List words with optional prefix/wildcard filter, type filter, and event filter.
pub fn list_words(
    conn: &Connection,
    q: &str,
    type_filter: &str,
    event_id: Option<i64>,
) -> rusqlite::Result<Vec<WordListItem>> {
    let pattern = if q.contains('*') || q.contains('?') {
        q.to_lowercase().replace('*', "%").replace('?', "_")
    } else if q.is_empty() {
        "%".to_string()
    } else {
        format!("{}%", q.to_lowercase())
    };

    let target_event_id = event_id.map(|eid| {
        conn.query_row(
            "SELECT event_id FROM events WHERE id = ?1",
            params![eid],
            |r| r.get(0),
        )
        .unwrap_or(eid)
    });

    let sql = "
        SELECT w.id, w.name, t.type,
               (SELECT COUNT(*) FROM definitions d WHERE d.word_id = w.id)
        FROM words w
        LEFT JOIN types t ON t.id = w.type
        WHERE LOWER(w.name) LIKE ?1
          AND (?2 = '' OR t.type = ?2)
          AND (?3 IS NULL
               OR (w.event_start <= ?3
                   AND (w.event_end IS NULL OR w.event_end > ?3)))
        ORDER BY LOWER(w.name),
                 CASE WHEN w.event_end IS NULL THEN 0 ELSE 1 END,
                 w.event_start DESC,
                 w.id DESC
    ";
    let mut stmt = conn.prepare(sql)?;
    let result: Vec<WordListItem> = stmt
        .query_map(params![pattern, type_filter, target_event_id], map_wli)?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(result)
}

/// Sort parent word names in morphological order according to `origin` (with alphabetical fallback).
fn sort_parents_by_origin(parents: &mut [String], origin: Option<&str>) {
    if parents.len() <= 1 {
        return;
    }
    let Some(orig) = origin.filter(|s| !s.is_empty()) else {
        parents.sort();
        return;
    };
    let clean_origin: String = orig
        .chars()
        .filter(|c| !matches!(c, '(' | ')' | '/'))
        .flat_map(char::to_lowercase)
        .collect();

    let pos_of = |p: &str| -> usize {
        let stem = p.trim_matches('-').to_lowercase();
        if stem.is_empty() {
            return usize::MAX;
        }
        if let Some(idx) = clean_origin.find(&stem) {
            return idx;
        }
        if stem.len() >= 4
            && let Some(prefix4) = stem.get(..4)
            && let Some(idx) = clean_origin.find(prefix4)
        {
            return idx;
        }
        if stem.len() >= 3
            && let Some(prefix3) = stem.get(..3)
            && let Some(idx) = clean_origin.find(prefix3)
        {
            return idx;
        }
        usize::MAX
    };

    parents.sort_by(|a, b| pos_of(a).cmp(&pos_of(b)).then_with(|| a.cmp(b)));
}

/// Fetch a word with all its related data (affixes, spellings, definitions, used-in, children).
pub fn get_word(conn: &Connection, id: i64) -> rusqlite::Result<WordDetail> {
    let has_connect_authors = table_exists(conn, "connect_authors");
    let authors_subquery = if has_connect_authors {
        "COALESCE((SELECT GROUP_CONCAT(abbreviation, '/') FROM (
            SELECT a.abbreviation FROM connect_authors ca
            JOIN authors a ON a.id = ca.\"AID\"
            WHERE ca.\"WID\" = w.id
            ORDER BY a.abbreviation
        )), '')"
    } else {
        "''"
    };

    let main_sql = format!(
        "SELECT w.id, w.name, w.origin, w.origin_x, w.\"match\", w.rank, w.year, w.notes,
                w.type, t.type as type_name, es.name as event_start_name, ee.name as event_end_name,
                {authors_subquery}
         FROM words w
         LEFT JOIN types t ON t.id = w.type
         LEFT JOIN events es ON es.event_id = w.event_start
         LEFT JOIN events ee ON ee.event_id = w.event_end
         WHERE w.id = ?1"
    );

    let mut word: WordDetail = conn.query_row(&main_sql, params![id], |r| {
        let raw_rank: Option<String> = r.get(5)?;
        let raw_year: Option<String> = r.get(6)?;
        let raw_notes: Option<String> = r.get(7)?;
        let authors_csv: String = r.get(12)?;
        let (source, year, rank, notes) =
            normalize_word_fields(Some(&authors_csv), raw_year, raw_rank, raw_notes);

        Ok(WordDetail {
            id: r.get(0)?,
            name: r.get(1)?,
            type_name: r.get(9)?,
            type_id: r.get(8)?,
            source,
            origin: r.get(2)?,
            origin_x: r.get(3)?,
            match_: r.get(4)?,
            rank,
            year,
            notes,
            event_start_name: r.get(10)?,
            event_end_name: r.get(11)?,
            affixes: vec![],
            spellings: vec![],
            definitions: vec![],
            used_in: vec![],
            parents: vec![],
            children: vec![],
        })
    })?;

    let has_connect_words = table_exists(conn, "connect_words");
    let has_word_affixes = table_exists(conn, "word_affixes");
    let has_word_spellings = table_exists(conn, "word_spellings");
    let has_word_usage = table_exists(conn, "word_usage");

    // ── 2. Affixes ────────────────────────────────────────────────────────────
    let mut affixes: Vec<String> = Vec::new();
    if has_connect_words {
        let mut s = conn.prepare(
            "SELECT DISTINCT REPLACE(w.name, '-', '')
             FROM connect_words cw
             JOIN words w ON w.id = cw.child_id
             LEFT JOIN types t ON t.id = w.type
             WHERE cw.parent_id = ?1 AND (t.type_x = 'Affix' OR t.type = 'Afx')
             ORDER BY 1",
        )?;
        for a in s
            .query_map(params![id], |r| r.get::<_, String>(0))?
            .filter_map(std::result::Result::ok)
        {
            if !a.is_empty() && !affixes.contains(&a) {
                affixes.push(a);
            }
        }
    }
    if has_word_affixes {
        let mut s =
            conn.prepare("SELECT affix FROM word_affixes WHERE word_id = ?1 ORDER BY id")?;
        for a in s
            .query_map(params![id], |r| r.get::<_, String>(0))?
            .filter_map(std::result::Result::ok)
        {
            if !a.is_empty() && !affixes.contains(&a) {
                affixes.push(a);
            }
        }
    }
    word.affixes = affixes;

    // ── 2b. Spellings ─────────────────────────────────────────────────────────
    let mut spellings: Vec<String> = Vec::new();
    if has_word_spellings {
        let mut s =
            conn.prepare("SELECT spelling FROM word_spellings WHERE word_id = ?1 ORDER BY id")?;
        for sp in s
            .query_map(params![id], |r| r.get::<_, String>(0))?
            .filter_map(std::result::Result::ok)
        {
            if !sp.is_empty() && !spellings.contains(&sp) {
                spellings.push(sp);
            }
        }
    }
    {
        let mut s = conn.prepare(
            "SELECT w2.name FROM words w1
             JOIN words w2 ON w2.id_old = w1.id_old AND w2.id != w1.id
             WHERE w1.id = ?1 AND w1.id_old > 0
             ORDER BY w2.id",
        )?;
        for sp in s
            .query_map(params![id], |r| r.get::<_, String>(0))?
            .filter_map(std::result::Result::ok)
        {
            if !sp.is_empty() && sp != word.name && !spellings.contains(&sp) {
                spellings.push(sp);
            }
        }
    }
    word.spellings = spellings;

    // ── 3. Definitions via json_group_array ──────────────────────────────────
    let json_str: String = conn
        .query_row(
            "SELECT COALESCE(
                json_group_array(
                    json_object(
                        'id',       id,
                        'position', position,
                        'grammar',  NULLIF(COALESCE(CAST(slots AS TEXT), '') || COALESCE(grammar_code, ''), ''),
                        'usage',    usage,
                        'body',     body,
                        'tags',     case_tags
                    )
                ),
                '[]'
            )
            FROM (
                SELECT id, position, slots, grammar_code, usage, body, case_tags
                FROM definitions
                WHERE word_id = ?1
                ORDER BY position
            )",
            params![id],
            |r| r.get(0),
        )
        .unwrap_or_else(|_| "[]".to_string());

    word.definitions = serde_json::from_str::<Vec<Definition>>(&json_str).unwrap_or_default();

    // ── 4. Used-in (complexes in connect_words + word_usage) ──────────────────
    let mut used_in: Vec<String> = Vec::new();
    if has_connect_words {
        let mut s = conn.prepare(
            "SELECT DISTINCT w.name
             FROM connect_words cw
             JOIN words w ON w.id = cw.child_id
             LEFT JOIN types t ON t.id = w.type
             WHERE cw.parent_id = ?1 AND t.\"group\" = 'Cpx'
             ORDER BY w.name",
        )?;
        for u in s
            .query_map(params![id], |r| r.get::<_, String>(0))?
            .filter_map(std::result::Result::ok)
        {
            if !u.is_empty() && !used_in.contains(&u) {
                used_in.push(u);
            }
        }
    }
    if has_word_usage {
        let mut s = conn.prepare(
            "SELECT DISTINCT used_in_word FROM word_usage
             WHERE word_id = ?1
             ORDER BY used_in_word",
        )?;
        for u in s
            .query_map(params![id], |r| r.get::<_, String>(0))?
            .filter_map(std::result::Result::ok)
        {
            if !u.is_empty() && !used_in.contains(&u) {
                used_in.push(u);
            }
        }
    }
    word.used_in = used_in;

    // ── 5. Parents & Children in connect_words ────────────────────────────────
    if has_connect_words {
        let mut s_parents = conn.prepare(
            "SELECT DISTINCT w.name FROM connect_words cw
             JOIN words w ON w.id = cw.parent_id
             WHERE cw.child_id = ?1
             ORDER BY 1",
        )?;
        let mut parents: Vec<String> = s_parents
            .query_map(params![id], |r| r.get(0))?
            .filter_map(std::result::Result::ok)
            .collect();
        sort_parents_by_origin(&mut parents, word.origin.as_deref());
        word.parents = parents;

        let mut s_children = conn.prepare(
            "SELECT DISTINCT w.name FROM connect_words cw
             JOIN words w ON w.id = cw.child_id
             LEFT JOIN types t ON t.id = w.type
             WHERE cw.parent_id = ?1
               AND COALESCE(t.type_x, '') != 'Affix'
               AND COALESCE(t.type, '') != 'Afx'
               AND COALESCE(t.\"group\", '') != 'Cpx'
             ORDER BY 1",
        )?;
        word.children = s_children
            .query_map(params![id], |r| r.get(0))?
            .filter_map(std::result::Result::ok)
            .collect();
    }

    Ok(word)
}

pub fn save_word(conn: &Connection, id: Option<i64>, data: &SaveWord) -> rusqlite::Result<i64> {
    let resolved_type_id: Option<i64> = if let Some(tn) = data
        .type_name
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        conn.query_row("SELECT id FROM types WHERE type=?1", params![tn], |r| {
            r.get(0)
        })
        .ok()
    } else {
        None
    };

    let fallback_type_id = || -> rusqlite::Result<i64> {
        if let Ok(tid) =
            conn.query_row("SELECT id FROM types ORDER BY id LIMIT 1", [], |r| r.get(0))
        {
            Ok(tid)
        } else {
            conn.execute(
                "INSERT INTO types (type, type_x, \"group\", parentable, id, created, updated)
                 VALUES ('Unk', '', '', 1, (SELECT COALESCE(MAX(id), 0) + 1 FROM types), datetime('now'), datetime('now'))",
                [],
            )?;
            Ok(conn.last_insert_rowid())
        }
    };

    let ev_start: i64 = data
        .event_start_id
        .or_else(|| {
            data.event_start
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .and_then(|en| {
                    conn.query_row(
                        "SELECT event_id FROM events WHERE name=?1 OR CAST(event_id AS TEXT)=?1",
                        params![en],
                        |r| r.get(0),
                    )
                    .ok()
                })
        })
        .unwrap_or(1);

    let ev_end: Option<i64> = data.event_end_id.or_else(|| {
        data.event_end
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .and_then(|en| {
                conn.query_row(
                    "SELECT event_id FROM events WHERE name=?1 OR CAST(event_id AS TEXT)=?1",
                    params![en],
                    |r| r.get(0),
                )
                .ok()
            })
    });

    let (base_source, db_year, db_rank, db_notes) = denormalize_word_fields(
        data.source.as_deref(),
        data.year.as_deref(),
        data.rank.as_deref(),
        data.notes.as_deref(),
    );

    let word_id = if let Some(wid) = id {
        let old_name: Option<String> = conn
            .query_row("SELECT name FROM words WHERE id=?1", params![wid], |r| {
                r.get(0)
            })
            .ok();

        let type_id = match resolved_type_id {
            Some(t) => t,
            None => conn
                .query_row("SELECT type FROM words WHERE id=?1", params![wid], |r| {
                    r.get(0)
                })
                .or_else(|_| fallback_type_id())?,
        };

        conn.execute(
            "UPDATE words SET name=?1, type=?2, \"match\"=?3, rank=?4, year=?5,
             origin=?6, origin_x=?7, notes=?8, id_old=COALESCE(?9, id_old), event_start=?10, event_end=?11,
             updated=datetime('now')
             WHERE id=?12",
            params![
                data.name,
                type_id,
                data.match_,
                db_rank,
                db_year,
                data.origin,
                data.origin_x,
                db_notes,
                data.id_old,
                ev_start,
                ev_end,
                wid,
            ],
        )?;

        if let Some(old) = old_name
            && old != data.name
            && table_exists(conn, "word_usage")
        {
            conn.execute(
                "UPDATE word_usage SET used_in_word = ?1 WHERE used_in_word = ?2",
                params![data.name, old],
            )?;
        }

        wid
    } else {
        let type_id = match resolved_type_id {
            Some(t) => t,
            None => fallback_type_id()?,
        };
        conn.execute(
            "INSERT INTO words (name, type, \"match\", rank, year, origin, origin_x, notes, id_old, event_start, event_end, created, updated)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11, datetime('now'), datetime('now'))",
            params![
                data.name,
                type_id,
                data.match_,
                db_rank,
                db_year,
                data.origin,
                data.origin_x,
                db_notes,
                data.id_old.unwrap_or(0),
                ev_start,
                ev_end,
            ],
        )?;
        conn.last_insert_rowid()
    };

    if table_exists(conn, "connect_authors") {
        conn.execute(
            "DELETE FROM connect_authors WHERE \"WID\"=?1",
            params![word_id],
        )?;
        if let Some(src) = base_source
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            for abbr in src
                .split(['/', ','])
                .map(str::trim)
                .filter(|s| !s.is_empty())
            {
                conn.execute(
                    "INSERT OR IGNORE INTO authors (abbreviation, id, created, updated)
                     VALUES (?1, (SELECT COALESCE(MAX(id), 0) + 1 FROM authors), datetime('now'), datetime('now'))",
                    params![abbr],
                )?;
                conn.execute(
                    "INSERT OR IGNORE INTO connect_authors (\"AID\", \"WID\")
                     SELECT id, ?2 FROM authors WHERE abbreviation = ?1",
                    params![abbr, word_id],
                )?;
            }
        }
    }

    if table_exists(conn, "connect_words") {
        conn.execute(
            "DELETE FROM connect_words
             WHERE parent_id = ?1
               AND child_id IN (
                   SELECT w.id FROM words w
                   LEFT JOIN types t ON t.id = w.type
                   WHERE t.type_x = 'Affix' OR t.type = 'Afx'
               )",
            params![word_id],
        )?;
        for a in &data.affixes {
            let clean_a = a.trim().trim_matches('-');
            if !clean_a.is_empty() {
                conn.execute(
                    "INSERT OR IGNORE INTO connect_words (parent_id, child_id)
                     SELECT ?1, w.id FROM words w
                     LEFT JOIN types t ON t.id = w.type
                     WHERE (t.type_x = 'Affix' OR t.type = 'Afx')
                       AND REPLACE(w.name, '-', '') = ?2",
                    params![word_id, clean_a],
                )?;
            }
        }
    }

    if table_exists(conn, "word_affixes") {
        conn.execute(
            "DELETE FROM word_affixes WHERE word_id=?1",
            params![word_id],
        )?;
        for a in &data.affixes {
            conn.execute(
                "INSERT INTO word_affixes (word_id, affix) VALUES (?1,?2)",
                params![word_id, a],
            )?;
        }
    }

    if table_exists(conn, "word_spellings") {
        conn.execute(
            "DELETE FROM word_spellings WHERE word_id=?1",
            params![word_id],
        )?;
        for s in &data.spellings {
            conn.execute(
                "INSERT INTO word_spellings (word_id, spelling) VALUES (?1,?2)",
                params![word_id, s],
            )?;
        }
    }

    Ok(word_id)
}

pub fn delete_word(conn: &Connection, id: i64) -> rusqlite::Result<()> {
    let def_ids: Vec<i64> = {
        let mut stmt = conn.prepare("SELECT id FROM definitions WHERE word_id = ?1")?;
        stmt.query_map(params![id], |r| r.get(0))?
            .filter_map(std::result::Result::ok)
            .collect()
    };
    for def_id in &def_ids {
        let _ = fts_update(conn, *def_id, "");
    }
    if table_exists(conn, "connect_keys") {
        conn.execute(
            "DELETE FROM connect_keys WHERE \"DID\" IN (SELECT id FROM definitions WHERE word_id=?1)",
            params![id],
        )?;
    }
    conn.execute("DELETE FROM definitions WHERE word_id=?1", params![id])?;
    if table_exists(conn, "connect_words") {
        conn.execute(
            "DELETE FROM connect_words WHERE parent_id=?1 OR child_id=?1",
            params![id],
        )?;
    }
    if table_exists(conn, "connect_authors") {
        conn.execute("DELETE FROM connect_authors WHERE \"WID\"=?1", params![id])?;
    }
    if table_exists(conn, "word_affixes") {
        conn.execute("DELETE FROM word_affixes WHERE word_id=?1", params![id])?;
    }
    if table_exists(conn, "word_spellings") {
        conn.execute("DELETE FROM word_spellings WHERE word_id=?1", params![id])?;
    }
    if table_exists(conn, "word_usage") {
        conn.execute("DELETE FROM word_usage WHERE word_id=?1", params![id])?;
    }
    conn.execute("DELETE FROM words WHERE id=?1", params![id])?;
    Ok(())
}

// ─── Definitions ──────────────────────────────────────────────────────────────

pub fn save_definition(
    conn: &Connection,
    id: Option<i64>,
    word_id: i64,
    data: &SaveDefinition,
) -> rusqlite::Result<()> {
    let (slots, grammar_code) = split_grammar(data.grammar.as_deref());
    if let Some(did) = id {
        conn.execute(
            "UPDATE definitions SET slots=?1, grammar_code=?2, usage=?3, body=?4, case_tags=?5, updated=datetime('now') WHERE id=?6",
            params![slots, grammar_code, data.usage, data.body, data.tags, did],
        )?;
    } else {
        let pos: i64 = conn
            .query_row(
                "SELECT COALESCE(MAX(position)+1, 0) FROM definitions WHERE word_id=?1",
                params![word_id],
                |r| r.get(0),
            )
            .unwrap_or(0);
        conn.execute(
            "INSERT INTO definitions (word_id, position, slots, grammar_code, usage, body, case_tags, created, updated)
             VALUES (?1,?2,?3,?4,?5,?6,?7, datetime('now'), datetime('now'))",
            params![
                word_id,
                pos,
                slots,
                grammar_code,
                data.usage,
                data.body,
                data.tags
            ],
        )?;
    }
    Ok(())
}

pub fn delete_definition(conn: &Connection, id: i64) -> rusqlite::Result<()> {
    if table_exists(conn, "connect_keys") {
        conn.execute("DELETE FROM connect_keys WHERE \"DID\"=?1", params![id])?;
    }
    conn.execute("DELETE FROM definitions WHERE id=?1", params![id])?;
    Ok(())
}

// ─── Events ──────────────────────────────────────────────────────────────────

pub fn list_events(conn: &Connection) -> rusqlite::Result<Vec<EventItem>> {
    let mut s = conn.prepare(
        "SELECT id, name, NULLIF(date, ''), NULLIF(annotation, ''), NULLIF(suffix, ''), NULLIF(definition, '')
         FROM events ORDER BY id",
    )?;
    let rows = s.query_map([], |r| {
        Ok(EventItem {
            id: r.get(0)?,
            name: r.get(1)?,
            date: r.get(2)?,
            annotation: r.get(3)?,
            suffix: r.get(4)?,
            notes: r.get(5)?,
        })
    })?;
    rows.collect()
}

pub fn save_event(conn: &Connection, id: Option<i64>, data: &SaveEvent) -> rusqlite::Result<i64> {
    let ev_date = data.date.as_deref().unwrap_or("");
    let ev_annotation = data.annotation.as_deref().unwrap_or("");
    let ev_suffix = data.suffix.as_deref().unwrap_or("");
    let ev_notes = data.notes.as_deref().unwrap_or("");
    if let Some(eid) = id {
        conn.execute(
            "UPDATE events SET name=?1, date=?2, annotation=?3, suffix=?4, definition=?5, updated=datetime('now') WHERE id=?6",
            params![data.name, ev_date, ev_annotation, ev_suffix, ev_notes, eid],
        )?;
        Ok(eid)
    } else {
        let next_event_id: i64 = conn
            .query_row(
                "SELECT COALESCE(MAX(event_id), 0) + 1 FROM events",
                [],
                |r| r.get(0),
            )
            .unwrap_or(1);
        conn.execute(
            "INSERT INTO events (event_id, name, date, definition, annotation, suffix, id, created, updated)
             VALUES (?1,?2,?3,?4,?5,?6, (SELECT COALESCE(MAX(id), 0) + 1 FROM events), datetime('now'), datetime('now'))",
            params![next_event_id, data.name, ev_date, ev_notes, ev_annotation, ev_suffix],
        )?;
        Ok(conn.last_insert_rowid())
    }
}

pub fn delete_event(conn: &Connection, id: i64) -> rusqlite::Result<()> {
    let target_event_id: i64 = conn
        .query_row(
            "SELECT event_id FROM events WHERE id = ?1",
            params![id],
            |r| r.get(0),
        )
        .unwrap_or(id);
    let in_use: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM words WHERE event_start = ?1 OR event_end = ?1",
            params![target_event_id],
            |r| r.get(0),
        )
        .unwrap_or(0);
    if in_use > 0 {
        return Err(rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_CONSTRAINT),
            Some(format!("Cannot delete event: used by {in_use} word(s)")),
        ));
    }
    conn.execute("DELETE FROM events WHERE id=?1", params![id])?;
    Ok(())
}

// ─── Types ───────────────────────────────────────────────────────────────────

pub fn list_types(conn: &Connection) -> rusqlite::Result<Vec<TypeItem>> {
    let mut s = conn.prepare(
        "SELECT t.id, t.type, NULLIF(t.type_x, ''), NULLIF(t.\"group\", ''), COUNT(w.id)
         FROM types t LEFT JOIN words w ON w.type=t.id
         GROUP BY t.id ORDER BY t.type",
    )?;
    let rows = s.query_map([], |r| {
        Ok(TypeItem {
            id: r.get(0)?,
            name: r.get(1)?,
            type_x: r.get(2)?,
            group_: r.get(3)?,
            word_count: r.get(4)?,
        })
    })?;
    rows.collect()
}

pub fn save_type(conn: &Connection, id: Option<i64>, data: &SaveType) -> rusqlite::Result<i64> {
    let type_x = data.type_x.as_deref().unwrap_or("");
    let group = data.group_.as_deref().unwrap_or("");
    if let Some(tid) = id {
        conn.execute(
            "UPDATE types SET type=?1, type_x=?2, \"group\"=?3, updated=datetime('now') WHERE id=?4",
            params![data.name, type_x, group, tid],
        )?;
        Ok(tid)
    } else {
        conn.execute(
            "INSERT INTO types (type, type_x, \"group\", parentable, description, id, created, updated)
             VALUES (?1,?2,?3,?4,?5, (SELECT COALESCE(MAX(id), 0) + 1 FROM types), datetime('now'), datetime('now'))",
            params![
                data.name,
                type_x,
                group,
                data.parentable.unwrap_or(true),
                data.description
            ],
        )?;
        Ok(conn.last_insert_rowid())
    }
}

pub fn delete_type(conn: &Connection, id: i64) -> rusqlite::Result<()> {
    let in_use: i64 = conn.query_row(
        "SELECT COUNT(*) FROM words WHERE type=?1",
        params![id],
        |r| r.get(0),
    )?;
    if in_use > 0 {
        return Err(rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_CONSTRAINT),
            Some(format!("Cannot delete type: used by {in_use} word(s)")),
        ));
    }
    conn.execute("DELETE FROM types WHERE id=?1", params![id])?;
    Ok(())
}

// ─── Authors ─────────────────────────────────────────────────────────────────

pub fn list_authors(conn: &Connection) -> rusqlite::Result<Vec<AuthorItem>> {
    let sql = if table_exists(conn, "connect_authors") {
        "SELECT a.id, a.abbreviation, a.full_name, a.notes, COUNT(ca.\"WID\")
         FROM authors a
         LEFT JOIN connect_authors ca ON ca.\"AID\" = a.id
         GROUP BY a.id
         ORDER BY a.abbreviation"
    } else {
        "SELECT id, abbreviation, full_name, notes, 0 FROM authors ORDER BY abbreviation"
    };
    let mut s = conn.prepare(sql)?;
    let rows = s.query_map([], |r| {
        Ok(AuthorItem {
            id: r.get(0)?,
            initials: r.get(1)?,
            full_name: r.get(2)?,
            notes: r.get(3)?,
            word_count: r.get(4)?,
        })
    })?;
    rows.collect()
}

pub fn save_author(conn: &Connection, id: Option<i64>, data: &SaveAuthor) -> rusqlite::Result<i64> {
    if let Some(aid) = id {
        conn.execute(
            "UPDATE authors SET abbreviation=?1, full_name=?2, notes=?3 WHERE id=?4",
            params![data.initials, data.full_name, data.notes, aid],
        )?;
        Ok(aid)
    } else {
        conn.execute(
            "INSERT INTO authors (abbreviation, full_name, notes, id, created, updated)
             VALUES (?1,?2,?3, (SELECT COALESCE(MAX(id), 0) + 1 FROM authors), datetime('now'), datetime('now'))",
            params![data.initials, data.full_name, data.notes],
        )?;
        Ok(conn.last_insert_rowid())
    }
}

pub fn delete_author(conn: &Connection, id: i64) -> rusqlite::Result<()> {
    if table_exists(conn, "connect_authors") {
        conn.execute("DELETE FROM connect_authors WHERE \"AID\"=?1", params![id])?;
    }
    conn.execute("DELETE FROM authors WHERE id=?1", params![id])?;
    Ok(())
}

// ─── Stats & Settings ────────────────────────────────────────────────────────

pub fn get_stats(conn: &Connection) -> rusqlite::Result<AppInfo> {
    let wc: i64 = conn.query_row("SELECT COUNT(*) FROM words", [], |r| r.get(0))?;
    let dc: i64 = conn.query_row("SELECT COUNT(*) FROM definitions", [], |r| r.get(0))?;
    Ok(AppInfo {
        db_path: String::new(),
        word_count: wc,
        definition_count: dc,
    })
}

pub fn get_db_stats(conn: &Connection) -> rusqlite::Result<DbStats> {
    let wc: i64 = conn.query_row("SELECT COUNT(*) FROM words", [], |r| r.get(0))?;
    let dc: i64 = conn.query_row("SELECT COUNT(*) FROM definitions", [], |r| r.get(0))?;
    let ec: i64 = conn.query_row("SELECT COUNT(*) FROM events", [], |r| r.get(0))?;
    let tc: i64 = conn.query_row("SELECT COUNT(*) FROM types", [], |r| r.get(0))?;
    let ac: i64 = conn.query_row("SELECT COUNT(*) FROM authors", [], |r| r.get(0))?;

    let mut axc: i64 = 0;
    if table_exists(conn, "word_affixes") {
        axc = conn
            .query_row("SELECT COUNT(*) FROM word_affixes", [], |r| r.get(0))
            .unwrap_or(0);
    }
    if axc == 0 && table_exists(conn, "connect_words") {
        axc = conn
            .query_row(
                "SELECT COUNT(*) FROM connect_words cw
                 JOIN words w ON w.id = cw.child_id
                 JOIN types t ON t.id = w.type
                 WHERE t.type_x = 'Affix' OR t.type = 'Afx'",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0);
    }

    let mut sc: i64 = 0;
    if table_exists(conn, "word_spellings") {
        sc = conn
            .query_row("SELECT COUNT(*) FROM word_spellings", [], |r| r.get(0))
            .unwrap_or(0);
    }
    if sc == 0 {
        sc = wc;
    }

    let settings = list_settings(conn)?;
    Ok(DbStats {
        db_path: String::new(),
        word_count: wc,
        definition_count: dc,
        event_count: ec,
        type_count: tc,
        author_count: ac,
        affix_count: axc,
        spelling_count: sc,
        settings,
    })
}

pub fn list_settings(conn: &Connection) -> rusqlite::Result<Vec<SettingItem>> {
    if !table_exists(conn, "settings") {
        return Ok(vec![]);
    }
    if column_exists(conn, "settings", "db_release") {
        let row: Option<(String, String, String, String)> = conn
            .query_row(
                "SELECT CAST(date AS TEXT), CAST(db_version AS TEXT),
                        CAST(last_word_id AS TEXT), CAST(db_release AS TEXT)
                 FROM settings ORDER BY id DESC LIMIT 1",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )
            .ok();
        if let Some((date, db_version, last_word_id, db_release)) = row {
            return Ok(vec![
                SettingItem {
                    key: "date".to_string(),
                    value: date,
                },
                SettingItem {
                    key: "db_version".to_string(),
                    value: db_version,
                },
                SettingItem {
                    key: "last_word_id".to_string(),
                    value: last_word_id,
                },
                SettingItem {
                    key: "db_release".to_string(),
                    value: db_release,
                },
            ]);
        }
        return Ok(vec![]);
    }
    if column_exists(conn, "settings", "key") {
        let mut s = conn.prepare("SELECT key, value FROM settings ORDER BY key")?;
        let rows = s.query_map([], |r| {
            Ok(SettingItem {
                key: r.get(0)?,
                value: r.get(1)?,
            })
        })?;
        return rows.collect();
    }
    Ok(vec![])
}

pub fn upsert_setting(conn: &Connection, key: &str, value: &str) -> rusqlite::Result<()> {
    if column_exists(conn, "settings", "db_release") {
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM settings", [], |r| r.get(0))
            .unwrap_or(0);
        if count == 0 {
            conn.execute(
                "INSERT INTO settings (date, db_version, last_word_id, db_release, id, created, updated)
                 VALUES (datetime('now'), 1, 0, '', 1, datetime('now'), datetime('now'))",
                [],
            )?;
        }
        match key {
            "date" => {
                conn.execute(
                    "UPDATE settings SET date=?1, updated=datetime('now') WHERE id=(SELECT MAX(id) FROM settings)",
                    params![value],
                )?;
            }
            "db_version" => {
                let v: i64 = value.parse().unwrap_or(1);
                conn.execute(
                    "UPDATE settings SET db_version=?1, updated=datetime('now') WHERE id=(SELECT MAX(id) FROM settings)",
                    params![v],
                )?;
            }
            "last_word_id" => {
                let v: i64 = value.parse().unwrap_or(0);
                conn.execute(
                    "UPDATE settings SET last_word_id=?1, updated=datetime('now') WHERE id=(SELECT MAX(id) FROM settings)",
                    params![v],
                )?;
            }
            "db_release" => {
                conn.execute(
                    "UPDATE settings SET db_release=?1, updated=datetime('now') WHERE id=(SELECT MAX(id) FROM settings)",
                    params![value],
                )?;
            }
            _ => {}
        }
        return Ok(());
    }
    if column_exists(conn, "settings", "key") {
        conn.execute(
            "INSERT INTO settings(key,value) VALUES(?1,?2) ON CONFLICT(key) DO UPDATE SET value=excluded.value",
            params![key, value],
        )?;
    }
    Ok(())
}

// ─── FTS5 full-text search ────────────────────────────────────────────────────

pub fn init_fts(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        "
        CREATE VIRTUAL TABLE IF NOT EXISTS def_fts
        USING fts5(
            body,
            content='definitions',
            content_rowid='id',
            tokenize='unicode61 remove_diacritics 1'
        );

        CREATE VIRTUAL TABLE IF NOT EXISTS def_kw_fts
        USING fts5(
            keywords,
            tokenize='unicode61 remove_diacritics 1'
        );
    ",
    )
}

/// Extract text from between «» markers in a definition body.
fn extract_keywords(body: &str) -> String {
    let mut out = String::new();
    let mut chars = body.char_indices().peekable();
    while let Some((_, c)) = chars.next() {
        if c == '\u{AB}' {
            let start_byte = chars.peek().map_or(body.len(), |&(i, _)| i);
            let mut end_byte = start_byte;
            for (i, c2) in chars.by_ref() {
                if c2 == '\u{BB}' {
                    end_byte = i;
                    break;
                }
            }
            if end_byte > start_byte {
                if !out.is_empty() {
                    out.push(' ');
                }
                out.push_str(&body[start_byte..end_byte]);
            }
        }
    }
    out
}

/// Rebuild both FTS indexes from all definitions (call after bulk import).
pub fn rebuild_fts(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        "
        DROP TABLE IF EXISTS def_fts;
        CREATE VIRTUAL TABLE def_fts
        USING fts5(
            body,
            content='definitions',
            content_rowid='id',
            tokenize='unicode61 remove_diacritics 1'
        );
        INSERT INTO def_fts(def_fts) VALUES('rebuild');
    ",
    )?;

    conn.execute_batch(
        "
        DROP TABLE IF EXISTS def_kw_fts;
        CREATE VIRTUAL TABLE def_kw_fts
        USING fts5(
            keywords,
            tokenize='unicode61 remove_diacritics 1'
        );
    ",
    )?;

    let mut sel = conn.prepare("SELECT id, body FROM definitions WHERE body LIKE '%\u{AB}%'")?;
    let rows: Vec<(i64, String)> = sel
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
        .filter_map(std::result::Result::ok)
        .collect();

    let mut ins = conn.prepare("INSERT INTO def_kw_fts(rowid, keywords) VALUES(?1, ?2)")?;
    for (id, body) in rows {
        let kw = extract_keywords(&body);
        if !kw.is_empty() {
            ins.execute(params![id, kw])?;
        }
    }
    Ok(())
}

/// Compact the database by running VACUUM.
pub fn vacuum_db(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch("VACUUM")
}

/// Update FTS when a single definition is saved or deleted (`body = ""`).
pub fn fts_update(conn: &Connection, def_id: i64, body: &str) -> rusqlite::Result<()> {
    if !table_exists(conn, "def_fts") {
        return Ok(());
    }
    if let Ok(old_body) = conn.query_row(
        "SELECT body FROM definitions WHERE id=?1",
        params![def_id],
        |r| r.get::<_, String>(0),
    ) {
        conn.execute(
            "INSERT INTO def_fts(def_fts, rowid, body) VALUES('delete', ?1, ?2)",
            params![def_id, old_body],
        )
        .ok();
    }
    conn.execute(
        "INSERT INTO def_fts(def_fts, rowid, body) VALUES('delete', ?1, '')",
        params![def_id],
    )
    .ok();
    if !body.is_empty() {
        conn.execute(
            "INSERT INTO def_fts(rowid, body) VALUES(?1, ?2)",
            params![def_id, body],
        )?;
    }

    if table_exists(conn, "def_kw_fts") {
        conn.execute(
            "INSERT INTO def_kw_fts(def_kw_fts, rowid, keywords) VALUES('delete', ?1, '')",
            params![def_id],
        )
        .ok();
        let kw = extract_keywords(body);
        if !kw.is_empty() {
            conn.execute(
                "INSERT INTO def_kw_fts(rowid, keywords) VALUES(?1, ?2)",
                params![def_id, kw],
            )?;
        }
    }
    Ok(())
}

/// FTS5-based E→L search over full definition bodies.
pub fn search_english_fts(
    conn: &Connection,
    q: &str,
    limit: i64,
) -> rusqlite::Result<Vec<ELResult>> {
    let fts_query = build_fts_query(q);
    let sql = "
        WITH ranked AS (
            SELECT
                w.id            AS word_id,
                w.name          AS word_name,
                t.type          AS type_name,
                NULLIF(COALESCE(CAST(d.slots AS TEXT), '') || COALESCE(d.grammar_code, ''), '') AS grammar,
                snippet(def_fts, 0, '«', '»', '…', 10) AS snip,
                fts.rank        AS rank
            FROM def_fts fts
            JOIN definitions d ON d.id  = fts.rowid
            JOIN words       w ON w.id  = d.word_id
            LEFT JOIN types  t ON t.id  = w.type
            WHERE def_fts MATCH ?1
            ORDER BY rank
            LIMIT ?2
        ),
        agg AS (
            SELECT
                word_id, word_name, type_name,
                MIN(grammar)    AS grammar,
                MIN(snip)       AS snippet,
                COUNT(*)        AS match_count,
                MIN(rank)       AS best_rank
            FROM ranked
            GROUP BY word_id
        )
        SELECT word_id, word_name, type_name, grammar, snippet, match_count
        FROM agg
        ORDER BY best_rank, word_name
        LIMIT ?2
    ";
    let mut stmt = conn.prepare(sql)?;
    let rows = stmt.query_map(params![fts_query, limit], |r| {
        Ok(ELResult {
            word_id: r.get(0)?,
            word_name: r.get(1)?,
            type_name: r.get(2)?,
            grammar: r.get(3)?,
            snippet: r.get::<_, String>(4).unwrap_or_default(),
            match_count: r.get(5)?,
        })
    })?;
    rows.collect()
}

/// FTS5 keyword-only E→L search (searches only «keyword» terms in definitions).
pub fn search_english_keywords_fts(
    conn: &Connection,
    q: &str,
    limit: i64,
) -> rusqlite::Result<Vec<ELResult>> {
    let fts_query = build_fts_query(q);
    let sql = "
        WITH ranked AS (
            SELECT
                w.id            AS word_id,
                w.name          AS word_name,
                t.type          AS type_name,
                NULLIF(COALESCE(CAST(d.slots AS TEXT), '') || COALESCE(d.grammar_code, ''), '') AS grammar,
                snippet(def_fts, 0, '«', '»', '…', 10) AS snip,
                kw.rank         AS rank
            FROM def_kw_fts kw
            JOIN definitions d ON d.id  = kw.rowid
            JOIN words       w ON w.id  = d.word_id
            LEFT JOIN types  t ON t.id  = w.type
            LEFT JOIN def_fts ON def_fts.rowid = d.id
            WHERE def_kw_fts MATCH ?1
            ORDER BY rank
            LIMIT ?2
        ),
        agg AS (
            SELECT
                word_id, word_name, type_name,
                MIN(grammar)    AS grammar,
                COALESCE(MIN(snip), '')  AS snippet,
                COUNT(*)        AS match_count,
                MIN(rank)       AS best_rank
            FROM ranked
            GROUP BY word_id
        )
        SELECT word_id, word_name, type_name, grammar, snippet, match_count
        FROM agg
        ORDER BY best_rank, word_name
        LIMIT ?2
    ";
    let mut stmt = conn.prepare(sql)?;
    let rows = stmt.query_map(params![fts_query, limit], |r| {
        Ok(ELResult {
            word_id: r.get(0)?,
            word_name: r.get(1)?,
            type_name: r.get(2)?,
            grammar: r.get(3)?,
            snippet: r.get::<_, String>(4).unwrap_or_default(),
            match_count: r.get(5)?,
        })
    })?;
    rows.collect()
}

/// LIKE-based E→L fallback (no FTS5 required, slower).
pub fn search_english_like(
    conn: &Connection,
    q: &str,
    limit: i64,
) -> rusqlite::Result<Vec<ELResult>> {
    let pat = format!("%{}%", q.trim().to_lowercase());
    let sql = "
        WITH matched AS (
            SELECT
                w.id            AS word_id,
                w.name          AS word_name,
                t.type          AS type_name,
                NULLIF(COALESCE(CAST(d.slots AS TEXT), '') || COALESCE(d.grammar_code, ''), '') AS grammar,
                d.body          AS body,
                COUNT(*) OVER (PARTITION BY w.id) AS match_count
            FROM definitions d
            JOIN words       w ON w.id = d.word_id
            LEFT JOIN types  t ON t.id = w.type
            WHERE LOWER(d.body) LIKE ?1
            ORDER BY w.name
            LIMIT ?2
        )
        SELECT word_id, word_name, type_name, grammar, body, match_count
        FROM matched
        GROUP BY word_id
        ORDER BY word_name
    ";
    let mut stmt = conn.prepare(sql)?;
    let rows = stmt.query_map(params![pat, limit * 3], |r| {
        let body: String = r.get(4)?;
        Ok(ELResult {
            word_id: r.get(0)?,
            word_name: r.get(1)?,
            type_name: r.get(2)?,
            grammar: r.get(3)?,
            snippet: body,
            match_count: r.get(5)?,
        })
    })?;
    let mut results: Vec<ELResult> = rows.filter_map(std::result::Result::ok).collect();
    let lim: usize = limit.try_into().unwrap_or(0);
    results.truncate(lim);
    Ok(results)
}

/// LIKE-based keyword-only fallback: matches only text inside «» markers.
pub fn search_english_keywords_like(
    conn: &Connection,
    q: &str,
    limit: i64,
) -> rusqlite::Result<Vec<ELResult>> {
    let q_clean = q.trim().to_lowercase();
    let pat = format!("%\u{AB}{q_clean}%\u{BB}%");
    let sql = "
        WITH matched AS (
            SELECT
                w.id            AS word_id,
                w.name          AS word_name,
                t.type          AS type_name,
                NULLIF(COALESCE(CAST(d.slots AS TEXT), '') || COALESCE(d.grammar_code, ''), '') AS grammar,
                d.body          AS body,
                COUNT(*) OVER (PARTITION BY w.id) AS match_count
            FROM definitions d
            JOIN words       w ON w.id = d.word_id
            LEFT JOIN types  t ON t.id = w.type
            WHERE LOWER(d.body) LIKE ?1
            ORDER BY w.name
            LIMIT ?2
        )
        SELECT word_id, word_name, type_name, grammar, body, match_count
        FROM matched
        GROUP BY word_id
        ORDER BY word_name
    ";
    let mut stmt = conn.prepare(sql)?;
    let rows = stmt.query_map(params![pat, limit * 3], |r| {
        let body: String = r.get(4)?;
        Ok(ELResult {
            word_id: r.get(0)?,
            word_name: r.get(1)?,
            type_name: r.get(2)?,
            grammar: r.get(3)?,
            snippet: body,
            match_count: r.get(5)?,
        })
    })?;
    let mut results: Vec<ELResult> = rows.filter_map(std::result::Result::ok).collect();
    let lim: usize = limit.try_into().unwrap_or(0);
    results.truncate(lim);
    Ok(results)
}

/// Sanitise a user query string into a valid FTS5 query.
fn build_fts_query(q: &str) -> String {
    let q_clean = q.trim().replace('"', "\"\"");
    if q_clean.contains(' ') {
        format!("\"{q_clean}\"")
    } else {
        format!("{q_clean}*")
    }
}

/// Check if BOTH FTS indexes are populated.
pub fn fts_is_ready(conn: &Connection) -> bool {
    let fts_ok = table_exists(conn, "def_fts")
        && conn
            .query_row("SELECT COUNT(*) FROM def_fts", [], |r| r.get::<_, i64>(0))
            .unwrap_or(0)
            > 0;

    let kw_ok = table_exists(conn, "def_kw_fts");

    fts_ok && kw_ok
}

/// Words added (`event_start`) and removed (`event_end`) for a given event.
pub fn get_event_words(
    conn: &Connection,
    event_id: i64,
) -> rusqlite::Result<(Vec<String>, Vec<String>)> {
    let target_event_id: i64 = conn
        .query_row(
            "SELECT event_id FROM events WHERE id = ?1",
            params![event_id],
            |r| r.get(0),
        )
        .unwrap_or(event_id);

    let mut s =
        conn.prepare("SELECT w.name FROM words w WHERE w.event_start = ?1 ORDER BY w.name")?;
    let added: Vec<String> = s
        .query_map(params![target_event_id], |r| r.get(0))?
        .filter_map(std::result::Result::ok)
        .collect();

    let mut s =
        conn.prepare("SELECT w.name FROM words w WHERE w.event_end = ?1 ORDER BY w.name")?;
    let removed: Vec<String> = s
        .query_map(params![target_event_id], |r| r.get(0))?
        .filter_map(std::result::Result::ok)
        .collect();

    Ok((added, removed))
}
