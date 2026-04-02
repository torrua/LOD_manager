# AGENTS.md - LOD Manager Developer Guide

This document provides coding guidelines and commands for agents working on this Tauri + Svelte 5 + Rust codebase.

---

## Project Overview

- **Frontend**: Svelte 5 (runes), TypeScript, Vite
- **Backend**: Rust with Tauri 2.0, rusqlite (SQLite)
- **Database**: Loglan Online Dictionary (LOD) format

---

## Build & Development Commands

### Frontend (Node/npm)

| Command             | Description              |
| ------------------- | ------------------------ |
| `npm run dev`       | Start Vite dev server    |
| `npm run build`     | Build production bundle  |
| `npm run tauri`     | Run Tauri app (dev mode) |
| `npm run dev:tauri` | Alias for `tauri dev`    |

### TypeScript / Linting

| Command                | Description              |
| ---------------------- | ------------------------ |
| `npm run check`        | Run TypeScript check     |
| `npm run lint`         | Run ESLint               |
| `npm run lint:fix`     | ESLint auto-fix          |
| `npm run format`       | Prettier format (writes) |
| `npm run format:check` | Prettier check only      |
| `npm run ci:check`     | **All frontend checks**  |

### Rust

| Command                                                          | Description                   |
| ---------------------------------------------------------------- | ----------------------------- |
| `npm run rust:fmt`                                               | rustfmt format                |
| `npm run rust:fmt:check`                                         | rustfmt check only            |
| `npm run rust:lint`                                              | Clippy lint (denies warnings) |
| `cargo test --manifest-path src-tauri/Cargo.toml`                | Run Rust unit tests           |
| `cargo test --manifest-path src-tauri/Cargo.toml -- <test_name>` | Run single Rust test          |

### Full CI Pipeline

```bash
npm run ci:check
cargo test --manifest-path src-tauri/Cargo.toml
```

---

## Code Style Guidelines

### TypeScript / Svelte

**Formatting (Prettier)**

- Semicolons: `true`
- Single quotes: `true`
- Tab width: `2`
- Print width: `100`
- Trailing commas: `es5`

**ESLint Rules**

- `prefer-const`: enforce constant declarations
- `no-var`: no legacy `var` keyword
- `eqeqeq`: always use `===` / `!==`
- `object-shorthand`: use `{ x }` not `{ x: x }`
- `prefer-template`: use backtick strings
- `@typescript-eslint/consistent-type-imports`: use `import type { Foo }`

**TypeScript (tsconfig.json)**

- Strict mode enabled
- `verbatimModuleSyntax`: must use `import type`
- `noUncheckedIndexedAccess`: array access returns `T | undefined`
- `exactOptionalPropertyTypes`: optional properties are truly optional

**Svelte 5**

- Use runes (`$state`, `$derived`, `$effect`) for reactivity
- Svelte 5 uses `import { mount } from 'svelte'` not `new App()`
- Use `$props()` for component props

### Rust

**Formatting (rustfmt)**

- Edition: 2021
- Max width: 100
- Tab spaces: 4

**Clippy**

- Cognitive complexity threshold: 20
- Too many lines threshold: 60

**Project-wide Rust lint attributes** (see `src-tauri/src/lib.rs`):

```rust
#![warn(clippy::all)]
#![warn(clippy::pedantic)]
#![allow(clippy::needless_pass_by_value)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::wildcard_imports)]
#![allow(clippy::too_many_lines)]
```

**Naming Conventions**

- Types: `PascalCase` (e.g., `WordDetail`, `SaveWord`)
- Functions/variables: `snake_case` (e.g., `open_database`, `get_word`)
- Modules: `snake_case` (e.g., `db`, `import`, `export`)
- Constants: `SCREAMING_SNAKE_CASE`

**Error Handling**

- Use `Result<T, String>` for Tauri commands
- Convert errors with `map_err(err)` helper
- Always use `?` operator for propagation
- Frontend errors: throw exceptions

### Import Conventions

**TypeScript**

```typescript
import type { Foo } from './types';
import { bar } from './utils';
```

**Rust**

```rust
use std::sync::Mutex;
use tauri::{Manager, State};
use rusqlite::Connection;
```

---

## Testing

### Rust Tests

- Tests are inline in `src-tauri/src/lib.rs` (under `#[cfg(test)]`)
- Run all: `cargo test --manifest-path src-tauri/Cargo.toml`
- Run one: `cargo test --manifest-path src-tauri/Cargo.toml -- test_name`

### Frontend Tests

- Currently no frontend test framework configured
- Use `npm run check` to validate types

---

## CI Requirements

All checks must pass before merging:

1. Prettier format check
2. ESLint
3. svelte-check (TypeScript)
4. rustfmt check
5. Clippy (deny warnings)
6. `cargo test`

---

## Commit Style

Use [Conventional Commits](https://www.conventionalcommits.org/):

```
feat: add event filter to word list
fix: preserve layout in read-only mode
chore: bump version to 1.1.0
docs: update README build instructions
```

---

## File Locations

- Frontend source: `src/`
- Rust source: `src-tauri/src/`
- Database files: `tables/*.db`
- Config files: `src-tauri/tauri.conf.json`, `tsconfig.json`, `vite.config.ts`
