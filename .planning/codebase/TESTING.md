# Testing Patterns

**Analysis Date:** 2026-04-07

## Summary

LOD Manager has a well-developed Rust unit test suite (22+ tests covering db, import, export, FTS, and migrations) but zero frontend tests. All Rust tests use in-memory SQLite and follow an Arrange-Act-Assert pattern. The `TESTING_PROMPT.md` file (at `/c/Users/User/Downloads/app/.claude/worktrees/thirsty-blackburn/TESTING_PROMPT.md`) documents the intent and template for adding tests. There is no JavaScript/TypeScript test framework configured — `npm run check` (tsc --noEmit) is the only automated frontend validation.

---

## Test Framework

**Rust:**
- Built-in `#[test]` with `cargo test`
- No external test crates (no `mockall`, no `proptest`)
- Assertion via standard `assert!`, `assert_eq!`, `assert!(condition, "message")`

**Frontend:**
- No test framework installed (no vitest, jest, or playwright in `package.json`)
- `npm run check` (TypeScript compiler check only) is the sole automated frontend quality gate

---

## Run Commands

```bash
# Rust — all tests
cargo test --manifest-path src-tauri/Cargo.toml

# Rust — single test by name
cargo test --manifest-path src-tauri/Cargo.toml -- test_name

# Rust — with stdout (println! output visible)
cargo test --manifest-path src-tauri/Cargo.toml -- --nocapture

# Frontend — type check only (no runtime tests)
npm run check

# Full CI check (format + lint + type check)
npm run ci:check
```

---

## Test File Locations

**Rust tests:**
- `src-tauri/src/lib.rs` — primary `#[cfg(test)]` module, 22 tests covering db + import + export
- `src-tauri/src/converter/tests.rs` — 3 converter-specific tests in a separate `tests.rs` module

**Frontend tests:**
- None. No `*.test.ts`, `*.spec.ts`, or `*.test.svelte` files exist in `src/`.

---

## Test Structure (Rust)

All tests follow a consistent Arrange-Act-Assert pattern using in-memory SQLite:

```rust
#[test]
fn test_feature_name() {
    // Setup: always start with in-memory database
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    db::init_schema(&conn).unwrap();
    db::init_fts(&conn).unwrap();  // only when FTS is needed

    // Arrange: insert test fixtures directly via SQL
    conn.execute("INSERT INTO types (name) VALUES ('gismu')", []).unwrap();
    let type_id: i64 = conn.last_insert_rowid();

    // Act: call the db::* function under test
    let words = db::list_words(&conn, "a", "", None).unwrap();

    // Assert: verify results
    assert_eq!(words.len(), 1);
    assert_eq!(words[0].name, "abc");
}
```

Tests with mutable connection (required for import):
```rust
#[test]
fn test_import_feature() {
    let mut conn = rusqlite::Connection::open_in_memory().unwrap();
    db::init_schema(&conn).unwrap();
    let result = import::import_files(&mut conn, &paths).unwrap();
    assert_eq!(result.types, 2);
}
```

---

## What Is Tested (Rust)

### Database — `src-tauri/src/lib.rs`

| Test | Coverage |
|------|----------|
| `test_in_memory_db_init` | Schema creation, `init_fts`, `Start` event seeded |
| `test_get_word_performance_optimal` | `db::get_word` correctness + 100-call performance benchmark (<1000ms) |
| `test_fts_update_incremental` | `db::fts_update` after `rebuild_fts` |
| `test_list_words_basic` | `db::list_words` with prefix filter, wildcard, type filter, event filter |
| `test_fts_search_basic` | `db::search_english_fts` after rebuild |
| `test_word_crud_operations` | Full word insert → read → add definitions → delete cycle |
| `test_definition_crud_operations` | `db::save_definition`, `db::delete_definition` |
| `test_type_crud_operations` | Type insert, update, referential integrity (cannot delete type with words) |
| `test_event_crud_operations` | Event insert, update, list |
| `test_author_crud_operations` | Full author CRUD including `db::delete_author` |
| `test_word_affixes_and_spellings` | `word_affixes` and `word_spellings` insert + partial delete |
| `test_migrate_words_unique_if_needed` | Migration idempotency + settings flag written |
| `test_migrate_event_columns_if_needed` | Column swap migration + idempotency |
| `test_fts_rebuild_and_search` | `db::rebuild_fts` + FTS + LIKE fallback (`search_english_like`) |
| `test_fts_update_after_save` | FTS incremental update after `db::save_definition` |
| `test_search_english_strategies` | FTS search with `«keyword»` markers |

### Export — `src-tauri/src/lib.rs`

| Test | Coverage |
|------|----------|
| `test_export_html_empty` | `export::generate_html` with no words → "No words found" |
| `test_export_html_with_data` | HTML output contains word name, definition, type name |

### Import — `src-tauri/src/lib.rs`

| Test | Coverage |
|------|----------|
| `test_import_files_empty_paths` | Empty paths slice → zero results, no panic |
| `test_import_files_malformed_data` | `@`-delimited file with blank/bad lines → partial import |
| `test_import_contents_android` | `import_contents()` (in-memory path for Android) |
| `test_import_skipped_rows_counted` | `skipped_rows` counter and "Skipped" message in result |
| `test_import_no_skipped_rows_clean_data` | Clean file produces `skipped_rows == 0` |
| `test_import_skipped_rows_empty_file` | Empty file produces zero counts, no panic |

### Converter — `src-tauri/src/converter/tests.rs`

| Test | Coverage |
|------|----------|
| `test_convert_text_files` | Full conversion from fixture directory to SQLite (requires `../test_converter` dir) |
| `test_convert_nonexistent_directory` | Returns error for missing path |
| `test_convert_empty_directory` | Returns error for directory with no LOD files |

---

## What Is NOT Tested

### Tauri Command Layer

The `commands/*.rs` wrappers are not tested. They are thin but the Mutex/State wiring around `AppState` is untested:
- `commands/database.rs` — `open_database`, `create_database`, WAL pragma, migration calls
- `commands/words.rs` — FTS update after `save_definition` inline logic
- `commands/search.rs` — `search_english` dispatch (FTS vs LIKE vs keywords-only mode)
- `commands/import.rs`, `commands/export.rs` — command-level paths

### Frontend / TypeScript

No frontend tests exist. The following behaviors are entirely untested:
- `src/lib/store.svelte.ts` — all application logic: `applyFilter`, `selectWord`, wildcard search, history push/pop, `autoSelectLatestEvent`, debounced search, update check/install flow
- `src/lib/text.ts` — `renderBody` (LOD formatting), `esc` (HTML escaping) — these are pure functions and are the easiest to add tests for
- All Svelte components — form validation in `WordForm.svelte` (`validate()`), keyboard navigation in `Sidebar.svelte`, Android content:// URI handling in `importFiles()`

### Edge Cases Not Covered in Rust Tests

- Concurrent database access (two simultaneous writes)
- Very large imports (performance with 10 000+ words)
- `get_event_words` command
- `compact_db` (VACUUM)
- Export with an event filter applied (`export_html_to_file` with non-null event name)
- `db::add_missing_indexes` (called during open but not tested independently)

---

## Testing Conventions

**In-memory database setup (required boilerplate for every Rust test):**
```rust
let conn = rusqlite::Connection::open_in_memory().unwrap();
db::init_schema(&conn).unwrap();
// Add db::init_fts(&conn).unwrap() only if test uses FTS search
```

**Temp file tests (for file-based import tests):**
Tests that need real files create temp directories under `std::env::temp_dir()` and clean up with `std::fs::remove_dir_all` at the end.

**Performance tests:**
Use `std::time::Instant` and assert total elapsed time: `assert!(duration.as_millis() < 1000, "message")`. The benchmark test (`test_get_word_performance_optimal`) runs the operation 100 times.

**Asserting errors:**
```rust
let result = db::get_word(&conn, 999_999);
assert!(result.is_err(), "Invalid word ID should return error");
```

**Asserting messages in ImportResult:**
```rust
let has_msg = result.messages.iter().any(|m: &String| m.contains("Skipped"));
assert!(has_msg, "Should have skipped rows message, got: {:?}", result.messages);
```

---

## Test Coverage Gaps by Priority

**High priority (pure functions, easy to add):**
- `src/lib/text.ts`: `renderBody()` and `esc()` — pure functions with no Tauri dependency. A vitest setup would let these be tested immediately.
- `applyFilter()` logic in `src/lib/store.svelte.ts` — wildcard and prefix matching logic is non-trivial and currently untested.

**Medium priority (requires mock or integration setup):**
- `src/lib/store.svelte.ts` store functions — require mocking `invoke()` or a full Tauri test driver
- `commands/search.rs` — `search_english` routing logic (FTS vs LIKE vs keywords-only) deserves direct coverage

**Lower priority (covered implicitly by command layer):**
- `db.rs` CRUD functions are well covered. Additional edge cases (null fields, very long text) could be added.

---

## Adding Tests

**To add a new Rust unit test:**
Add it to the `#[cfg(test)]` block in `src-tauri/src/lib.rs`. Reference `crate::db`, `crate::import`, `crate::models`, etc. directly:

```rust
#[test]
fn test_new_feature() {
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    db::init_schema(&conn).unwrap();

    // Arrange
    // Act
    // Assert
}
```

**To add frontend tests (requires setup first):**
No test runner is configured. To add vitest:
```bash
npm install -D vitest @testing-library/svelte
```
Then create `src/lib/text.test.ts` adjacent to `text.ts`, or `src/lib/store.test.ts` for store logic.
