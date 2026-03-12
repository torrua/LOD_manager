use crate::models::*;
use rusqlite::{params, Connection};
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
        CREATE INDEX IF NOT EXISTS idx_words_name ON words(name);
        CREATE INDEX IF NOT EXISTS idx_words_name_lower ON words(LOWER(name));

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
        CREATE INDEX IF NOT EXISTS idx_word_affixes_affix ON word_affixes(affix);

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
        CREATE INDEX IF NOT EXISTS idx_def_word ON definitions(word_id);

        INSERT OR IGNORE INTO events (name) VALUES ('Start');
    ",
    )
}

/// Add missing indexes for better performance
pub fn add_missing_indexes(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        "
        CREATE INDEX IF NOT EXISTS idx_word_spellings_word_id ON word_spellings(word_id);
        CREATE INDEX IF NOT EXISTS idx_word_affixes_word_id ON word_affixes(word_id);
        CREATE INDEX IF NOT EXISTS idx_word_affixes_affix ON word_affixes(affix);
        CREATE INDEX IF NOT EXISTS idx_words_type_id ON words(type_id);
        CREATE INDEX IF NOT EXISTS idx_words_event_start_id ON words(event_start_id);
        CREATE INDEX IF NOT EXISTS idx_words_event_end_id ON words(event_end_id);
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

            CREATE INDEX IF NOT EXISTS idx_words_name ON words(name);
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
        format!("%{}%", q.to_lowercase())
    };

    // Event filter: word appeared at or before this event AND has not ended before it.
    // event_start_id <= event_id  AND  (event_end_id IS NULL OR event_end_id > event_id)
    let ev_clause = match event_id {
        Some(_) => {
            " AND w.event_start_id <= ?2 AND (w.event_end_id IS NULL OR w.event_end_id > ?2)"
        }
        None => "",
    };

    // Use parameterized type filter to avoid any SQL injection risk.
    // Four combinations: (search, event_id) × (type_filter present/absent).
    let sql_base = "SELECT w.id, w.name, t.name,
                    (SELECT COUNT(*) FROM definitions d WHERE d.word_id=w.id)
                    FROM words w
                    LEFT JOIN types t ON t.id=w.type_id";

    let rows: rusqlite::Result<Vec<WordListItem>> = match (event_id, type_filter.is_empty()) {
        (None, true) => {
            let sql = format!("{sql_base} WHERE LOWER(w.name) LIKE ?1 ORDER BY LOWER(w.name)");
            conn.prepare(&sql)?
                .query_map(params![pattern], map_wli)?
                .collect()
        }
        (None, false) => {
            let sql = format!(
                "{sql_base} WHERE LOWER(w.name) LIKE ?1 AND t.name=?2 ORDER BY LOWER(w.name)"
            );
            conn.prepare(&sql)?
                .query_map(params![pattern, type_filter], map_wli)?
                .collect()
        }
        (Some(eid), true) => {
            let sql =
                format!("{sql_base} WHERE LOWER(w.name) LIKE ?1{ev_clause} ORDER BY LOWER(w.name)");
            conn.prepare(&sql)?
                .query_map(params![pattern, eid], map_wli)?
                .collect()
        }
        (Some(eid), false) => {
            let sql = format!("{sql_base} WHERE LOWER(w.name) LIKE ?1{ev_clause} AND t.name=?3 ORDER BY LOWER(w.name)");
            conn.prepare(&sql)?
                .query_map(params![pattern, eid, type_filter], map_wli)?
                .collect()
        }
    };
    rows
}

pub fn get_word(conn: &Connection, id: i64) -> rusqlite::Result<WordDetail> {
    // First get the main word data
    let mut word: WordDetail = conn.query_row(
        "SELECT w.id, w.name, t.name, w.type_id,
                w.source, w.year, w.rank, w.match_,
                w.origin, w.origin_x, w.notes,
                es.name, ee.name
         FROM words w
         LEFT JOIN types t ON t.id=w.type_id
         LEFT JOIN events es ON es.id=w.event_start_id
         LEFT JOIN events ee ON ee.id=w.event_end_id
         WHERE w.id=?1",
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

    // Step 1: Combine affixes and spellings in one query
    let mut stmt = conn.prepare(
        "SELECT 
            (SELECT GROUP_CONCAT(affix, '\x1f') FROM word_affixes WHERE word_id=?1) as affixes,
            (SELECT GROUP_CONCAT(spelling, '\x1f') FROM word_spellings WHERE word_id=?1) as spellings"
    )?;

    let (affixes_str, spellings_str) = stmt.query_row(params![id], |r| {
        let affixes: Option<String> = r.get(0)?;
        let spellings: Option<String> = r.get(1)?;
        Ok((affixes.unwrap_or_default(), spellings.unwrap_or_default()))
    })?;

    word.affixes = if !affixes_str.is_empty() {
        affixes_str.split('\x1f').map(|s| s.to_string()).collect()
    } else {
        Vec::new()
    };

    word.spellings = if !spellings_str.is_empty() {
        spellings_str.split('\x1f').map(|s| s.to_string()).collect()
    } else {
        Vec::new()
    };

    // Step 2: Get definitions in separate query
    let definitions_str: String = conn
        .query_row(
            "SELECT GROUP_CONCAT(
            id || '\x1e' || position || '\x1e' || 
            COALESCE(grammar, '') || '\x1e' || COALESCE(usage, '') || '\x1e' || 
            COALESCE(body, '') || '\x1e' || COALESCE(tags, ''), '\x1d'
         ) FROM definitions WHERE word_id=?1 ORDER BY position",
            params![id],
            |r| r.get(0),
        )
        .unwrap_or_default();

    word.definitions = if !definitions_str.is_empty() {
        definitions_str
            .split('\x1d')
            .filter_map(|def_str| {
                let parts: Vec<&str> = def_str.split('\x1e').collect();
                if parts.len() >= 6 {
                    Some(Definition {
                        id: parts[0].parse().unwrap_or(0),
                        position: parts[1].parse().unwrap_or(0),
                        grammar: if parts[2].is_empty() {
                            None
                        } else {
                            Some(parts[2].to_string())
                        },
                        usage: if parts[3].is_empty() {
                            None
                        } else {
                            Some(parts[3].to_string())
                        },
                        body: parts[4].to_string(),
                        tags: if parts[5].is_empty() {
                            None
                        } else {
                            Some(parts[5].to_string())
                        },
                    })
                } else {
                    None
                }
            })
            .collect()
    } else {
        Vec::new()
    };

    // Get used_in words (still separate query as it's complex)
    let mut s = conn.prepare(
        "SELECT DISTINCT w.name FROM words w
         WHERE w.id != ?1
           AND EXISTS (
             SELECT 1 FROM word_affixes wa
             WHERE wa.word_id = ?1
               AND LOWER(w.name) LIKE '%'||LOWER(wa.affix)||'%'
           )
         ORDER BY w.name LIMIT 100",
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
    let wc: i64 = conn.query_row("SELECT COUNT(*) FROM words", [], |r| r.get(0))?;
    let dc: i64 = conn.query_row("SELECT COUNT(*) FROM definitions", [], |r| r.get(0))?;
    let ec: i64 = conn.query_row("SELECT COUNT(*) FROM events", [], |r| r.get(0))?;
    let tc: i64 = conn.query_row("SELECT COUNT(*) FROM types", [], |r| r.get(0))?;
    let ac: i64 = conn.query_row("SELECT COUNT(*) FROM authors", [], |r| r.get(0))?;
    let axc: i64 = conn.query_row("SELECT COUNT(*) FROM word_affixes", [], |r| r.get(0))?;
    let sc: i64 = conn.query_row("SELECT COUNT(*) FROM word_spellings", [], |r| r.get(0))?;
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
        CREATE VIRTUAL TABLE IF NOT EXISTS def_fts
        USING fts5(
            body,
            content='definitions',
            content_rowid='id',
            tokenize='unicode61 remove_diacritics 1'
        );
    ",
    )
}

/// Rebuild FTS index from all definitions (call after import).
pub fn rebuild_fts(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        "
        DELETE FROM def_fts;
        INSERT INTO def_fts(rowid, body) SELECT id, body FROM definitions;
    ",
    )
}

/// Update FTS when a single definition is saved.
#[allow(dead_code)]
pub fn fts_update(conn: &Connection, def_id: i64, body: &str) -> rusqlite::Result<()> {
    // FTS5 content table: delete old, insert new
    conn.execute(
        "INSERT INTO def_fts(def_fts, rowid, body) VALUES('delete', ?1, '')",
        params![def_id],
    )
    .ok();
    conn.execute(
        "INSERT INTO def_fts(rowid, body) VALUES(?1, ?2)",
        params![def_id, body],
    )?;
    Ok(())
}

/// FTS5-based E→L search. Returns one row per matched definition,
/// grouped by word on the frontend (or here if we pre-aggregate).
pub fn search_english_fts(
    conn: &Connection,
    q: &str,
    limit: i64,
) -> rusqlite::Result<Vec<ELResult>> {
    // Sanitise query: escape special FTS5 chars, add * for prefix matching
    let q_clean = q.trim().replace('"', "\"\"");
    let fts_query = if q_clean.contains(' ') {
        // phrase search
        format!("\"{q_clean}\"")
    } else {
        // prefix
        format!("{q_clean}*")
    };

    let sql = "
        WITH ranked AS (
            SELECT
                w.id            AS word_id,
                w.name          AS word_name,
                t.name          AS type_name,
                d.grammar       AS grammar,
                -- FTS5 snippet: 10 tokens, bold markers
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
    let q_pat = pat.clone();
    let mut stmt = conn.prepare(sql)?;
    let rows = stmt.query_map(params![q_pat, limit * 3], |r| {
        let body: String = r.get(4)?;
        Ok(ELResult {
            word_id: r.get(0)?,
            word_name: r.get(1)?,
            type_name: r.get(2)?,
            grammar: r.get(3)?,
            snippet: body, // full body; frontend truncates
            match_count: r.get(5)?,
        })
    })?;
    let mut results: Vec<ELResult> = rows.filter_map(std::result::Result::ok).collect();
    let lim: usize = limit.try_into().unwrap_or(0);
    results.truncate(lim);
    Ok(results)
}

/// Check if FTS index is populated.
pub fn fts_is_ready(conn: &Connection) -> bool {
    conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='def_fts'",
        [],
        |r| r.get::<_, i64>(0),
    )
    .unwrap_or(0)
        > 0
        && conn
            .query_row("SELECT COUNT(*) FROM def_fts", [], |r| r.get::<_, i64>(0))
            .unwrap_or(0)
            > 0
}

/// Words added (`event_start`) and removed (`event_end`) for a given event.
pub fn get_event_words(
    conn: &Connection,
    event_id: i64,
) -> rusqlite::Result<(Vec<String>, Vec<String>)> {
    let mut s = conn.prepare(
        "SELECT w.name, t.name FROM words w
         LEFT JOIN types t ON t.id = w.type_id
         WHERE w.event_start_id = ?1 ORDER BY w.name",
    )?;
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
