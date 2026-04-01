# Codebase Concerns

**Analysis Date:** 2026-04-01

## Database Migrations

**Migrations are safe but limited:**

- `migrate_words_unique_if_needed` in `src-tauri/src/db.rs` (lines 112-191) handles schema evolution by detecting existing migration flags in the `settings` table
- `migrate_event_columns_if_needed` swaps annotation/notes columns for databases created before the fix
- `add_missing_indexes` (lines 94-106) uses `CREATE INDEX IF NOT EXISTS` to safely add missing indexes

**Limitation:** No rollback mechanism exists for migrations. If a migration fails mid-way, the database may be left in an inconsistent state. The migrations are designed to be idempotent but not transactional across the entire operation.

## FTS Rebuild Performance

**Memory concern:** The `rebuild_fts` function in `src-tauri/src/db.rs` (lines 753-799) loads all definitions into memory:

```rust
let rows: Vec<(i64, String)> = sel.query_map([], ...)?
    .filter_map(std::result::Result::ok)
    .collect();
```

For large databases (100k+ definitions), this could consume significant memory.

**Mitigation in place:** The `rebuild_fts` command in `src-tauri/src/lib.rs` (lines 247-267) opens a separate connection with WAL mode (`PRAGMA synchronous=NORMAL`) to avoid blocking other operations. However, no progress indicator or streaming is provided.

**Unused function:** `fts_update` in `db.rs` (lines 802-829) is marked `#[allow(dead_code)]`. Single-definition FTS updates are not implemented, meaning FTS stays out of sync until a full rebuild.

## Android File Handling

**Content URI handling:** The `import_lod_contents` command in `src-tauri/src/lib.rs` (lines 192-196) receives file contents directly because Android's `content://` URIs cannot be read by `std::fs`.

**Risk:** The frontend reads files via `tauri_plugin_fs` and passes UTF-8 content to the backend. No validation of file size or content length is performed in the Rust command before processing. Large files could cause memory issues or UI freezing.

**Recommendation:** Add size limits in the frontend or accept a streaming/chunked approach.

## Database Path Handling

**Cross-platform path:** `get_default_db_path` in `src-tauri/src/lib.rs` (lines 297-301) uses `app.path().app_data_dir()` to get a writable, persistent path.

**Concern:** On first launch, if `app_data_dir()` fails (rare permission issues), the error bubbles up as a generic string. No fallback location is attempted.

## Error Handling

**Pattern:** All Tauri commands return `Result<T, String>` with a custom `err` helper (line 28-30) that converts Display traits.

**Gap:** Errors are not categorized. A database not found vs. a corrupted database vs. a permission error all return generic string messages. This makes debugging harder for users.

## Security

**No secrets detected:** No API keys, tokens, or credentials in the codebase. No `.env` files or secrets directories found.

**Database is unencrypted:** The SQLite database has no encryption. Anyone with file access can read the dictionary data. This is acceptable for a local dictionary but worth noting if sensitive data is ever added.

## Known Bugs

**No active bugs identified.** No TODO/FIXME/HACK comments found in the codebase.

## Test Coverage Gaps

**Rust tests exist but are limited:**

- `test_in_memory_db_init` tests basic initialization
- `test_get_word_performance_optimal` tests query performance but uses artificial data
- No tests for migrations (running against real old-format databases)
- No tests for import/export functions
- No tests for FTS search or rebuild

---

_Concerns audit: 2026-04-01_
