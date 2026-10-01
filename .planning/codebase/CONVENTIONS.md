# Coding Conventions

**Analysis Date:** 2026-04-07

## Summary

LOD Manager uses strict TypeScript (strict mode plus several extra tsconfig flags) with Svelte 5 runes for the frontend, and idiomatic Rust 2024 edition with Clippy pedantic for the backend. Prettier handles all JS/TS/Svelte formatting; `cargo fmt` handles Rust. Both linters run in CI via `npm run ci:check` and `cargo clippy -- -D warnings`. The two sides of the stack follow different but internally consistent conventions: TypeScript uses camelCase throughout, while Rust uses snake_case for everything except types which use PascalCase.

---

## Formatting

**Frontend (Prettier 3):**
Config: `/c/Users/User/Downloads/app/.prettierrc`
- Single quotes, semicolons on, 2-space indent, 100-char print width
- Trailing commas: `es5`
- Svelte files parsed with `prettier-plugin-svelte`

Run:
```bash
npm run format          # write in place
npm run format:check    # CI check only
```

**Rust (rustfmt):**
Edition 2024, Cargo defaults.
Run:
```bash
npm run rust:fmt         # writes
npm run rust:fmt:check   # CI check
```

---

## Linting

**Frontend (ESLint 10, flat config):**
Config: `/c/Users/User/Downloads/app/eslint.config.js`

Key enforced rules:
- `@typescript-eslint/consistent-type-imports` — **error**: must use `import type` for type-only imports
- `prefer-const` — **error**: `let` only when reassignment is needed
- `no-var` — **error**
- `eqeqeq: always` — **error**
- `prefer-template` — **error**: template literals over string concatenation
- `object-shorthand` — **error**
- `@typescript-eslint/no-explicit-any` — **warn**
- `@typescript-eslint/no-unused-vars` — **warn** (args prefixed `_` are exempt)
- `no-console` — **warn** (`console.warn` and `console.error` are allowed; `console.log` is technically warned against but widely used in current debug paths in `src/lib/store.svelte.ts`)

Run:
```bash
npm run lint         # check
npm run lint:fix     # auto-fix
```

**Rust (Clippy):**
Configured at the top of `/c/Users/User/Downloads/app/src-tauri/src/lib.rs`:
```rust
#![warn(clippy::all)]
#![warn(clippy::pedantic)]
#![allow(clippy::needless_pass_by_value)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::wildcard_imports)]
#![allow(clippy::too_many_lines)]
#![allow(clippy::cast_precision_loss)]
```
CI runs with `-D warnings` (all warnings become errors).

---

## TypeScript Configuration

Config: `/c/Users/User/Downloads/app/tsconfig.json`

Strict flags enabled beyond `strict: true`:
- `noUncheckedIndexedAccess` — array index access returns `T | undefined`
- `noImplicitReturns` — all code paths must return
- `noFallthroughCasesInSwitch`
- `exactOptionalPropertyTypes`
- `verbatimModuleSyntax` — enforces `import type` at the module level

Run type check: `npm run check` (tsc --noEmit)

---

## Naming Conventions

**TypeScript/Svelte:**
- **Component files:** `PascalCase.svelte` (`WordDetail.svelte`, `EventForm.svelte`, `AuthorsPanel.svelte`)
- **Module files:** `camelCase.ts` (`store.svelte.ts`, `text.ts`) with `.svelte.ts` suffix when using runes outside a component
- **Types/interfaces:** PascalCase (`WordListItem`, `WordDetail`, `EventItem`, `TypeItem`, `ELResult`)
- **Functions:** camelCase (`loadWords`, `selectWord`, `applyFilter`, `toggleTheme`, `renderBody`)
- **Variables:** camelCase (`curWord`, `typeFilter`, `sbWidth`, `loadingWordId`)
- **Type imports:** always `import type { ... }` (enforced by ESLint)
- **Store state fields:** camelCase matching the concept (`dbOpen`, `curWord`, `elFtsReady`, `updateDownloading`)

**Rust:**
- **Functions:** `snake_case` (`get_words`, `save_word`, `list_types`, `with_db`, `init_schema`)
- **Types/structs:** `PascalCase` (`WordDetail`, `AppState`, `ImportResult`, `SaveWord`, `ELResult`)
- **Modules:** `snake_case` matching filename (`commands/words.rs`, `db.rs`, `import.rs`, `converter.rs`)
- **Type aliases:** `PascalCase` (`Res<T>`, `Db<'a>`)

**Tauri Command Naming:**
Commands follow a verb-noun pattern in `snake_case`. The command name registered in Rust matches exactly what the frontend calls via `invoke()`:
- `get_words`, `get_word`, `save_word`, `delete_word`
- `open_database`, `create_database`, `get_db_stats`
- `import_lod_files`, `import_lod_contents`, `export_html`
- `rebuild_fts`, `fts_is_ready`, `compact_db`, `search_english`

CRUD convention: `save_*` handles both create and update (id is `Option`). `get_*` fetches a single item by id. `list_*` is used internally in `db.rs` for bulk queries; the Tauri command surface uses `get_*` even for lists.

---

## Module / File Organization

**Frontend:**
```
src/
  main.ts                    # Svelte mount point only
  App.svelte                 # Root component: keyboard handlers, DB auto-open, layout
  types.ts                   # All shared TypeScript interfaces and union types
  lib/
    store.svelte.ts          # ALL application state ($state singleton) + ALL invoke() calls
    text.ts                  # Pure text/HTML utility functions (renderBody, esc)
    components/
      *.svelte               # UI components — one concern per file
```

**Critical rule:** All Tauri `invoke()` calls are centralized in `src/lib/store.svelte.ts`. Components import named functions from `store.svelte` and never call `invoke()` directly.

**Rust:**
```
src-tauri/src/
  lib.rs           # Tauri builder setup, invoke_handler registration, all top-level tests
  main.rs          # Binary entry point (calls lib::run())
  models.rs        # All Serde structs — Serialize for outputs, Deserialize for inputs
  db.rs            # All SQL: schema, migrations, CRUD, FTS management
  import.rs        # LOD @-delimited text file parsing and import pipeline
  export.rs        # HTML export generation
  commands/
    mod.rs         # AppState struct, Res<T> alias, with_db / with_db_mut helpers, err()
    database.rs    # open_database, create_database, get_db_stats, get_default_db_path
    words.rs       # Word and definition CRUD commands
    events.rs      # Event CRUD commands
    types.rs       # Type CRUD commands
    authors.rs     # Author CRUD commands
    search.rs      # search_english, rebuild_fts, compact_db, fts_is_ready
    import.rs      # import_lod_contents, import_lod_files, convert_text_files
    export.rs      # export_html, export_html_to_file
  converter/
    mod.rs         # Re-exports
    converter.rs   # Text-to-SQLite converter logic
    tests.rs       # Converter-specific unit tests
```

**Critical rule:** All database logic lives in `db.rs`. Commands in `commands/*.rs` are thin wrappers that call `db::*` functions through the `with_db` / `with_db_mut` helpers. Commands must not contain SQL.

---

## Error Handling

**Rust:**
All Tauri commands return `Res<T>`, a type alias defined in `src-tauri/src/commands/mod.rs`:
```rust
pub type Res<T> = Result<T, String>;
```

Errors are converted to `String` via the `err()` helper:
```rust
pub fn err(e: impl std::fmt::Display) -> String { e.to_string() }
```

The `with_db` helper encapsulates the Mutex lock and Option unwrap:
```rust
pub fn with_db<T, F: FnOnce(&Connection) -> rusqlite::Result<T>>(state: &AppState, f: F) -> Res<T> {
    let guard = state.db.lock().map_err(err)?;
    let conn = guard.as_ref().ok_or("No database open.")?;
    f(conn).map_err(err)
}
```

Non-critical failures (FTS updates on save, migrations) use `.ok()` or `let _ =` to silently discard errors rather than propagating them. `eprintln!` is used in the updater path for diagnostics, prefixed with `[Updater]`.

**TypeScript:**
- Async store functions wrap `invoke()` calls in `try/catch`
- User-visible errors surface via the `toast(msg, 'err')` function — the sole user feedback mechanism
- Silent discard with `.catch(() => {})` is used for non-critical background operations: `checkFts().catch(() => {})`, `loadDbStats().catch(() => {})`
- `console.error` is used for debugging (permitted by ESLint); `console.log` is technically warned but widely present in the current codebase

---

## Import Organization

**TypeScript — import order (observed pattern):**
1. Third-party/Tauri APIs (`@tauri-apps/api/core`, `@tauri-apps/plugin-dialog`, etc.)
2. Local store/utilities (`'../store.svelte'`, `'../text'`)
3. Local components (`'./Icon.svelte'`, `'./Sidebar.svelte'`)
4. Type imports as a separate `import type { ... }` statement (enforced)

**Rust — `use` ordering:**
Super/module imports first (`use super::*`), then crate-local (`use crate::db`, `use crate::models::*`), then standard library (`use std::*`, `use std::collections::HashMap`).

---

## Svelte 5 Runes Usage

Svelte 5 runes are used exclusively. No Svelte 4 stores (`writable`, `readable`, `derived`) exist in the codebase.

- `$state()` — reactive state in components and the `app` singleton in `store.svelte.ts`
- `$derived()` — computed values (e.g., virtual scroll bounds in `Sidebar.svelte`)
- `onMount()` — used in `App.svelte` for initialization (platform detect, DB auto-open, event listeners)
- Components read from the shared `app` singleton rather than receiving props in most cases

---

## Svelte Component Conventions

- All scripts are `<script lang="ts">` — TypeScript is always required
- `<!-- eslint-disable svelte/no-at-html-tags -->` at file top when `{@html ...}` is required (e.g., `WordDetail.svelte` for rendered definition bodies)
- Form components declare a local `$state` form object, then spread into `invoke` payloads on submit
- Empty string fields convert to `null` before being passed to Rust (pattern in `WordForm.svelte`, `EventForm.svelte`)
- No deep prop drilling — components import what they need from `store.svelte` directly

---

## Comments and Documentation Style

**Rust:**
- Module-level `//!` doc comments on every `.rs` file describing purpose and key patterns
- Inline `// comment` used for non-obvious logic
- `eprintln!` used for runtime diagnostics only in the updater path, prefixed `[Updater]`

**TypeScript:**
- Section dividers use the pattern: `// ─── Section Name ───────────────────────────────` (em-dash box-drawing characters)
- `/** JSDoc */` used for exported utility functions in `src/lib/text.ts`
- Inline comments used freely in complex logic (Android content:// URI handling, virtual scroll math)
