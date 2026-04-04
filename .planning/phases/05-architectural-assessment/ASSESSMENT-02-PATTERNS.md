# Design Patterns Assessment

**Assessment Date:** 2026-04-04

---

## Patterns Identified

### Command Pattern

- **Location:** `src-tauri/src/lib.rs` — all 28 `#[tauri::command]` functions (lines 47-903)
- **Usage:** Frontend invokes Rust commands via `invoke('command_name', {args})`. Commands act as API endpoints.
- **Appropriateness:** ✓ **Highly appropriate.** Separates frontend (UI) from backend (data/logic). Standard pattern for Tauri apps. Commands are thin wrappers that delegate to `db.rs`.
- **Verdict:** Keep

**Examples:**

- `open_database` → delegates to `db::init_schema`, `db::init_fts`, migrations
- `get_words` → delegates to `db::list_words`
- `import_lod_contents` → delegates to `import::import_lod_files`

---

### Repository Pattern

- **Location:** `src-tauri/src/db.rs` — functions like `list_words`, `get_word`, `save_word`, `search_english`
- **Usage:** Database operations wrapped in functions, not direct SQL in commands.
- **Appropriateness:** ✓ **Appropriate.** Clean separation between command layer (lib.rs) and data access layer (db.rs). Commands don't contain raw SQL.
- **Verdict:** Keep

**Evidence:**

```rust
// lib.rs (command layer)
fn get_words(state: Db, q: String, ...) -> Res<Vec<WordListItem>> {
    with_db(&state, |conn| db::list_words(conn, &q, &type_filter, event_id))
}

// db.rs (data access layer)
pub fn list_words(conn: &Connection, q: &str, ...) -> rusqlite::Result<Vec<WordListItem>> {
    // Actual SQL query here
}
```

---

### State Pattern (AppState)

- **Location:** `src-tauri/src/lib.rs` lines 20-23 — `AppState` struct
- **Usage:** `AppState` holds `Mutex<Option<Connection>>` and `Mutex<String>` for current database path.
- **Appropriateness:** ⚠️ **Acceptable but has trade-offs.** Global state via Mutex is simple but:
  - Only one database at a time
  - No connection pooling
  - Thread-safety via Mutex (not async-safe without async Mutex)
- **Verdict:** Keep (for app size) — would reconsider for multi-database or web-scale

**Alternative considered:** Could use `tauri::State` with async connection pool, but adds complexity not needed for single-user desktop app.

---

### Observer Pattern (Svelte Reactivity)

- **Location:** `src/lib/store.svelte.ts` — global `app` state object using `$state`
- **Usage:** Frontend components react to `app` state changes via runes. No explicit event emitters.
- **Appropriateness:** ✓ **Appropriate.** Svelte 5 runes provide reactive updates. Clear and simple.
- **Verdict:** Keep

**Evidence:**

```typescript
// Store definition (lines 35-104)
export const app = $state({ ... });

// Component usage
let { word } = $props();
let count = $state(0);
let doubled = $derived(count * 2);
```

---

### Singleton Pattern

- **Location:** `src/lib/store.svelte.ts` — single `app` export
- **Usage:** One global application state object
- **Appropriateness:** ✓ **Appropriate.** Single global store is standard for Svelte apps. No issues observed.
- **Verdict:** Keep

---

### Builder Pattern

- **Location:** `src-tauri/src/db.rs` — FTS query building functions
- **Usage:** `build_fts_query`, `build_keywords_query` construct SQL dynamically
- **Appropriateness:** ⚠️ **Could be simplified.** Current implementation is readable but creates complex query strings. Not an anti-pattern, just complex.
- **Verdict:** Refactor if needed — acceptable for now

---

## Anti-Patterns Found

| Anti-Pattern           | Severity | Location            | Evidence                                        |
| ---------------------- | -------- | ------------------- | ----------------------------------------------- |
| God Object (lib.rs)    | **Low**  | lib.rs (~903 lines) | Commands gateway — acceptable for this app size |
| Generic Error Messages | **Low**  | lib.rs:28-30        | `err()` helper produces same-format errors      |
| Magic Strings          | **None** | —                   | No hardcoded strings for business logic found   |

### lib.rs File Size Note

At ~903 lines, lib.rs is large but not a god object. It:

- Contains only command definitions (thin wrappers)
- Delegates all logic to `db.rs`, `import.rs`, `export.rs`, `models.rs`
- Each module has clear single responsibility

**Verdict:** Not an anti-pattern for this architecture.

---

## Pattern Appropriateness for Domain

**Context:** This is a dictionary manager (~28 commands), not enterprise software.

| Assessment                 | Finding                                                         |
| -------------------------- | --------------------------------------------------------------- |
| **Pattern overhead**       | Low — no over-engineering detected                              |
| **Separation of concerns** | Clear — 5 modules, each with single responsibility              |
| **Complexity**             | Proportionate — matches app domain                              |
| **Missing patterns**       | None critical — could benefit from more tests (see CONCERNS.md) |

---

## Recommendations

### Keep (No Action)

1. **Command Pattern** — Standard, works well
2. **Repository Pattern** — Clean separation
3. **Svelte State** — Proper use of runes

### Refactor (Low Priority)

1. **Error categorization** — Instead of generic strings, could use error enum:

   ```rust
   enum AppError {
       DatabaseNotFound,
       CorruptedDatabase(String),
       PermissionDenied,
       // ...
   }
   ```

2. **FTS query builders** — Could be simplified or made more testable

### Not Recommended

- Connection pooling (not needed for single-user app)
- Event sourcing (overkill for dictionary manager)
- CQRS (no complex read/write separation needed)

---

## Summary

| Pattern                  | Verdict    | Rationale                     |
| ------------------------ | ---------- | ----------------------------- |
| Command Pattern          | Keep       | Standard Tauri pattern        |
| Repository Pattern       | Keep       | Clean data access separation  |
| State Pattern (AppState) | Keep       | Simple, adequate for app size |
| Observer (Svelte)        | Keep       | Proper runes usage            |
| Singleton                | Keep       | Standard global state         |
| Builder (FTS)            | Acceptable | Could simplify later          |

**Overall:** Architecture is appropriate for a dictionary manager. No major anti-patterns. Code is clean and well-organized.
