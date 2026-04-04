# Risk Mitigation Playbook

## Risk Register with Mitigations

### Risk 1: Transaction Panic in Import

- **Severity:** High
- **Likelihood:** Medium
- **Impact:** High
- **Source:** ASSESSMENT-03-STABILITY.md, import.rs:113

**Description:** `import.rs:113` uses `conn.transaction().unwrap()`. If the database is locked by another operation or corrupt, this will panic and crash the entire application during import — the user's primary data ingestion path.

**Mitigation Strategy:**

1. Replace `.unwrap()` with `?` operator and descriptive error message
2. Add retry logic for transient lock conflicts (up to 3 retries with 100ms backoff)
3. Validate database is open and accessible before starting import

**Fallback Plan:**

- If transaction still fails, return error to frontend with message: "Database is busy. Please close other operations and try again."
- Do NOT leave partial import state — transaction ensures atomicity

**Monitoring:**

- Add `eprintln!` log on transaction failure for debugging
- Track import error rate in frontend (count failed imports / total imports)

**Code Changes Required:**

- `src-tauri/src/import.rs:113` — replace `.unwrap()` with `.map_err(...)?`
- `src-tauri/src/lib.rs:209-212` — add pre-import validation

---

### Risk 2: Partial State on Database Open Failure

- **Severity:** High
- **Likelihood:** Low
- **Impact:** High
- **Source:** ASSESSMENT-03-STABILITY.md, lib.rs:48-60

**Description:** `open_database` runs migrations sequentially. If `migrate_words_unique_if_needed` succeeds but `migrate_event_columns_if_needed` fails, the database is partially migrated but `AppState` is not updated. Next open attempt may see inconsistent schema state.

**Mitigation Strategy:**

1. Add a "migration_in_progress" flag in settings table before starting migrations
2. Clear flag after all migrations complete successfully
3. On open, check flag — if set, re-run all migrations (they are idempotent)
4. Alternatively: wrap all migrations in a single transaction (SQLite supports DDL in transactions)

**Fallback Plan:**

- If migration fails mid-way, user sees error message
- On next open, migrations re-run (idempotent — safe to retry)
- For `migrate_words_unique_if_needed`: table rebuild is atomic (CREATE + INSERT + DROP + RENAME)

**Monitoring:**

- Log migration start/complete/fail with `eprintln!`
- Track migration failure rate across app versions

**Code Changes Required:**

- `src-tauri/src/lib.rs:48-60` — add migration flag or transaction wrapper
- `src-tauri/src/db.rs:112` — check/set migration flag

---

### Risk 3: FTS Rebuild Memory Concern

- **Severity:** Medium
- **Likelihood:** Low
- **Impact:** Medium
- **Source:** ASSESSMENT-03-STABILITY.md, db.rs:800-813

**Description:** `rebuild_fts` loads ALL definitions containing `«` markers into a `Vec<(i64, String)>` in memory. For databases with 100k+ definitions with keywords, this could consume significant memory (estimated 50-200MB).

**Mitigation Strategy:**

1. Process definitions in batches of 1000 instead of loading all at once
2. Use a cursor-based approach: `SELECT id, body FROM definitions WHERE body LIKE '%«%' LIMIT 1000 OFFSET ?`
3. Insert keyword FTS entries batch by batch

**Fallback Plan:**

- If OOM occurs during rebuild, FTS will be empty — search falls back to LIKE mode automatically
- User can retry rebuild after closing other apps

**Monitoring:**

- Log definition count before rebuild: `eprintln!("[FTS] Rebuilding with {count} definitions")`
- Track rebuild duration — warn if > 30 seconds

**Code Changes Required:**

- `src-tauri/src/db.rs:800-813` — replace `Vec::collect()` with batched iteration

---

### Risk 4: No Migration Rollback

- **Severity:** Medium
- **Likelihood:** Low
- **Impact:** Medium
- **Source:** ASSESSMENT-03-STABILITY.md, db.rs:150-183

**Description:** `migrate_words_unique_if_needed` rebuilds the words table (CREATE + INSERT + DROP + RENAME). If power fails or process is killed during this operation, data may be lost. No rollback mechanism exists.

**Mitigation Strategy:**

1. Wrap migration in a transaction (SQLite supports DDL in transactions)
2. Create backup table before DROP (`CREATE TABLE words_backup AS SELECT * FROM words`)
3. Delete backup table only after successful rename

**Fallback Plan:**

- If migration fails, `words_backup` table exists — can be renamed back to `words`
- Add recovery code in `open_database` that checks for `words_backup` and restores it

**Monitoring:**

- Log migration start with table row count
- Verify row count after migration matches before migration

**Code Changes Required:**

- `src-tauri/src/db.rs:150-183` — add backup table and transaction wrapper
- `src-tauri/src/db.rs:112` — add row count verification

---

### Risk 5: Android File Size Not Validated

- **Severity:** Medium
- **Likelihood:** Medium
- **Impact:** Medium
- **Source:** ASSESSMENT-03-STABILITY.md, import.rs:39-71

**Description:** `import_lod_contents` receives file content directly from frontend (Android reads via plugin-fs). No size validation — a 500MB file would be loaded into memory, likely causing OOM on Android devices with limited RAM.

**Mitigation Strategy:**

1. Add size check at command entry point (`lib.rs:209`): reject if total content > 100MB
2. Frontend should validate file size before reading (use `stat` via plugin-fs)
3. Return clear error: "File too large. Maximum size: 100MB"

**Fallback Plan:**

- If OOM occurs, app will crash — no graceful recovery possible
- Prevention is the only viable strategy

**Monitoring:**

- Log rejected file sizes for tuning limit
- Track OOM crash rate on Android via crash reporting

**Code Changes Required:**

- `src-tauri/src/lib.rs:209` — add size validation before calling `import_contents`
- Frontend: add file size check before reading

---

### Risk 6: Error Categorization Missing

- **Severity:** Medium
- **Likelihood:** High
- **Impact:** Medium
- **Source:** ASSESSMENT-03-STABILITY.md, all commands

**Description:** All Tauri commands return `Result<T, String>`. Frontend receives generic error strings and must parse them to determine error type. This is fragile — error message changes break frontend logic. No structured error handling.

**Mitigation Strategy:**

1. Create `enum AppError` with variants: `NotOpen`, `Query(String)`, `IO(String)`, `Migration(String)`, `Import(String)`
2. Implement `Display` for `AppError` (for error messages) and `Serialize` (for Tauri)
3. Replace `type Res<T> = Result<T, String>` with `Result<T, AppError>`
4. Frontend can check error type via string prefix or structured error codes

**Fallback Plan:**

- Continue using string errors but add error type prefix: `[NOT_OPEN] No database open.`
- Frontend parses prefix instead of full message

**Monitoring:**

- Track error type distribution in frontend analytics
- Log all errors server-side for debugging

**Code Changes Required:**

- `src-tauri/src/models.rs` — add `AppError` enum
- `src-tauri/src/lib.rs` — update all command return types and error conversions
- Frontend: update error handling to check error type

---

## Summary

| Risk                    | Severity | Likelihood | Mitigation Effort | Status              |
| ----------------------- | -------- | ---------- | ----------------- | ------------------- |
| 1. Transaction panic    | High     | Medium     | XS                | Ready to fix        |
| 2. Partial state        | High     | Low        | S                 | Requires design     |
| 3. FTS memory           | Medium   | Low        | M                 | Batch processing    |
| 4. Migration rollback   | Medium   | Low        | S                 | Transaction wrapper |
| 5. Android file size    | Medium   | Medium     | XS                | Size validation     |
| 6. Error categorization | Medium   | High       | M                 | New error type      |

**Priority order for implementation:** 1 → 5 → 2 → 4 → 3 → 6
