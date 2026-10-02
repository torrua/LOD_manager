"""DTO serializers mapping loglan_core database models to LOD Manager TypeScript models."""

import json
from typing import Any, Dict, List, Optional, Tuple


def _get(obj: Any, key: str, default: Any = None) -> Any:
    """Helper to retrieve attribute from ORM object or key from dict."""
    if isinstance(obj, dict):
        return obj.get(key, default)
    return getattr(obj, key, default)


def normalize_word_fields(
    authors_csv: Optional[str],
    raw_year: Optional[str],
    raw_rank: Optional[str],
    raw_notes: Optional[str],
) -> Tuple[Optional[str], Optional[str], Optional[str], Optional[str]]:
    """
    Extract clean source, year, rank, and notes from database columns.
    Matches src-tauri/src/db.rs::normalize_word_fields.
    """
    note_author: Optional[str] = None
    note_year: Optional[str] = None
    note_rank: Optional[str] = None
    clean_notes: Optional[str] = None

    if raw_notes:
        rn = str(raw_notes).strip()
        if rn and rn.lower() != "null":
            if rn.startswith("{"):
                try:
                    data = json.loads(rn)
                    if isinstance(data, dict):
                        if data.get("author"):
                            note_author = str(data["author"]).strip() or None
                        if data.get("year"):
                            note_year = str(data["year"]).strip() or None
                        if data.get("rank"):
                            note_rank = str(data["rank"]).strip() or None

                        extra = []
                        for k, v in data.items():
                            if k not in ("author", "year", "rank") and v:
                                s = str(v).strip()
                                if s:
                                    if k == "notes":
                                        extra.append(s)
                                    else:
                                        extra.append(f"{k}: {s}")
                        if extra:
                            clean_notes = "; ".join(extra)
                except Exception:
                    clean_notes = rn
            else:
                clean_notes = rn

    base_authors = authors_csv.strip() if authors_csv and str(authors_csv).strip() else None
    if base_authors and note_author:
        parts = [p.strip() for p in base_authors.split("/")]
        if note_author not in parts:
            source = f"{base_authors}/{note_author}"
        else:
            source = base_authors
    elif base_authors:
        source = base_authors
    else:
        source = note_author

    year = None
    if raw_year:
        year = str(raw_year)[:4]
    elif note_year:
        year = str(note_year)[:4]

    rank = str(raw_rank) if raw_rank is not None else note_rank

    return source, year, rank, clean_notes


def denormalize_word_fields(
    source: Optional[str],
    year: Optional[str],
    rank: Optional[str],
    notes: Optional[str],
) -> Tuple[List[str], Optional[str], Optional[str], Optional[str]]:
    """
    Split combined source, year, rank, and notes back into relational / JSON columns.
    Matches src-tauri/src/db.rs::denormalize_word_fields.
    """
    authors = (
        [a.strip() for a in str(source).replace(",", "/").split("/") if a.strip()]
        if source
        else []
    )

    db_year = None
    if year:
        y_str = str(year).strip()
        db_year = f"{y_str}-01-01" if len(y_str) == 4 and y_str.isdigit() else y_str

    db_rank = str(rank).strip() if rank is not None else None

    notes_dict: Dict[str, Any] = {}
    if notes:
        n_str = str(notes).strip()
        if n_str:
            notes_dict["notes"] = n_str
    db_notes = json.dumps(notes_dict) if notes_dict else None

    return authors, db_year, db_rank, db_notes


def serialize_definition(defn: Any) -> Dict[str, Any]:
    """Serialize a Definition model/row to TypeScript Definition interface."""
    grammar_code = _get(defn, "grammar_code")
    slots = _get(defn, "slots")

    # In loglan_core / LOD schema, slots + grammar_code forms the full grammar string (e.g. '2a')
    grammar = _get(defn, "grammar")
    if grammar is None:
        slots_str = str(slots) if slots is not None else ""
        code_str = str(grammar_code) if grammar_code is not None else ""
        combined = f"{slots_str}{code_str}".strip()
        grammar = combined if combined else None

    tags = _get(defn, "tags") or _get(defn, "case_tags")

    return {
        "id": int(_get(defn, "id", 0)),
        "position": int(_get(defn, "position", 1)),
        "grammar": grammar,
        "usage": _get(defn, "usage"),
        "body": _get(defn, "body", "") or "",
        "tags": tags,
    }


def serialize_word_list_item(
    word: Any,
    def_count: Optional[int] = None,
    type_name: Optional[str] = None,
) -> Dict[str, Any]:
    """Serialize a Word model/row to TypeScript WordListItem interface."""
    word_id = int(_get(word, "id", 0))
    name = str(_get(word, "name", "") or "")

    if type_name is None:
        type_obj = _get(word, "type")
        if type_obj is not None:
            type_name = str(_get(type_obj, "type") or _get(type_obj, "name") or type_obj)
        else:
            type_name = _get(word, "type_name")

    if def_count is None:
        defs = _get(word, "definitions")
        if defs is not None and hasattr(defs, "__len__"):
            def_count = len(defs)
        else:
            def_count = int(_get(word, "def_count", 0))

    return {
        "id": word_id,
        "name": name,
        "type_name": type_name,
        "def_count": def_count,
    }


def serialize_word_detail(word: Any) -> Dict[str, Any]:
    """Serialize a complete Word model/row to TypeScript WordDetail interface."""
    word_id = int(_get(word, "id", 0))
    name = str(_get(word, "name", "") or "")

    type_name = None
    type_id = None
    type_obj = _get(word, "type")
    if type_obj is not None:
        type_name = str(_get(type_obj, "type") or _get(type_obj, "name") or type_obj)
        type_id = _get(type_obj, "id")
    else:
        type_name = _get(word, "type_name")
        type_id = _get(word, "type_id")

    # Author, Year, Rank, Notes normalization
    authors_csv = _get(word, "authors_csv")
    raw_year = _get(word, "year")
    raw_rank = _get(word, "rank")
    raw_notes = _get(word, "notes")
    source, year_str, rank_str, notes_str = normalize_word_fields(
        authors_csv, raw_year, raw_rank, raw_notes
    )
    if not source:
        source = _get(word, "source")

    # Event names
    ev_start = _get(word, "event_start")
    ev_start_name = (
        _get(ev_start, "name")
        if ev_start is not None and not isinstance(ev_start, (int, str))
        else _get(word, "event_start_name") or (str(ev_start) if ev_start else None)
    )

    ev_end = _get(word, "event_end")
    ev_end_name = (
        _get(ev_end, "name")
        if ev_end is not None and not isinstance(ev_end, (int, str))
        else _get(word, "event_end_name") or (str(ev_end) if ev_end else None)
    )

    # Affixes & Spellings (support both djifoa and affixes)
    affixes_raw = _get(word, "djifoa") or _get(word, "affixes", []) or []
    affixes = [
        str(_get(a, "name", a)).replace("-", "")
        for a in affixes_raw
        if a is not None and str(_get(a, "name", a)).replace("-", "")
    ]

    spellings_raw = _get(word, "spellings", []) or []
    spellings = [str(_get(s, "name", s)) for s in spellings_raw if s is not None]

    # Definitions
    definitions_raw = _get(word, "definitions", []) or []
    definitions = [serialize_definition(d) for d in definitions_raw]
    definitions.sort(key=lambda d: d.get("position", 0))

    # Relationships
    parents_raw = _get(word, "parents", []) or []
    parents = [str(_get(p, "name", p)) for p in parents_raw if p is not None]

    children_raw = _get(word, "children", []) or []
    children = [str(_get(c, "name", c)) for c in children_raw if c is not None]

    used_in_raw = _get(word, "used_in", []) or []
    used_in = [str(_get(u, "name", u)) for u in used_in_raw if u is not None]

    return {
        "id": word_id,
        "name": name,
        "type_name": type_name,
        "type_id": int(type_id) if type_id is not None else None,
        "source": source,
        "year": year_str,
        "rank": rank_str,
        "match_": str(_get(word, "match_") or _get(word, "match") or ""),
        "origin": _get(word, "origin"),
        "origin_x": _get(word, "origin_x"),
        "notes": notes_str,
        "event_start_name": ev_start_name,
        "event_end_name": ev_end_name,
        "affixes": affixes,
        "spellings": spellings,
        "definitions": definitions,
        "used_in": used_in,
        "parents": parents,
        "children": children,
    }


def serialize_event(event: Any) -> Dict[str, Any]:
    """Serialize an Event model/row to TypeScript EventItem interface."""
    date_val = _get(event, "date")
    date_str = str(date_val) if date_val is not None else None

    return {
        "id": int(_get(event, "id", 0)),
        "name": str(_get(event, "name", "") or ""),
        "date": date_str,
        "annotation": _get(event, "annotation"),
        "suffix": _get(event, "suffix"),
        "notes": _get(event, "notes") or _get(event, "definition"),
    }


def serialize_type(type_item: Any, word_count: int = 0) -> Dict[str, Any]:
    """Serialize a Type model/row to TypeScript TypeItem interface."""
    name = str(_get(type_item, "type") or _get(type_item, "name") or "")
    group = _get(type_item, "group_") or _get(type_item, "group")

    return {
        "id": int(_get(type_item, "id", 0)),
        "name": name,
        "type_x": _get(type_item, "type_x"),
        "group_": group,
        "word_count": int(_get(type_item, "word_count", word_count)),
    }


def serialize_author(author: Any, word_count: int = 0) -> Dict[str, Any]:
    """Serialize an Author model/row to TypeScript AuthorItem interface."""
    initials = str(_get(author, "initials") or _get(author, "abbreviation") or "")

    return {
        "id": int(_get(author, "id", 0)),
        "initials": initials,
        "full_name": _get(author, "full_name"),
        "notes": _get(author, "notes"),
        "word_count": int(_get(author, "word_count", word_count)),
    }


def serialize_el_result(
    word_id: int,
    word_name: str,
    type_name: Optional[str],
    grammar: Optional[str],
    snippet: str,
    match_count: int = 1,
) -> Dict[str, Any]:
    """Serialize an English search result to TypeScript ELResult interface."""
    return {
        "word_id": word_id,
        "word_name": word_name,
        "type_name": type_name,
        "grammar": grammar,
        "snippet": snippet,
        "match_count": match_count,
    }


def serialize_db_stats(
    db_path: str,
    word_count: int,
    definition_count: int,
    event_count: int,
    type_count: int,
    author_count: int,
    affix_count: int = 0,
    spelling_count: int = 0,
    settings: Optional[List[Dict[str, str]]] = None,
) -> Dict[str, Any]:
    """Serialize statistics to TypeScript DbStats interface."""
    return {
        "db_path": db_path,
        "word_count": word_count,
        "definition_count": definition_count,
        "event_count": event_count,
        "type_count": type_count,
        "author_count": author_count,
        "affix_count": affix_count,
        "spelling_count": spelling_count,
        "settings": settings or [],
    }
