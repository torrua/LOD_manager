# Test Writing Prompt for LOD Manager

## Project Overview

**LOD Manager** - A desktop + mobile editor for the Loglan Online Dictionary (LOD).

**Tech Stack:**

- Frontend: Svelte 5 (runes), TypeScript, Vite 8
- Backend: Rust with Tauri 2.0
- Database: SQLite with FTS5 full-text search

## Testing Setup

### Rust Tests

- **Location**: `src-tauri/src/lib.rs` under `#[cfg(test)]` module
- **Run**: `cargo test --manifest-path src-tauri/Cargo.toml`
- **Single test**: `cargo test --manifest-path src-tauri/Cargo.toml -- test_name`

### Current Tests

```rust
#[cfg(test)]
mod tests {
    // test_in_memory_db_init - verifies database schema creation
    // test_get_word_performance_optimal - performance benchmark
    // test_fts_update_incremental - FTS index updates
}
```

### Frontend Tests

- No test framework configured yet
- Use `npm run check` for TypeScript type validation

## Database Schema

**Tables:**

- `words` - Core vocabulary (id, name, type*id, source, year, rank, match*, origin, notes)
- `definitions` - Word definitions (id, word_id, position, grammar, usage, body, tags)
- `types` - Word type classifications (id, name, group\_)
- `events` - Lexical events (id, name, date, annotation, suffix, notes)
- `authors` - Word authors (id, name)
- `word_affixes` - Word components (id, word_id, affix)
- `word_spellings` - Alternative spellings (id, word_id, spelling)

**FTS Tables:**

- `def_fts` - FTS5 full-text search for definitions
- `def_kw_fts` - Keyword FTS for «keyword» markers

## Key Tauri Commands to Test

1. **Database**: `open_database`, `create_database`, `get_db_stats`, `get_default_db_path`
2. **Words**: `list_words`, `get_word`, `save_word`, `delete_word`, `filter_words`
3. **Definitions**: `save_definition`, `delete_definition`
4. **Events**: `get_events`, `save_event`, `delete_event`, `get_event_words`
5. **Types**: `get_types`, `save_type`, `delete_type`
6. **Authors**: `get_authors`, `save_author`, `delete_author`
7. **Search**: `search_english` (FTS + LIKE fallback)
8. **FTS**: `fts_is_ready`, `rebuild_fts`
9. **Import/Export**: `import_lod_contents`, `import_lod_files`, `export_html`

## Test Requirements

### Unit Tests

- Test each Tauri command with valid and invalid inputs
- Test database operations (CRUD for words, definitions, events, types, authors)
- Test FTS search functionality
- Test import parsing of @ delimited LOD files

### Integration Tests

- Test full workflow: open DB → import → search → export
- Test performance with large datasets (10k+ words)

### Edge Cases

- Empty database
- Invalid file paths
- Malformed LOD files
- Concurrent operations

## Test Template

```rust
#[test]
fn test_feature_name() {
    // Setup: Create in-memory database
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    db::init_schema(&conn).unwrap();
    db::init_fts(&conn).unwrap();

    // Arrange: Insert test data

    // Act: Call the function to test

    // Assert: Verify results
    assert!(/* condition */);
}
```

## Run Tests

```bash
# All tests
cargo test --manifest-path src-tauri/Cargo.toml

# Single test
cargo test --manifest-path src-tauri/Cargo.toml -- test_name

# With output
cargo test --manifest-path src-tauri/Cargo.toml -- --nocapture
```
