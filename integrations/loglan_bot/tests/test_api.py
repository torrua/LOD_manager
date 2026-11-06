"""Comprehensive test suite for LOD Manager Python Quart API and auth mechanisms."""

import hashlib
import hmac
import json
import os
import sqlite3
import tempfile
import time
import urllib.parse
import pytest
from quart import Quart

from app.api import api_bp
from app.api.auth import validate_init_data


BOT_TOKEN = "123456789:ABC-DEF1234ghIkl-zyx57W2v1u123ew11"
ADMIN_ID = 999888
REGULAR_USER_ID = 111222


def make_init_data(user_dict: dict, auth_date: int = None, bot_token: str = BOT_TOKEN) -> str:
    """Helper to generate a cryptographically valid Telegram initData query string."""
    if auth_date is None:
        auth_date = int(time.time())

    params = {
        "auth_date": str(auth_date),
        "query_id": "AAHdF6IQAAAAAN0XohDhrOrc",
        "user": json.dumps(user_dict, separators=(",", ":")),
    }
    data_check_string = "\n".join(f"{k}={v}" for k, v in sorted(params.items()))
    secret_key = hmac.new(b"WebAppData", bot_token.encode("utf-8"), hashlib.sha256).digest()
    sig = hmac.new(secret_key, data_check_string.encode("utf-8"), hashlib.sha256).hexdigest()
    params["hash"] = sig
    return urllib.parse.urlencode(params)


@pytest.fixture
def temp_db():
    """Create a temporary SQLite database with loglan_core schema and FTS5 tables."""
    fd, path = tempfile.mkstemp(suffix=".db")
    os.close(fd)

    conn = sqlite3.connect(path)
    conn.executescript(
        """
        PRAGMA foreign_keys = ON;

        CREATE TABLE types (
            id INTEGER PRIMARY KEY,
            type TEXT NOT NULL,
            type_x TEXT,
            "group" TEXT,
            parentable BOOLEAN DEFAULT 1,
            description TEXT
        );

        CREATE TABLE events (
            id INTEGER PRIMARY KEY,
            event_id INTEGER NOT NULL UNIQUE,
            name TEXT NOT NULL,
            date TEXT,
            definition TEXT,
            annotation TEXT,
            suffix TEXT
        );

        CREATE TABLE authors (
            id INTEGER PRIMARY KEY,
            abbreviation TEXT NOT NULL,
            full_name TEXT,
            notes TEXT
        );

        CREATE TABLE words (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            type INTEGER REFERENCES types(id),
            origin TEXT,
            origin_x TEXT,
            "match" TEXT,
            rank TEXT,
            year TEXT,
            notes TEXT,
            id_old INTEGER,
            "TID_old" INTEGER,
            event_start INTEGER REFERENCES events(event_id),
            event_end INTEGER REFERENCES events(event_id)
        );

        CREATE TABLE definitions (
            id INTEGER PRIMARY KEY,
            word_id INTEGER NOT NULL REFERENCES words(id),
            position INTEGER DEFAULT 1,
            body TEXT NOT NULL,
            usage TEXT,
            grammar_code TEXT,
            slots INTEGER,
            case_tags TEXT,
            language TEXT DEFAULT 'en',
            notes TEXT
        );

        CREATE TABLE connect_words (
            parent_id INTEGER NOT NULL REFERENCES words(id),
            child_id INTEGER NOT NULL REFERENCES words(id),
            PRIMARY KEY (parent_id, child_id)
        );

        CREATE TABLE connect_authors (
            "AID" INTEGER NOT NULL REFERENCES authors(id),
            "WID" INTEGER NOT NULL REFERENCES words(id),
            PRIMARY KEY ("AID", "WID")
        );

        CREATE TABLE connect_keys (
            "KID" INTEGER NOT NULL,
            "DID" INTEGER NOT NULL REFERENCES definitions(id),
            PRIMARY KEY ("KID", "DID")
        );

        CREATE TABLE word_affixes (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            word_id INTEGER NOT NULL REFERENCES words(id),
            affix TEXT NOT NULL
        );

        CREATE TABLE word_spellings (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            word_id INTEGER NOT NULL REFERENCES words(id),
            spelling TEXT NOT NULL
        );

        CREATE TABLE word_usage (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            word_id INTEGER NOT NULL REFERENCES words(id),
            used_in_word TEXT NOT NULL
        );

        -- FTS tables
        CREATE VIRTUAL TABLE def_fts USING fts5(
            body,
            content='definitions',
            content_rowid='id'
        );

        CREATE VIRTUAL TABLE def_kw_fts USING fts5(
            keywords
        );

        -- Initial seed
        INSERT INTO events (id, event_id, name, date) VALUES (1, 1, 'Baseline', '1975-01-01');
        INSERT INTO events (id, event_id, name, date) VALUES (2, 2, 'Revision 2', '1989-01-01');

        INSERT INTO types (id, type, type_x, "group") VALUES (1, 'C-Prim', 'Primitive', 'Prim');
        INSERT INTO types (id, type, type_x, "group") VALUES (2, 'Afx', 'Affix', 'Affix');
        INSERT INTO types (id, type, type_x, "group") VALUES (3, 'Cpx', 'Complex', 'Cpx');

        INSERT INTO authors (id, abbreviation, full_name) VALUES (1, 'JCB', 'James Cooke Brown');
        """
    )
    conn.commit()
    conn.close()

    yield path

    if os.path.exists(path):
        os.remove(path)


@pytest.fixture
def app(temp_db):
    """Create Quart test application configured with test DB and admin credentials."""
    app = Quart(__name__)
    app.config["TESTING"] = True
    app.config["DATABASE_PATH"] = temp_db
    app.config["BOT_TOKEN"] = BOT_TOKEN
    app.config["ADMIN_IDS"] = f"{ADMIN_ID}"

    app.register_blueprint(api_bp)
    return app


# ─── 1. HMAC Auth Unit Tests ──────────────────────────────────────────────────
def test_validate_init_data_success():
    init_data = make_init_data({"id": ADMIN_ID, "first_name": "Admin"})
    valid, user, err = validate_init_data(init_data, BOT_TOKEN)
    assert valid is True
    assert user["id"] == ADMIN_ID
    assert err is None


def test_validate_init_data_expired():
    old_time = int(time.time()) - 100000
    init_data = make_init_data({"id": ADMIN_ID}, auth_date=old_time)
    valid, user, err = validate_init_data(init_data, BOT_TOKEN)
    assert valid is False
    assert user is None
    assert "expired" in err


def test_validate_init_data_tampered():
    init_data = make_init_data({"id": ADMIN_ID})
    tampered = init_data.replace(str(ADMIN_ID), "123456")
    valid, user, err = validate_init_data(tampered, BOT_TOKEN)
    assert valid is False
    assert err == "Hash mismatch"


def test_validate_init_data_missing_fields():
    valid, user, err = validate_init_data("", BOT_TOKEN)
    assert valid is False
    assert "Missing" in err

    valid, user, err = validate_init_data("foo=bar", BOT_TOKEN)
    assert valid is False
    assert "hash" in err


# ─── 2. Auth me endpoint & Query fallback rejection (CWE-598) ─────────────────
@pytest.mark.asyncio
async def test_auth_me_anonymous(app):
    client = app.test_client()
    res = await client.get("/api/v1/auth/me")
    assert res.status_code == 200
    data = await res.get_json()
    assert data["user"] is None
    assert data["is_admin"] is False


@pytest.mark.asyncio
async def test_auth_me_admin_header(app):
    client = app.test_client()
    init_data = make_init_data({"id": ADMIN_ID, "first_name": "Boss"})
    headers = {"X-Telegram-Init-Data": init_data}
    res = await client.get("/api/v1/auth/me", headers=headers)
    assert res.status_code == 200
    data = await res.get_json()
    assert data["user"]["id"] == ADMIN_ID
    assert data["is_admin"] is True


@pytest.mark.asyncio
async def test_auth_me_regular_user_bearer(app):
    client = app.test_client()
    init_data = make_init_data({"id": REGULAR_USER_ID, "first_name": "User"})
    headers = {"Authorization": f"tma {init_data}"}
    res = await client.get("/api/v1/auth/me", headers=headers)
    assert res.status_code == 200
    data = await res.get_json()
    assert data["user"]["id"] == REGULAR_USER_ID
    assert data["is_admin"] is False


@pytest.mark.asyncio
async def test_cwe_598_query_param_ignored(app):
    """Ensure sensitive initData in query parameter is NOT accepted."""
    client = app.test_client()
    init_data = make_init_data({"id": ADMIN_ID, "first_name": "Boss"})
    # Passing in query string must NOT authenticate admin
    res = await client.get(f"/api/v1/auth/me?init_data={urllib.parse.quote(init_data)}")
    assert res.status_code == 200
    data = await res.get_json()
    assert data["user"] is None
    assert data["is_admin"] is False


# ─── 3. Event words endpoint ──────────────────────────────────────────────────
@pytest.mark.asyncio
async def test_get_event_words(app):
    client = app.test_client()
    admin_auth = {"X-Telegram-Init-Data": make_init_data({"id": ADMIN_ID})}

    # Create words associated with event 1 (Baseline) and event 2
    await client.post(
        "/api/v1/words",
        json={"name": "blinu", "type_id": 1, "event_start": "Baseline", "event_end": "Revision 2"},
        headers=admin_auth,
    )
    await client.post(
        "/api/v1/words",
        json={"name": "canro", "type_id": 1, "event_start": "Revision 2"},
        headers=admin_auth,
    )

    # Event 1: added blinu, removed none
    res1 = await client.get("/api/v1/events/1/words")
    assert res1.status_code == 200
    added1, removed1 = await res1.get_json()
    assert "blinu" in added1
    assert len(removed1) == 0

    # Event 2: added canro, removed blinu
    res2 = await client.get("/api/v1/events/2/words")
    assert res2.status_code == 200
    added2, removed2 = await res2.get_json()
    assert "canro" in added2
    assert "blinu" in removed2


# ─── 4. Admin CRUD Endpoints (events, types, authors) ─────────────────────────
@pytest.mark.asyncio
async def test_admin_crud_events(app):
    client = app.test_client()
    admin_auth = {"X-Telegram-Init-Data": make_init_data({"id": ADMIN_ID})}
    user_auth = {"X-Telegram-Init-Data": make_init_data({"id": REGULAR_USER_ID})}

    # 401 unauthenticated
    res = await client.post("/api/v1/events", json={"name": "Ev3"})
    assert res.status_code == 401

    # 403 regular user
    res = await client.post("/api/v1/events", json={"name": "Ev3"}, headers=user_auth)
    assert res.status_code == 403

    # Create event
    res = await client.post("/api/v1/events", json={"name": "Event 3", "date": "2020-01-01"}, headers=admin_auth)
    assert res.status_code == 200
    ev = await res.get_json()
    ev_id = ev["id"]
    assert ev["name"] == "Event 3"

    # Update event
    res = await client.put(f"/api/v1/events/{ev_id}", json={"name": "Event 3 Updated"}, headers=admin_auth)
    assert res.status_code == 200
    ev_updated = await res.get_json()
    assert ev_updated["name"] == "Event 3 Updated"

    # Delete unused event
    res = await client.delete(f"/api/v1/events/{ev_id}", headers=admin_auth)
    assert res.status_code == 200

    # Attempt to delete event used by words (event 1) -> 400 error
    await client.post(
        "/api/v1/words",
        json={"name": "vetci", "type_id": 1, "event_start_id": 1},
        headers=admin_auth,
    )
    res = await client.delete("/api/v1/events/1", headers=admin_auth)
    assert res.status_code == 400
    err_data = await res.get_json()
    assert "Cannot delete event" in err_data["error"]


@pytest.mark.asyncio
async def test_admin_crud_types(app):
    client = app.test_client()
    admin_auth = {"X-Telegram-Init-Data": make_init_data({"id": ADMIN_ID})}

    # Create type
    res = await client.post(
        "/api/v1/types",
        json={"name": "Experimental", "type_x": "Exp", "group": "Exp"},
        headers=admin_auth,
    )
    assert res.status_code == 200
    t = await res.get_json()
    t_id = t["id"]
    assert t["name"] == "Experimental"

    # Update type
    res = await client.put(f"/api/v1/types/{t_id}", json={"name": "ExpUpdated"}, headers=admin_auth)
    assert res.status_code == 200
    t_up = await res.get_json()
    assert t_up["name"] == "ExpUpdated"

    # Delete unused type
    res = await client.delete(f"/api/v1/types/{t_id}", headers=admin_auth)
    assert res.status_code == 200

    # In-use type rejection
    res = await client.delete("/api/v1/types/1", headers=admin_auth)
    # If type 1 is used or unused
    # Insert word using type 1
    await client.post("/api/v1/words", json={"name": "ditca", "type_id": 1}, headers=admin_auth)
    res = await client.delete("/api/v1/types/1", headers=admin_auth)
    assert res.status_code == 400
    assert "Cannot delete type" in (await res.get_json())["error"]


@pytest.mark.asyncio
async def test_admin_crud_authors(app):
    client = app.test_client()
    admin_auth = {"X-Telegram-Init-Data": make_init_data({"id": ADMIN_ID})}

    # Create author
    res = await client.post(
        "/api/v1/authors",
        json={"initials": "RAM", "full_name": "Robert A. McIvor", "notes": "Loglan Pioneer"},
        headers=admin_auth,
    )
    assert res.status_code == 200
    a = await res.get_json()
    a_id = a["id"]
    assert a["initials"] == "RAM"

    # Update author
    res = await client.put(
        f"/api/v1/authors/{a_id}",
        json={"notes": "Updated note"},
        headers=admin_auth,
    )
    assert res.status_code == 200
    a_up = await res.get_json()
    assert a_up["notes"] == "Updated note"

    # Delete author
    res = await client.delete(f"/api/v1/authors/{a_id}", headers=admin_auth)
    assert res.status_code == 200
    assert (await res.get_json())["success"] is True


# ─── 5. Word Relationships & FTS Synchronization ──────────────────────────────
@pytest.mark.asyncio
async def test_word_relationships_and_fts_sync(app, temp_db):
    client = app.test_client()
    admin_auth = {"X-Telegram-Init-Data": make_init_data({"id": ADMIN_ID})}

    # Create affix word in DB
    await client.post(
        "/api/v1/words",
        json={"name": "-flo-", "type_id": 2},
        headers=admin_auth,
    )

    # 1. Create word with affixes and spellings
    res = await client.post(
        "/api/v1/words",
        json={
            "name": "flora",
            "type_id": 1,
            "affixes": ["flo"],
            "spellings": ["floras"],
            "event_start": "Baseline",
        },
        headers=admin_auth,
    )
    assert res.status_code == 200
    w = await res.get_json()
    w_id = w["id"]
    assert "flo" in w["affixes"]

    # Verify connect_words in DB
    conn = sqlite3.connect(temp_db)
    cw = conn.execute("SELECT COUNT(*) FROM connect_words WHERE parent_id = ?", (w_id,)).fetchone()[0]
    assert cw == 1
    wa = conn.execute("SELECT COUNT(*) FROM word_affixes WHERE word_id = ?", (w_id,)).fetchone()[0]
    assert wa == 1
    ws = conn.execute("SELECT COUNT(*) FROM word_spellings WHERE word_id = ?", (w_id,)).fetchone()[0]
    assert ws == 1
    conn.close()

    # 2. Add definition with keywords
    res = await client.post(
        f"/api/v1/words/{w_id}/definitions",
        json={"body": "x1 is a flower/bloom of species «flower» and «bloom»"},
        headers=admin_auth,
    )
    assert res.status_code == 200
    w_detail = await res.get_json()
    def_id = w_detail["definitions"][0]["id"]

    # Verify FTS tables
    conn = sqlite3.connect(temp_db)
    fts_rows = conn.execute("SELECT COUNT(*) FROM def_fts WHERE def_fts MATCH 'flower'").fetchone()[0]
    assert fts_rows == 1
    kw_rows = conn.execute("SELECT keywords FROM def_kw_fts WHERE rowid = ?", (def_id,)).fetchall()
    assert len(kw_rows) == 1
    assert "flower bloom" in kw_rows[0][0]
    conn.close()

    # 3. Update definition to new keywords
    res = await client.put(
        f"/api/v1/words/{w_id}/definitions/{def_id}",
        json={"body": "x1 is an ornamental «plant»"},
        headers=admin_auth,
    )
    assert res.status_code == 200

    conn = sqlite3.connect(temp_db)
    kw_rows = conn.execute("SELECT keywords FROM def_kw_fts WHERE rowid = ?", (def_id,)).fetchall()
    assert len(kw_rows) == 1
    assert kw_rows[0][0] == "plant"
    conn.close()

    # 4. Delete definition
    res = await client.delete(f"/api/v1/words/{w_id}/definitions/{def_id}", headers=admin_auth)
    assert res.status_code == 200

    conn = sqlite3.connect(temp_db)
    kw_rows = conn.execute("SELECT COUNT(*) FROM def_kw_fts WHERE rowid = ?", (def_id,)).fetchone()[0]
    assert kw_rows == 0
    conn.close()

    # 5. Delete word and check cascade
    res = await client.delete(f"/api/v1/words/{w_id}", headers=admin_auth)
    assert res.status_code == 200

    conn = sqlite3.connect(temp_db)
    assert conn.execute("SELECT COUNT(*) FROM words WHERE id = ?", (w_id,)).fetchone()[0] == 0
    assert conn.execute("SELECT COUNT(*) FROM connect_words WHERE parent_id = ?", (w_id,)).fetchone()[0] == 0
    assert conn.execute("SELECT COUNT(*) FROM word_affixes WHERE word_id = ?", (w_id,)).fetchone()[0] == 0
    assert conn.execute("SELECT COUNT(*) FROM word_spellings WHERE word_id = ?", (w_id,)).fetchone()[0] == 0
    conn.close()


@pytest.mark.asyncio
async def test_word_usage_cleanup_on_delete(app, temp_db):
    client = app.test_client()
    admin_auth = {"X-Telegram-Init-Data": make_init_data({"id": ADMIN_ID})}

    res = await client.post("/api/v1/words", json={"name": "testusage", "type_id": 1}, headers=admin_auth)
    assert res.status_code == 200
    w_id = (await res.get_json())["id"]

    # Insert entry in word_usage referencing w_id
    conn = sqlite3.connect(temp_db)
    conn.execute("INSERT INTO word_usage (word_id, used_in_word) VALUES (?, 'complexword')", (w_id,))
    conn.commit()
    conn.close()

    # Deleting word must clean up word_usage without foreign key errors
    del_res = await client.delete(f"/api/v1/words/{w_id}", headers=admin_auth)
    assert del_res.status_code == 200
    assert (await del_res.get_json())["success"] is True

    conn = sqlite3.connect(temp_db)
    assert conn.execute("SELECT COUNT(*) FROM word_usage WHERE word_id = ?", (w_id,)).fetchone()[0] == 0
    conn.close()


@pytest.mark.asyncio
async def test_word_event_end_cleared_on_update(app):
    client = app.test_client()
    admin_auth = {"X-Telegram-Init-Data": make_init_data({"id": ADMIN_ID})}

    res = await client.post(
        "/api/v1/words",
        json={"name": "testend", "type_id": 1, "event_start": "Baseline", "event_end": "Revision 2"},
        headers=admin_auth,
    )
    assert res.status_code == 200
    w_id = (await res.get_json())["id"]

    # Clear event_end
    update_res = await client.put(
        f"/api/v1/words/{w_id}",
        json={"event_end": None},
        headers=admin_auth,
    )
    assert update_res.status_code == 200
    updated_word = await update_res.get_json()
    assert updated_word["event_end_name"] is None

