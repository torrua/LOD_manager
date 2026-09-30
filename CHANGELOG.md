# Changelog

All notable changes to LOD Manager are documented here. See [conventional commits](https://www.conventionalcommits.org/) for commit guidelines.

---

## [1.7.1](https://github.com/torrua/LOD_manager/releases/tag/v1.7.1) — 2026-09-30

### Bug Fixes

- **Android: Database Persistence** — Auto-open canonical `app_data_dir/lod.db` at startup instead of showing useless file picker dialogs blocked by Scoped Storage (Android 10+); data now survives app restarts and updates
- **Android: content:// Import** — Copy imported database as `lod.db` (canonical path) instead of timestamped filename, ensuring data persists across restarts
- **Android: UI Adaptation** — Hide desktop-only controls (Switch DB, New DB, Close, Browse files, Text Converter) on Android; show "↓ From GitHub" as the primary import method
- **Mobile: Form Buttons** — Add bottom padding to Save/Cancel buttons so they're not hidden behind the mobile navigation bar
- **Import UI** — Make file list scrollable with max height to prevent overflow; enlarge Import and GitHub download buttons for better tap targets
- **Code Badges** — Restyle inline file name badges (`Types.txt`, etc.) to be thinner and more elegant

---

## [1.7.0](https://github.com/torrua/LOD_manager/releases/tag/v1.7.0) — 2026-09-30

### Features & `torrua/loglan_core` Compatibility

- **100% `torrua/loglan_core` (`export.db`) Schema Parity**:
  - Support canonical `"group"` (`types`) and `"match"` (`words`) columns with automatic legacy migration from `group_` / `match_`
  - Combine `definitions.slots` and `definitions.grammar_code` on read (e.g. `(2a)`, `(3v)`) and split leading slot digits on save/import
  - Extract word affixes (`djifoa`), derived complexes (`Used In`), morphological `Parents` (ordered by `origin` formula), and non-complex `Children` from `connect_words`
  - Resolve word `Source` and author word counts via `connect_authors` + JSON `words.notes`
  - Format `words.year` (`DATE` `'YYYY-01-01'`) and `words.rank` with JSON `words.notes` (`{"author", "year", "rank"}`), suppressing `'null'` strings
  - Support `loglan_core` `settings` schema (`date, db_version, last_word_id, db_release`) in database statistics
  - Add `syllables`, `keys`, `connect_keys`, and `connect_authors` tables to `init_schema`
- **Dedicated `Parents` & `Children` Sections**: Separate complex constituent words (`Parents`, sorted morphologically by `origin`) from derived words (`Children`) in `WordDetail.svelte`
- **Smart Affix vs. Word Navigation**: Prioritize `Afx`/`Affix` entries (including hyphenated forms like `hei-` and `-kai`) when clicking affix chips, while preferring standalone words (`LW`) in `{xref}`, `Used In`, `Parents`, and `Children`
- **Project & Architecture Documentation**: Add `PROJECT.md`, `ARCHITECTURE.md`, `AGENTS.md`, `CLAUDE.md`, `AUDIT_REPORT.md`, and `TODO.md`

### Bug Fixes

- Synchronize `Definition` and `SaveDefinition` IPC fields (`grammar` ↔ `grammar_code`, `tags` ↔ `case_tags`) so grammar codes and case tags display and save without data loss
- Remove `UNIQUE(name, type)` constraint on `words` to preserve all 11 homonymous/historical word pairs across events in `export.db`
- Fix `FOREIGN KEY constraint failed` when deleting words, definitions, or authors on `export.db` (where foreign keys lack `ON DELETE CASCADE`)
- Fix `db::save_type` (`SET type=?1`), `db::delete_type` (`NOT NULL` guard), `db::get_event_words` (`event_start`/`event_end`), and `db::save_word` (`id_old` preservation, `event_id` resolution, and `denormalize_word_fields` round-trip)
- Fix `import.rs` and `converter/converter.rs` column swaps (`annotation`/`suffix`, `parentable`/`description`), case-insensitive `"False"` boolean parsing, `event_end = 9999` handling, and multi-spelling `id_old -> Vec<word_id>` definition mapping
- Make `debug_update_check` async (`updater.check().await`) to prevent `block_on` panic on Tokio runtime
- Remove restrictive `max-width` truncation on event metadata chips (`From` / `Until`) and top-bar event badges when horizontal space is available
- Fix ` release.yml` JSON syntax for `windows-x86_64-msi` and update `tauri.conf.json` updater endpoint to GitHub Releases

### Performance & UX Improvements

- Incremental FTS5 cleanup on `delete_word` instead of full `rebuild_fts`
- Reuse active `AppState` connection in `rebuild_fts` and `compact_db`
- Direct in-memory Android import in `import_contents` without temporary disk round-trip
- Add live search filtering and `ArrowUp`/`ArrowDown` keyboard focus navigation on the `Events` tab
- Fix `NaN` date sorting on empty event dates and avoid in-place `$state` array mutation in `autoSelectLatestEvent`
- Introduce typed `AppError` enum in Rust backend

---

## [1.6.8](https://github.com/torrua/LOD_manager/releases/tag/v1.6.8) — 2026-04-04

### Features

- Add skipped rows tracking and reporting in import results
- Display import results in debug panel with detailed breakdown
- Enable WAL mode by default for better concurrent access
- Add file size validation (100MB limit) for Android imports

### Refactoring

- Split lib.rs (1304 lines) into 8 command submodules by domain
  - commands/database.rs — open, create, stats, default path
  - commands/words.rs — word/definition CRUD
  - commands/events.rs — event CRUD + get_event_words
  - commands/types.rs — type CRUD
  - commands/authors.rs — author CRUD
  - commands/import.rs — import commands
  - commands/search.rs — search, rebuild_fts, fts_is_ready
  - commands/export.rs — export commands

### Bug Fixes

- Replace `.unwrap()` in import transaction with proper error handling
- Fix potential panic during database import on locked/corrupt DB

### Testing

- Add 10 new tests: migrations, import/export, FTS rebuild, skipped rows
- Total test count: 11 → 24

### Documentation

- Add module-level documentation to db.rs, import.rs, lib.rs
- Document complex SQL strategy in get_word function

---

## [1.6.6](https://github.com/torrua/LOD_manager/releases/tag/v1.6.6) — 2026-04-02

### Bug Fixes

- Remove winres dependency to fix Windows linker duplicate VERSION resource error
- Fix Rust formatting in app.handle() chain

---

## [1.6.5](https://github.com/torrua/LOD_manager/releases/tag/v1.6.5) — 2026-04-02

### Features

- Add Tauri auto-updater with GitHub releases integration
- Embed Windows version info and metadata into exe
- Enable signed Android APK builds in release workflow

### Bug Fixes

- Add missing esbuild dependency required by vite 8
- Broaden APK find pattern and add debug output
- Patch build.gradle.kts to enable Android release signing

### Documentation

- Add MIT license
- Add SECURITY.md, PRIVACY.md, and NOTICES.md

### Dependencies

- Add `@tauri-apps/plugin-updater` and `@tauri-apps/plugin-process`
- Add `tauri-plugin-updater` and `tauri-plugin-process` (Rust)
- Add `winres` for Windows version resources
- Add `esbuild` for Vite 8 compatibility

## [1.6.4] — 2026-04-01

### 🚀 Performance Improvements

- **FTS Updates**: Implemented incremental FTS index updates on definition save/delete
- **Database Stats**: Optimized `get_db_stats` to use single query with subqueries
- **Import**: Added FTS rebuild to `import_lod_contents` command

### 🧪 Testing

- Added `test_fts_update_incremental` test for FTS functionality

## [1.6.0] — 2026-03-12

### 🚀 Performance Improvements

- **Search Speed**: Significantly improved English-to-Loglan search performance with optimized database queries
- **Database Indexes**: Added missing indexes for better query performance across all tables
- **FTS Rebuild**: Enhanced full-text search index rebuilding process with better error handling

### 🐛 Bug Fixes

- **Word Navigation**: Fixed jump to word functionality for smoother navigation between search results
- **Unicode Handling**: Fixed unicode escape sequences in keyword search patterns
- **Code Quality**: Resolved Clippy linting warnings and enforced stricter code standards
- **Formatting**: Fixed prettier formatting issues across frontend components

### 🔧 Technical Improvements

- **Database Schema**: Improved database consistency with proper indexing strategy
- **Error Handling**: Enhanced error handling in database operations
- **Build Process**: Improved CI/CD pipeline with comprehensive formatting checks
- **Documentation**: Updated inline documentation with proper markdown formatting

### 📝 Code Quality

- Enforced stricter Rust linting rules with Clippy
- Improved code comments for better maintainability
- Standardized error handling patterns

## [1.5.0] — 2026-03-11

- Add Settings button to desktop header
- Fix mobile "+" button to directly create new elements for current tab
- Remove sidebar completely for Types and Authors sections in desktop mode
- Improve button height consistency in header
- Fix linting issues and code quality
- Update version to 1.5.0

## [1.1.0] — 2026-03-09

- Add GitHub download functionality for LOD files
- Improve TypeScript typing for GitHub API responses
- Auto-create database in app directory on Android first launch
- Fix GitHub download list duplication on repeated clicks
- Add UNIQUE constraint on (word_id, position) for definitions to prevent duplicates on re-import

## [1.0.7] — 2026-03-09

- Bump version to 1.0.7

## [1.0.6] — 2026-03-09

- Bump version to 1.0.6

## [1.0.0] — Initial release

- Import LOD text files (Words, Definitions, Events, Types, Authors)
- Loglan → English dictionary browser with virtual scroll
- English → Loglan full-text search (FTS5) with LIKE fallback
- Edit words, definitions, events, types, authors
- Export to HTML (Loglan → English)
- Collapsible "Used In" and "Words Added / Removed" sections
- Dark / light theme
- Narrow (mobile) layout
