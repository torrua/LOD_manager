# Concerns

**Analysis Date:** 2026-04-07
**Version analyzed:** 1.6.10

## Summary

The codebase has one critical runtime bug introduced by the lib.rs refactoring into command submodules: `export.rs` queries column names (`grammar`, `tags`, `t.name`) that do not exist in the schema (`grammar_code`, `case_tags`, `type`), meaning HTML export silently returns words with no definitions and no type names. Compounding this, the schema has a two-tier identity problem — `init_schema` creates a `words` table with legacy column names (`type`, `event_start`, `event_end`), while the migration rebuilds the table with modern names (`type_id`, `event_start_id`, `event_end_id`), and different modules query different names depending on whether they were written before or after the migration. Several High concerns carry over from the April 2026 analysis: no migration rollback, blocking `block_on` in the async updater path, excessive debug `println!` left in production commands, and a dual-schema `settings` table. Android-specific concerns around file size validation are partially addressed (100 MB cap added in `commands/import.rs`) but the copy-to-temp pattern for content:// URIs still has residual risks.

---

## Critical Concerns

### C1 — Export Queries Non-Existent Column Names
- **Severity:** Critical
- **Files:** `src-tauri/src/export.rs:200`, `src-tauri/src/export.rs:158,173`
- **Issue:** `generate_html` queries `SELECT word_id, grammar, usage, body, tags FROM definitions` but the schema names those columns `grammar_code` and `case_tags`. It also queries `t.name` on the `types` table but that column is named `type`. The query result is processed with `filter_map(ok)`, so all definition rows fail to deserialize and are silently dropped. The type name column also silently returns NULL for every word.
- **Impact:** Every HTML export produces a document with all definition bodies missing and all type labels blank. This is a data-loss bug from the user's perspective.
- **Fix approach:** Change line 200 to `SELECT word_id, grammar_code, usage, body, case_tags FROM definitions` and change `t.name` to `t.type` on lines 158 and 173.

### C2 — Schema / Query Column Name Divergence Across Modules
- **Severity:** Critical
- **Files:** `src-tauri/src/db.rs:54,63,64`, `src-tauri/src/db.rs:311,315,316,339,342,343,344`, `src-tauri/src/db.rs:1180,1187`, `src-tauri/src/export.rs:161,162,163,175,176`
- **Issue:** `init_schema` creates the `words` table with columns `type` (FK), `event_start`, `event_end`. The migration `migrate_words_unique_if_needed` rebuilds it with `type_id`, `event_start_id`, `event_end_id`. Modules written against the old schema (`db::list_words`, `db::get_word`, `db::save_word`, `db::delete_type`) still query `w.type`, `w.event_start`, `w.event_end`. Modules written after the migration (`db::get_event_words`, `export.rs`) query `w.event_start_id`, `w.event_end_id`, `w.type_id`. Additionally, `add_missing_indexes` tries to create indexes on `words(type_id)`, `words(event_start_id)`, `words(event_end_id)` — columns that do not exist in freshly created (pre-migration) databases.
- **Impact:** On databases that have not been through the migration, `get_event_words`, the export event filter, and `add_missing_indexes` will fail at runtime. On migrated databases, `save_word`, `delete_type`, and `list_words` queries may fail because they still reference the old column names. The application can be left in an inconsistent state silently.
- **Fix approach:** Standardize on one column naming convention (preferably the migration target: `type_id`, `event_start_id`, `event_end_id`). Update `init_schema` to use the new names so fresh databases match the migrated structure. Audit every SQL string in `db.rs`, `export.rs`, `import.rs`, and `converter/converter.rs` against the live column names.

---

## High Concerns

### H1 — Settings Table Has Dual Incompatible Schema
- **Severity:** High
- **Files:** `src-tauri/src/db.rs:99-108`, `src-tauri/src/db.rs:777,788-791`, `src-tauri/src/import.rs:388,395`
- **Issue:** `init_schema` creates `settings` with columns `(date, db_version, last_word_id, db_release)` and a `UNIQUE(date)` constraint. However, migrations and the settings importer also write to it using a completely different `(key, value)` schema via `INSERT INTO settings(key,value)`. These two usages are physically incompatible — `key` and `value` columns do not exist in the defined schema. The `ON CONFLICT(key)` upsert in `upsert_setting` references a constraint that does not exist.
- **Impact:** Migration flag writes (`words_unique_migrated`, `ev_col_migrated`) and settings import will fail with "table settings has no column named key" on databases where the table was created by `init_schema` without a prior manual `ALTER TABLE`. This silently prevents migrations from recording their completion flag, meaning `migrate_words_unique_if_needed` will re-run destructively on every open.
- **Fix approach:** Consolidate the `settings` table to one schema. Either adopt the `(key, value)` form with a `UNIQUE(key)` constraint and drop the multi-column legacy form, or keep both schemas separated into two tables.

### H2 — No Migration Rollback; Destructive Table Rebuild Without Backup
- **Severity:** High
- **Files:** `src-tauri/src/db.rs:199-232`
- **Issue:** `migrate_words_unique_if_needed` drops the `words` table and recreates it via `INSERT OR IGNORE INTO words_new … FROM words`. If the process is interrupted (power loss, crash), the original table is gone and `words_new` may be partially populated. `PRAGMA foreign_keys=OFF` is set during the rebuild but not restored if the batch fails partway through. There is no WAL checkpoint, no backup, and no rollback path.
- **Impact:** Any interruption during migration destroys user data permanently.
- **Fix approach:** Wrap the rebuild in an explicit `BEGIN IMMEDIATE` transaction so SQLite's journal handles atomicity. Add a pre-migration `VACUUM INTO 'backup.db'` or at minimum write a backup copy of the database file before executing the DDL. Restore `PRAGMA foreign_keys=ON` in a finally-equivalent pattern.

### H3 — `block_on` Called on Tauri's Async Runtime Thread
- **Severity:** High
- **Files:** `src-tauri/src/lib.rs:95,102`
- **Issue:** `debug_update_check` uses `tauri::async_runtime::block_on(updater.check())`. Calling `block_on` on a thread already inside a Tokio runtime (which Tauri uses) will panic at runtime with "cannot block the current thread from within an asynchronous context".
- **Impact:** Any invocation of `debug_update_check` from the frontend will panic and crash the Tauri process on desktop builds. This is a desktop-only command so mobile is unaffected.
- **Fix approach:** Make `debug_update_check` an `async` Tauri command and `await` the updater directly, or use `tauri::async_runtime::spawn` and communicate results via the event system.

### H4 — Excessive Production Debug Logging via `println!`
- **Severity:** High
- **Files:** `src-tauri/src/commands/words.rs:14-23,30-34`, `src-tauri/src/commands/authors.rs:9-13`, `src-tauri/src/db.rs:292-295,319-320,324,429,431`, `src-tauri/src/converter/converter.rs` (30+ call sites)
- **Issue:** Every call to `get_words`, `get_word`, `get_authors`, `list_words`, and every line processed by the converter emits `println!` to stdout. In a production desktop application these go to the system console or log file and add I/O overhead proportional to word count on every list refresh and word selection. The frontend `store.svelte.ts` has pervasive `console.log` calls in the same hot paths (`loadWords`, `applyFilter`, `selectWord`).
- **Impact:** Performance degradation at scale; internal data (word names, IDs, counts) leak to process stdout which may be captured by OS logging or developer tools visible to end users.
- **Fix approach:** Gate behind `#[cfg(debug_assertions)]` or replace with a structured log crate (`tracing`). Remove or gate frontend `console.log` calls.

### H5 — `import_contents` Writes to `temp_dir` Then Delegates to `import_files`
- **Severity:** High
- **Files:** `src-tauri/src/import.rs:69-86`
- **Issue:** The Android import path writes all file contents to `std::env::temp_dir()` then calls `import_files` which reads them back from disk. On Android, `temp_dir()` may resolve to a location outside the app sandbox or an unguaranteed path. The cleanup `remove_dir_all` is called regardless of success or failure, but if `import_files` panics the cleanup is skipped. Filename collision is guarded only by process ID (`lod_import_{pid}`), not a UUID, so rapid successive imports from the same process could collide.
- **Fix approach:** On Android, pass content directly to an in-memory import pipeline rather than bouncing through the filesystem. If disk staging is required, use a unique temp path per import and ensure cleanup via Rust's `Drop` or a `scopeguard`.

---

## Medium Concerns

### M1 — FTS Rebuild Opens a Second Database Connection
- **Severity:** Medium
- **Files:** `src-tauri/src/commands/search.rs:47`, `src-tauri/src/commands/search.rs:66`
- **Issue:** `rebuild_fts` and `compact_db` open a second `Connection` to the same database file rather than using the shared `AppState` connection. With WAL mode this is safe for reads, but running `INSERT INTO def_fts(def_fts) VALUES('rebuild')` or `VACUUM` from a second connection while the primary connection holds open statements can cause `SQLITE_BUSY` or lock conflicts on non-WAL databases. The second connection is dropped at end-of-function without explicit close, relying on Rust's `Drop`.
- **Impact:** On databases opened without WAL (e.g., imported external databases), concurrent access from two connections during FTS rebuild may return a busy error that surfaces as a toast without retrying.
- **Fix approach:** Route FTS rebuild and compact through `with_db` / `with_db_mut` to reuse the shared connection, or document that WAL mode is required and enforce it on open.

### M2 — `fts_update` Is `#[allow(dead_code)]` Despite Active Use
- **Severity:** Medium
- **Files:** `src-tauri/src/db.rs:904`, `src-tauri/src/commands/words.rs:64,71,81`
- **Issue:** `fts_update` is annotated `#[allow(dead_code)]`, indicating the compiler was reporting it as unused at some point. It is currently called from `save_definition` and `delete_definition` in `commands/words.rs`. The annotation suppresses a useful warning; if `fts_update` is ever made truly dead again the warning will remain silenced and the FTS index will drift from the definitions table.
- **Fix approach:** Remove the `#[allow(dead_code)]` attribute since the function is actively called.

### M3 — `delete_word` Triggers Full FTS Rebuild
- **Severity:** Medium
- **Files:** `src-tauri/src/commands/words.rs:47-52`
- **Issue:** `delete_word` calls `db::rebuild_fts` which drops and recreates both FTS virtual tables from all definitions. For a database with 10,000+ definitions this is an expensive operation triggered by every single word deletion. Compare to `save_definition` and `delete_definition` which correctly use incremental `fts_update`.
- **Impact:** UI blocking during word deletion on large databases. FTS rebuild on a large dataset holds the WAL write lock for the full duration.
- **Fix approach:** Replace the `rebuild_fts` call in `delete_word` with targeted `fts_update(conn, def_id, "")` calls for each definition belonging to the deleted word (the definitions are cascade-deleted by the FK).

### M4 — Error Types Are All `String` — No Categorization
- **Severity:** Medium
- **Files:** `src-tauri/src/commands/mod.rs:24,26-28`
- **Issue:** `Res<T> = Result<T, String>` converts all errors to strings at the IPC boundary. The frontend receives no structured error type (e.g., "not found" vs "constraint violation" vs "db locked"). Errors from rusqlite, IO, and logic failures are all flattened to the same `String`.
- **Impact:** The UI cannot distinguish recoverable errors (retry, suggest a fix) from fatal ones. Error messages shown in toast notifications contain raw Rust error strings (e.g., `"UNIQUE constraint failed: words.name"`) that are not user-friendly.
- **Fix approach:** Introduce a structured error enum (`NotFound`, `Constraint`, `IoError`, `DbError`) serialized to JSON with a `code` field so the frontend can pattern-match and localize error messages.

### M5 — `autoSelectLatestEvent` Mutates Reactive Array In Place
- **Severity:** Medium
- **Files:** `src/lib/store.svelte.ts:361-377`
- **Issue:** `autoSelectLatestEvent` calls `app.events.sort(...)` which sorts the reactive array in place, reordering the events list in the UI as a side effect of auto-selection on DB open. `.sort()` on a Svelte 5 `$state` array mutates the proxy directly, which may trigger unexpected reactivity cascades.
- **Fix approach:** Use `[...app.events].sort(...)` to sort a copy and only read the first element without mutating the store.

### M6 — Word Rename Does Not Update `word_usage` Cross-References
- **Severity:** Medium
- **Files:** `src-tauri/src/commands/words.rs:40-43`, `src-tauri/src/db.rs:437-534`
- **Issue:** `save_word` updates the word record and syncs affixes/spellings but does not update `word_usage.used_in_word` entries that reference the old name. The `used_in_word` column stores the string name (not a foreign key), so renames leave stale "used in" cross-references that point to the old word name.
- **Impact:** After renaming a word, other words' detail views will show stale "used in" entries with the old name, which may or may not resolve to a valid word.
- **Fix approach:** Add a cascade rename step in `save_word` that executes `UPDATE word_usage SET used_in_word = new_name WHERE used_in_word = old_name` when the name changes.

### M7 — Cargo.toml Version Mismatch
- **Severity:** Medium (cosmetic but affects builds)
- **Files:** `src-tauri/Cargo.toml:3`
- **Issue:** `Cargo.toml` declares `version = "1.6.9"` while the repository is at 1.6.10.
- **Fix approach:** Bump `Cargo.toml` version to match the release version as part of the release process.

---

## Low Concerns

### L1 — Import Transaction Commit Error Is Silently Ignored
- **Severity:** Low
- **Files:** `src-tauri/src/import.rs:372`
- **Issue:** `import_files` calls `let _ = tx.commit()`. If the commit fails (disk full, WAL error), the partial import is silently discarded without surfacing an error to the caller or the user.
- **Fix approach:** Propagate the commit error: `tx.commit().map_err(|e| format!("Import commit failed: {e}"))?;`

### L2 — Android File Read Without Per-File Size Validation
- **Severity:** Low (partially mitigated)
- **Files:** `src/lib/store.svelte.ts:483-488`
- **Issue:** The content:// import path reads file bytes with `readFile(p)` and decodes to UTF-8 before sending to Rust. There is no per-file size check before decoding; the 100 MB aggregate cap in Rust (`commands/import.rs:15-20`) only triggers after all files are already decoded in JS memory. A single 200 MB file will OOM the WebView before reaching the Rust guard.
- **Fix approach:** Add a per-file size check in the JS loop before calling `readFile`, rejecting files above a reasonable limit (e.g., 50 MB).

### L3 — `search_english_like` Fetches `limit * 3` Rows Then Truncates in Rust
- **Severity:** Low
- **Files:** `src-tauri/src/db.rs:1071,1083`
- **Issue:** The LIKE fallback fetches `limit * 3` rows from SQLite then truncates in Rust. With the default `limit = 300` this fetches up to 900 rows and loads them all into a `Vec<ELResult>` before discarding two-thirds.
- **Fix approach:** Use a `ROW_NUMBER() OVER (PARTITION BY word_id)` window function or a `DISTINCT word_id` subquery to deduplicate at the SQL layer and fetch exactly `limit` words.

### L4 — GitHub URI Parsing Uses Fragile Colon Delimiter
- **Severity:** Low
- **Files:** `src/lib/store.svelte.ts:439-452`
- **Issue:** The `github://filename:content` URI scheme uses a colon as the separator between filename and content. Any filename containing a colon would be misparted. The protocol has no validation or escaping.
- **Fix approach:** Use a base64-encoded or JSON-wrapped payload rather than a raw colon delimiter.

### L5 — Test Coverage Gaps in Critical Paths
- **Severity:** Low
- **Files:** `src-tauri/src/lib.rs` (test module), `src-tauri/src/converter/tests.rs`
- **Issue:** No tests cover:
  - The full `generate_html` export pipeline with real `grammar_code`/`case_tags` data (a test would have caught C1 immediately)
  - Pre-migration vs post-migration database compatibility (would reveal C2)
  - The `block_on` updater path panicking (H3)
  - Import error cases: disk full mid-import, malformed UTF-8 in content files
  - The `import_contents` temp-file staging path
- **Fix approach:** Add an integration test that calls `generate_html` on an in-memory DB with a definition row and asserts the output contains the definition body. Add a migration round-trip test using a pre-migration schema fixture.

---

*Concerns audit: 2026-04-07*
