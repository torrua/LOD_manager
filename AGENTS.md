# AGENTS.md — LOD Manager Developer & AI Agent Guide

This document provides authoritative coding guidelines, architecture conventions, and build/verification commands for AI agents and developers working on the **LOD Manager** codebase (Tauri v2 + Svelte 5 + Rust + SQLite/FTS5).

---

## 1. Project Overview & Key Docs

- **Frontend**: Svelte 5 (Runes: `$state`, `$derived`, `$effect`, `$props`), TypeScript 6.0, Vite 8.3
- **Backend**: Rust (Cargo edition `2024`, rustfmt edition `2021`), Tauri 2.12, `rusqlite` 0.40 (bundled SQLite + FTS5)
- **Database**: Loglan Online Dictionary (LOD) relational + FTS5 schema

### Essential Project Documents

| Document                                                                             | Purpose                                                                       |
| ------------------------------------------------------------------------------------ | ----------------------------------------------------------------------------- |
| [`PROJECT.md`](file:///c:/Users/User/OneDrive/JS/LOD%20Manager/PROJECT.md)           | Project specification, directory map, and full table of 30 Tauri IPC commands |
| [`ARCHITECTURE.md`](file:///c:/Users/User/OneDrive/JS/LOD%20Manager/ARCHITECTURE.md) | System architecture, C4 diagrams, database schema, data flows, and ADRs       |
| [`AUDIT_REPORT.md`](file:///c:/Users/User/OneDrive/JS/LOD%20Manager/AUDIT_REPORT.md) | Senior Architect audit report, verified bugs, and optimization roadmap        |
| [`TODO.md`](file:///c:/Users/User/OneDrive/JS/LOD%20Manager/TODO.md)                 | Prioritized P0–P3 bug-fix and optimization checklist                          |

---

## 2. Build, Lint & Test Commands

### Frontend (Node / npm)

| Command                | Description                                                     |
| ---------------------- | --------------------------------------------------------------- |
| `npm run dev`          | Start Vite dev server (`http://localhost:5173`)                 |
| `npm run build`        | Build production frontend bundle (`dist/`)                      |
| `npm run tauri`        | Run Tauri app in development mode                               |
| `npm run dev:tauri`    | Alias for `tauri dev`                                           |
| `npm run check`        | Run TypeScript compiler check (`tsc --noEmit`)                  |
| `npm run lint`         | Run ESLint across TS and Svelte files                           |
| `npm run lint:fix`     | Run ESLint with auto-fix                                        |
| `npm run format`       | Format files with Prettier (writes changes)                     |
| `npm run format:check` | Check formatting with Prettier (read-only)                      |
| `npm run ci:check`     | **Run all frontend checks** (`format:check` + `lint` + `check`) |

### Backend (Rust / Cargo)

| Command                                                          | Description                           |
| ---------------------------------------------------------------- | ------------------------------------- |
| `npm run rust:fmt`                                               | Format Rust code via `cargo fmt`      |
| `npm run rust:fmt:check`                                         | Check Rust formatting (`--check`)     |
| `npm run rust:lint`                                              | Run Clippy (`-- -D warnings`)         |
| `cargo test --manifest-path src-tauri/Cargo.toml`                | Run all Rust unit & integration tests |
| `cargo test --manifest-path src-tauri/Cargo.toml -- <test_name>` | Run a single Rust test                |

> **Windows MSVC Linker Note**: If `link.exe` is not on your shell's default `PATH`, initialize the Visual Studio x64 environment before running `cargo`:
>
> ```powershell
> cmd /c "`"C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Auxiliary\Build\vcvars64.bat`" >nul && cargo test --manifest-path src-tauri/Cargo.toml"
> ```

### Full CI Pipeline

```bash
npm run ci:check
npm run rust:fmt:check
npm run rust:lint
cargo test --manifest-path src-tauri/Cargo.toml
```

---

## 3. Code Style & Conventions

### TypeScript / Svelte 5

**Formatting (Prettier — `.prettierrc`)**

- Semicolons: `true`
- Single quotes: `true`
- Tab width: `2`
- Print width: `100`
- Trailing commas: `es5`

**ESLint Rules (`eslint.config.js`)**

- `prefer-const`: enforce `const` declarations
- `no-var`: forbid `var`
- `eqeqeq`: always use `===` / `!==`
- `object-shorthand`: use `{ x }` instead of `{ x: x }`
- `prefer-template`: use template literals instead of string concatenation
- `@typescript-eslint/consistent-type-imports`: always use `import type { Foo }` for types
- `no-console`: `console.log` triggers warnings; only `console.warn` and `console.error` are permitted

**TypeScript (`tsconfig.json`)**

- `strict: true`
- `verbatimModuleSyntax: true` — must use `import type`
- `noUncheckedIndexedAccess: true` — indexing arrays/records returns `T | undefined`
- `exactOptionalPropertyTypes: true`

**Svelte 5 Runes**

- Use `$state`, `$derived`, `$effect`, and `$props()` exclusively (do not use legacy `export let` or `writable` stores).
- Do **not** mutate reactive `$state` arrays in-place when computing derived values (e.g., use `[...app.events].sort(...)` instead of `app.events.sort(...)`).
- Mount with `import { mount } from 'svelte'`.

### Rust (`src-tauri/`)

**Formatting (`src-tauri/rustfmt.toml`)**

- `max_width = 100`
- `tab_spaces = 4`
- `newline_style = "Unix"` (**CRITICAL on Windows**: all `.rs` files must use `LF` line endings, or `cargo fmt -- --check` will fail with `Incorrect newline style`).

**Clippy (`src-tauri/.clippy.toml` & `src-tauri/src/lib.rs`)**

- `cognitive-complexity-threshold = 20`
- `too-many-lines-threshold = 60`
- Project-wide lint attributes in `src-tauri/src/lib.rs`:
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
- Always inline format arguments (`format!("{x}")` instead of `format!("{}", x)`).
- Always wrap technical identifiers (`SQLite`, `WordSpell`, `loglan_converter`) in backticks in `///` and `//!` doc comments (`clippy::doc_markdown`).
- Use method references (`str::trim`) instead of redundant closures (`|s| s.trim()`).

**Naming & Error Handling**

- Types/Structs: `PascalCase` (`WordDetail`, `SaveWord`)
- Functions/Variables/Modules: `snake_case` (`open_database`, `get_word`)
- Constants: `SCREAMING_SNAKE_CASE` (`SCRIPT_PREFIX`, `SEP`)
- Tauri commands return `Res<T>` (`Result<T, String>`) and access the database through `with_db(&state, |conn| ...)` or `with_db_mut(&state, |conn| ...)` defined in `src-tauri/src/commands/mod.rs`.
- Never use `.unwrap()` or `block_on()` inside Tauri command handlers.

---

## 4. Architecture & IPC Contract Rules

1. **Frontend ↔ Backend Model Parity**:
   - Every struct in `src-tauri/src/models.rs` must match its corresponding TypeScript interface in `src/types.ts` (field names and nullability).
   - Note: `db::get_word` constructs `Definition` JSON directly inside SQLite via `json_object(...)` (`src-tauri/src/db.rs:417-424`). The keys in `json_object(...)`, `models::Definition`, `models::SaveDefinition`, `src/types.ts::Definition`, and `WordDetail.svelte` must stay strictly synchronized.

2. **Database Schema Consistency (`torrua/loglan_core` Parity)**:
   - The database schema originates from [`torrua/loglan_core`](https://github.com/torrua/loglan_core) and `loglan_convert` (`export.db`). All SQL queries across `db.rs`, `export.rs`, `import.rs`, `converter/converter.rs`, and `lib.rs` tests must stay compatible with the canonical `loglan_core` schema:
     - `words`: `id, name, type, origin, origin_x, "match", rank, year, notes (JSON), id_old, "TID_old", event_start, event_end` (note: do **not** enforce `UNIQUE(name, type)` on `words`, as the same word spelling and type can exist across different event intervals `event_start`/`event_end`; `year` is stored as `'YYYY-01-01'` `DATE` in `export.db`, and `notes` is a JSON dict `{"author", "year", "rank"}` or `'null'`).
     - `types`: `id, type, type_x, "group", parentable, description`
     - `events`: `id, event_id, name, date, definition, annotation, suffix`
     - `authors`: `id, abbreviation, full_name, notes`
     - `definitions`: `id, word_id, position, body, usage, grammar_code, slots, case_tags, language, notes` (`slots` + `grammar_code` together form the full grammar string like `2a`).
     - `settings`: `id, date, db_version, last_word_id, db_release`
     - `syllables`: `id, name, type, allowed`
     - `keys`: `id, word, language`
     - M2M tables: `connect_words (parent_id, child_id)` (links words to both derived affixes `type_x='Affix'` and derived complexes `"group"='Cpx'`), `connect_authors ("AID", "WID")`, `connect_keys ("KID", "DID")`.
   - Starting from `loglan_core` 0.6.0, schema foreign keys define `ON DELETE CASCADE` (`connect_words`, `connect_authors`, `connect_keys`, `definitions`). While `LOD Manager` sets `PRAGMA foreign_keys=ON`, delete operations on `words`, `definitions`, or `authors` continue to explicitly delete child rows from referencing tables first to preserve backward compatibility with legacy pre-0.6.0 databases.
   - Any multi-statement migration in `execute_batch` will abort at the first failing SQL statement; ensure migrations check column/table existence or run safely across both fresh and legacy databases.

3. **Adding a New Tauri Command**:
   1. Add the data model (if needed) to `src-tauri/src/models.rs` and `src/types.ts`.
   2. Implement the database query in `src-tauri/src/db.rs`.
   3. Create the `#[tauri::command]` handler in the appropriate `src-tauri/src/commands/*.rs` submodule.
   4. Register the command in `tauri::generate_handler![...]` in `src-tauri/src/lib.rs`.
   5. Add the `invoke(...)` wrapper in `src/lib/store.svelte.ts`.
   6. Add a unit test in `src-tauri/src/lib.rs` (`#[cfg(test)] mod tests`).

---

## 5. Commit & Release Style

Use [Conventional Commits](https://www.conventionalcommits.org/):

- `feat: ...` — new user-facing feature
- `fix: ...` — bug fix
- `refactor: ...` — code restructuring without behavior change
- `perf: ...` — performance optimization
- `test: ...` — adding or fixing tests
- `docs: ...` — documentation updates
- `chore: ...` — version bumps, tooling, CI

When bumping versions for release, update **all three** manifests:

1. `package.json` (`"version"`)
2. `src-tauri/tauri.conf.json` (`"version"`)
3. `src-tauri/Cargo.toml` (`version`)
