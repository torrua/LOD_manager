# Stability Assessment

**Assessment Date:** 2026-04-04

---

## Risk Register

| Area               | Risk                                                                       | Severity   | Evidence                                                   | Mitigation Status                                  |
| ------------------ | -------------------------------------------------------------------------- | ---------- | ---------------------------------------------------------- | -------------------------------------------------- |
| **Error Handling** | Generic error messages - all errors return `String` without categorization | **Medium** | lib.rs:28-30 `err()` helper converts any Display to String | Not mitigated - may make debugging harder          |
| **Error Handling** | No stack traces on errors                                                  | **Low**    | Errors converted to strings, no backtrace                  | Not mitigated - acceptable for user-facing app     |
| **Database**       | No rollback for migrations                                                 | **Medium** | CONCERNS.md: "No rollback mechanism" db.rs:112-191         | Mitigated: idempotent, uses flags                  |
| **Database**       | FTS rebuild loads all definitions to memory                                | **High**   | CONCERNS.md: db.rs:753-799 loads Vec<(i64, String)>        | Partially mitigated: WAL mode, separate connection |
| **Database**       | Large file import without size validation                                  | **Medium** | CONCERNS.md: lib.rs:192-196, no size check                 | Not mitigated - could freeze UI                    |
| **Database**       | Race condition in FTS update                                               | **Low**    | db.rs:802-829 fts_update marked #[allow(dead_code)]        | Unused - not an issue                              |
| **Concurrency**    | Mutex blocks thread during DB operations                                   | **Low**    | AppState uses std::sync::Mutex (not async)                 | Acceptable for desktop app                         |
| **State**          | No fallback if app_data_dir() fails                                        | **Low**    | lib.rs:297-301                                             | Error bubbles up to user                           |

---

## Edge Case Analysis

### Empty Database

| Scenario                  | Current Behavior                  | Assessment          |
| ------------------------- | --------------------------------- | ------------------- |
| `get_words` with empty DB | Returns empty `Vec<WordListItem>` | ✓ Works correctly   |
| `get_word` with ID 0      | Returns "Word not found" error    | ✓ Graceful          |
| Search with empty query   | Returns all words                 | ✓ Expected behavior |
| Export HTML with empty DB | Generates empty HTML file         | ✓ No crash          |

### Large Datasets

| Scenario                         | Current Behavior                           | Assessment                                  |
| -------------------------------- | ------------------------------------------ | ------------------------------------------- |
| 10k+ words in list               | Loads all via `get_words`, paginated in UI | ⚠️ Could be slow - but UI handles rendering |
| 100k+ definitions in FTS rebuild | Loads ALL to memory (db.rs:753-799)        | ⚠️ **High severity** - memory concern       |
| Search with 1000+ results        | Returns up to 300 (code line 534)          | ✓ Limited to prevent overload               |

### Malformed Input

| Scenario                           | Current Behavior                                | Assessment                             |
| ---------------------------------- | ----------------------------------------------- | -------------------------------------- |
| Import invalid LOD format          | `import::parse_lod_file` returns error          | ✓ Errors handled                       |
| Save word with null required field | SQLite NOT NULL constraint may fail             | ⚠️ Not tested - needs validation layer |
| Search with invalid regex          | Falls back to LIKE search (store.svelte.ts:540) | ✓ Graceful fallback                    |
| Unicode in definitions             | SQLite stores UTF-8 natively                    | ✓ Works                                |

### Special Characters

| Scenario                           | Current Behavior                            | Assessment  |
| ---------------------------------- | ------------------------------------------- | ----------- |
| Search with `'` (apostrophe)       | Escaped by regex in store.svelte.ts:256-261 | ✓ Handled   |
| Search with `*` or `?` (wildcards) | Converted to regex (`.*`, `.`)              | ✓ Supported |
| Non-ASCII Loglan characters        | Stored as UTF-8                             | ✓ Works     |

---

## Cross-Platform Notes

### Windows

| Concern       | Status                                              |
| ------------- | --------------------------------------------------- |
| Path handling | ✓ Uses `app.path().app_data_dir()` - cross-platform |
| File dialogs  | ✓ Uses tauri dialog plugin                          |
| Line endings  | ✓ SQLite handles internally                         |

### Android

| Concern                     | Status                                        |
| --------------------------- | --------------------------------------------- |
| Content URIs (`content://`) | ✓ Handled in store.svelte.ts:144-157          |
| File size limits            | ⚠️ **Not validated** - CONCERNS.md notes this |
| No file system access       | ✓ Content passed as string to backend         |
| App data directory          | ✓ Works correctly                             |

### iOS

| Concern      | Status                          |
| ------------ | ------------------------------- |
| File access  | Similar to Android (not tested) |
| Notarization | N/A - development build         |

---

## Critical Risks

### High Priority

1. **FTS Rebuild Memory Issue** (CONCERNS.md)
   - Severity: **High**
   - Evidence: `db.rs:753-799` loads all definitions into `Vec`
   - Impact: App could crash or become unresponsive with large DBs (100k+ definitions)
   - Current mitigation: Separate connection with WAL mode
   - Recommendation: Implement streaming/chunked rebuild

2. **Import File Size Not Validated**
   - Severity: **Medium**
   - Evidence: No size check in `import_lod_contents` (lib.rs:192-196)
   - Impact: Large files could freeze UI or exhaust memory
   - Current mitigation: None
   - Recommendation: Add frontend size limit (e.g., 50MB)

### Medium Priority

3. **Generic Error Messages**
   - Severity: **Medium**
   - Evidence: `err()` helper (lib.rs:28-30) converts all errors to strings
   - Impact: Harder to debug specific issues
   - Current mitigation: None
   - Recommendation: Use error enum for common cases

4. **No Migration Rollback**
   - Severity: **Medium**
   - Evidence: CONCERNS.md documents no rollback
   - Impact: Failed migration leaves DB in inconsistent state
   - Current mitigation: Idempotent, uses settings flags
   - Recommendation: Consider transaction wrapping

---

## Recommendations

### Immediate (High Priority)

1. **Add file size validation for imports**
   - File: Frontend or `import_lod_contents`
   - Action: Reject files > 50MB with clear error message

2. **Implement FTS streaming (if DB > 50k definitions)**
   - File: `db.rs::rebuild_fts`
   - Action: Use SQLite transaction + batch inserts instead of loading all to memory

### Soon (Medium Priority)

3. **Categorize errors** — Add enum for common error types instead of generic strings
4. **Add migration transaction** — Wrap migrations in transaction for atomicity

### Later (Low Priority)

5. **Add connection timeout** — Prevent hung queries from blocking indefinitely
6. **Add query logging for debugging** — Conditional debug logging for troubleshooting

---

## Summary

| Severity     | Count | Top Issues                                              |
| ------------ | ----- | ------------------------------------------------------- |
| **Critical** | 0     | None                                                    |
| **High**     | 1     | FTS rebuild memory (large DBs)                          |
| **Medium**   | 3     | File size validation, error categorization, no rollback |
| **Low**      | 3     | Stack traces, FTS unused, no fallback path              |

**Overall Assessment:** The codebase is reasonably stable for a dictionary manager. The main risks are around large dataset handling (FTS rebuild) and input validation (file size). These are manageable for the app's use case.
