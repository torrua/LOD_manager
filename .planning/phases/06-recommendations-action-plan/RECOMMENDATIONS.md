# Recommendations: Prioritized Action List

## P0 — Must Fix (stability/correctness)

### 1. Replace `.unwrap()` in import transaction

- **Source:** ASSESSMENT-03-STABILITY.md, ASSESSMENT-01-COMPLIANCE.md
- **Problem:** `import.rs:113` uses `conn.transaction().unwrap()` — will panic if database is locked or corrupt, causing app crash during import
- **Action:** Replace with `conn.transaction().map_err(|e| format!("Failed to start import transaction: {e}"))?`
- **Effort:** XS
- **Impact:** High
- **Files:** `src-tauri/src/import.rs:113`

### 2. Handle partial state on database open failure

- **Source:** ASSESSMENT-03-STABILITY.md
- **Problem:** `lib.rs:48-60` — if `migrate_words_unique_if_needed` succeeds but `migrate_event_columns_if_needed` fails, the database is partially migrated but `AppState` is not updated. Subsequent opens may see inconsistent state.
- **Action:** Wrap all migrations in a single transaction, or set a "migration in progress" flag in settings that triggers retry on next open
- **Effort:** S
- **Impact:** High
- **Files:** `src-tauri/src/lib.rs:48-60`, `src-tauri/src/db.rs:112-219`

### 3. Add migration tests

- **Source:** ASSESSMENT-05-TESTING.md
- **Problem:** `migrate_words_unique_if_needed` and `migrate_event_columns_if_needed` are complex table-rebuild operations with zero test coverage. A bug here causes data loss.
- **Action:** Add test functions that create old-schema databases, run migrations, and verify new schema correctness
- **Effort:** S
- **Impact:** High
- **Files:** `src-tauri/src/lib.rs` (add tests under `#[cfg(test)]`)

## P1 — Should Fix (reliability/confidence)

### 4. Add import integration tests

- **Source:** ASSESSMENT-05-TESTING.md
- **Problem:** Core data ingestion (`import_files`, `import_contents`) has zero test coverage. Malformed input handling untested.
- **Action:** Add tests with sample LOD content strings: valid import, missing files, malformed data, empty files
- **Effort:** M
- **Impact:** High
- **Files:** `src-tauri/src/lib.rs` (add tests)

### 5. Add export tests

- **Source:** ASSESSMENT-05-TESTING.md
- **Problem:** `generate_html` and `write_html_to_file` have no tests. HTML output structure unverified.
- **Action:** Test `generate_html` with known data, verify HTML contains expected elements (entries, definitions, affixes)
- **Effort:** S
- **Impact:** Medium
- **Files:** `src-tauri/src/lib.rs` (add tests)

### 6. Add FTS rebuild and search tests

- **Source:** ASSESSMENT-05-TESTING.md
- **Problem:** `rebuild_fts` loads all definitions into memory — untested at scale. `search_english` has 4-way strategy selection — only basic FTS search tested.
- **Action:** Test full FTS cycle: insert → rebuild → search. Test each strategy combination (FTS/LIKE × keywords/full). Test `fts_is_ready` before/after rebuild.
- **Effort:** M
- **Impact:** High
- **Files:** `src-tauri/src/lib.rs` (add tests)

### 7. Create typed error enum

- **Source:** ASSESSMENT-03-STABILITY.md, ASSESSMENT-01-COMPLIANCE.md
- **Problem:** All errors returned as `Result<T, String>` — frontend cannot distinguish "no database open" from "query failed" from "file not found". Must parse string content.
- **Action:** Create `enum AppError { NotOpen, Query(String), IO(String), Migration(String), Import(String) }` with `Display` impl. Replace `type Res<T> = Result<T, String>` with `Result<T, AppError>`.
- **Effort:** M
- **Impact:** Medium
- **Files:** `src-tauri/src/lib.rs`, all command return types

### 8. Add file size validation for Android imports

- **Source:** ASSESSMENT-03-STABILITY.md
- **Problem:** `import_lod_contents` receives file content directly with no size limit. Large files could cause OOM on Android.
- **Action:** Add size check in `import_lod_contents` — reject files > 100MB with clear error message
- **Effort:** XS
- **Impact:** Medium
- **Files:** `src-tauri/src/import.rs:39-71`, `src-tauri/src/lib.rs:209`

## P2 — Nice to Have (maintainability)

### 9. Add module-level documentation

- **Source:** ASSESSMENT-04-MAINTAINABILITY.md
- **Problem:** Only `export.rs` has `//!` module-level docs. `db.rs`, `import.rs`, `lib.rs` have no module documentation.
- **Action:** Add `//!` doc comments at top of each module explaining purpose and key patterns
- **Effort:** S
- **Impact:** Medium
- **Files:** `src-tauri/src/db.rs:1`, `src-tauri/src/import.rs:1`, `src-tauri/src/lib.rs:1`

### 10. Document complex SQL in `get_word`

- **Source:** ASSESSMENT-04-MAINTAINABILITY.md
- **Problem:** `get_word` uses 4 queries with GROUP_CONCAT and json_group_array — optimal but non-obvious. Future maintainers may "simplify" it and degrade performance.
- **Action:** Add `///` comment explaining the 3-query strategy and why it avoids N+1
- **Effort:** XS
- **Impact:** Medium
- **Files:** `src-tauri/src/db.rs:271`

### 11. Report skipped import rows to user

- **Source:** ASSESSMENT-03-STABILITY.md
- **Problem:** Import silently skips malformed rows (`import.rs:196`, `import.rs:282`). User has no way to know data was lost.
- **Action:** Track skipped row count and add warning message to `ImportResult.messages`
- **Effort:** S
- **Impact:** Medium
- **Files:** `src-tauri/src/import.rs`

### 12. Enable WAL mode by default

- **Source:** ASSESSMENT-03-STABILITY.md
- **Problem:** WAL mode only used during `rebuild_fts` (separate connection). Main connection uses default journal mode, which blocks readers during writes.
- **Action:** Add `PRAGMA journal_mode=WAL` to `open_database` after connection opens
- **Effort:** XS
- **Impact:** Medium
- **Files:** `src-tauri/src/lib.rs:50`

## P3 — Future (strategic)

### 13. Split lib.rs into command submodules

- **Source:** ASSESSMENT-02-PATTERNS.md, ASSESSMENT-04-MAINTAINABILITY.md
- **Problem:** `lib.rs` at 903 lines with 28 commands. Acceptable now, but will become unwieldy at 35+ commands.
- **Action:** When command count exceeds 35, split into `commands/words.rs`, `commands/events.rs`, `commands/search.rs`, `commands/import_export.rs`
- **Effort:** L
- **Impact:** Low (for now)
- **Files:** `src-tauri/src/lib.rs`

### 14. Add typed wrappers for raw strings

- **Source:** ASSESSMENT-02-PATTERNS.md
- **Problem:** Commands accept raw `String` for paths, queries, filters. Typed wrappers (e.g., `DbPath`, `SearchQuery`) could provide validation at type level.
- **Action:** Create newtype wrappers with validation constructors. Only worth it if app grows significantly.
- **Effort:** L
- **Impact:** Low
- **Files:** `src-tauri/src/models.rs`, `src-tauri/src/lib.rs`

### 15. Consider frontend test framework

- **Source:** ASSESSMENT-05-TESTING.md
- **Problem:** No frontend tests. For a desktop app, ROI is moderate.
- **Action:** If specific UI bugs justify it, add Vitest for Svelte component tests. Focus on form validation and search behavior. Skip E2E.
- **Effort:** XL
- **Impact:** Low
- **Files:** `src/` (frontend)

## Summary Table

| #   | Priority | Title                                | Effort | Impact | Files                          |
| --- | -------- | ------------------------------------ | ------ | ------ | ------------------------------ |
| 1   | P0       | Replace `.unwrap()` in import        | XS     | High   | `import.rs:113`                |
| 2   | P0       | Handle partial state on open failure | S      | High   | `lib.rs:48-60`, `db.rs`        |
| 3   | P0       | Add migration tests                  | S      | High   | `lib.rs` (tests)               |
| 4   | P1       | Add import integration tests         | M      | High   | `lib.rs` (tests)               |
| 5   | P1       | Add export tests                     | S      | Medium | `lib.rs` (tests)               |
| 6   | P1       | Add FTS rebuild and search tests     | M      | High   | `lib.rs` (tests)               |
| 7   | P1       | Create typed error enum              | M      | Medium | `lib.rs`, all commands         |
| 8   | P1       | Android file size validation         | XS     | Medium | `import.rs`, `lib.rs`          |
| 9   | P2       | Add module-level documentation       | S      | Medium | `db.rs`, `import.rs`, `lib.rs` |
| 10  | P2       | Document complex SQL in get_word     | XS     | Medium | `db.rs:271`                    |
| 11  | P2       | Report skipped import rows           | S      | Medium | `import.rs`                    |
| 12  | P2       | Enable WAL mode by default           | XS     | Medium | `lib.rs:50`                    |
| 13  | P3       | Split lib.rs into submodules         | L      | Low    | `lib.rs`                       |
| 14  | P3       | Add typed wrappers for raw strings   | L      | Low    | `models.rs`, `lib.rs`          |
| 15  | P3       | Consider frontend test framework     | XL     | Low    | `src/` (frontend)              |

**Totals:** 3 P0, 5 P1, 4 P2, 3 P3 | **Effort distribution:** 4 XS, 5 S, 3 M, 2 L, 1 XL
