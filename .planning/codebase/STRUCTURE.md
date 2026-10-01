# Codebase Structure

**Analysis Date:** 2026-04-07

## Summary

LOD Manager is organized as a standard Tauri v2 monorepo with a Vite/Svelte frontend in `src/` and a Rust Cargo crate in `src-tauri/src/`. After commit 975d1bb the Rust backend commands were extracted from a single `lib.rs` into a `commands/` submodule directory grouped by domain. The frontend has no sub-routing; all view logic is in `src/App.svelte` and delegated to flat component files under `src/lib/components/`.

---

## Directory Layout

```
app/                              # Project root
├── src/                          # Svelte 5 frontend source
│   ├── App.svelte                # Root component and layout shell
│   ├── main.ts                   # Vite entry point — mounts App
│   ├── styles.css                # Global CSS custom properties and theme vars
│   ├── types.ts                  # TypeScript interfaces mirroring Rust models
│   └── lib/
│       ├── store.svelte.ts       # Single $state store + all IPC action functions
│       ├── text.ts               # Text utility helpers
│       └── components/           # All UI components (flat, no sub-directories)
│           ├── AuthorsPanel.svelte
│           ├── DeleteModal.svelte
│           ├── ELResults.svelte
│           ├── EventDetail.svelte
│           ├── EventForm.svelte
│           ├── Icon.svelte
│           ├── Sidebar.svelte
│           ├── Toast.svelte
│           ├── ToolsDrawer.svelte
│           ├── TypesPanel.svelte
│           ├── WordDetail.svelte
│           ├── WordForm.svelte
│           └── _placeholder.ts   # Keeps directory in git when components move
├── src-tauri/                    # Rust Tauri backend (Cargo workspace member)
│   ├── src/
│   │   ├── main.rs               # Binary entry — calls lod_manager::run()
│   │   ├── lib.rs                # Library root: plugin registration, invoke_handler, AppState init
│   │   ├── models.rs             # Serde structs shared between commands and db layer
│   │   ├── db.rs                 # All SQLite data access functions and migrations
│   │   ├── export.rs             # HTML generation (bulk 4-query approach)
│   │   ├── import.rs             # @-delimited LOD text file parser and importer
│   │   └── commands/             # Tauri command handlers grouped by domain
│   │       ├── mod.rs            # AppState, Db, Res, err, with_db, with_db_mut
│   │       ├── database.rs       # open_database, create_database, get_db_stats, get_default_db_path
│   │       ├── words.rs          # get_words, get_word, save_word, delete_word, save_definition, delete_definition
│   │       ├── events.rs         # get_events, save_event, delete_event, get_event_words
│   │       ├── types.rs          # get_types, save_type, delete_type
│   │       ├── authors.rs        # get_authors, save_author, delete_author
│   │       ├── search.rs         # search_english, rebuild_fts, compact_db, fts_is_ready
│   │       ├── export.rs         # export_html, export_html_to_file
│   │       └── import.rs         # import_lod_contents, import_lod_files, convert_text_files
│   │   └── converter/            # Text-to-SQLite batch converter (separate from import pipeline)
│   │       ├── mod.rs
│   │       ├── converter.rs      # convert_text_to_sqlite — directory-level bulk import
│   │       └── tests.rs
│   ├── capabilities/             # Tauri v2 capability JSON files (per-platform permissions)
│   ├── icons/                    # App icons for all platforms
│   │   ├── android/              # Android mipmap densities
│   │   └── ios/                  # iOS icon set
│   ├── gen/                      # Tauri-generated platform project files (Android/iOS)
│   ├── .cargo/                   # Cargo config (target overrides, etc.)
│   ├── Cargo.toml                # Rust package manifest
│   └── tauri.conf.json           # Tauri app configuration
├── .planning/                    # GSD planning documents (not shipped)
│   ├── codebase/                 # Codebase analysis documents
│   └── phases/                   # Implementation phase plans
├── .github/
│   └── workflows/                # CI/CD GitHub Actions workflows
├── icons/                        # Source icons for `tauri icon` generation
├── dist/                         # Vite build output (generated, not committed)
├── node_modules/                 # npm dependencies (generated)
├── index.html                    # Vite HTML entry
├── package.json                  # npm manifest
├── vite.config.ts                # Vite configuration
├── svelte.config.js              # Svelte preprocessor config
├── tsconfig.json                 # TypeScript config
└── eslint.config.js              # ESLint flat config
```

---

## Key Files and Their Roles

**Entry Points:**
- `src/main.ts`: Imports and mounts `App.svelte` — the single Vite entry point
- `src-tauri/src/main.rs`: Binary entry; calls `lod_manager::run()` (one line)
- `src-tauri/src/lib.rs`: Real entry — registers Tauri plugins, `AppState`, and all commands via `invoke_handler`; also contains `debug_update_check` command and the full integration test suite

**Configuration:**
- `src-tauri/tauri.conf.json`: App identifier, version (`1.6.10`), update endpoints, window defaults
- `src-tauri/Cargo.toml`: Rust dependencies (`rusqlite`, `tauri`, `serde`, Tauri plugins)
- `vite.config.ts`: Vite build config targeting Tauri IPC (`@tauri-apps/api`)
- `src-tauri/capabilities/`: Per-platform permission declarations required by Tauri v2

**Core Logic:**
- `src/lib/store.svelte.ts`: The frontend application layer — contains every `invoke()` call, `app` reactive state, localStorage persistence, filter logic, history stack, platform detection, and update management
- `src-tauri/src/db.rs`: All database access — schema DDL, CRUD for every entity, FTS5 dual-table search, migrations, VACUUM
- `src-tauri/src/commands/mod.rs`: Shared types and `with_db`/`with_db_mut` helpers used by every command module

**Data contracts:**
- `src/types.ts`: TypeScript interfaces that must stay in sync with `src-tauri/src/models.rs`
- `src-tauri/src/models.rs`: Rust structs with `#[derive(Serialize, Deserialize)]` for IPC serialization

---

## src/ Frontend Structure

```
src/
├── App.svelte           # Root shell: tab bar, toolbar, main panel switcher, keyboard shortcuts
├── main.ts              # mount(App, { target: document.body })
├── styles.css           # CSS custom properties (--bg, --fg, --accent, etc.), theme variants
├── types.ts             # All shared TypeScript types
└── lib/
    ├── store.svelte.ts  # $state app object + all exported action functions
    ├── text.ts          # Text utilities (e.g., Loglan-specific string helpers)
    └── components/
        ├── Sidebar.svelte       # Word/event list with virtual scroll, search bar, tab switcher
        ├── ELResults.svelte     # Renders app.elResults for English->Loglan search
        ├── WordDetail.svelte    # Read-only word view with definitions, affixes, spellings
        ├── WordForm.svelte      # Create/edit word form
        ├── EventDetail.svelte   # Read-only event view with word lists
        ├── EventForm.svelte     # Create/edit event form
        ├── TypesPanel.svelte    # Types tab: list + inline create/edit
        ├── AuthorsPanel.svelte  # Authors tab: list + inline create/edit
        ├── ToolsDrawer.svelte   # Slide-out overlay: import, export, database tools, settings
        ├── DeleteModal.svelte   # Reusable confirmation dialog
        ├── Toast.svelte         # Fixed-position notification (auto-dismisses at 2800ms)
        └── Icon.svelte          # SVG sprite icon lookup by name string
```

**Panel routing in App.svelte:**

`app.panel` controls which main content area is shown:
- `'welcome'` — default/empty state
- `'word'` — `<WordDetail>` (when `!app.editing`)
- `'word-form'` — `<WordForm>` (when `app.editing`)
- `'event'` — `<EventDetail>` (when `!app.editing`)
- `'event-form'` — `<EventForm>` (when `app.editing`)

`app.tab` controls which list the sidebar renders:
- `'words'` — virtual-scrolled word list or EL search results
- `'events'` — event list
- `'types'` — `<TypesPanel>` replaces the main content area
- `'authors'` — `<AuthorsPanel>` replaces the main content area

---

## src-tauri/ Backend Structure

```
src-tauri/src/
├── main.rs              # Binary stub
├── lib.rs               # Plugin registration, invoke_handler, integration tests
├── models.rs            # All serde types (read/write structs separated by convention)
├── db.rs                # Data access layer — every SQL operation lives here
├── export.rs            # HTML export: 4-query bulk approach, inline CSS/JS
├── import.rs            # @-delimited LOD format parser; import_files + import_contents
└── commands/
    ├── mod.rs           # AppState struct, type aliases, with_db helpers
    ├── database.rs      # Connection lifecycle, stats, migration orchestration
    ├── words.rs         # Word + definition CRUD commands
    ├── events.rs        # Event CRUD + get_event_words
    ├── types.rs         # Type CRUD
    ├── authors.rs       # Author CRUD
    ├── search.rs        # FTS/LIKE search, rebuild_fts, compact_db
    ├── export.rs        # export_html, export_html_to_file (thin wrappers over export.rs)
    └── import.rs        # import_lod_files, import_lod_contents, convert_text_files
└── converter/
    ├── mod.rs
    ├── converter.rs     # convert_text_to_sqlite: directory-scan, ordered insert
    └── tests.rs         # Converter unit tests
```

**Dependency direction inside src-tauri/src/:**

```
commands/* -> db.rs, models.rs, export.rs, import.rs, converter/
lib.rs     -> commands/*, (debug_update_check inline)
db.rs      -> models.rs
export.rs  -> (no internal deps, only rusqlite)
import.rs  -> models.rs
converter/ -> models.rs
```

No circular dependencies. `commands/mod.rs` is the only place that knows about `AppState`; leaf modules (`words.rs`, `authors.rs`, etc.) import it via `super::`.

---

## Naming Conventions

**Files:**
- Svelte components: PascalCase (`WordDetail.svelte`, `ToolsDrawer.svelte`)
- Rust modules: snake_case (`db.rs`, `models.rs`, `commands/words.rs`)
- TypeScript non-components: camelCase (`store.svelte.ts`, `text.ts`)

**Rust naming:**
- Command functions: snake_case matching the Tauri command name (`get_word`, `save_word`)
- Model structs: PascalCase read variants (`WordDetail`, `TypeItem`), prefix `Save*` for write inputs (`SaveWord`, `SaveType`)
- DB functions: prefixed by operation (`list_words`, `get_word`, `save_word`, `delete_word`)

**TypeScript naming:**
- Interfaces: PascalCase matching Rust struct names (`WordDetail`, `ELResult`)
- Store action functions: camelCase verbs (`selectWord`, `saveWord`, `loadWords`, `openDb`)
- The `app` state object fields: camelCase (`curWord`, `dbOpen`, `elFtsReady`)

---

## Where to Add New Code

**New Tauri command (new domain entity):**
1. Add model structs to `src-tauri/src/models.rs` (read struct + `Save*` write struct)
2. Add DB functions to `src-tauri/src/db.rs`
3. Create `src-tauri/src/commands/myentity.rs` with `#[tauri::command]` functions using `with_db`
4. Add `pub mod myentity;` to `src-tauri/src/commands/mod.rs`
5. Register commands in `src-tauri/src/lib.rs` `invoke_handler![]`
6. Add TypeScript interface to `src/types.ts`
7. Add `invoke()` action functions to `src/lib/store.svelte.ts`
8. Add `app.myEntities` field to the `$state` object

**New UI component:**
- Place `.svelte` file directly in `src/lib/components/`
- Import into `src/App.svelte` or the parent component that needs it
- Read/write `app` state from `src/lib/store.svelte.ts`; do not hold local state for data already in the store

**New utility function (frontend):**
- Pure text/string helpers: `src/lib/text.ts`
- Anything touching `app` state or `invoke`: must go in `src/lib/store.svelte.ts`

**New migration:**
- Add an idempotent function to `src-tauri/src/db.rs` following the `migrate_*_if_needed` pattern
- Call it from `commands/database.rs` `open_database` after `init_schema`

---

## Special Directories

**`dist/`:**
- Purpose: Vite production build output
- Generated: Yes (by `npm run build` / Tauri bundler)
- Committed: No

**`src-tauri/gen/`:**
- Purpose: Tauri-generated Android/iOS project scaffolding
- Generated: Yes (by `tauri android init` / `tauri ios init`)
- Committed: Yes (required for mobile builds)

**`.planning/`:**
- Purpose: GSD planning docs (architecture, concerns, phases)
- Generated: No (human/AI authored)
- Committed: Yes

**`icons/` (root):**
- Purpose: Source icon images fed to `tauri icon` to generate all platform sizes
- Generated: No
- Committed: Yes

---

*Structure analysis: 2026-04-07*
