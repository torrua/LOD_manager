# PROJECT.md — LOD Manager

> **Cross-platform Desktop & Mobile Dictionary Editor for the Loglan Online Dictionary (LOD)**

---

## 1. Project Identity

| Attribute            | Value                                                                 |
| -------------------- | --------------------------------------------------------------------- |
| **Name**             | LOD Manager (`com.lod.manager`)                                       |
| **Version**          | `1.7.0`                                                               |
| **Application Type** | Cross-platform desktop (Windows/Linux/macOS) & mobile (Android) app   |
| **Domain**           | Lexicography / Constructed Language ([Loglan](http://www.loglan.org)) |
| **Frontend Stack**   | Svelte 5 (Runes), TypeScript 6.0, Vite 8.0                            |
| **Backend Stack**    | Rust (Edition 2024), Tauri 2.10, `rusqlite` 0.31 (bundled SQLite 3)   |
| **Search Engine**    | SQLite FTS5 (`unicode61 remove_diacritics 1`) + `LIKE` fallback       |
| **License**          | MIT                                                                   |

---

## 2. Purpose & Core Capabilities

**LOD Manager** provides maintainers and users of the Loglan Online Dictionary with a fast, offline-first relational dictionary browser and editor:

1. **Loglan → English (L→E) Dictionary Browser**:
   - Instant prefix and wildcard (`*`, `?`) filtering over 10,000+ Loglan words.
   - Filter by word type (`C-prim`, `2-cpx`, `LW`, etc.), type group (` Predicate`, `Structure`, etc.), and lexical event timeline (`Start`, `L1`, `L4`, etc.).
   - Custom virtual scrolling (`Sidebar.svelte`, 28px row height) for 60fps rendering of large word lists.
   - Word detail view with metadata chips, etymology/origins, ordered definitions with grammar/usage/case-tag tooltips, affixes (`djifoa`), alternate spellings, "Used In" reverse references, and "Children" (`connect_words` descendants).

2. **English → Loglan (E→L) Full-Text Search**:
   - 4-mode search engine:
     1. **FTS5 Full-Body** (`def_fts` virtual table, ranked with highlighted `snippet()`).
     2. **FTS5 Keywords-Only** (`def_kw_fts` virtual table, indexing `«keyword»` terms).
     3. **LIKE Full-Body Fallback** (for unindexed databases or special characters).
     4. **LIKE Keywords-Only Fallback** (`LIKE '%«query%»%'`).

3. **Full Dictionary CRUD**:
   - Create, edit, and delete **Words** (including affixes and spellings).
   - Create, edit, and delete **Definitions** (with incremental FTS5 index updates).
   - Create, edit, and delete **Lexical Events**, **Word Types**, and **Authors**.
   - **Read-Only Mode** toggle to prevent accidental edits while browsing.

4. **Data Import & Conversion**:
   - Import legacy `@`-delimited LOD text files (`Type.txt`, `Author.txt`, `LexEvent.txt`, `Words.txt`, `WordSpell.txt`, `WordDefinition.txt`, `Settings.txt`) from local disk, Android `content://` URIs, or directly from GitHub (`torrua/LOD`).
   - Directory-based text-to-SQLite converter (`converter/converter.rs`) compatible with the Python `loglan_converter` pipeline.

5. **Static HTML Dictionary Export**:
   - Generates a self-contained, responsive HTML dictionary with alphabetical navigation, live client-side search, and optional lexical event filtering using a 4-query bulk export pipeline (`export.rs`).

6. **Database Maintenance & Auto-Updates**:
   - Database statistics inspector, FTS5 index rebuild (`rebuild_fts`), and `VACUUM` compaction (`compact_db`).
   - Desktop auto-updater via `tauri-plugin-updater` signed with Minisign.

---

## 3. Repository Structure

```text
LOD Manager/
├── index.html                  # Vite HTML entry point
├── package.json                # Node scripts & dependencies (v1.7.0)
├── tsconfig.json               # Strict TypeScript configuration
├── vite.config.ts              # Vite + Svelte 5 bundler config
├── svelte.config.js            # Svelte preprocessor config
├── eslint.config.js            # Flat ESLint 10 config (TS + Svelte)
├── .prettierrc                 # Prettier formatting rules
├── PROJECT.md                  # Project overview & specification (this file)
├── ARCHITECTURE.md             # System architecture & data flow design
├── AGENTS.md                   # Coding guidelines & commands for AI agents
├── CLAUDE.md                   # Quick context entrypoint for Claude/AI assistants
├── AUDIT_REPORT.md             # Comprehensive architectural audit & bug report
├── TODO.md                     # Prioritized bug-fix & optimization checklist
├── README.md                   # User & developer quickstart
├── CONTRIBUTING.md             # Contributor workflow guide
├── CHANGELOG.md                # Release history
├── SECURITY.md                 # Vulnerability reporting policy
├── PRIVACY.md                  # Privacy policy
├── NOTICES.md                  # Third-party license notices
├── src/                        # Svelte 5 Frontend
│   ├── main.ts                 # App mount entry point
│   ├── App.svelte              # Root shell, topbar, shortcuts, panel router
│   ├── styles.css              # Global CSS variables & reset
│   ├── types.ts                # Shared TypeScript interfaces for IPC models
│   └── lib/
│       ├── store.svelte.ts     # Global reactive $state singleton & IPC actions
│       ├── text.ts             # LOD markup formatter ({xref}, «kw», --, %)
│       └── components/
│           ├── Sidebar.svelte      # Resizable sidebar, filters, virtual list
│           ├── ELResults.svelte    # English → Loglan search results list
│           ├── WordDetail.svelte   # Word view, definition editor, tooltips
│           ├── WordForm.svelte     # Word create/edit form
│           ├── EventDetail.svelte  # Lexical event view (added/removed words)
│           ├── EventForm.svelte    # Lexical event create/edit form
│           ├── TypesPanel.svelte   # Word types CRUD table
│           ├── AuthorsPanel.svelte # Authors CRUD table
│           ├── ToolsDrawer.svelte  # Slide-out drawer (Settings/DB/Import/Export)
│           ├── Toast.svelte        # Notification toast overlay
│           └── Icon.svelte         # Inline SVG icon component
└── src-tauri/                  # Rust / Tauri v2 Backend
    ├── Cargo.toml              # Rust crate manifest
    ├── tauri.conf.json         # Tauri app, window, bundler & updater config
    ├── rustfmt.toml            # Rust formatting rules (Unix newlines, 100 cols)
    ├── .clippy.toml            # Clippy complexity thresholds
    ├── capabilities/           # Tauri v2 permission capabilities (desktop/mobile)
    └── src/
        ├── main.rs             # Binary entry point
        ├── lib.rs              # Tauri builder, plugin setup, inline test suite
        ├── models.rs           # Serde IPC data transfer structs
        ├── db.rs               # SQLite schema, migrations, CRUD & FTS5 queries
        ├── import.rs           # @-delimited LOD file importer
        ├── export.rs           # Bulk 4-query HTML dictionary generator
        ├── commands/           # Tauri #[tauri::command] handlers by domain
        │   ├── mod.rs          # AppState, Db<'a>, Res<T>, with_db helpers
        │   ├── database.rs     # open_database, create_database, get_db_stats, get_default_db_path
        │   ├── words.rs        # get_words, get_word, save_word, delete_word, save_definition, delete_definition
        │   ├── events.rs       # get_events, save_event, delete_event, get_event_words
        │   ├── types.rs        # get_types, save_type, delete_type
        │   ├── authors.rs      # get_authors, save_author, delete_author
        │   ├── search.rs       # search_english, rebuild_fts, compact_db, fts_is_ready
        │   ├── import.rs       # import_lod_contents, import_lod_files, convert_text_files
        │   └── export.rs       # export_html, export_html_to_file
        └── converter/          # Python loglan_converter compatible text-to-SQLite module
            ├── mod.rs
            ├── converter.rs
            └── tests.rs
```

---

## 4. Tauri IPC Command Reference (30 Commands: 29 Cross-Platform + 1 Desktop)

| Domain       | Command               | Module               | Signature / Purpose                                                                |
| ------------ | --------------------- | -------------------- | ---------------------------------------------------------------------------------- |
| **Database** | `open_database`       | `commands::database` | `(path: String) -> Res<AppInfo>` — Opens DB, enables WAL, runs schema & migrations |
| **Database** | `create_database`     | `commands::database` | `(path: String) -> Res<AppInfo>` — Deletes existing file and initializes fresh DB  |
| **Database** | `get_db_stats`        | `commands::database` | `() -> Res<DbStats>` — Counts all entity tables and loads `settings`               |
| **Database** | `get_default_db_path` | `commands::database` | `() -> Res<String>` — Returns canonical `<app_data_dir>/lod.db`                    |
| **Words**    | `get_words`           | `commands::words`    | `(q, type_filter, event_id) -> Res<Vec<WordListItem>>`                             |
| **Words**    | `get_word`            | `commands::words`    | `(id: i64) -> Res<WordDetail>` — 5-step query for full word details                |
| **Words**    | `save_word`           | `commands::words`    | `(id: Option<i64>, data: SaveWord) -> Res<WordDetail>`                             |
| **Words**    | `delete_word`         | `commands::words`    | `(id: i64) -> Res<()>`                                                             |
| **Defs**     | `save_definition`     | `commands::words`    | `(id: Option<i64>, word_id: i64, data: SaveDefinition) -> Res<WordDetail>`         |
| **Defs**     | `delete_definition`   | `commands::words`    | `(id: i64, word_id: i64) -> Res<WordDetail>`                                       |
| **Events**   | `get_events`          | `commands::events`   | `() -> Res<Vec<EventItem>>`                                                        |
| **Events**   | `save_event`          | `commands::events`   | `(id: Option<i64>, data: SaveEvent) -> Res<EventItem>`                             |
| **Events**   | `delete_event`        | `commands::events`   | `(id: i64) -> Res<()>`                                                             |
| **Events**   | `get_event_words`     | `commands::events`   | `(event_id: i64) -> Res<(Vec<String>, Vec<String>)>`                               |
| **Types**    | `get_types`           | `commands::types`    | `() -> Res<Vec<TypeItem>>`                                                         |
| **Types**    | `save_type`           | `commands::types`    | `(id: Option<i64>, data: SaveType) -> Res<Vec<TypeItem>>`                          |
| **Types**    | `delete_type`         | `commands::types`    | `(id: i64) -> Res<Vec<TypeItem>>`                                                  |
| **Authors**  | `get_authors`         | `commands::authors`  | `() -> Res<Vec<AuthorItem>>`                                                       |
| **Authors**  | `save_author`         | `commands::authors`  | `(id: Option<i64>, data: SaveAuthor) -> Res<Vec<AuthorItem>>`                      |
| **Authors**  | `delete_author`       | `commands::authors`  | `(id: i64) -> Res<Vec<AuthorItem>>`                                                |
| **Search**   | `search_english`      | `commands::search`   | `(params: ELSearchParams) -> Res<Vec<ELResult>>`                                   |
| **Search**   | `rebuild_fts`         | `commands::search`   | `() -> Res<i64>` — Rebuilds `def_fts` and `def_kw_fts`                             |
| **Search**   | `compact_db`          | `commands::search`   | `() -> Res<String>` — Runs `VACUUM` and returns new DB size in MB                  |
| **Search**   | `fts_is_ready`        | `commands::search`   | `() -> bool` — Checks if `def_fts` and `def_kw_fts` are populated                  |
| **Import**   | `import_lod_files`    | `commands::import`   | `(paths: Vec<String>) -> Res<ImportResult>`                                        |
| **Import**   | `import_lod_contents` | `commands::import`   | `(files: Vec<(String, String)>) -> Res<ImportResult>` (100 MB cap)                 |
| **Import**   | `convert_text_files`  | `commands::import`   | `(text_dir: String) -> Res<ImportResult>`                                          |
| **Export**   | `export_html`         | `commands::export`   | `(event_name: Option<String>) -> Res<String>`                                      |
| **Export**   | `export_html_to_file` | `commands::export`   | `(path: String, event_name: Option<String>) -> Res<()>`                            |
| **Updater**  | `debug_update_check`  | `lib.rs` (`desktop`) | `(app: AppHandle) -> Res<String>`                                                  |

---

## 5. Related Documentation

- **[ARCHITECTURE.md](file:///c:/Users/User/OneDrive/JS/LOD%20Manager/ARCHITECTURE.md)** — Detailed architecture, data flows, state management, and ADRs.
- **[AGENTS.md](file:///c:/Users/User/OneDrive/JS/LOD%20Manager/AGENTS.md)** — Developer & AI agent coding rules, build commands, and conventions.
- **[AUDIT_REPORT.md](file:///c:/Users/User/OneDrive/JS/LOD%20Manager/AUDIT_REPORT.md)** — Comprehensive Senior Architect audit report, root-cause bug analysis, and optimization guide.
- **[TODO.md](file:///c:/Users/User/OneDrive/JS/LOD%20Manager/TODO.md)** — Prioritized action plan and bug-fix checklist.
