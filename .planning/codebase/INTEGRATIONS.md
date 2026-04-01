# External Integrations

**Analysis Date:** 2026-04-01

## Tauri Commands

All commands are defined in `src-tauri/src/lib.rs` and exposed via `tauri::generate_handler![]`.

### Database Commands

- `open_database(path: String) -> AppInfo` - Open an existing SQLite database
- `create_database(path: String) -> AppInfo` - Create new database (removes existing)
- `get_db_stats() -> DbStats` - Get database statistics and path
- `get_default_db_path() -> String` - Get app data directory path for default database

### Word Commands

- `get_words(q: String, type_filter: String, event_id: Option<i64>) -> Vec<WordListItem>` - List words with search/filter
- `get_word(id: i64) -> WordDetail` - Get full word details
- `save_word(id: Option<i64>, data: SaveWord) -> WordDetail` - Create/update word
- `delete_word(id: i64) -> ()` - Delete word

### Definition Commands

- `save_definition(id: Option<i64>, word_id: i64, data: SaveDefinition) -> WordDetail` - Create/update definition
- `delete_definition(id: i64, word_id: i64) -> WordDetail` - Delete definition

### Event Commands

- `get_events() -> Vec<EventItem>` - List all events
- `save_event(id: Option<i64>, data: SaveEvent) -> EventItem` - Create/update event
- `delete_event(id: i64) -> ()` - Delete event
- `get_event_words(event_id: i64) -> (Vec<String>, Vec<String>)` - Get word names by event

### Type Commands

- `get_types() -> Vec<TypeItem>` - List all word types
- `save_type(id: Option<i64>, data: SaveType) -> Vec<TypeItem>` - Create/update type
- `delete_type(id: i64) -> Vec<TypeItem>` - Delete type

### Author Commands

- `get_authors() -> Vec<AuthorItem>` - List all authors
- `save_author(id: Option<i64>, data: SaveAuthor) -> Vec<AuthorItem>` - Create/update author
- `delete_author(id: i64) -> Vec<AuthorItem>` - Delete author

### Import Commands

- `import_lod_files(paths: Vec<String>) -> ImportResult` - Import LOD files from paths
- `import_lod_contents(files: Vec<(String, String)>) -> ImportResult` - Import from file contents (Android)

### Search Commands

- `search_english(params: ELSearchParams) -> Vec<ELResult>` - Full-text English search
- `rebuild_fts() -> i64` - Rebuild FTS5 search index
- `fts_is_ready() -> bool` - Check FTS index status

### Export Commands

- `export_html(event_name: Option<String>) -> String` - Generate HTML export
- `export_html_to_file(path: String, event_name: Option<String>) -> ()` - Write HTML to file

## Plugin Integrations

### tauri-plugin-dialog

- Initialization: `tauri_plugin_dialog::init()`
- Usage: Native file open/save dialogs via `@tauri-apps/plugin-dialog`
- Config: `tauri.conf.json` plugins.dialog = null (default permissions)

### tauri-plugin-fs

- Initialization: `tauri_plugin_fs::init()`
- Usage: File system read/write via `@tauri-apps/plugin-fs`
- Config: `tauri.conf.json` plugins.fs = {} (default permissions)

### tauri-plugin-os

- Initialization: `tauri_plugin_os::init()`
- Usage: OS-level utilities via `@tauri-apps/plugin-os`
- Config: `tauri.conf.json` plugins.os = null (default permissions)

## Database

**Provider:** SQLite via rusqlite

- Version: 0.31 (bundled feature enabled)
- Connection: Direct file path

**Full-Text Search:**

- FTS5 virtual tables: `def_fts` (definition body), `def_kw_fts` (keyword markers)
- Rebuild command: `rebuild_fts`
- Ready check: `fts_is_ready`

**Schema Tables:**

- `words` - Word entries with type, source, year, rank, origin
- `definitions` - Word definitions with grammar, usage, body, tags
- `word_affixes` - Affix mappings
- `word_spellings` - Alternative spellings
- `events` - Lexical events with dates, annotations
- `types` - Word types with groups
- `authors` - Author records
- `settings` - Key-value settings

## File Formats

### Import (LOD format)

- Text files with `@` delimiter
- Types: `*type.txt`
- Authors: `*author*.txt`
- Events: `*lexevent*.txt` or `*event*.txt`
- Words: `word.txt`, `words.txt`
- Spellings: `*wordspell*.txt`
- Definitions: `*worddef*.txt`, `*definition*.txt`
- Settings: `*setting*.txt`

### Export (HTML)

- Standalone HTML file with embedded CSS/JS
- Sidebar with alphabet navigation and search
- Word entries grouped by first letter
- Keyword markers `«...»` rendered as styled spans

## Environment Configuration

**Required env vars:** None

**Secrets location:** Not applicable (no external secrets)

**App data:** Platform-specific app data directory via Tauri path API

## Webhooks & Callbacks

**Incoming:** None

**Outgoing:** None

---

_Integration audit: 2026-04-01_
