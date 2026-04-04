# Test Coverage Assessment

## Current Coverage

### Rust Tests

| Test                                | Location    | Coverage                           | Quality     |
| ----------------------------------- | ----------- | ---------------------------------- | ----------- |
| `test_in_memory_db_init`            | lib.rs:L419 | Schema + FTS initialization        | Unit        |
| `test_get_word_performance_optimal` | lib.rs:L432 | get_word performance (100 calls)   | Performance |
| `test_fts_update_incremental`       | lib.rs:L518 | FTS update after definition change | Integration |
| `test_list_words_basic`             | lib.rs:L569 | Word listing with filters          | Unit        |
| `test_fts_search_basic`             | lib.rs:L619 | FTS search functionality           | Integration |
| `test_word_crud_operations`         | lib.rs:L649 | Word create/read/update/delete     | Unit        |
| `test_definition_crud_operations`   | lib.rs:L697 | Definition CRUD                    | Unit        |
| `test_type_crud_operations`         | lib.rs:L749 | Type CRUD with FK constraint       | Unit        |
| `test_event_crud_operations`        | lib.rs:L792 | Event CRUD                         | Unit        |
| `test_author_crud_operations`       | lib.rs:L828 | Author CRUD                        | Unit        |
| `test_word_affixes_and_spellings`   | lib.rs:L862 | Affix/spelling management          | Unit        |

**Total: 11 tests** — all in `lib.rs` under `#[cfg(test)]`

### Frontend Tests

**Status: None configured**

No test framework exists in `package.json`. No Vitest, Playwright, or similar dependencies. This is consistent with the TESTING.md assessment.

## Coverage Gaps

### Critical Priority

| Gap                 | Location            | Impact                                                                   | Recommendation                                                                                                          |
| ------------------- | ------------------- | ------------------------------------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------- |
| Database migrations | `db.rs:112-219`     | Migrations are complex table rebuilds — if they fail, data loss possible | Unit tests for `migrate_words_unique_if_needed` and `migrate_event_columns_if_needed` with pre/post schema verification |
| Import functions    | `import.rs:39-322`  | Core data ingestion — no tests at all                                    | Integration tests with sample LOD files (valid, malformed, empty)                                                       |
| Export functions    | `export.rs:164-479` | HTML generation — no tests                                               | Unit tests for `generate_html` with known data, verify HTML structure                                                   |
| FTS rebuild         | `db.rs:768-814`     | Complex dual-table rebuild — only partially tested                       | Integration test: insert definitions → rebuild → verify search works                                                    |

### High Priority

| Gap                                 | Location         | Impact                                             | Recommendation                                                              |
| ----------------------------------- | ---------------- | -------------------------------------------------- | --------------------------------------------------------------------------- |
| `search_english` command            | `lib.rs:228-261` | 4-way strategy selection with fallback             | Unit tests for each strategy combination (FTS/LIKE × keywords/full)         |
| `save_definition` with FTS update   | `lib.rs:108-128` | FTS update after save is `.ok()` — errors silently | Test that FTS is updated after save                                         |
| `delete_definition` with FTS update | `lib.rs:131-138` | FTS update on delete                               | Test that deleted definition is removed from FTS                            |
| `rebuild_fts` command               | `lib.rs:264-284` | Separate connection logic                          | Test that rebuild works while main connection is in use                     |
| `fts_is_ready`                      | `lib.rs:287-294` | Returns bool based on FTS state                    | Test before/after rebuild, after corrupt FTS                                |
| `export_html_to_file`               | `lib.rs:304-313` | File I/O path                                      | Integration test: export to temp file, verify file exists and is valid HTML |

### Medium Priority

| Gap                         | Location          | Impact                           | Recommendation                                  |
| --------------------------- | ----------------- | -------------------------------- | ----------------------------------------------- |
| `get_db_stats`              | `lib.rs:70-77`    | Stats accuracy                   | Unit test: insert data, verify counts           |
| `get_event_words`           | `lib.rs:223-225`  | Event word filtering             | Unit test with event-scoped words               |
| `create_database`           | `lib.rs:64-67`    | Deletes existing file then opens | Test idempotency                                |
| Import error handling       | `import.rs:54-59` | Temp dir creation failure        | Test graceful failure when temp dir unavailable |
| `import_contents` (Android) | `import.rs:39-71` | Content-based import path        | Test with sample content tuples                 |

### Low Priority

| Gap                   | Location         | Impact                 | Recommendation                                        |
| --------------------- | ---------------- | ---------------------- | ----------------------------------------------------- |
| `get_default_db_path` | `lib.rs:353-357` | Platform-specific path | Manual testing sufficient                             |
| `debug_update_check`  | `lib.rs:318-347` | Debug-only command     | Manual testing sufficient                             |
| Frontend components   | `src/`           | UI interactions        | Consider lightweight component tests if ROI justified |

## Recommendations

### Immediate Actions

1. **Add migration tests** — Create test functions that:
   - Create a DB with old schema (unique on name only)
   - Run `migrate_words_unique_if_needed`
   - Verify new schema has `UNIQUE(name, type_id)`
   - Same for `migrate_event_columns_if_needed`

2. **Add import integration tests** — Create sample LOD content strings and test:
   - Valid import with all file types
   - Import with missing files
   - Import with malformed data
   - Import with empty files

3. **Add export tests** — Test `generate_html` with:
   - Empty database
   - Single word with definition
   - Multiple words with affixes and cross-references

4. **Add FTS rebuild test** — Test the full cycle:
   - Insert words with definitions
   - Call `rebuild_fts`
   - Verify `fts_is_ready` returns true
   - Verify search finds inserted content

### Future Considerations

1. **Frontend component tests** — For a desktop app, the ROI is moderate. If added, recommend:
   - Vitest for unit testing Svelte components
   - Focus on form validation and search behavior
   - Skip E2E unless user-reported bugs justify it

2. **Property-based tests** — For FTS5 query sanitization (`build_fts_query`), property tests could verify:
   - All queries are valid FTS5 syntax
   - Special characters are properly escaped

3. **Performance regression tests** — `test_get_word_performance_optimal` exists. Consider adding:
   - `list_words` performance with 10k+ words
   - `search_english` performance with 100k+ definitions

## Test Strategy

**Recommended approach for this app size:**

Focus on **integration tests** that test full workflows rather than isolated unit tests. The existing 11 unit tests are good for core operations. Priority should be:

1. **Migrations** (critical — data integrity)
2. **Import/Export** (critical — core features untested)
3. **FTS operations** (high — search is a primary user feature)
4. **Edge cases** (medium — empty data, malformed input)

Target: **20-25 total tests** (adding ~10-14 to current 11). This provides good confidence without over-testing for a dictionary manager.

**Do NOT add:**

- E2E tests (overkill for this app)
- Mock-heavy unit tests (the in-memory DB is fast enough for integration tests)
- Frontend test framework (unless specific bugs justify the maintenance cost)
