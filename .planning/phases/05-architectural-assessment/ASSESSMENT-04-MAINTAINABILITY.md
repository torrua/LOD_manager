# Maintainability Scorecard

## Dimension Scores

| Dimension     | Score (1-10) | Justification                                                                                                                              |
| ------------- | ------------ | ------------------------------------------------------------------------------------------------------------------------------------------ |
| Organization  | 8            | Clean module separation (lib/db/models/import/export). lib.rs at 903 lines is manageable but approaching limit                             |
| Coupling      | 7            | Commands depend directly on db.rs functions. `with_db`/`with_db_mut` helpers reduce coupling. No abstraction layer between commands and DB |
| Documentation | 5            | Only `export.rs` has module-level `//!` docs. No `///` comments on complex functions like `get_word` or migrations                         |
| Extensibility | 7            | Adding new word fields requires changes in models.rs, db.rs, and frontend types. Adding new entity requires new module                     |

**Overall Score: 6.8/10**

## Improvement Areas

### High Priority

1. **Add module-level documentation** (`src-tauri/src/db.rs`, `src-tauri/src/import.rs`, `src-tauri/src/lib.rs`)
   - Explain the purpose of each module
   - Document the `with_db`/`with_db_mut` helper pattern
   - File: `src-tauri/src/db.rs:1` — add `//! Database operations module...`

2. **Document complex SQL queries** (`src-tauri/src/db.rs:271-373`)
   - `get_word` uses 4 queries with GROUP_CONCAT and json_group_array
   - Add `///` explaining the 3-query strategy and why it's optimal

3. **Replace `.unwrap()` calls with proper error handling** (`src-tauri/src/import.rs:113`)
   - Transaction creation should return `Result` not panic
   - File: `import.rs:113` — `let tx = conn.transaction().map_err(...)?;`

### Medium Priority

4. **Split lib.rs if command count grows** (`src-tauri/src/lib.rs`)
   - Currently 903 lines with 28 commands — acceptable
   - If >35 commands, split into `commands/` submodules
   - Pattern: `commands/words.rs`, `commands/events.rs`, `commands/search.rs`

5. **Create error type enum** (`src-tauri/src/lib.rs`)
   - Replace `Result<T, String>` with `Result<T, AppError>`
   - Variants: `NotOpen`, `Query(String)`, `IO(String)`, `Migration(String)`, `Import(String)`
   - File: `lib.rs:26` — current `type Res<T> = Result<T, String>`

6. **Add inline comments to migration logic** (`src-tauri/src/db.rs:112-191`, `src-tauri/src/db.rs:196-219`)
   - Explain why table rebuild is needed for `migrate_words_unique_if_needed`
   - Document the `ev_col_migrated` swap logic

7. **Consolidate duplicate code in import** (`src-tauri/src/import.rs`)
   - Types, authors, events import follow same pattern — could be DRY'd with a helper
   - File: `import.rs:116-186` — three similar import loops

## Strengths

1. **Consistent patterns** — All CRUD operations follow the same structure (list/save/delete)
2. **Helper functions** — `with_db`/`with_db_mut` abstract mutex access cleanly
3. **Type aliases** — `Db<'a>` and `Res<T>` improve readability
4. **Optimized queries** — `get_word` uses 3 queries instead of N+1; export uses 4 bulk queries
5. **Clean module boundaries** — Each module has a single responsibility
6. **Good test coverage for core operations** — 10 test functions covering CRUD, FTS, and performance
7. **Proportionate complexity** — No over-engineering; patterns match app size
8. **CI pipeline** — All checks (prettier, eslint, svelte-check, rustfmt, clippy, tests) automated
