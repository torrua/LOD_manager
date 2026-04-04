# Stability Assessment

## Risk Register

| Area                   | Risk                                               | Severity | Evidence                                             | Mitigation Status                                                  |
| ---------------------- | -------------------------------------------------- | -------- | ---------------------------------------------------- | ------------------------------------------------------------------ |
| Database connection    | `open_database` can leave partial state on failure | High     | `lib.rs:48-60` — state set after migrations          | None — if migration fails after schema init, state is inconsistent |
| Transaction unwrap     | `import_files` uses `.unwrap()` on transaction     | High     | `import.rs:113`                                      | None — panics on failure                                           |
| FTS rebuild memory     | `rebuild_fts` loads all definitions into memory    | Medium   | `db.rs:800-813` — `Vec<(i64, String)>`               | Uses separate connection with WAL mode                             |
| Migration rollback     | No rollback if migration fails mid-execution       | Medium   | `db.rs:150-183` — table rebuild                      | `PRAGMA foreign_keys=OFF` during rebuild                           |
| Android file size      | No validation of file size before import           | Medium   | `import.rs:39-71`                                    | None                                                               |
| Error categorization   | All errors returned as generic `String`            | Medium   | All commands use `Res<T>` = `Result<T, String>`      | Frontend cannot distinguish error types                            |
| Concurrent FTS rebuild | `rebuild_fts` opens separate connection            | Low      | `lib.rs:264-284`                                     | WAL mode reduces contention                                        |
| Empty database         | Commands handle empty DB gracefully                | Low      | `lib.rs:34` — `ok_or("No database open.")`           | Proper error returned                                              |
| Malformed LOD input    | Import silently skips malformed rows               | Low      | `import.rs:196-197` — `if r.len() < 2 { continue; }` | No error reported to user                                          |
| Unicode handling       | FTS5 uses `unicode61 remove_diacritics 1`          | Low      | `db.rs:724-725`                                      | Good — handles non-ASCII                                           |

## Edge Case Analysis

### Empty Database

- `get_words("")` returns empty vec — handled correctly
- `get_word(999999)` returns error — handled correctly (`lib.rs:92-94`)
- `search_english("")` returns empty vec — handled correctly (`lib.rs:229-231`)

### Large Datasets

- `get_word` uses 3-query approach — optimal for large DBs (`db.rs:271-373`)
- `list_words` uses single query with covering index — efficient (`db.rs:237-269`)
- `export_html` uses 4 bulk queries instead of N+1 — handles 10k+ words well (`export.rs:4-6`)
- `rebuild_fts` loads ALL definitions into memory — concern at 100k+ definitions (`db.rs:800-801`)

### Malformed Input

- Import: rows with insufficient columns are silently skipped (`import.rs:196`, `import.rs:282-283`)
- Import: empty body definitions are skipped (`import.rs:290-292`)
- Import: no validation of file encoding — assumes UTF-8 (`import.rs:117`, `import.rs:193`)
- Search: special characters handled via FTS5 sanitization (`db.rs:1051-1058`)

### Unicode Handling

- FTS5 configured with `unicode61 remove_diacritics 1` — good for Loglan
- LIKE fallback uses `LOWER()` — works for ASCII but may not handle all Unicode case folding
- Keyword extraction uses char indices — correct for Unicode (`db.rs:744-765`)

## Cross-Platform Notes

### Windows

- Path handling: raw strings passed to `Connection::open()` — works on Windows with proper path separators
- File dialogs: `tauri_plugin_dialog` used — native Windows dialogs

### Android

- Content URIs: `import_lod_contents` receives file content directly (frontend reads via plugin-fs) — correct approach
- Temp directory: `import_contents` writes to temp dir (`import.rs:53-69`) — may fail on restricted Android storage
- No file size validation — large files could cause OOM

### Desktop (macOS/Linux)

- `import_lod_files` uses `std::fs::read_to_string` — works on all POSIX systems
- `get_default_db_path` uses `app.path().app_data_dir()` — cross-platform correct

## Critical Risks

### 1. Transaction Panic in Import (High)

**Location:** `import.rs:113`
**Impact:** Complete import failure with panic if database is locked or corrupt
**Current Handling:** None — `.unwrap()` will panic

### 2. Partial State on Open Failure (High)

**Location:** `lib.rs:48-60`
**Impact:** If `migrate_words_unique_if_needed` succeeds but `migrate_event_columns_if_needed` fails, `AppState` is not updated but database is partially migrated
**Current Handling:** Error returned to caller, but DB is in migrated state

### 3. No Error Categorization (Medium)

**Location:** All commands
**Impact:** Frontend cannot distinguish between "no database open", "query failed", and "file not found"
**Current Handling:** All errors are generic strings — frontend must parse string content

## Recommendations

### Priority Stability Improvements

1. **Replace `.unwrap()` in import.rs:113** with proper error handling
2. **Add file size validation** for Android imports (suggest 100MB limit)
3. **Create error type enum** with variants: `NotOpen`, `Query`, `IO`, `Migration`, `Import`
4. **Add import row error counting** — report how many rows were skipped and why
5. **Consider WAL mode by default** — `PRAGMA journal_mode=WAL` in `open_database` for better concurrent access
