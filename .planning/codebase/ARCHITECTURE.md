# Architecture

**Analysis Date:** 2026-04-07

## Summary

LOD Manager is a Tauri v2 desktop/mobile application built with a Svelte 5 frontend and a Rust backend. The overall pattern is a command-based IPC bridge: the frontend calls named Tauri commands via `invoke()`, the Rust backend executes SQLite operations, and serialized results flow back as typed JSON. Global UI state is held in a single Svelte 5 `$state` object exported from `src/lib/store.svelte.ts`. The Rust backend manages a single shared SQLite connection wrapped in `Mutex<Option<Connection>>` inside `AppState`, injected as Tauri managed state and accessed via `with_db` / `with_db_mut` helpers.

---

## Overall Pattern

**Type:** Tauri command/IPC architecture

**Key characteristics:**
- No REST API — all frontend-to-backend communication goes through Tauri's `invoke()` IPC bridge
- Single SQLite database file opened at runtime; path stored in `AppState.db_path`
- All commands return `Res<T>` (`Result<T, String>`) — errors surface as rejected JavaScript promises
- No routing library — panel visibility is controlled by `app.panel` and `app.tab` strings in the global store

---

## Layers

**Frontend — View Layer:**
- Purpose: Render UI, respond to user actions, display store state
- Location: `src/lib/components/`
- Contains: Svelte 5 components (`*.svelte`)
- Depends on: store functions and `app` state from `src/lib/store.svelte.ts`
- Used by: `src/App.svelte` (root mount point)

**Frontend — Store / Application Layer:**
- Purpose: All business logic, IPC calls, and derived state
- Location: `src/lib/store.svelte.ts`
- Contains: The single exported `app` reactive object, exported action functions (e.g., `openDb`, `selectWord`, `importFiles`), and localStorage persistence helpers
- Depends on: `@tauri-apps/api/core` (`invoke`), Tauri plugins
- Used by: All Svelte components

**Frontend — Types:**
- Purpose: Shared TypeScript interface definitions mirroring Rust model structs
- Location: `src/types.ts`
- Contains: `WordListItem`, `WordDetail`, `Definition`, `EventItem`, `TypeItem`, `AuthorItem`, `DbStats`, `ELResult`, `ELSearchParams`, `ImportResult`, `SearchMode`, `Tab`

**Backend — Command Layer:**
- Purpose: Tauri `#[tauri::command]` handlers grouped by domain
- Location: `src-tauri/src/commands/`
- Contains: Eight submodules (see Backend Modules section)
- Depends on: `db.rs`, `models.rs`, `export.rs`, `import.rs`
- Used by: `lib.rs` via `tauri::generate_handler![]`

**Backend — Data Access Layer:**
- Purpose: All SQLite operations — schema init, CRUD, FTS, migrations
- Location: `src-tauri/src/db.rs`
- Contains: `init_schema`, `init_fts`, `list_words`, `get_word`, `save_word`, `delete_word`, `save_definition`, `delete_definition`, `list_events`, `save_event`, `delete_event`, `list_types`, `save_type`, `delete_type`, `list_authors`, `save_author`, `delete_author`, `search_english_fts`, `search_english_like`, `rebuild_fts`, `fts_is_ready`, `fts_update`, `vacuum_db`, migration helpers
- Depends on: `models.rs`, `rusqlite`
- Used by: all `commands/` submodules

**Backend — Models:**
- Purpose: Shared Rust structs with `serde::Serialize`/`Deserialize` for IPC serialization
- Location: `src-tauri/src/models.rs`
- Contains: `WordListItem`, `WordDetail`, `Definition`, `SaveWord`, `SaveDefinition`, `EventItem`, `SaveEvent`, `TypeItem`, `SaveType`, `AuthorItem`, `SaveAuthor`, `ImportResult`, `DbStats`, `AppInfo`, `ELResult`, `ELSearchParams`, `SettingItem`

**Backend — Domain Services:**
- Purpose: Complex operations that are not pure CRUD
- Location: `src-tauri/src/export.rs`, `src-tauri/src/import.rs`, `src-tauri/src/converter/`
- Contains: HTML generation (`export.rs`), `@`-delimited LOD file parsing (`import.rs`), text-to-SQLite batch converter (`converter/converter.rs`)

---

## Backend Modules (commands/)

After the lib.rs refactor (commit 975d1bb), all Tauri commands are organized in `src-tauri/src/commands/` as a module tree:

| File | Commands | Notes |
|---|---|---|
| `commands/mod.rs` | Shared types only | Defines `AppState`, `Db<'a>`, `Res<T>`, `err()`, `with_db`, `with_db_mut` |
| `commands/database.rs` | `open_database`, `create_database`, `get_db_stats`, `get_default_db_path` | Handles connection lifecycle, runs all migrations on open |
| `commands/words.rs` | `get_words`, `get_word`, `save_word`, `delete_word`, `save_definition`, `delete_definition` | Saves return the updated `WordDetail` so the frontend can refresh in one round-trip |
| `commands/events.rs` | `get_events`, `save_event`, `delete_event`, `get_event_words` | |
| `commands/types.rs` | `get_types`, `save_type`, `delete_type` | |
| `commands/authors.rs` | `get_authors`, `save_author`, `delete_author` | Mutations return `Vec<AuthorItem>` (full list refresh) |
| `commands/search.rs` | `search_english`, `rebuild_fts`, `compact_db`, `fts_is_ready` | FTS operations open a second connection to avoid locking the main Mutex |
| `commands/export.rs` | `export_html`, `export_html_to_file` | Delegates to `export::generate_html` / `export::write_html_to_file` |
| `commands/import.rs` | `import_lod_contents`, `import_lod_files`, `convert_text_files` | `import_lod_contents` accepts `(filename, text)` pairs for Android; `import_lod_files` uses filesystem paths for desktop |

---

## State Management

**Rust side:**

`AppState` is defined in `src-tauri/src/commands/mod.rs` and registered with `.manage()` in `lib.rs`:

```rust
pub struct AppState {
    pub db: Mutex<Option<Connection>>,
    pub db_path: Mutex<String>,
}
```

Commands receive it as `state: Db<'_>` (a `tauri::State<AppState>` alias). Two helpers abstract locking:

```rust
pub fn with_db<T, F: FnOnce(&Connection) -> rusqlite::Result<T>>(state: &AppState, f: F) -> Res<T>
pub fn with_db_mut<T, F: FnOnce(&mut Connection) -> rusqlite::Result<T>>(state: &AppState, f: F) -> Res<T>
```

FTS rebuild and database compaction open a dedicated second connection on the stored `db_path` to avoid contention with the shared mutex connection.

**Frontend side:**

All state lives in a single exported `$state` object in `src/lib/store.svelte.ts`:

```typescript
export const app = $state({
  dbOpen: false,
  dbPath: '',
  words: [] as WordListItem[],
  filteredWords: [] as WordListItem[],
  curWord: null as WordDetail | null,
  tab: 'words' as Tab,
  panel: 'welcome' as string,
  // ... (all UI and data state in one object)
});
```

Svelte 5 `$state` makes the object deeply reactive. Components import `app` and read `app.field` directly. No Svelte stores (`writable`, `readable`) are used. Derived values use `$derived` inside components (e.g., `vStart`/`vEnd` in `Sidebar.svelte` for virtual scrolling) or plain computed functions exported from the store (e.g., `getActiveEvent()`, `canGoBack()`).

Persistence to `localStorage`:
- Theme: `lod-theme`
- Read-only mode: `lod-ro`
- Last opened DB path: `lod-last-db`
- User preferences: `lod-prefs` (JSON object)
- Sidebar width: `sb-w`

---

## Data Flow

**Standard read (L->E word list):**
1. User opens DB -> `openDb(path)` in store calls `invoke('open_database', { path })`
2. Rust `open_database` opens SQLite, runs schema init and migrations, stores `Connection` in `AppState.db`
3. Store calls `loadAll()` which parallel-invokes `get_words`, `get_types`, `get_events`, `get_authors`
4. Rust handlers call `db::list_*` functions, serialize results to JSON
5. Store sets `app.words`, `app.types`, `app.events`, `app.authors`
6. Components read from `app` reactively — no explicit subscriptions needed

**Word detail view:**
1. User clicks word in `Sidebar.svelte` -> `selectWord(id)`
2. Store sets `app.loadingWordId = id`, calls `invoke('get_word', { id })`
3. Rust calls `db::get_word` which runs 3 queries (word row, affixes+spellings, definitions)
4. Store sets `app.curWord = word`, `app.panel = 'word'`
5. `App.svelte` conditionally renders `<WordDetail>` or `<WordForm>` based on `app.panel` and `app.editing`

**E->L search:**
1. User types in search box with `searchMode === 'el'`
2. `searchEnglishDebounced(q)` debounces 250ms then calls `invoke('search_english', { params })`
3. Rust `search_english` tries FTS5, falls back to LIKE if empty result
4. Store sets `app.elResults`; `Sidebar.svelte` renders `<ELResults>`

**Write (save word):**
1. `WordForm.svelte` submits -> `saveWord(id, data)` in store
2. `invoke('save_word', { id, data })` -> Rust upserts word, returns refreshed `WordDetail`
3. Store sets `app.curWord = result`, then calls `loadWords()` to refresh the list

**Import flow:**
1. User selects files in `ToolsDrawer.svelte`
2. On desktop: `invoke('import_lod_files', { paths })` -> Rust reads files via `std::fs`, parses `@`-delimited format, inserts in transaction order (types -> authors -> events -> words -> definitions -> settings)
3. On Android (content:// URIs): JS reads files as bytes, decodes to UTF-8, passes `(name, text)` pairs to `invoke('import_lod_contents', { files })`
4. After import: store calls `loadAll()` and `autoSelectLatestEvent()`

---

## Error Handling

**Strategy:** All commands return `Res<T>` = `Result<T, String>`. Errors convert via `err(e: impl Display) -> String`. The frontend catches rejected promises in `try/catch` blocks and calls `toast(msg, 'err')`.

**Patterns:**
- `with_db` converts `rusqlite::Error` to `String` via `err()`
- Import and export functions return `Result<T, String>` directly
- FTS search silently falls back to LIKE on error (both in Rust command and in frontend `searchEnglishNow`)
- Android content:// errors throw with a user-readable message suggesting to use "New Database + Import" instead

---

## Frontend Component Hierarchy

```
src/main.ts
└── src/App.svelte                              # Root: layout, keyboard shortcuts, DB auto-open
    ├── src/lib/components/Sidebar.svelte       # Left panel: word/event list, search, virtual scroll
    │   └── src/lib/components/ELResults.svelte # E->L search results list
    ├── src/lib/components/WordDetail.svelte    # Word read view
    ├── src/lib/components/WordForm.svelte      # Word create/edit form
    ├── src/lib/components/EventDetail.svelte   # Event read view
    ├── src/lib/components/EventForm.svelte     # Event create/edit form
    ├── src/lib/components/TypesPanel.svelte    # Types tab content
    ├── src/lib/components/AuthorsPanel.svelte  # Authors tab content
    ├── src/lib/components/ToolsDrawer.svelte   # Slide-out drawer: import/export/database/settings
    ├── src/lib/components/DeleteModal.svelte   # Confirmation modal
    ├── src/lib/components/Toast.svelte         # Transient notification overlay
    └── src/lib/components/Icon.svelte          # SVG icon wrapper
```

`App.svelte` uses `{#if app.panel === '...'}` conditionals to show exactly one main panel at a time. The tab bar (`app.tab`: `'words'|'events'|'types'|'authors'`) controls which list is active in the sidebar.

---

## Cross-Cutting Concerns

**Logging:** `println!` / `eprintln!` to stderr in Rust commands (development-level). No structured logging framework.

**Validation:** Input validation happens implicitly via rusqlite constraint errors (UNIQUE, FK) which bubble up as `Res<T>` errors. No explicit validation layer.

**Platform detection:** `initPlatform()` in the store calls `@tauri-apps/plugin-os` `platform()` and sets `app.currentPlatform`. Conditional behavior branches on `'android'` for DB path handling and `'android'|'ios'` to skip update checks.

**Auto-update:** Desktop-only. Uses `tauri_plugin_updater`. Check triggered manually or on startup. `debug_update_check` command defined directly in `lib.rs` with `#[cfg(desktop)]`.

**Migrations:** Run automatically on `open_database`. Three idempotent migration functions in `db.rs`: `migrate_words_unique_if_needed`, `migrate_event_columns_if_needed`, `add_missing_indexes`. Each sets a flag in the `settings` table to avoid re-running.

---

*Architecture analysis: 2026-04-07*
