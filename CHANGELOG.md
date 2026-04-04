# Changelog

All notable changes to LOD Manager are documented here. See [conventional commits](https://www.conventionalcommits.org/) for commit guidelines.

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
