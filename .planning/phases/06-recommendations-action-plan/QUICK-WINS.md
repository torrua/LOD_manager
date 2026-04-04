# Quick Wins: High Impact, Low Effort

Quick wins are recommendations with **XS/S effort** and **High/Medium impact** that can be implemented immediately with minimal risk.

## Quick Wins

### 1. Replace `.unwrap()` in import transaction (~15 min)

- **Why quick:** Single line change — replace `.unwrap()` with `?` and error message
- **Change:** `import.rs:113` — `let tx = conn.transaction().map_err(|e| format!("Failed to start import transaction: {e}"))?;`
- **Benefit:** Eliminates app crash during import if database is locked or corrupt

### 2. Add file size validation for Android imports (~30 min)

- **Why quick:** Simple size check before processing — one `if` statement
- **Change:** `import.rs:39` — add `if files.iter().map(|(_, c)| c.len()).sum::<usize>() > 100_000_000 { return Err(...) }`
- **Benefit:** Prevents OOM crashes on Android from large file imports

### 3. Enable WAL mode by default (~10 min)

- **Why quick:** One PRAGMA statement added to `open_database`
- **Change:** `lib.rs:50` — add `conn.execute_batch("PRAGMA journal_mode=WAL;").map_err(err)?;` after connection opens
- **Benefit:** Readers not blocked during writes — better UX during FTS rebuild and saves

### 4. Document complex SQL in `get_word` (~20 min)

- **Why quick:** Add `///` comment above existing code — no logic changes
- **Change:** `db.rs:271` — explain the 4-query strategy (main + GROUP_CONCAT + json_group_array + used-in)
- **Benefit:** Prevents future maintainers from "simplifying" and degrading performance

### 5. Add module-level documentation (~1 hour)

- **Why quick:** Write `//!` comments at top of 3 files — no logic changes
- **Change:** `db.rs:1`, `import.rs:1`, `lib.rs:1` — explain module purpose and key patterns
- **Benefit:** New developers understand codebase structure faster

### 6. Add migration tests (~2 hours)

- **Why quick:** In-memory SQLite tests — no external dependencies needed. Pattern already established in existing tests.
- **Change:** Add 2 test functions to `lib.rs` under `#[cfg(test)]` — one for each migration
- **Benefit:** Confidence that migrations work correctly — prevents data loss bugs

### 7. Add export tests (~1 hour)

- **Why quick:** Test `generate_html` with known in-memory data — no file I/O needed
- **Change:** Add 1-2 test functions to `lib.rs` — verify HTML contains expected elements
- **Benefit:** Confidence that export produces valid HTML

## Impact Summary

| Quick Win                       | Time    | Impact | Risk |
| ------------------------------- | ------- | ------ | ---- |
| 1. Replace `.unwrap()`          | 15 min  | High   | Low  |
| 2. Android file size validation | 30 min  | Medium | Low  |
| 3. Enable WAL mode              | 10 min  | Medium | Low  |
| 4. Document get_word SQL        | 20 min  | Medium | None |
| 5. Module-level docs            | 1 hour  | Medium | None |
| 6. Migration tests              | 2 hours | High   | Low  |
| 7. Export tests                 | 1 hour  | Medium | Low  |

**Total effort: ~5 hours** — all can be completed in a single work session.

**Risk assessment:** All quick wins are low-risk. Items 4-5 are documentation-only (zero risk). Items 1-3 are single-line changes with clear error handling. Items 6-7 add tests without modifying production code.
