# Architecture

**Analysis Date:** 2026-04-01

## Pattern Overview

**Overall:** Tauri 2.0 desktop application with Rust backend and Svelte 5 frontend

**Key Characteristics:**

- Frontend communicates with Rust backend via Tauri invoke commands
- State management: Rust side uses Mutex for database connection (AppState struct)
- Command pattern: Tauri commands exposed in lib.rs
- Database: SQLite via rusqlite with FTS5 full-text search

## Layers

**Frontend (Svelte 5):**

- Purpose: UI rendering and user interaction
- Location: `src/`
- Contains: Svelte components, TypeScript types, stores
- Depends on: @tauri-apps/api for backend communication

**Backend (Rust):**

- Purpose: Database operations, file I/O, export generation
- Location: `src-tauri/src/`
- Contains: lib.rs (commands), db.rs (database), models.rs (data structures), import.rs, export.rs

**Database Layer:**

- Purpose: SQLite with FTS5 for full-text search
- Location: `tables/*.db` files
- Key tables: words, definitions, types, events, authors, settings, word_affixes, word_spellings

## Data Flow

**Word Query Flow:**

1. Frontend calls `invoke('get_words', { q, type_filter, event_id })`
2. lib.rs receives command, calls `with_db()` helper
3. db.rs executes query, returns `Vec<WordListItem>`
4. Frontend renders in Svelte component

**State Management:**

- `AppState` struct in lib.rs holds:
  - `db: Mutex<Option<Connection>>` - wrapped SQLite connection
  - `db_path: Mutex<String>` - current database path
- Mutex ensures thread-safe database access

## Key Abstractions

**Tauri Commands:**

- Database: `open_database`, `create_database`, `get_db_stats`
- Words: `get_words`, `get_word`, `save_word`, `delete_word`
- Definitions: `save_definition`, `delete_definition`
- Events: `get_events`, `save_event`, `delete_event`
- Types: `get_types`, `save_type`, `delete_type`
- Authors: `get_authors`, `save_author`, `delete_author`
- Import/Export: `import_lod_files`, `import_lod_contents`, `export_html`, `export_html_to_file`
- Search: `search_english`, `rebuild_fts`, `fts_is_ready`

**Data Models (models.rs):**

- `WordListItem`, `WordDetail`, `Definition` - word-related types
- `EventItem`, `TypeItem`, `AuthorItem` - lookup data types
- `SaveWord`, `SaveDefinition`, `SaveEvent` - input types
- `ELResult`, `ELSearchParams` - English-to-Loglan search

## Database Schema

**Core Tables:**

```sql
words (id, name, type_id, source, year, rank, match_, origin, origin_x, notes, event_start_id, event_end_id)
definitions (id, word_id, position, grammar, usage, body, tags)
types (id, name, type_x, group_)
events (id, name, date, annotation, suffix, notes)
authors (id, initials, full_name, notes)
word_spellings (id, word_id, spelling)
word_affixes (id, word_id, affix)
settings (key, value)
```

**Indexes:**

- `idx_words_name`, `idx_words_name_lower` on words(name)
- `idx_words_type_id`, `idx_words_ev_start`, `idx_words_ev_end`
- `idx_word_spellings_word_id`, `idx_word_affixes_word_id`, `idx_word_affixes_affix`
- `idx_def_word_pos` (covering index on word_id, position)

## FTS Implementation

**Two FTS5 Virtual Tables:**

- `def_fts`: Full-body search, content-linked to definitions table
- `def_kw_fts`: Keyword-only search (text extracted from «» markers)

**Search Functions:**

- `search_english_fts()` - FTS5 full-body search with snippet highlighting
- `search_english_keywords_fts()` - FTS5 keyword-only search
- `search_english_like()` - LIKE-based fallback
- `search_english_keywords_like()` - LIKE keyword-only fallback
- `rebuild_fts()` - Rebuilds both FTS indexes (uses separate connection)
- `extract_keywords()` - Extracts text between «» markers

**FTS Query Building:**

- Phrase search: `"query term"` for multi-word queries
- Prefix search: `term*` for single-word queries

---

_Architecture analysis: 2026-04-01_
