"""REST API route handlers for LOD Manager."""

import asyncio
import os
import re
import sqlite3
from typing import Any, Dict, List, Optional, Tuple

from quart import current_app, jsonify, request

from . import api_bp
from .auth import optional_auth, require_admin
from .serializers import (
    denormalize_word_fields,
    serialize_author,
    serialize_db_stats,
    serialize_definition,
    serialize_el_result,
    serialize_event,
    serialize_type,
    serialize_word_detail,
    serialize_word_list_item,
)


def get_db_path() -> str:
    """Resolve database file path from config, environment, or common locations."""
    if "DATABASE_PATH" in current_app.config:
        return current_app.config["DATABASE_PATH"]
    if os.environ.get("DATABASE_PATH"):
        return os.environ["DATABASE_PATH"]

    candidates = [
        "export.db",
        "lod.db",
        "loglan.db",
        "../export.db",
        "../../export.db",
        "/app/data/export.db",
    ]
    for c in candidates:
        if os.path.exists(c):
            return os.path.abspath(c)

    return os.path.abspath("export.db")


def _run_query_sync(db_path: str, sql: str, params: Tuple = ()) -> List[Dict[str, Any]]:
    """Execute read-only SQL query in thread pool using standard library sqlite3."""
    conn = sqlite3.connect(f"file:{db_path}?mode=ro", uri=True)
    conn.row_factory = sqlite3.Row
    try:
        cursor = conn.cursor()
        cursor.execute(sql, params)
        rows = cursor.fetchall()
        return [dict(r) for r in rows]
    finally:
        conn.close()


def _run_write_sync(db_path: str, sql: str, params: Tuple = ()) -> int:
    """Execute write SQL statement in thread pool."""
    conn = sqlite3.connect(db_path)
    try:
        cursor = conn.cursor()
        cursor.execute(sql, params)
        conn.commit()
        return cursor.lastrowid
    finally:
        conn.close()


async def run_query(sql: str, params: Tuple = ()) -> List[Dict[str, Any]]:
    """Async wrapper for SQL queries."""
    db_path = get_db_path()
    return await asyncio.to_thread(_run_query_sync, db_path, sql, params)


async def run_write(sql: str, params: Tuple = ()) -> int:
    """Async wrapper for SQL writes."""
    db_path = get_db_path()
    return await asyncio.to_thread(_run_write_sync, db_path, sql, params)


# ─── Health check ─────────────────────────────────────────────────────────────
@api_bp.route("/health", methods=["GET"])
async def health_check():
    return jsonify({"status": "ok", "app": "LOD Manager API"})


# ─── Words List ───────────────────────────────────────────────────────────────
@api_bp.route("/words", methods=["GET"])
@optional_auth
async def get_words():
    """
    List words matching optional query filters.
    Query parameters:
      - q: word name prefix or wildcard (* / ?)
      - typeFilter: type name or '__g__' prefixed group
      - eventId: event ID filter
      - limit: max words to return (default: 10000)
      - offset: pagination offset (default: 0)
    """
    q = request.args.get("q", "").strip()
    type_filter = request.args.get("typeFilter", "").strip()
    event_id_raw = request.args.get("eventId")
    limit = min(int(request.args.get("limit", 10000)), 20000)
    offset = max(int(request.args.get("offset", 0)), 0)

    where_clauses = ["1=1"]
    params: List[Any] = []

    if q:
        if "*" in q or "?" in q:
            # SQLite LIKE wildcard translation
            like_pat = q.lower().replace("*", "%").replace("?", "_")
            where_clauses.append("LOWER(w.name) LIKE ?")
            params.append(like_pat)
        else:
            where_clauses.append("LOWER(w.name) LIKE ?")
            params.append(f"{q.lower()}%")

    if type_filter:
        if type_filter.startswith("__g__"):
            group = type_filter[5:]
            where_clauses.append('t."group" = ?')
            params.append(group)
        else:
            where_clauses.append("t.type = ?")
            params.append(type_filter)

    if event_id_raw is not None and event_id_raw != "" and event_id_raw.lower() != "null":
        try:
            ev_input_id = int(event_id_raw)
            # Resolve event_id column from events table
            ev_rows = await run_query("SELECT event_id FROM events WHERE id = ?", (ev_input_id,))
            target_ev_id = ev_rows[0]["event_id"] if ev_rows else ev_input_id

            where_clauses.append(
                """(w.event_start IS NULL OR w.event_start <= ?)
                   AND (w.event_end IS NULL OR w.event_end > ?)"""
            )
            params.extend([target_ev_id, target_ev_id])
        except ValueError:
            pass

    where_sql = " AND ".join(where_clauses)
    sql = f"""
        SELECT
            w.id,
            w.name,
            t.type as type_name,
            (SELECT COUNT(*) FROM definitions d WHERE d.word_id = w.id) as def_count
        FROM words w
        LEFT JOIN types t ON w.type = t.id
        WHERE {where_sql}
        ORDER BY LOWER(w.name),
                 CASE WHEN w.event_end IS NULL THEN 0 ELSE 1 END,
                 w.event_start DESC,
                 w.id DESC
        LIMIT ? OFFSET ?
    """
    params.extend([limit, offset])

    try:
        rows = await run_query(sql, tuple(params))
        items = [serialize_word_list_item(r) for r in rows]
        return jsonify(items)
    except Exception as e:
        return jsonify({"error": str(e)}), 500


# ─── Word Detail ──────────────────────────────────────────────────────────────
@api_bp.route("/words/<int:word_id>", methods=["GET"])
@optional_auth
async def get_word_detail(word_id: int):
    """Retrieve complete word details including definitions, affixes, and relationships."""
    word_sql = """
        SELECT
            w.id,
            w.name,
            t.type as type_name,
            t.id as type_id,
            w.year,
            w.rank,
            w."match" as match_,
            w.origin,
            w.origin_x,
            w.notes,
            es.name as event_start_name,
            ee.name as event_end_name,
            COALESCE((
                SELECT GROUP_CONCAT(a.abbreviation, '/')
                FROM connect_authors ca
                JOIN authors a ON a.id = ca."AID"
                WHERE ca."WID" = w.id
                ORDER BY a.abbreviation
            ), '') as authors_csv
        FROM words w
        LEFT JOIN types t ON w.type = t.id
        LEFT JOIN events es ON es.event_id = w.event_start
        LEFT JOIN events ee ON ee.event_id = w.event_end
        WHERE w.id = ?
    """
    words = await run_query(word_sql, (word_id,))
    if not words:
        return jsonify({"error": "Word not found"}), 404

    word = words[0]

    # Definitions
    def_sql = """
        SELECT id, position, grammar_code, slots, usage, body, case_tags as tags
        FROM definitions
        WHERE word_id = ?
        ORDER BY position ASC
    """
    defs = await run_query(def_sql, (word_id,))
    word["definitions"] = defs

    # Connected children / affixes (strip hyphens)
    affix_sql = """
        SELECT DISTINCT REPLACE(w2.name, '-', '') as name
        FROM connect_words cw
        JOIN words w2 ON cw.child_id = w2.id
        LEFT JOIN types t2 ON w2.type = t2.id
        WHERE cw.parent_id = ? AND (t2.type_x = 'Affix' OR t2.type = 'Afx')
        ORDER BY 1
    """
    affixes = await run_query(affix_sql, (word_id,))
    word["affixes"] = [r["name"] for r in affixes]

    # Spellings
    spelling_sql = """
        SELECT w2.name
        FROM words w1
        JOIN words w2 ON w2.id_old = w1.id_old AND w2.id != w1.id
        WHERE w1.id = ? AND w1.id_old > 0
        ORDER BY w2.id
    """
    spellings = await run_query(spelling_sql, (word_id,))
    word["spellings"] = [r["name"] for r in spellings]

    # Complexes using this word (used_in)
    used_in_sql = """
        SELECT DISTINCT w2.name
        FROM connect_words cw
        JOIN words w2 ON cw.child_id = w2.id
        LEFT JOIN types t2 ON w2.type = t2.id
        WHERE cw.parent_id = ? AND t2."group" = 'Cpx'
        ORDER BY w2.name
    """
    used_in = await run_query(used_in_sql, (word_id,))
    word["used_in"] = [r["name"] for r in used_in]

    # Parents
    parent_sql = """
        SELECT DISTINCT w2.name
        FROM connect_words cw
        JOIN words w2 ON cw.parent_id = w2.id
        WHERE cw.child_id = ?
        ORDER BY 1
    """
    parents = await run_query(parent_sql, (word_id,))
    word["parents"] = [r["name"] for r in parents]

    # Children (excluding affixes and complexes)
    children_sql = """
        SELECT DISTINCT w2.name
        FROM connect_words cw
        JOIN words w2 ON cw.child_id = w2.id
        LEFT JOIN types t2 ON w2.type = t2.id
        WHERE cw.parent_id = ?
          AND COALESCE(t2.type_x, '') != 'Affix'
          AND COALESCE(t2.type, '') != 'Afx'
          AND COALESCE(t2."group", '') != 'Cpx'
        ORDER BY 1
    """
    children = await run_query(children_sql, (word_id,))
    word["children"] = [r["name"] for r in children]

    return jsonify(serialize_word_detail(word))


# ─── Word CRUD (Admin Only) ──────────────────────────────────────────────────
@api_bp.route("/words", methods=["POST"])
@require_admin
async def create_word():
    """Create a new word (Admin only)."""
    data = await request.get_json()
    if not data or not data.get("name"):
        return jsonify({"error": "Word name is required"}), 400

    name = data["name"].strip()
    type_name = data.get("type_name")

    type_id = None
    if type_name:
        types = await run_query("SELECT id FROM types WHERE type = ?", (type_name,))
        if types:
            type_id = types[0]["id"]

    authors, db_year, db_rank, db_notes = denormalize_word_fields(
        data.get("source"),
        data.get("year"),
        data.get("rank"),
        data.get("notes"),
    )

    insert_sql = """
        INSERT INTO words (name, type, origin, origin_x, "match", rank, year, notes)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?)
    """
    new_id = await run_write(
        insert_sql,
        (
            name,
            type_id,
            data.get("origin"),
            data.get("origin_x"),
            data.get("match_") or data.get("match"),
            db_rank,
            db_year,
            db_notes,
        ),
    )

    # Sync connect_authors
    if authors:
        for abbr in authors:
            await run_write(
                """INSERT OR IGNORE INTO authors (abbreviation, id)
                   VALUES (?, (SELECT COALESCE(MAX(id), 0) + 1 FROM authors))""",
                (abbr,),
            )
            await run_write(
                """INSERT OR IGNORE INTO connect_authors ("AID", "WID")
                   SELECT id, ? FROM authors WHERE abbreviation = ?""",
                (new_id, abbr),
            )

    return await get_word_detail(new_id)


@api_bp.route("/words/<int:word_id>", methods=["PUT"])
@require_admin
async def update_word(word_id: int):
    """Update an existing word (Admin only)."""
    data = await request.get_json()
    if not data:
        return jsonify({"error": "Missing payload"}), 400

    type_id = None
    type_name = data.get("type_name")
    if type_name:
        types = await run_query("SELECT id FROM types WHERE type = ?", (type_name,))
        if types:
            type_id = types[0]["id"]

    authors, db_year, db_rank, db_notes = denormalize_word_fields(
        data.get("source"),
        data.get("year"),
        data.get("rank"),
        data.get("notes"),
    )

    update_sql = """
        UPDATE words
        SET name = COALESCE(?, name),
            type = COALESCE(?, type),
            origin = COALESCE(?, origin),
            origin_x = COALESCE(?, origin_x),
            "match" = COALESCE(?, "match"),
            rank = COALESCE(?, rank),
            year = COALESCE(?, year),
            notes = COALESCE(?, notes)
        WHERE id = ?
    """
    await run_write(
        update_sql,
        (
            data.get("name"),
            type_id,
            data.get("origin"),
            data.get("origin_x"),
            data.get("match_") or data.get("match"),
            db_rank,
            db_year,
            db_notes,
            word_id,
        ),
    )

    # Sync connect_authors if source was passed
    if "source" in data:
        await run_write('DELETE FROM connect_authors WHERE "WID" = ?', (word_id,))
        for abbr in authors:
            await run_write(
                """INSERT OR IGNORE INTO authors (abbreviation, id)
                   VALUES (?, (SELECT COALESCE(MAX(id), 0) + 1 FROM authors))""",
                (abbr,),
            )
            await run_write(
                """INSERT OR IGNORE INTO connect_authors ("AID", "WID")
                   SELECT id, ? FROM authors WHERE abbreviation = ?""",
                (word_id, abbr),
            )

    return await get_word_detail(word_id)


@api_bp.route("/words/<int:word_id>", methods=["DELETE"])
@require_admin
async def delete_word(word_id: int):
    """Delete word and related definitions/connections (Admin only)."""
    # Delete child references first to obey foreign keys
    await run_write("DELETE FROM definitions WHERE word_id = ?", (word_id,))
    await run_write(
        "DELETE FROM connect_words WHERE parent_id = ? OR child_id = ?", (word_id, word_id)
    )
    await run_write('DELETE FROM connect_authors WHERE "WID" = ?', (word_id,))
    await run_write("DELETE FROM words WHERE id = ?", (word_id,))
    return jsonify({"success": True, "deleted_id": word_id})


# ─── Definitions CRUD (Admin Only) ───────────────────────────────────────────
@api_bp.route("/words/<int:word_id>/definitions", methods=["POST"])
@require_admin
async def add_definition(word_id: int):
    """Add a definition to a word (Admin only)."""
    data = await request.get_json()
    if not data or not data.get("body"):
        return jsonify({"error": "Definition body is required"}), 400

    pos_rows = await run_query(
        "SELECT COALESCE(MAX(position), 0) + 1 as next_pos FROM definitions WHERE word_id = ?",
        (word_id,),
    )
    next_pos = pos_rows[0]["next_pos"] if pos_rows else 1

    grammar = data.get("grammar") or ""
    slots = None
    grammar_code = None
    if grammar:
        m = re.match(r"^(\d+)?(.*)$", grammar.strip())
        if m:
            slots = int(m.group(1)) if m.group(1) else None
            grammar_code = m.group(2) if m.group(2) else None

    insert_sql = """
        INSERT INTO definitions (word_id, position, body, usage, grammar_code, slots, case_tags)
        VALUES (?, ?, ?, ?, ?, ?, ?)
    """
    await run_write(
        insert_sql,
        (
            word_id,
            data.get("position", next_pos),
            data["body"],
            data.get("usage"),
            grammar_code,
            slots,
            data.get("tags"),
        ),
    )
    return await get_word_detail(word_id)


@api_bp.route("/words/<int:word_id>/definitions/<int:def_id>", methods=["PUT"])
@require_admin
async def update_definition(word_id: int, def_id: int):
    """Update an existing definition (Admin only)."""
    data = await request.get_json()
    if not data:
        return jsonify({"error": "Missing payload"}), 400

    grammar = data.get("grammar")
    slots = None
    grammar_code = None
    if grammar is not None:
        m = re.match(r"^(\d+)?(.*)$", grammar.strip())
        if m:
            slots = int(m.group(1)) if m.group(1) else None
            grammar_code = m.group(2) if m.group(2) else None

    update_sql = """
        UPDATE definitions
        SET body = COALESCE(?, body),
            usage = COALESCE(?, usage),
            grammar_code = COALESCE(?, grammar_code),
            slots = COALESCE(?, slots),
            case_tags = COALESCE(?, case_tags),
            position = COALESCE(?, position)
        WHERE id = ? AND word_id = ?
    """
    await run_write(
        update_sql,
        (
            data.get("body"),
            data.get("usage"),
            grammar_code,
            slots,
            data.get("tags"),
            data.get("position"),
            def_id,
            word_id,
        ),
    )
    return await get_word_detail(word_id)


@api_bp.route("/words/<int:word_id>/definitions/<int:def_id>", methods=["DELETE"])
@require_admin
async def delete_definition(word_id: int, def_id: int):
    """Delete a definition (Admin only)."""
    await run_write('DELETE FROM connect_keys WHERE "DID" = ?', (def_id,))
    await run_write("DELETE FROM definitions WHERE id = ? AND word_id = ?", (def_id, word_id))
    return await get_word_detail(word_id)


# ─── English-to-Loglan Search ─────────────────────────────────────────────────
@api_bp.route("/search/english", methods=["GET"])
@optional_auth
async def search_english():
    """
    Search definitions by English keyword.
    Supports FTS5 (def_fts / def_kw_fts) with automatic LIKE fallback and keyword-only search.
    """
    q = (request.args.get("query") or request.args.get("q") or "").strip()
    use_like = request.args.get("use_like", "false").lower() in ("true", "1")
    use_keywords = request.args.get("use_keywords_only", "false").lower() in ("true", "1")
    limit = min(int(request.args.get("limit", 300)), 500)

    if not q:
        return jsonify([])

    results: List[Dict[str, Any]] = []

    if not use_like:
        # Strategy 1: FTS5 search (def_fts or def_kw_fts)
        fts_table = "def_kw_fts" if use_keywords else "def_fts"
        fts_sql = f"""
            SELECT
                d.word_id,
                w.name as word_name,
                t.type as type_name,
                NULLIF(COALESCE(CAST(d.slots AS TEXT), '') || COALESCE(d.grammar_code, ''), '') as grammar,
                snippet({fts_table}, 0, '<b>', '</b>', '…', 10) as snippet,
                COUNT(d.id) as match_count
            FROM {fts_table} fts
            JOIN definitions d ON d.id = fts.rowid
            JOIN words w ON w.id = d.word_id
            LEFT JOIN types t ON w.type = t.id
            WHERE {fts_table} MATCH ?
            GROUP BY d.word_id
            ORDER BY rank
            LIMIT ?
        """
        try:
            clean_q = q.replace('"', '""')
            safe_q = f'"{clean_q}"' if " " in clean_q else f"{clean_q}*"
            rows = await run_query(fts_sql, (safe_q, limit))
            for r in rows:
                results.append(
                    serialize_el_result(
                        word_id=r["word_id"],
                        word_name=r["word_name"],
                        type_name=r["type_name"],
                        grammar=r["grammar"],
                        snippet=r["snippet"] or "",
                        match_count=r["match_count"],
                    )
                )
            if results:
                return jsonify(results)
        except Exception:
            # Fallback to LIKE
            pass

    # Strategy 2: SQL LIKE search
    if use_keywords:
        like_sql = """
            SELECT
                d.word_id,
                w.name as word_name,
                t.type as type_name,
                NULLIF(COALESCE(CAST(d.slots AS TEXT), '') || COALESCE(d.grammar_code, ''), '') as grammar,
                d.body as snippet,
                COUNT(d.id) as match_count
            FROM definitions d
            JOIN words w ON w.id = d.word_id
            LEFT JOIN types t ON w.type = t.id
            WHERE d.body LIKE ? OR d.body LIKE ? OR d.body LIKE ?
            GROUP BY d.word_id
            ORDER BY w.name COLLATE NOCASE ASC
            LIMIT ?
        """
        params = (f"%«{q}»%", f"%«{q} %", f"% {q}»%", limit)
    else:
        like_sql = """
            SELECT
                d.word_id,
                w.name as word_name,
                t.type as type_name,
                NULLIF(COALESCE(CAST(d.slots AS TEXT), '') || COALESCE(d.grammar_code, ''), '') as grammar,
                d.body as snippet,
                COUNT(d.id) as match_count
            FROM definitions d
            JOIN words w ON w.id = d.word_id
            LEFT JOIN types t ON w.type = t.id
            WHERE d.body LIKE ?
            GROUP BY d.word_id
            ORDER BY w.name COLLATE NOCASE ASC
            LIMIT ?
        """
        params = (f"%{q}%", limit)

    rows = await run_query(like_sql, params)
    for r in rows:
        body = r["snippet"]
        escaped_q = re.escape(q)
        highlighted = re.sub(f"({escaped_q})", r"<b>\1</b>", body, flags=re.IGNORECASE)
        results.append(
            serialize_el_result(
                word_id=r["word_id"],
                word_name=r["word_name"],
                type_name=r["type_name"],
                grammar=r["grammar"],
                snippet=highlighted,
                match_count=r["match_count"],
            )
        )

    return jsonify(results)


# ─── Types ────────────────────────────────────────────────────────────────────
@api_bp.route("/types", methods=["GET"])
@optional_auth
async def get_types():
    """List all word types with member count."""
    sql = """
        SELECT
            t.id,
            t.type as name,
            t.type_x,
            t."group" as group_,
            COUNT(w.id) as word_count
        FROM types t
        LEFT JOIN words w ON w.type = t.id
        GROUP BY t.id
        ORDER BY t.id ASC
    """
    rows = await run_query(sql)
    return jsonify([serialize_type(r) for r in rows])


# ─── Events ───────────────────────────────────────────────────────────────────
@api_bp.route("/events", methods=["GET"])
@optional_auth
async def get_events():
    """List all dictionary revision events."""
    sql = """
        SELECT id, name, date, annotation, suffix, definition as notes
        FROM events
        ORDER BY id ASC
    """
    rows = await run_query(sql)
    return jsonify([serialize_event(r) for r in rows])


# ─── Authors ──────────────────────────────────────────────────────────────────
@api_bp.route("/authors", methods=["GET"])
@optional_auth
async def get_authors():
    """List all dictionary authors with associated word count."""
    sql = """
        SELECT
            a.id,
            a.abbreviation as initials,
            a.full_name,
            a.notes,
            COUNT(ca."WID") as word_count
        FROM authors a
        LEFT JOIN connect_authors ca ON ca."AID" = a.id
        GROUP BY a.id
        ORDER BY a.abbreviation ASC
    """
    rows = await run_query(sql)
    return jsonify([serialize_author(r) for r in rows])


# ─── Database Statistics ──────────────────────────────────────────────────────
@api_bp.route("/stats", methods=["GET"])
@optional_auth
async def get_stats():
    """Return dictionary database metrics and settings."""
    db_path = get_db_path()
    w_count = (await run_query("SELECT COUNT(*) as c FROM words"))[0]["c"]
    d_count = (await run_query("SELECT COUNT(*) as c FROM definitions"))[0]["c"]
    e_count = (await run_query("SELECT COUNT(*) as c FROM events"))[0]["c"]
    t_count = (await run_query("SELECT COUNT(*) as c FROM types"))[0]["c"]
    a_count = (await run_query("SELECT COUNT(*) as c FROM authors"))[0]["c"]

    # Affix count
    affix_sql = """
        SELECT COUNT(*) as c FROM connect_words cw
        JOIN words w ON w.id = cw.child_id
        JOIN types t ON t.id = w.type
        WHERE t.type_x = 'Affix' OR t.type = 'Afx'
    """
    ax_rows = await run_query(affix_sql)
    ax_count = ax_rows[0]["c"] if ax_rows else 0

    # Spelling count
    spelling_rows = await run_query(
        "SELECT COUNT(*) as c FROM sqlite_master WHERE type='table' AND name='word_spellings'"
    )
    if spelling_rows and spelling_rows[0]["c"] > 0:
        sc_rows = await run_query("SELECT COUNT(*) as c FROM word_spellings")
        sp_count = sc_rows[0]["c"] if sc_rows else w_count
    else:
        sp_count = w_count

    settings_rows = await run_query("SELECT * FROM settings ORDER BY id DESC LIMIT 1")
    settings: List[Dict[str, str]] = []
    if settings_rows:
        for k, v in settings_rows[0].items():
            if v is not None and k not in ("id", "created", "updated"):
                settings.append({"key": str(k), "value": str(v)})

    stats = serialize_db_stats(
        db_path=os.path.basename(db_path),
        word_count=w_count,
        definition_count=d_count,
        event_count=e_count,
        type_count=t_count,
        author_count=a_count,
        affix_count=ax_count,
        spelling_count=sp_count,
        settings=settings,
    )
    return jsonify(stats)
