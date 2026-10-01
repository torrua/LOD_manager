# External Integrations

**Analysis Date:** 2026-04-07

## Summary

LOD Manager integrates with the host OS via the Tauri IPC bridge, which exposes 25 Rust-backed commands to the Svelte frontend. External integrations are minimal: a GitHub-hosted updater endpoint for OTA updates on desktop, native file system and dialog access on all platforms, and platform detection to adjust behavior between desktop and Android. There are no third-party cloud APIs, authentication providers, or remote databases.

## Tauri Commands (IPC Bridge)

All commands are registered in `src-tauri/src/lib.rs` via `tauri::generate_handler![]` and implemented in submodules under `src-tauri/src/commands/`. The frontend invokes them via `invoke()` from `@tauri-apps/api/core`.

Return type convention: all commands return `Result<T, String>` (aliased as `Res<T>` in `src-tauri/src/commands/mod.rs`). Errors are returned as plain strings.

State management: `AppState` in `src-tauri/src/commands/mod.rs` holds `Mutex<Option<Connection>>` and `Mutex<String>` (db path). Commands access state via `with_db` or `with_db_mut` helpers.

### Database Commands (`src-tauri/src/commands/database.rs`)

| Command | Signature | Description |
|---|---|---|
| `open_database` | `(path: String) -> AppInfo` | Opens or creates a SQLite file, runs schema init + migrations, sets WAL mode |
| `create_database` | `(path: String) -> AppInfo` | Deletes existing file then calls `open_database` |
| `get_db_stats` | `() -> DbStats` | Returns counts for all entity types + settings rows |
| `get_default_db_path` | `() -> String` | Returns `{app_data_dir}/lod.db` via Tauri path API |

### Word Commands (`src-tauri/src/commands/words.rs`)

| Command | Signature | Description |
|---|---|---|
| `get_words` | `(query, type_filter, event_id) -> Vec<WordListItem>` | Filtered word list |
| `get_word` | `(id: i64) -> WordDetail` | Full word record with definitions, affixes, spellings |
| `save_word` | `(id: Option<i64>, data: SaveWord) -> i64` | Insert or update word |
| `delete_word` | `(id: i64) -> ()` | Delete word and cascade-delete related rows |
| `save_definition` | `(id: Option<i64>, word_id: i64, data: SaveDefinition) -> i64` | Insert or update definition |
| `delete_definition` | `(id: i64) -> ()` | Delete a single definition |

### Event Commands (`src-tauri/src/commands/events.rs`)

| Command | Signature | Description |
|---|---|---|
| `get_events` | `() -> Vec<EventItem>` | All dictionary edition events |
| `save_event` | `(id: Option<i64>, data: SaveEvent) -> i64` | Insert or update event |
| `delete_event` | `(id: i64) -> ()` | Delete event |
| `get_event_words` | `(event_id: i64) -> Vec<WordListItem>` | Words introduced in a given event |

### Type Commands (`src-tauri/src/commands/types.rs`)

| Command | Signature | Description |
|---|---|---|
| `get_types` | `() -> Vec<TypeItem>` | All word types with word counts |
| `save_type` | `(id: Option<i64>, data: SaveType) -> i64` | Insert or update type |
| `delete_type` | `(id: i64) -> ()` | Delete type (fails if words reference it) |

### Author Commands (`src-tauri/src/commands/authors.rs`)

| Command | Signature | Description |
|---|---|---|
| `get_authors` | `() -> Vec<AuthorItem>` | All authors with word counts |
| `save_author` | `(id: Option<i64>, data: SaveAuthor) -> i64` | Insert or update author |
| `delete_author` | `(id: i64) -> ()` | Delete author |

### Search Commands (`src-tauri/src/commands/search.rs`)

| Command | Signature | Description |
|---|---|---|
| `search_english` | `(params: ELSearchParams) -> Vec<ELResult>` | English-to-Loglan search; uses FTS5 with automatic LIKE fallback |
| `rebuild_fts` | `() -> i64` | Rebuilds both FTS virtual tables (`def_fts` + `def_kw_fts`); returns total row count |
| `compact_db` | `() -> String` | Runs VACUUM and returns new file size in MB |
| `fts_is_ready` | `() -> bool` | Returns true if FTS tables have been populated |

`ELSearchParams` flags: `use_like: bool` (force LIKE), `use_keywords_only: bool` (search `«»`-delimited keyword sections only), `limit: i64`.

### Import Commands (`src-tauri/src/commands/import.rs`)

| Command | Signature | Description |
|---|---|---|
| `import_lod_contents` | `(files: Vec<(String, String)>) -> ImportResult` | Android variant: receives `(filename, utf8_content)` pairs (content:// URIs pre-read by frontend via plugin-fs); max 100MB total |
| `import_lod_files` | `(paths: Vec<String>) -> ImportResult` | Desktop variant: receives file system paths, reads files directly in Rust |
| `convert_text_files` | `(text_dir: String) -> ImportResult` | Converts `@`-delimited text files (Python loglan_converter format) to SQLite |

All three import commands trigger `db::rebuild_fts` after completion.

### Export Commands (`src-tauri/src/commands/export.rs`)

| Command | Signature | Description |
|---|---|---|
| `export_html` | `(event_name: Option<String>) -> String` | Generates full HTML dictionary string in memory |
| `export_html_to_file` | `(path: String, event_name: Option<String>) -> ()` | Writes HTML dictionary directly to a file path |

### Desktop-Only Command (`src-tauri/src/lib.rs`)

| Command | Signature | Description |
|---|---|---|
| `debug_update_check` | `() -> String` | Manually triggers updater check; returns version string or error message |

Compiled only when `#[cfg(desktop)]`.

## File System Integration

**Plugin:** `tauri-plugin-fs` (JS: `@tauri-apps/plugin-fs`, Rust: `tauri-plugin-fs`)

**Capabilities:**

Desktop (`src-tauri/capabilities/desktop.json`):
- `fs:allow-read-file` — read arbitrary files
- `fs:allow-write-file` — write arbitrary files
- `fs:allow-appdata-read` / `fs:allow-appdata-write` — app data directory access
- `fs:scope-appdata-recursive` — recursive scope on app data dir

Mobile (`src-tauri/capabilities/mobile.json`):
- Same fs permissions as desktop; no updater permissions

**Android-specific pattern:** Android `content://` URIs cannot be accessed by Rust's `std::fs`. The frontend reads file content via `readFile` from `@tauri-apps/plugin-fs` and sends the raw UTF-8 string to `import_lod_contents` as a `Vec<(String, String)>` pair list. See `src-tauri/src/commands/import.rs`.

## Native Dialog Integration

**Plugin:** `tauri-plugin-dialog` (JS: `@tauri-apps/plugin-dialog`, Rust: `tauri-plugin-dialog`)

**Capabilities:** `dialog:allow-open`, `dialog:allow-save` (both desktop and mobile)

Used in the frontend for file picker dialogs when selecting DB files or export destinations.

## OS/Platform Detection

**Plugin:** `tauri-plugin-os` (JS: `@tauri-apps/plugin-os`, Rust: `tauri-plugin-os`)

**Capabilities:** `os:allow-platform`, `os:allow-os-type` (both desktop and mobile)

**Usage in `src/lib/store.svelte.ts`:**
```typescript
import { platform } from '@tauri-apps/plugin-os';
```
Called during startup to determine whether to use the Android import path (`import_lod_contents`) or the desktop file-path import path (`import_lod_files`).

## Auto-Updater Integration

**Plugin:** `tauri-plugin-updater` (desktop only)

**Endpoint:** `https://raw.githubusercontent.com/torrua/LOD_manager/main/latest.json`
- JSON file generated and uploaded to GitHub Releases by the release workflow
- Contains version, release notes URL, and per-platform download URLs with minisign signatures

**Public key:** Stored as `pubkey` in `src-tauri/tauri.conf.json` (base64-encoded minisign key)

**Flow:**
1. `check()` called from frontend via `@tauri-apps/plugin-updater` in `src/lib/store.svelte.ts`
2. If update found, download triggered with `DownloadEvent` progress callbacks
3. After download, `relaunch()` from `@tauri-apps/plugin-process` restarts the app

**Install mode (Windows):** `passive` (shows progress, no user interaction required)

**Capabilities:** `updater:default`, `updater:allow-check`, `updater:allow-download-and-install` (desktop capability only — absent from mobile capability)

**Debug command:** `debug_update_check` Tauri command available in desktop builds for manual update testing.

## CI/CD and Release Pipeline

**Platform:** GitHub Actions (`.github/workflows/`)

**Workflows:**
- `autoformat.yml` — runs auto-format on push to main/develop
- `ci.yml` — frontend checks (ESLint, Prettier, TypeScript) on PRs; Rust checks must be run locally
- `release.yml` — triggered by version tags (`v*.*.*`); two parallel jobs:
  - `windows` job: builds MSI + NSIS on `windows-latest`, generates `latest.json`, publishes to GitHub Releases
  - `android` job: builds APK (aarch64) on `ubuntu-latest`, signs with keystore from secrets

**Version sync:** Release workflow patches `src-tauri/tauri.conf.json` and `package.json` with the tag version at build time (no manual version bump in files required).

**Android signing:** Keystore decoded from `ANDROID_KEYSTORE_BASE64` secret; `build.gradle.kts` patched at CI time via Python script to inject signing configuration.

**Artifacts published to GitHub Releases:**
- `*.msi`, `*.msi.sig`
- `*.exe` (NSIS), `*.exe.sig`
- `*.AppImage`, `*.AppImage.sig`
- `*.app.tar.gz`, `*.dmg`
- `LOD.Manager_{version}_aarch64.apk`
- `latest.json` (updater manifest)

## Platform Capability Matrix

| Feature | Windows | macOS | Linux | Android | iOS |
|---|---|---|---|---|---|
| File open/save dialogs | Yes | Yes | Yes | Yes | Yes |
| File system read/write | Yes | Yes | Yes | Yes | Yes |
| Auto-updater | Yes | Yes | Yes | No | No |
| Process relaunch | Yes | Yes | Yes | No | No |
| Platform detection | Yes | Yes | Yes | Yes | Yes |
| content:// import path | No | No | No | Yes | No |

---

*Integration audit: 2026-04-07*
