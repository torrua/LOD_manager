//! Database operations for the Loglan Online Dictionary.
//!
//! This module provides all data access functions: schema management,
//! CRUD operations for words/definitions/events/types/authors, FTS5 search,
//! and migrations.
//!
//! # Key patterns
//! - All functions take `&Connection` — callers manage the connection lifecycle
//! - Migrations are idempotent (safe to call repeatedly)
//! - FTS5 uses dual virtual tables: `def_fts` (full body) and `def_kw_fts` (keywords)
use crate::models::*;
use rusqlite::{Connection, params};
use std::convert::TryInto;

pub fn init_schema(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        "
        PRAGMA foreign_keys=ON;

        CREATE TABLE IF NOT EXISTS types (
            id      INTEGER PRIMARY KEY AUTOINCREMENT,
            name    TEXT NOT NULL UNIQUE,
            type_x  TEXT,
            group_  TEXT
        );

        CREATE TABLE IF NOT EXISTS authors (
            id        INTEGER PRIMARY KEY AUTOINCREMENT,
            initials  TEXT NOT NULL UNIQUE,
            full_name TEXT,
            notes     TEXT
        );

        CREATE TABLE IF NOT EXISTS events (
            id         INTEGER PRIMARY KEY AUTOINCREMENT,
            name       TEXT NOT NULL UNIQUE,
            date       TEXT,
            annotation TEXT,
            suffix     TEXT,
            notes      TEXT
        );

        CREATE TABLE IF NOT EXISTS words (
            id              INTEGER PRIMARY KEY AUTOINCREMENT,
            name            TEXT NOT NULL,
            type_id         INTEGER REFERENCES types(id),
            source          TEXT,
            year            TEXT,
            rank            TEXT,
            match_          TEXT,
            origin          TEXT,
            origin_x        TEXT,
            notes           TEXT,
            event_start_id  INTEGER REFERENCES events(id),
            event_end_id    INTEGER REFERENCES events(id),
            UNIQUE(name, type_id)
        );
        CREATE INDEX IF NOT EXISTS idx_words_name       ON words(name);
        CREATE INDEX IF NOT EXISTS idx_words_name_lower ON words(LOWER(name));
        CREATE INDEX IF NOT EXISTS idx_words_type_id    ON words(type_id);
        CREATE INDEX IF NOT EXISTS idx_words_ev_start   ON words(event_start_id);
        CREATE INDEX IF NOT EXISTS idx_words_ev_end     ON words(event_end_id);

        CREATE TABLE IF NOT EXISTS word_spellings (
            id      INTEGER PRIMARY KEY AUTOINCREMENT,
            word_id INTEGER NOT NULL REFERENCES words(id) ON DELETE CASCADE,
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

        CREATE TABLE IF NOT EXISTS settings (
            key   TEXT PRIMARY KEY,
            value TEXT NOT NULL DEFAULT ''
        );

        CREATE TABLE IF NOT EXISTS definitions (
            id       INTEGER PRIMARY KEY AUTOINCREMENT,
            word_id  INTEGER NOT NULL REFERENCES words(id) ON DELETE CASCADE,
            position INTEGER NOT NULL DEFAULT 0,
            grammar  TEXT,
            usage    TEXT,
            body     TEXT NOT NULL DEFAULT '',
            tags     TEXT,
            UNIQUE(word_id, position)
        );
        -- Covering index: WHERE word_id=? ORDER BY position — no separate sort step.
        CREATE INDEX IF NOT EXISTS idx_def_word_pos ON definitions(word_id, position);

        INSERT OR IGNORE INTO events (name) VALUES ('Start');
    ",
    )
}

/// Add any indexes that may be missing in databases created before they were
/// added to `init_schema`.  Safe to call on every open (all are IF NOT EXISTS).
pub fn add_missing_indexes(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        "
        CREATE INDEX IF NOT EXISTS idx_word_spellings_word_id ON word_spellings(word_id);
        CREATE INDEX IF NOT EXISTS idx_word_affixes_word_id   ON word_affixes(word_id);
        CREATE INDEX IF NOT EXISTS idx_word_affixes_affix     ON word_affixes(affix);
        CREATE INDEX IF NOT EXISTS idx_words_type_id          ON words(type_id);
        CREATE INDEX IF NOT EXISTS idx_words_ev_start         ON words(event_start_id);
        CREATE INDEX IF NOT EXISTS idx_words_ev_end           ON words(event_end_id);
        CREATE INDEX IF NOT EXISTS idx_def_word_pos           ON definitions(word_id, position);
        ",
    )
}

/// One-time migration: ensure words table has UNIQUE(name, `type_id`) and NOT a
/// standalone UNIQUE(name). `SQLite` can't drop constraints directly — we use
/// CREATE TABLE + INSERT + DROP + RENAME if the old unique index exists.
/// Safe to call multiple times (checks flag first).
pub fn migrate_words_unique_if_needed(conn: &Connection) -> rusqlite::Result<()> {
    let already: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM settings WHERE key='words_unique_migrated'",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);
    if already > 0 {
        return Ok(());
    }

    // Check if a standalone unique index on words(name) exists
    let has_bad_unique: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master
         WHERE type='index' AND tbl_name='words'
         AND sql LIKE '%UNIQUE%' AND sql NOT LIKE '%(name%type_id%)'",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);

    // Also check if the table itself was CREATE'd with UNIQUE(name) inline
    let table_sql: String = conn
        .query_row(
            "SELECT sql FROM sqlite_master WHERE type='table' AND name='words'",
            [],
            |r| r.get(0),
        )
        .unwrap_or_default();

    let needs_rebuild = has_bad_unique > 0
        || (table_sql.contains("UNIQUE")
            && !table_sql.contains("name, type_id")
            && !table_sql.contains("name,type_id"));

    if needs_rebuild {
        conn.execute_batch(
            "
            PRAGMA foreign_keys=OFF;

            CREATE TABLE words_new (
                id              INTEGER PRIMARY KEY AUTOINCREMENT,
                name            TEXT NOT NULL,
                type_id         INTEGER REFERENCES types(id),
                source          TEXT,
                year            TEXT,
                rank            TEXT,
                match_          TEXT,
                origin          TEXT,
                origin_x        TEXT,
                notes           TEXT,
                event_start_id  INTEGER REFERENCES events(id),
                event_end_id    INTEGER REFERENCES events(id),
                UNIQUE(name, type_id)
            );

            INSERT OR IGNORE INTO words_new
                SELECT id, name, type_id, source, year, rank, match_,
                       origin, origin_x, notes, event_start_id, event_end_id
                FROM words;

            DROP TABLE words;
            ALTER TABLE words_new RENAME TO words;

            CREATE INDEX IF NOT EXISTS idx_words_name       ON words(name);
            CREATE INDEX IF NOT EXISTS idx_words_name_lower ON words(LOWER(name));

            PRAGMA foreign_keys=ON;
        ",
        )?;
    }

    conn.execute(
        "INSERT OR IGNORE INTO settings(key,value) VALUES('words_unique_migrated','1')",
        [],
    )?;
    Ok(())
}

/// One-time migration: swap annotation ↔ notes in events table.
/// Needed because earlier import had the columns in wrong order.
/// Runs only if settings flag '`ev_col_migrated`' is not set.
pub fn migrate_event_columns_if_needed(conn: &Connection) -> rusqlite::Result<()> {
    let already: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM settings WHERE key='ev_col_migrated'",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);
    if already > 0 {
        return Ok(());
    }

    conn.execute_batch(
        "
        UPDATE events
        SET annotation = notes,
            notes      = annotation
        WHERE annotation IS NOT NULL OR notes IS NOT NULL;

        INSERT OR IGNORE INTO settings(key,value) VALUES('ev_col_migrated','1');
    ",
    )?;
    Ok(())
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
///
/// Uses a single parameterised query across all filter combinations instead of
/// four format! branches, so the prepared statement is compiled once per connection.
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
        // Prefix search — can use idx_words_name_lower
        format!("{}%", q.to_lowercase())
    };

    // Single query: optional type filter is handled by (?2 = '' OR t.name = ?2).
    // Optional event filter: ?3 IS NULL skips the clause entirely.
    let sql = "
        SELECT w.id, w.name, t.name,
               (SELECT COUNT(*) FROM definitions d WHERE d.word_id = w.id)
        FROM words w
        LEFT JOIN types t ON t.id = w.type_id
        WHERE LOWER(w.name) LIKE ?1
          AND (?2 = '' OR t.name = ?2)
          AND (?3 IS NULL
               OR (w.event_start_id <= ?3
                   AND (w.event_end_id IS NULL OR w.event_end_id > ?3)))
        ORDER BY LOWER(w.name)
    ";
    conn.prepare(sql)?
        .query_map(params![pattern, type_filter, event_id], map_wli)?
        .collect()
}

/// Fetch a word with all its related data (affixes, spellings, definitions, used-in).
///
/// Uses an optimized 4-query strategy to avoid N+1:
/// 1. Main word row with type/event joins
/// 2. Affixes + spellings via `GROUP_CONCAT` (single round-trip)
/// 3. Definitions via `json_group_array` (safe — no separator collision)
/// 4. Used-in: words whose name contains this word's affixes (EXISTS with index)
pub fn get_word(conn: &Connection, id: i64) -> rusqlite::Result<WordDetail> {
    // ── 1. Main word row ──────────────────────────────────────────────────────
    let mut word: WordDetail = conn.query_row(
        "SELECT w.id, w.name, t.name, w.type_id,
                w.source, w.year, w.rank, w.match_,
                w.origin, w.origin_x, w.notes,
                es.name, ee.name
         FROM words w
         LEFT JOIN types t  ON t.id  = w.type_id
         LEFT JOIN events es ON es.id = w.event_start_id
         LEFT JOIN events ee ON ee.id = w.event_end_id
         WHERE w.id = ?1",
        params![id],
        |r| {
            Ok(WordDetail {
                id: r.get(0)?,
                name: r.get(1)?,
                type_name: r.get(2)?,
                type_id: r.get(3)?,
                source: r.get(4)?,
                year: r.get(5)?,
                rank: r.get(6)?,
                match_: r.get(7)?,
                origin: r.get(8)?,
                origin_x: r.get(9)?,
                notes: r.get(10)?,
                event_start_name: r.get(11)?,
                event_end_name: r.get(12)?,
                affixes: vec![],
                spellings: vec![],
                definitions: vec![],
                used_in: vec![],
            })
        },
    )?;

    // ── 2. Affixes + spellings in one round-trip ───────────────────────────────
    let (affixes_str, spellings_str): (String, String) = conn.query_row(
        "SELECT
            COALESCE((SELECT GROUP_CONCAT(affix,   x'1f') FROM word_affixes   WHERE word_id=?1), ''),
            COALESCE((SELECT GROUP_CONCAT(spelling, x'1f') FROM word_spellings WHERE word_id=?1), '')",
        params![id],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;

    word.affixes = if affixes_str.is_empty() {
        vec![]
    } else {
        affixes_str.split('\x1f').map(str::to_string).collect()
    };
    word.spellings = if spellings_str.is_empty() {
        vec![]
    } else {
        spellings_str.split('\x1f').map(str::to_string).collect()
    };

    // ── 3. Definitions via json_group_array (safe — no separator collision) ───
    // idx_def_word_pos covers (word_id, position) so the ORDER BY is free.
    let json_str: String = conn
        .query_row(
            "SELECT COALESCE(
                json_group_array(
                    json_object(
                        'id',       id,
                        'position', position,
                        'grammar',  grammar,
                        'usage',    usage,
                        'body',     body,
                        'tags',     tags
                    )
                ),
                '[]'
            )
            FROM definitions
            WHERE word_id = ?1
            ORDER BY position",
            params![id],
            |r| r.get(0),
        )
        .unwrap_or_else(|_| "[]".to_string());

    word.definitions = serde_json::from_str::<Vec<Definition>>(&json_str).unwrap_or_default();

    // ── 4. Used-in: words whose name contains one of this word's affixes ───────
    // EXISTS with idx_word_affixes_word_id makes the inner scan O(affixes).
    let mut s = conn.prepare(
        "SELECT DISTINCT w.name FROM words w
         WHERE w.id != ?1
           AND EXISTS (
               SELECT 1 FROM word_affixes wa
               WHERE wa.word_id = ?1
                 AND LOWER(w.name) LIKE '%' || LOWER(wa.affix) || '%'
           )
         ORDER BY w.name
         LIMIT 100",
    )?;
    word.used_in = s
        .query_map(params![id], |r| r.get(0))?
        .filter_map(std::result::Result::ok)
        .collect();

    Ok(word)
}

pub fn save_word(conn: &Connection, id: Option<i64>, data: &SaveWord) -> rusqlite::Result<i64> {
    let type_id: Option<i64> = if let Some(tn) = &data.type_name {
        conn.query_row("SELECT id FROM types WHERE name=?1", params![tn], |r| {
            r.get(0)
        })
        .ok()
    } else {
        None
    };

    let ev_start_id: Option<i64> = if let Some(en) = &data.event_start {
        conn.query_row("SELECT id FROM events WHERE name=?1", params![en], |r| {
            r.get(0)
        })
        .ok()
    } else {
        None
    };

    let ev_end_id: Option<i64> = if let Some(en) = &data.event_end {
        conn.query_row("SELECT id FROM events WHERE name=?1", params![en], |r| {
            r.get(0)
        })
        .ok()
    } else {
        None
    };

    let word_id = if let Some(wid) = id {
        conn.execute(
            "UPDATE words SET name=?1, type_id=?2, source=?3, year=?4, rank=?5,
             match_=?6, origin=?7, origin_x=?8, notes=?9, event_start_id=?10, event_end_id=?11
             WHERE id=?12",
            params![
                data.name,
                type_id,
                data.source,
                data.year,
                data.rank,
                data.match_,
                data.origin,
                data.origin_x,
                data.notes,
                ev_start_id,
                ev_end_id,
                wid
            ],
        )?;
        wid
    } else {
        conn.execute(
            "INSERT INTO words (name, type_id, source, year, rank, match_, origin, origin_x, notes, event_start_id, event_end_id)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",
            params![data.name, type_id, data.source, data.year, data.rank,
                    data.match_, data.origin, data.origin_x, data.notes,
                    ev_start_id, ev_end_id])?;
        conn.last_insert_rowid()
    };

    // sync affixes
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

    // sync spellings
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

    Ok(word_id)
}

pub fn delete_word(conn: &Connection, id: i64) -> rusqlite::Result<()> {
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
    if let Some(did) = id {
        conn.execute(
            "UPDATE definitions SET grammar=?1, usage=?2, body=?3, tags=?4 WHERE id=?5",
            params![data.grammar, data.usage, data.body, data.tags, did],
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
            "INSERT INTO definitions (word_id, position, grammar, usage, body, tags) VALUES (?1,?2,?3,?4,?5,?6)",
            params![word_id, pos, data.grammar, data.usage, data.body, data.tags])?;
    }
    Ok(())
}

pub fn delete_definition(conn: &Connection, id: i64) -> rusqlite::Result<()> {
    conn.execute("DELETE FROM definitions WHERE id=?1", params![id])?;
    Ok(())
}

// ─── Events ──────────────────────────────────────────────────────────────────

pub fn list_events(conn: &Connection) -> rusqlite::Result<Vec<EventItem>> {
    let mut s =
        conn.prepare("SELECT id, name, date, annotation, suffix, notes FROM events ORDER BY id")?;
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
    if let Some(eid) = id {
        conn.execute(
            "UPDATE events SET name=?1, date=?2, annotation=?3, suffix=?4, notes=?5 WHERE id=?6",
            params![
                data.name,
                data.date,
                data.annotation,
                data.suffix,
                data.notes,
                eid
            ],
        )?;
        Ok(eid)
    } else {
        conn.execute(
            "INSERT INTO events (name, date, annotation, suffix, notes) VALUES (?1,?2,?3,?4,?5)",
            params![
                data.name,
                data.date,
                data.annotation,
                data.suffix,
                data.notes
            ],
        )?;
        Ok(conn.last_insert_rowid())
    }
}

pub fn delete_event(conn: &Connection, id: i64) -> rusqlite::Result<()> {
    conn.execute("DELETE FROM events WHERE id=?1", params![id])?;
    Ok(())
}

// ─── Types ───────────────────────────────────────────────────────────────────

pub fn list_types(conn: &Connection) -> rusqlite::Result<Vec<TypeItem>> {
    let mut s = conn.prepare(
        "SELECT t.id, t.name, t.type_x, t.group_, COUNT(w.id)
         FROM types t LEFT JOIN words w ON w.type_id=t.id
         GROUP BY t.id ORDER BY t.name",
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
    if let Some(tid) = id {
        conn.execute(
            "UPDATE types SET name=?1, type_x=?2, group_=?3 WHERE id=?4",
            params![data.name, data.type_x, data.group_, tid],
        )?;
        Ok(tid)
    } else {
        conn.execute(
            "INSERT INTO types (name, type_x, group_) VALUES (?1,?2,?3)",
            params![data.name, data.type_x, data.group_],
        )?;
        Ok(conn.last_insert_rowid())
    }
}

pub fn delete_type(conn: &Connection, id: i64) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE words SET type_id=NULL WHERE type_id=?1",
        params![id],
    )?;
    conn.execute("DELETE FROM types WHERE id=?1", params![id])?;
    Ok(())
}

// ─── Authors ─────────────────────────────────────────────────────────────────

pub fn list_authors(conn: &Connection) -> rusqlite::Result<Vec<AuthorItem>> {
    let mut s =
        conn.prepare("SELECT id, initials, full_name, notes, 0 FROM authors ORDER BY initials")?;
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
            "UPDATE authors SET initials=?1, full_name=?2, notes=?3 WHERE id=?4",
            params![data.initials, data.full_name, data.notes, aid],
        )?;
        Ok(aid)
    } else {
        conn.execute(
            "INSERT INTO authors (initials, full_name, notes) VALUES (?1,?2,?3)",
            params![data.initials, data.full_name, data.notes],
        )?;
        Ok(conn.last_insert_rowid())
    }
}

pub fn delete_author(conn: &Connection, id: i64) -> rusqlite::Result<()> {
    conn.execute("DELETE FROM authors WHERE id=?1", params![id])?;
    Ok(())
}

// ─── Stats ────────────────────────────────────────────────────────────────────

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
    let mut s = conn.prepare(
        "SELECT
            (SELECT COUNT(*) FROM words) AS wc,
            (SELECT COUNT(*) FROM definitions) AS dc,
            (SELECT COUNT(*) FROM events) AS ec,
            (SELECT COUNT(*) FROM types) AS tc,
            (SELECT COUNT(*) FROM authors) AS ac,
            (SELECT COUNT(*) FROM word_affixes) AS axc,
            (SELECT COUNT(*) FROM word_spellings) AS sc",
    )?;
    let (wc, dc, ec, tc, ac, axc, sc): (i64, i64, i64, i64, i64, i64, i64) =
        s.query_row([], |r| {
            Ok((
                r.get(0)?,
                r.get(1)?,
                r.get(2)?,
                r.get(3)?,
                r.get(4)?,
                r.get(5)?,
                r.get(6)?,
            ))
        })?;
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
    // table may not exist yet in old DBs
    let ok: bool = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='settings'",
            [],
            |r| r.get::<_, i64>(0),
        )
        .unwrap_or(0)
        > 0;
    if !ok {
        return Ok(vec![]);
    }
    let mut s = conn.prepare("SELECT key, value FROM settings ORDER BY key")?;
    let rows = s.query_map([], |r| {
        Ok(SettingItem {
            key: r.get(0)?,
            value: r.get(1)?,
        })
    })?;
    rows.collect()
}

#[allow(dead_code)]
pub fn upsert_setting(conn: &Connection, key: &str, value: &str) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO settings(key,value) VALUES(?1,?2) ON CONFLICT(key) DO UPDATE SET value=excluded.value",
        params![key, value])?;
    Ok(())
}

// ─── FTS5 full-text search ────────────────────────────────────────────────────

pub fn init_fts(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        "
        -- Full-body FTS: used by default E→L search.
        CREATE VIRTUAL TABLE IF NOT EXISTS def_fts
        USING fts5(
            body,
            content='definitions',
            content_rowid='id',
            tokenize='unicode61 remove_diacritics 1'
        );

        -- Keyword-only FTS: indexes text extracted from «keyword» markers.
        -- Standalone table (not content-linked) so we populate it manually.
        CREATE VIRTUAL TABLE IF NOT EXISTS def_kw_fts
        USING fts5(
            keywords,
            tokenize='unicode61 remove_diacritics 1'
        );
    ",
    )
}

/// Extract text from between «» markers in a definition body.
/// Returns a space-joined string of all keyword tokens, ready for FTS indexing.
fn extract_keywords(body: &str) -> String {
    let mut out = String::new();
    let mut chars = body.char_indices().peekable();
    while let Some((_, c)) = chars.next() {
        if c == '\u{AB}' {
            // opening «
            let start_byte = chars.peek().map_or(body.len(), |&(i, _)| i);
            let mut end_byte = start_byte;
            for (i, c2) in chars.by_ref() {
                if c2 == '\u{BB}' {
                    // closing »
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
    // ── 1. Full-body FTS ──────────────────────────────────────────────────────
    // DROP + CREATE is the only reliable way to recover from corrupt / out-of-sync
    // FTS5 shadow tables (which cause "database disk image is malformed").
    // After a clean CREATE the 'rebuild' command repopulates from the content table.
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

    // ── 2. Keyword FTS (standalone) ───────────────────────────────────────────
    // Same approach: drop/create guarantees a clean state.
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
/// Reclaims freed space from deleted rows and defragments the database file.
pub fn vacuum_db(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch("VACUUM")
}

/// Update FTS when a single definition is saved.
#[allow(dead_code)]
pub fn fts_update(conn: &Connection, def_id: i64, body: &str) -> rusqlite::Result<()> {
    // Full-body FTS5 content table: delete old, insert new.
    conn.execute(
        "INSERT INTO def_fts(def_fts, rowid, body) VALUES('delete', ?1, '')",
        params![def_id],
    )
    .ok();
    conn.execute(
        "INSERT INTO def_fts(rowid, body) VALUES(?1, ?2)",
        params![def_id, body],
    )?;

    // Keyword FTS: replace.
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
                t.name          AS type_name,
                d.grammar       AS grammar,
                snippet(def_fts, 0, '«', '»', '…', 10) AS snip,
                fts.rank        AS rank
            FROM def_fts fts
            JOIN definitions d ON d.id  = fts.rowid
            JOIN words       w ON w.id  = d.word_id
            LEFT JOIN types  t ON t.id  = w.type_id
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
                t.name          AS type_name,
                d.grammar       AS grammar,
                -- Use the full body for the snippet (more readable than keywords-only)
                snippet(def_fts, 0, '«', '»', '…', 10) AS snip,
                kw.rank         AS rank
            FROM def_kw_fts kw
            JOIN definitions d ON d.id  = kw.rowid
            JOIN words       w ON w.id  = d.word_id
            LEFT JOIN types  t ON t.id  = w.type_id
            -- Also join def_fts so we can call snippet() on the body column
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
                t.name          AS type_name,
                d.grammar       AS grammar,
                d.body          AS body,
                COUNT(*) OVER (PARTITION BY w.id) AS match_count
            FROM definitions d
            JOIN words       w ON w.id = d.word_id
            LEFT JOIN types  t ON t.id = w.type_id
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
    // Match definitions where the query appears as the start of a «keyword».
    // Pattern: «<query>…»  (prefix match inside keyword markers).
    let q_clean = q.trim().to_lowercase();
    let pat = format!("%\u{AB}{q_clean}%\u{BB}%");
    let sql = "
        WITH matched AS (
            SELECT
                w.id            AS word_id,
                w.name          AS word_name,
                t.name          AS type_name,
                d.grammar       AS grammar,
                d.body          AS body,
                COUNT(*) OVER (PARTITION BY w.id) AS match_count
            FROM definitions d
            JOIN words       w ON w.id = d.word_id
            LEFT JOIN types  t ON t.id = w.type_id
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
        format!("\"{q_clean}\"") // phrase search
    } else {
        format!("{q_clean}*") // prefix search
    }
}

/// Check if BOTH FTS indexes are populated.
pub fn fts_is_ready(conn: &Connection) -> bool {
    let fts_ok = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='def_fts'",
            [],
            |r| r.get::<_, i64>(0),
        )
        .unwrap_or(0)
        > 0
        && conn
            .query_row("SELECT COUNT(*) FROM def_fts", [], |r| r.get::<_, i64>(0))
            .unwrap_or(0)
            > 0;

    let kw_ok = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='def_kw_fts'",
            [],
            |r| r.get::<_, i64>(0),
        )
        .unwrap_or(0)
        > 0;

    fts_ok && kw_ok
}

/// Words added (`event_start`) and removed (`event_end`) for a given event.
pub fn get_event_words(
    conn: &Connection,
    event_id: i64,
) -> rusqlite::Result<(Vec<String>, Vec<String>)> {
    let mut s =
        conn.prepare("SELECT w.name FROM words w WHERE w.event_start_id = ?1 ORDER BY w.name")?;
    let added: Vec<String> = s
        .query_map(params![event_id], |r| r.get(0))?
        .filter_map(std::result::Result::ok)
        .collect();

    let mut s =
        conn.prepare("SELECT w.name FROM words w WHERE w.event_end_id = ?1 ORDER BY w.name")?;
    let removed: Vec<String> = s
        .query_map(params![event_id], |r| r.get(0))?
        .filter_map(std::result::Result::ok)
        .collect();

    Ok((added, removed))
}
