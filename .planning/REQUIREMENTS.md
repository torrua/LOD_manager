# Requirements: LOD Manager

## Project Context

**Mode**: Analysis - Understand existing codebase state and capabilities

**Technology Stack**:

- Frontend: Svelte 5 (runes), TypeScript, Vite
- Backend: Rust with Tauri 2.0
- Database: SQLite with FTS5 full-text search

---

## Current Capabilities

### 1. Database Management

- [x] Open existing SQLite database
- [x] Create new database
- [x] Get database statistics (word, definition, event counts)
- [x] Get default DB path (app data directory)
- [x] Database schema initialization with FTS indexes

### 2. Word Management

- [x] List all words with type and definition count
- [x] Filter words by search query (prefix match with wildcards)
- [x] Filter words by type
- [x] Filter words by lexical event
- [x] Get single word detail with all related data
- [x] Save word (create/update)
- [x] Delete word

### 3. Definition Management

- [x] Save definition (create/update)
- [x] Delete definition
- [x] Multiple definitions per word with position ordering

### 4. Word Relations

- [x] Affixes (word components)
- [x] Spellings (alternative spellings)
- [x] "Used In" tracking (words that reference this word)

### 5. Lexical Events

- [x] List all events
- [x] Save event (create/update)
- [x] Delete event
- [x] Auto-select latest event on database open
- [x] Filter words by event

### 6. Word Types

- [x] List all types with word counts
- [x] Group types by group name
- [x] Save type (create/update)
- [x] Delete type

### 7. Authors

- [x] List all authors with word counts
- [x] Save author (create/update)
- [x] Delete author

### 8. Import (LOD Text Files)

- [x] Import from file paths (desktop)
- [x] Import from content URIs (Android)
- [x] Import from GitHub URLs (embedded content)
- [x] Parse @ delimited format
- [x] Return import statistics

### 9. English → Loglan Search

- [x] FTS5 full-text search in definitions
- [x] LIKE fallback search
- [x] Keyword-only search (within «» markers)
- [x] Snippet generation
- [x] Grammar and type display in results

### 10. FTS Index Management

- [x] Check if FTS index is ready
- [x] Rebuild FTS index
- [x] WAL mode for non-blocking rebuild

### 11. Export

- [x] Generate HTML dictionary (Loglan → English)
- [x] Export to file

### 12. UI Features

- [x] Dark/light theme
- [x] Read-only mode
- [x] History navigation (back/forward)
- [x] Toast notifications
- [x] Delete confirmations
- [x] Mobile-responsive layout
- [x] Virtual scrolling for large lists
- [x] Preferences (stored in localStorage)

---

## Frontend Architecture

### State Management

- Svelte 5 runes (`$state`, `$derived`)
- Global store in `src/lib/store.svelte.ts`
- Tauri invoke for backend communication

### Components

- `App.svelte` - Main application
- `Sidebar.svelte` - Navigation sidebar
- `WordDetail.svelte` - Word display/edit
- `WordForm.svelte` - Word editing form
- `EventDetail.svelte` - Event display/edit
- `EventForm.svelte` - Event editing form
- `TypesPanel.svelte` - Type management
- `AuthorsPanel.svelte` - Author management
- `ELResults.svelte` - English search results
- `ToolsDrawer.svelte` - Import/export/settings
- `DeleteModal.svelte` - Delete confirmation
- `Toast.svelte` - Notifications
- `Icon.svelte` - Icon component

---

## Backend Architecture

### Tauri Commands (27 commands)

See `src-tauri/src/lib.rs` for complete list.

### Rust Modules

- `db.rs` - Database operations, schema, migrations, FTS
- `import.rs` - LOD file parsing
- `export.rs` - HTML generation
- `models.rs` - Data structures

---

## Acceptance Criteria for Analysis

1. All 27 Tauri commands are documented and functional
2. Database schema supports all CRUD operations
3. FTS search returns relevant results with snippets
4. Import correctly parses @ delimited LOD files
5. Export generates valid HTML dictionary
6. UI handles 10,000+ words with virtual scrolling
7. Cross-platform (Windows, Android) functionality verified
8. No critical bugs in core functionality
