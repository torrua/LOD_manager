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
        "PRAGMA foreign_keys=ON;

        CREATE TABLE IF NOT EXISTS types (
            id      INTEGER PRIMARY KEY AUTOINCREMENT,
            type    TEXT NOT NULL UNIQUE,
            type_x  TEXT,
            group_  TEXT,
            parentable BOOLEAN DEFAULT TRUE,
            description TEXT,
            created   DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated   DATETIME DEFAULT CURRENT_TIMESTAMP
        );

        CREATE TABLE IF NOT EXISTS authors (
            id        INTEGER PRIMARY KEY AUTOINCREMENT,
            abbreviation  TEXT NOT NULL UNIQUE,
            full_name TEXT,
            notes     TEXT,
            created   DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated   DATETIME DEFAULT CURRENT_TIMESTAMP
        );

        CREATE TABLE IF NOT EXISTS events (
            event_id  INTEGER NOT NULL UNIQUE,
            name       TEXT NOT NULL,
            date       TEXT,
            definition TEXT,
            annotation TEXT,
            suffix     TEXT,
            id         INTEGER PRIMARY KEY AUTOINCREMENT,
            created    DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated    DATETIME DEFAULT CURRENT_TIMESTAMP
        );

        CREATE TABLE IF NOT EXISTS words (
            id              INTEGER PRIMARY KEY AUTOINCREMENT,
            name            TEXT NOT NULL,
            type            INTEGER NOT NULL REFERENCES types(id),
            origin          TEXT,
            origin_x        TEXT,
            match_          TEXT,
            rank            TEXT,
            year            TEXT,
            notes           TEXT,
            id_old          INTEGER NOT NULL,
            TID_old         INTEGER,
            event_start     INTEGER NOT NULL REFERENCES events(event_id),
            event_end       INTEGER REFERENCES events(event_id),
            created         DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated         DATETIME DEFAULT CURRENT_TIMESTAMP,
            UNIQUE(name, type)
        );
        CREATE INDEX IF NOT EXISTS idx_words_name       ON words(name);
        CREATE INDEX IF NOT EXISTS idx_words_name_lower ON words(LOWER(name));
        CREATE INDEX IF NOT EXISTS idx_words_type      ON words(type);
        CREATE INDEX IF NOT EXISTS idx_words_ev_start   ON words(event_start);
        CREATE INDEX IF NOT EXISTS idx_words_ev_end     ON words(event_end);

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

        CREATE TABLE IF NOT EXISTS word_usage (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            word_id     INTEGER NOT NULL REFERENCES words(id) ON DELETE CASCADE,
            used_in_word TEXT NOT NULL,
            created     DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
        );
        CREATE INDEX IF NOT EXISTS idx_word_usage_word_id ON word_usage(word_id);
        CREATE INDEX IF NOT EXISTS idx_word_usage_used_in ON word_usage(used_in_word);

        CREATE TABLE IF NOT EXISTS settings (
            date        DATETIME NOT NULL,
            db_version  INTEGER NOT NULL,
            last_word_id INTEGER NOT NULL,
            db_release  TEXT NOT NULL,
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            created     DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated     DATETIME DEFAULT CURRENT_TIMESTAMP,
            UNIQUE(date)
        );

        CREATE TABLE IF NOT EXISTS definitions (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            word_id     INTEGER NOT NULL REFERENCES words(id) ON DELETE CASCADE,
            position    INTEGER NOT NULL DEFAULT 0,
            body        TEXT NOT NULL DEFAULT '',
            usage       TEXT,
            grammar_code TEXT,
            slots       INTEGER,
            case_tags   TEXT,
            language    TEXT,
            notes       TEXT,
            created     DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated     DATETIME DEFAULT CURRENT_TIMESTAMP,
            UNIQUE(word_id, position)
        );
        CREATE INDEX IF NOT EXISTS idx_def_word_pos ON definitions(word_id, position);

        CREATE TABLE IF NOT EXISTS connect_words (
            parent_id INTEGER NOT NULL REFERENCES words(id) ON DELETE CASCADE,
            child_id  INTEGER NOT NULL REFERENCES words(id) ON DELETE CASCADE,
            PRIMARY KEY (parent_id, child_id)
        );
        CREATE INDEX IF NOT EXISTS index_parent_id ON connect_words (parent_id);
        CREATE INDEX IF NOT EXISTS index_child_id  ON connect_words (child_id);

        INSERT OR IGNORE INTO events (event_id, name, date, definition, annotation, suffix) VALUES (1, 'Start', '', '', '', '');"
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
        
        -- Migration: Add connect_words table if it doesn't exist
        CREATE TABLE IF NOT EXISTS connect_words (
            parent_id INTEGER NOT NULL REFERENCES words(id) ON DELETE CASCADE,
            child_id  INTEGER NOT NULL REFERENCES words(id) ON DELETE CASCADE,
            PRIMARY KEY (parent_id, child_id)
        );
        CREATE INDEX IF NOT EXISTS index_parent_id ON connect_words (parent_id);
        CREATE INDEX IF NOT EXISTS index_child_id  ON connect_words (child_id);

        -- Migration: Add word_usage table if it doesn't exist (for databases created before usedin support)
        CREATE TABLE IF NOT EXISTS word_usage (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            word_id     INTEGER NOT NULL REFERENCES words(id) ON DELETE CASCADE,
            used_in_word TEXT NOT NULL,
            created     DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
        );
        CREATE INDEX IF NOT EXISTS idx_word_usage_word_id ON word_usage(word_id);
        CREATE INDEX IF NOT EXISTS idx_word_usage_used_in ON word_usage(used_in_word);

        -- Clean up pipe separators mistakenly stored as word entries
        DELETE FROM word_usage WHERE used_in_word = '|';
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
    println!(
        "list_words called with q='{}', type_filter='{}', event_id={:?}",
        q, type_filter, event_id
    );
    let pattern = if q.contains('*') || q.contains('?') {
        q.to_lowercase().replace('*', "%").replace('?', "_")
    } else if q.is_empty() {
        "%".to_string()
    } else {
        // Prefix search — can use idx_words_name_lower
        format!("{}%", q.to_lowercase())
    };

    // Single query: optional type filter is handled by (?2 = '' OR t.type = ?2).
    // Optional event filter: ?3 IS NULL skips the clause entirely.
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
        ORDER BY LOWER(w.name)
    ";
    println!("list_words: executing SQL query");
    let mut stmt = conn.prepare(sql)?;
    let result: Vec<WordListItem> = stmt
        .query_map(params![pattern, type_filter, event_id], map_wli)?
        .collect::<Result<Vec<_>, _>>()?;
    println!("list_words: collected {} results", result.len());
    Ok(result)
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
        "SELECT w.id, w.name, w.origin, w.origin_x, w.match_, w.rank, w.year, w.notes,
                w.id_old, w.TID_old, w.type, w.event_start, w.event_end,
                t.type as type_name, es.name as event_start_name, ee.name as event_end_name
         FROM words w
         LEFT JOIN types t ON t.id = w.type
         LEFT JOIN events es ON es.event_id = w.event_start
         LEFT JOIN events ee ON ee.event_id = w.event_end
         WHERE w.id = ?1",
        params![id],
        |r| {
            Ok(WordDetail {
                id: r.get(0)?,                // w.id
                name: r.get(1)?,              // w.name
                type_name: r.get(13)?,        // t.type as type_name
                type_id: r.get(10)?,          // w.type (INTEGER)
                source: None,                 // source not in database
                origin: r.get(2)?,            // w.origin
                origin_x: r.get(3)?,          // w.origin_x
                match_: r.get(4)?,            // w.match_
                rank: r.get(5)?,              // w.rank
                year: r.get(6)?,              // w.year
                notes: r.get(7)?,             // w.notes
                event_start_name: r.get(14)?, // es.name as event_start_name
                event_end_name: r.get(15)?,   // ee.name as event_end_name
                affixes: vec![],
                spellings: vec![],
                definitions: vec![],
                used_in: vec![],
                children: vec![],
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
                        'grammar_code',  grammar_code,
                        'usage',    usage,
                        'body',     body,
                        'case_tags',     case_tags
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

    // ── 4. Used-in: direct relationships from word_usage table ───────────────────
    let mut s = conn.prepare(
        "SELECT DISTINCT used_in_word FROM word_usage
         WHERE word_id = ?1
         ORDER BY used_in_word",
    )?;
    word.used_in = s
        .query_map(params![id], |r| r.get(0))?
        .filter_map(std::result::Result::ok)
        .collect();
    
    // ── 5. Children: words that list this word as a parent in connect_words ──────
    let mut s = conn.prepare(
        "SELECT w.name FROM words w
         JOIN connect_words cw ON cw.child_id = w.id
         WHERE cw.parent_id = ?1
         ORDER BY w.name",
    )?;
    word.children = s
        .query_map(params![id], |r| r.get(0))?
        .filter_map(std::result::Result::ok)
        .collect();

    Ok(word)
}

pub fn save_word(conn: &Connection, id: Option<i64>, data: &SaveWord) -> rusqlite::Result<i64> {
    let type_id: Option<i64> = if let Some(tn) = &data.type_name {
        conn.query_row("SELECT id FROM types WHERE type=?1", params![tn], |r| {
            r.get(0)
        })
        .ok()
    } else {
        None
    };

    let ev_start: Option<i64> = if let Some(en) = &data.event_start {
        conn.query_row(
            "SELECT id FROM events WHERE event_id=?1",
            params![en],
            |r| r.get(0),
        )
        .ok()
    } else {
        None
    };

    let ev_end: Option<i64> = if let Some(en) = &data.event_end {
        conn.query_row(
            "SELECT id FROM events WHERE event_id=?1",
            params![en],
            |r| r.get(0),
        )
        .ok()
    } else {
        None
    };

    let word_id = if let Some(wid) = id {
        conn.execute(
            "UPDATE words SET name=?1, type=?2, match_=?3, rank=?4, year=?5,
             origin=?6, origin_x=?7, notes=?8, id_old=?9, event_start=?10, event_end=?11
             WHERE id=?12",
            params![
                data.name,
                type_id,
                data.match_,
                data.rank,
                data.year,
                data.origin,
                data.origin_x,
                data.notes,
                data.id_old.unwrap_or(0),
                ev_start,
                ev_end,
                wid,
            ],
        )?;
        wid
    } else {
        conn.execute(
            "INSERT INTO words (name, type, match_, rank, year, origin, origin_x, notes, id_old, event_start, event_end, created, updated) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11, datetime('now'), datetime('now'))",
            params![
                data.name,
                type_id,
                data.match_,
                data.rank,
                data.year,
                data.origin,
                data.origin_x,
                data.notes,
                data.id_old.unwrap_or(0),
                ev_start,
                ev_end,
            ],
        )?;
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
            "UPDATE definitions SET grammar_code=?1, usage=?2, body=?3, case_tags=?4 WHERE id=?5",
            params![
                data.grammar_code,
                data.usage,
                data.body,
                data.case_tags,
                did
            ],
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
            "INSERT INTO definitions (word_id, position, grammar_code, usage, body, case_tags, created, updated) VALUES (?1,?2,?3,?4,?5,?6, datetime('now'), datetime('now'))",
            params![word_id, pos, data.grammar_code, data.usage, data.body, data.case_tags])?;
    }
    Ok(())
}

pub fn delete_definition(conn: &Connection, id: i64) -> rusqlite::Result<()> {
    conn.execute("DELETE FROM definitions WHERE id=?1", params![id])?;
    Ok(())
}

// ─── Events ──────────────────────────────────────────────────────────────────

pub fn list_events(conn: &Connection) -> rusqlite::Result<Vec<EventItem>> {
    let mut s = conn
        .prepare("SELECT id, name, date, annotation, suffix, definition FROM events ORDER BY id")?;
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
            "UPDATE events SET name=?1, date=?2, annotation=?3, suffix=?4, definition=?5 WHERE id=?6",
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
            "INSERT INTO events (event_id, name, date, definition, annotation, suffix, id, created, updated) VALUES (?1,?2,?3,?4,?5,?6, (SELECT COALESCE(MAX(id), 0) + 1 FROM events), datetime('now'), datetime('now'))",
            params![
                (conn.query_row("SELECT COALESCE(MAX(event_id), 0) + 1 FROM events", [], |r| r.get(0)).unwrap_or(1)),
                data.name,
                data.date,
                data.notes,
                data.annotation,
                data.suffix,
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
        "SELECT t.id, t.type, t.type_x, t.group_, COUNT(w.id)
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
    if let Some(tid) = id {
        conn.execute(
            "UPDATE types SET name=?1, type_x=?2, group_=?3 WHERE id=?4",
            params![data.name, data.type_x, data.group_, tid],
        )?;
        Ok(tid)
    } else {
        conn.execute(
            "INSERT INTO types (type, type_x, group_, parentable, description, id, created, updated) VALUES (?1,?2,?3,?4,?5, (SELECT COALESCE(MAX(id), 0) + 1 FROM types), datetime('now'), datetime('now'))",
            params![data.name, data.type_x, data.group_, data.parentable, data.description],
        )?;
        Ok(conn.last_insert_rowid())
    }
}

pub fn delete_type(conn: &Connection, id: i64) -> rusqlite::Result<()> {
    conn.execute("UPDATE words SET type=NULL WHERE type=?1", params![id])?;
    conn.execute("DELETE FROM types WHERE id=?1", params![id])?;
    Ok(())
}

// ─── Authors ─────────────────────────────────────────────────────────────────

pub fn list_authors(conn: &Connection) -> rusqlite::Result<Vec<AuthorItem>> {
    let mut s = conn.prepare(
        "SELECT id, abbreviation, full_name, notes, 0 FROM authors ORDER BY abbreviation",
    )?;
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
            "INSERT INTO authors (abbreviation, full_name, notes, id, created, updated) VALUES (?1,?2,?3, (SELECT COALESCE(MAX(id), 0) + 1 FROM authors), datetime('now'), datetime('now'))",
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
                t.type          AS type_name,
                d.grammar_code  AS grammar,
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
                d.grammar_code  AS grammar,
                -- Use the full body for the snippet (more readable than keywords-only)
                snippet(def_fts, 0, '«', '»', '…', 10) AS snip,
                kw.rank         AS rank
            FROM def_kw_fts kw
            JOIN definitions d ON d.id  = kw.rowid
            JOIN words       w ON w.id  = d.word_id
            LEFT JOIN types  t ON t.id  = w.type
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
                t.type          AS type_name,
                d.grammar_code  AS grammar,
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
    // Match definitions where the query appears as the start of a «keyword».
    // Pattern: «<query>…»  (prefix match inside keyword markers).
    let q_clean = q.trim().to_lowercase();
    let pat = format!("%\u{AB}{q_clean}%\u{BB}%");
    let sql = "
        WITH matched AS (
            SELECT
                w.id            AS word_id,
                w.name          AS word_name,
                t.type          AS type_name,
                d.grammar_code  AS grammar,
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
