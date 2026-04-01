# Technology Stack

**Analysis Date:** 2026-04-01

## Languages

**Primary:**

- TypeScript 6.x - Frontend runtime
- Rust - Backend/Desktop application
- Svelte 5 - Frontend framework (runes-based reactivity)

**Secondary:**

- SQL - Database queries (rusqlite)

## Runtime

**Environment:**

- Node.js 20.x (via Vite)
- Tauri 2.0 (desktop runtime)

**Package Manager:**

- npm
- Lockfile: `package-lock.json` (present)

## Frameworks

**Core:**

- Svelte 5.x - UI framework with runes (`$state`, `$derived`, `$effect`)
- Tauri 2.0 - Desktop application framework
- Vite 8.x - Build tool and dev server
- rusqlite 0.31 - SQLite database with bundled feature

**Testing:**

- Rust: Built-in `cargo test` with inline tests in `src-tauri/src/lib.rs`
- TypeScript: svelte-check for type validation

**Build/Dev:**

- Vite 8.0.3 - Build tool and dev server
- Tauri CLI 2.0 - Desktop build orchestration

## Key Dependencies

**Frontend (npm):**

- `@tauri-apps/api` ^2.0.0 - Tauri IPC and app APIs
- `@tauri-apps/plugin-dialog` ^2.0.0 - Native file dialogs
- `@tauri-apps/plugin-fs` ^2.0.0 - File system access
- `@tauri-apps/plugin-os` ^2.3.2 - OS-level utilities

**Dev Dependencies (npm):**

- Vite 8.x
- @sveltejs/vite-plugin-svelte 7.x
- ESLint 10.x
- eslint-plugin-svelte 3.x
- svelte-check 4.x
- TypeScript 6.x
- prettier 3.x

**Tauri Plugins (Rust):**

- `tauri` 2.0 - Core framework
- `tauri-plugin-dialog` 2.0
- `tauri-plugin-fs` 2.0
- `tauri-plugin-os` 2.0

**Data Layer:**

- `rusqlite` 0.31 - SQLite with FTS5 full-text search
- `serde` + `serde_json` - Serialization

## Configuration

**Environment:**

- `.env` not present (no env vars needed at runtime)
- Configuration via `tauri.conf.json` and `tsconfig.json`

**Build:**

- `package.json` - npm scripts and dependencies
- `src-tauri/Cargo.toml` - Rust dependencies
- `src-tauri/tauri.conf.json` - Tauri configuration
- `vite.config.ts` - Vite build configuration
- `tsconfig.json` - TypeScript configuration

## Platform Requirements

**Development:**

- Node.js 20.x+
- Rust (for Tauri backend)

**Production:**

- Windows/macOS/Linux desktop via Tauri
- Android (minSdk 24) via Tauri mobile
- iOS (16+) via Tauri mobile

---

_Stack analysis: 2026-04-01_
