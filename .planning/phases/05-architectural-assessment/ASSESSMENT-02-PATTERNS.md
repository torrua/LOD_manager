# Design Patterns Assessment

## Patterns Identified

### Command Pattern

- **Location:** `src-tauri/src/lib.rs` — all 28 `#[tauri::command]` functions
- **Usage:** Tauri's IPC mechanism exposes Rust functions as commands callable from the frontend via `invoke()`. Commands like `open_database`, `get_words`, `save_word`, etc.
- **Appropriateness:** Excellent fit for a desktop app. Tauri's command pattern is the idiomatic way to bridge frontend and backend. Commands are thin wrappers delegating to `db.rs` functions.
- **Verdict:** Keep

### Repository Pattern

- **Location:** `src-tauri/src/db.rs` — functions like `list_words`, `get_word`, `save_word`, `rebuild_fts`
- **Usage:** Database operations are encapsulated in module-level functions that take `&Connection`. No ORM — raw SQL with `rusqlite`.
- **Assessment:** Clear separation between data access (`db.rs`) and command layer (`lib.rs`). Commands act as a service layer, calling repository functions. The `with_db`/`with_db_mut` helpers in `lib.rs` provide a clean abstraction over mutex access.
- **Verdict:** Keep

### State Pattern (Global AppState)

- **Location:** `src-tauri/src/lib.rs` — `AppState` struct with `Mutex<Option<Connection>>` and `Mutex<String>` for `db_path`
- **Usage:** Single shared state managed via Tauri's `.manage()` and accessed through `State<'a, AppState>` in commands.
- **Assessment:** Appropriate for a single-user desktop app. The `Option<Connection>` allows the app to start without a database open. The separate `db_path` mutex is necessary for `rebuild_fts` which opens its own connection.
- **Verdict:** Keep

### Observer Pattern (Svelte Reactivity)

- **Location:** Frontend stores in `src/lib/store.svelte.ts` and component runes
- **Usage:** Svelte 5 runes (`$state`, `$derived`, `$effect`) provide reactive state management. Components automatically update when store values change.
- **Assessment:** Svelte's built-in reactivity is the correct observer pattern for this framework. No external state management library needed.
- **Verdict:** Keep

### Strategy Pattern (FTS Search)

- **Location:** `src-tauri/src/db.rs` — `search_english_fts`, `search_english_keywords_fts`, `search_english_like`, `search_english_keywords_like`
- **Usage:** The `search_english` command in `lib.rs` selects between FTS and LIKE strategies based on `use_keywords_only` and `use_like` flags, with automatic fallback from FTS to LIKE.
- **Assessment:** Clean strategy selection with graceful degradation. The FTS→LIKE fallback ensures search works even if FTS indexes are corrupt or not yet built.
- **Verdict:** Keep

### Template Method Pattern (Import)

- **Location:** `src-tauri/src/import.rs` — `import_files` follows a fixed pipeline: types → authors → events → words → definitions → settings
- **Usage:** Import always processes files in dependency order. The `rows()` helper and file-type detection are reused across import types.
- **Assessment:** Appropriate for a fixed-format import. The pipeline is clear and maintainable.
- **Verdict:** Keep

## Anti-Patterns Found

### God Object (Mild)

- **Location:** `src-tauri/src/lib.rs` (~903 lines)
- **Severity:** Low
- **Description:** All 28 Tauri commands live in one file. For a ~28-command app this is acceptable, but the file is approaching the upper limit of readability. The `with_db`/`with_db_mut` helpers mitigate this by keeping command bodies short.

### Tight Coupling in Import

- **Location:** `src-tauri/src/import.rs:113`
- **Severity:** Medium
- **Description:** `let tx = conn.transaction().unwrap()` — uses `.unwrap()` which could panic if the transaction fails to start. All import logic is tightly coupled to this single transaction.

### Primitive Obsession (Minor)

- **Location:** `src-tauri/src/lib.rs` — commands accept `String` for paths, queries, filters
- **Severity:** Low
- **Description:** Raw strings used where typed wrappers (e.g., `DbPath`, `SearchQuery`) could provide validation at the type level. Acceptable for this app size.

### Magic Strings

- **Location:** `src-tauri/src/db.rs` — SQL queries as inline strings
- **Severity:** Low
- **Description:** SQL is embedded directly in Rust code rather than using a query builder or migration tool. This is standard for rusqlite but makes refactoring harder.

## Recommendations

### Pattern Improvements (Proportionate to App Size)

1. **Add `.expect()` messages to unwrap calls** — Replace `.unwrap()` in `import.rs:113` and test code with `.expect("descriptive message")` for better panic messages.

2. **Consider splitting lib.rs** — If command count grows beyond 35, split into `commands/` submodules (e.g., `commands/words.rs`, `commands/events.rs`).

3. **Add a `DbError` type** — Instead of `Result<T, String>`, use `Result<T, DbError>` with variants like `ConnectionError`, `QueryError`, `MigrationError`. This would improve error categorization (noted in CONCERNS.md).

4. **Document complex SQL** — The `get_word` function (db.rs:271-373) uses 4 queries with GROUP_CONCAT and json_group_array. A brief `///` comment explaining the strategy would help future maintainers.
