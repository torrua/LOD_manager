# Technology Stack

**Analysis Date:** 2026-04-07

## Summary

LOD Manager (v1.6.10) is a cross-platform desktop and Android application built with Tauri v2 as the application framework, Svelte 5 for the frontend UI, and Rust for all backend logic. The data layer is SQLite with FTS5 full-text search, accessed through the bundled `rusqlite` crate. The project targets Windows, macOS, Linux, and Android.

## Languages

**Primary:**
- Rust (edition 2024) — all backend logic, Tauri commands, database access, import/export, FTS
- TypeScript — frontend application logic, Tauri IPC calls, state management
- Svelte 5 — UI component authoring (`.svelte` files with runes syntax)

**Secondary:**
- CSS — global styles in `src/styles.css`
- SQL — schema and queries embedded in `src-tauri/src/db.rs`

## Runtime

**Environment:**
- Node.js 24 (specified in CI workflows via `actions/setup-node@v6 node-version: 24`)
- Rust stable toolchain (via `dtolnay/rust-toolchain@stable`)

**Package Manager:**
- npm
- Lockfile: present (`package-lock.json`, implied by `cache: npm` in CI)

## Frameworks

**Core:**
- Tauri v2 (`tauri = "2.0"`) — application shell, IPC bridge, native OS integration
- Svelte 5 (`svelte = "^5.0.0"`) — reactive UI framework using runes (`$state`, `$derived`)

**Build/Dev:**
- Vite 8 (`vite = "^8.0.0"`) — frontend bundler and dev server (port 5173)
  - Config: `vite.config.ts`
  - Build target: `esnext`, minification enabled, no sourcemaps
- `@sveltejs/vite-plugin-svelte ^7.0.0` — Svelte integration for Vite
- `@tauri-apps/cli ^2.0.0` — Tauri CLI for `tauri dev` and `tauri build`
- `tauri-build = "2.0"` — Rust build dependency

**Linting/Formatting:**
- ESLint 10 (`eslint ^10.1.0`) with `eslint-plugin-svelte ^3.16.0` and `typescript-eslint ^8.58.0`
- Prettier 3 (`prettier ^3.8.1`) with `prettier-plugin-svelte ^3.2.0`
- `husky ^9.1.7` — git hooks for pre-commit checks
- `cargo fmt` + `cargo clippy` — Rust formatting and linting
  - Clippy configured project-wide in `src-tauri/src/lib.rs`: `warn(clippy::all)`, `warn(clippy::pedantic)`
  - npm scripts: `npm run rust:fmt`, `npm run rust:lint`

**CI:**
- GitHub Actions — three workflows in `.github/workflows/`
  - `autoformat.yml` — auto-formats on push
  - `ci.yml` — ESLint + Prettier + `tsc --noEmit` on PRs (Rust checks run locally only due to heavy GTK/webkit2gtk dependencies)
  - `release.yml` — builds Windows (MSI + NSIS) and Android (APK aarch64) on version tags

## Key Dependencies

**Frontend (runtime, from `package.json`):**
- `@tauri-apps/api ^2.0.0` — core Tauri JS API (`invoke`, `path` utilities)
- `@tauri-apps/plugin-dialog ^2.0.0` — native file open/save dialogs
- `@tauri-apps/plugin-fs ^2.0.0` — file system read/write (used for Android import)
- `@tauri-apps/plugin-os ^2.3.2` — platform detection (`platform()` call in `src/lib/store.svelte.ts`)
- `@tauri-apps/plugin-process ^2.3.1` — `relaunch()` after OTA update
- `@tauri-apps/plugin-updater ^2.10.0` — OTA update check and download

**Frontend (devDependencies):**
- `svelte-check ^4.0.0` — TypeScript type checking for Svelte files
- `typescript ^6.0.2` — TypeScript compiler
- `tslib ^2.6.0` — TypeScript runtime helpers
- `esbuild ^0.27.7` — bundler used internally by Vite

**Backend Rust (from `src-tauri/Cargo.toml`):**
- `tauri = "2.0"` — core framework
- `tauri-plugin-dialog = "2.0"` — native dialogs
- `tauri-plugin-fs = "2.0"` — file system access
- `tauri-plugin-os = "2.0"` — OS/platform info
- `rusqlite = { version = "0.31", features = ["bundled"] }` — SQLite bindings; `bundled` feature statically links SQLite so no system SQLite dependency is required
- `serde = { version = "1.0", features = ["derive"] }` — serialization for all IPC data structs in `src-tauri/src/models.rs`
- `serde_json = "1.0"` — JSON support

**Desktop-only Rust (conditional via `cfg(not(android, ios))`):**
- `tauri-plugin-updater = "2.0"` — auto-updater
- `tauri-plugin-process = "2.0"` — process control for post-update relaunch

## Database Technology

**Engine:** SQLite 3 (statically linked via `rusqlite` bundled feature — no system dependency)

**Extension:** FTS5 (Full-Text Search) — dual virtual tables:
- `def_fts` — indexes full definition body text
- `def_kw_fts` — indexes keyword-only text (content between `«»` markers)

Search strategy in `src-tauri/src/commands/search.rs`: FTS5 primary, automatic LIKE fallback when FTS returns no results.

**Schema tables** (defined in `src-tauri/src/db.rs::init_schema`):
- `types` — word type taxonomy (gismu, cmavo, lujvo, etc.)
- `authors` — contributor records with initials
- `events` — dictionary edition/release events
- `words` — main lexicon, FK to `types` and `events`
- `word_spellings` — alternate spellings per word (cascade delete)
- `word_affixes` — affix forms per word (cascade delete)
- `word_usage` — cross-references between words (cascade delete)
- `definitions` — definitions with grammar codes, usage, case tags (cascade delete)
- `settings` — DB metadata and migration tracking flags

**Performance configuration:**
- WAL mode enabled on every open: `PRAGMA journal_mode=WAL`
- Foreign keys enforced: `PRAGMA foreign_keys=ON`
- Covering indexes on `words.name`, `words.LOWER(name)`, `words.type`, `words.event_start`, `words.event_end`, `word_affixes.affix`, `definitions(word_id, position)`
- `add_missing_indexes` runs on every `open_database` call to add indexes to older DB files

**Migrations** (idempotent, tracked via `settings` table flags):
- `migrate_words_unique_if_needed` — adds UNIQUE constraint on `(name, type)`
- `migrate_event_columns_if_needed` — corrects swapped `annotation`/`notes` column data

**Default database path:** `{app_data_dir}/lod.db` (resolved via Tauri `app.path().app_data_dir()`)

## Configuration

**App configuration:**
- `src-tauri/tauri.conf.json` — Tauri config (version, window dimensions, bundle targets, updater endpoint, plugin settings)
  - Window: 1280×820, min 360×500, resizable
  - Bundle: `createUpdaterArtifacts: true`, targets all platforms
  - Android `minSdkVersion: 24`, iOS `minimumSystemVersion: "16"`

**Vite:**
- `vite.config.ts` — plugin registration, dev server port 5173, env prefix `VITE_` and `TAURI_ENV_*`

**TypeScript:**
- `tsconfig.json` — checked via `npm run check` = `tsc --noEmit`

**Environment variables:**
- No `.env` file in project root
- `VITE_` and `TAURI_ENV_*` prefixed vars forwarded to frontend
- CI secrets required: `TAURI_SIGNING_PRIVATE_KEY`, `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`, `GITHUB_TOKEN`, `ANDROID_KEYSTORE_BASE64`, `ANDROID_KEYSTORE_PASSWORD`, `ANDROID_KEY_ALIAS`

## npm Scripts

```bash
npm run dev             # Vite dev server only
npm run dev:tauri       # tauri dev (Vite + Rust backend)
npm run build           # vite build
npm run tauri           # tauri CLI passthrough
npm run check           # tsc --noEmit
npm run lint            # eslint .
npm run lint:fix        # eslint . --fix
npm run format          # prettier --write .
npm run format:check    # prettier --check .
npm run rust:fmt        # cargo fmt
npm run rust:fmt:check  # cargo fmt -- --check
npm run rust:lint       # cargo clippy -- -D warnings
npm run ci:check        # format:check + lint + check
```

## Platform Requirements

**Development:**
- Node.js 24+
- Rust stable toolchain
- Android development: JDK 17 (Temurin), Android SDK, NDK, `cargo-ndk` crate
- Linux system packages for Tauri: `libgtk-3-dev`, `libwebkit2gtk-4.1-dev`, `libappindicator3-dev`, `librsvg2-dev`

**Production targets:**
- Windows: MSI installer and NSIS installer (x86_64)
- macOS: `.app.tar.gz` and `.dmg`
- Linux: AppImage
- Android: APK (aarch64; also armv7, x86_64, i686 Rust targets configured in CI)

---

*Stack analysis: 2026-04-07*
