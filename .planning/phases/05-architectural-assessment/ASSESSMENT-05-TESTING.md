# Test Coverage Assessment

**Assessment Date:** 2026-04-04

---

## Current Coverage

### Rust Tests

| Test                                | Location       | Coverage                                          | Quality                                            |
| ----------------------------------- | -------------- | ------------------------------------------------- | -------------------------------------------------- |
| `test_in_memory_db_init`            | lib.rs:418-429 | Schema initialization                             | Good - verifies default events are created         |
| `test_get_word_performance_optimal` | lib.rs:431-568 | Query performance, data retrieval, word structure | Good - comprehensive with type, affixes, spellings |
| `test_save_and_get_word`            | lib.rs:516-567 | CRUD operations                                   | Good - tests round-trip word save                  |
| `test_definition_crud`              | lib.rs:569-620 | Definition create/read/update/delete              | Good                                               |
| `test_event_crud`                   | lib.rs:622-649 | Event CRUD                                        | Good                                               |
| `test_type_crud`                    | lib.rs:651-697 | Type CRUD                                         | Good                                               |
| `test_affix_and_spelling_crud`      | lib.rs:699-750 | Affix and spelling operations                     | Good                                               |
| `test_search_english_like`          | lib.rs:752-791 | English-to-Loglan search (LIKE fallback)          | Good                                               |
| `test_words_unique_constraint`      | lib.rs:827-862 | Unique constraint on words                        | Good                                               |
| `test_word_affix_deletion`          | lib.rs:861-903 | Affix deletion via SQL                            | Good                                               |

**Summary:** 10 unit tests in lib.rs under `#[cfg(test)]` module.

### Frontend Tests

**Status:** None configured

- No test framework in package.json
- No test scripts defined
- No `*.test.ts` or `*.spec.ts` files in `src/`

---

## Coverage Gaps

### Critical Priority

| Gap                     | Location                                                                                            | Impact                                          | Recommendation                                                       |
| ----------------------- | --------------------------------------------------------------------------------------------------- | ----------------------------------------------- | -------------------------------------------------------------------- |
| **Database migrations** | `db.rs:112-191` (migrate_words_unique_if_needed), `db.rs:200-240` (migrate_event_columns_if_needed) | Migration failures could corrupt database       | Integration test: create old-schema DB, run migration, verify result |
| **Import functions**    | `lib.rs:182-196` (import_lod_contents), `import.rs` (parse_lod_file)                                | Import failures silently drop data              | Integration test: import sample LOD file, verify all words imported  |
| **Export functions**    | `lib.rs:275-285` (export_html_to_file), `export.rs`                                                 | Export may produce malformed HTML               | Integration test: export populated DB, validate HTML output          |
| **FTS rebuild**         | `lib.rs:247-267` (rebuild_fts), `db.rs:753-799`                                                     | Memory issues with large DBs (see STABILITY.md) | Performance test with 100k+ definitions                              |

### High Priority

| Gap                                       | Location                                                                    | Impact                                               | Recommendation                                          |
| ----------------------------------------- | --------------------------------------------------------------------------- | ---------------------------------------------------- | ------------------------------------------------------- |
| **CRUD operations (Tauri command layer)** | lib.rs commands: save_word, save_definition, delete_word, delete_definition | Commands may behave differently than db.rs functions | Integration test: call via invoke, verify state changes |
| **Search edge cases**                     | `store.svelte.ts:515-554` (searchEnglishNow)                                | Empty query, special characters, large results       | Unit tests                                              |
| **State management**                      | `store.svelte.ts:35-104` (app state)                                        | Concurrent access issues                             | Manual testing currently                                |
| **Event filter behavior**                 | `store.svelte.ts:240-281` (applyFilter)                                     | Filtering may behave incorrectly with null events    | Unit tests                                              |

### Medium Priority

| Gap                           | Impact                             | Recommendation                    |
| ----------------------------- | ---------------------------------- | --------------------------------- |
| **UI component tests**        | Form validation, user interactions | Manual testing sufficient for now |
| **Full workflow integration** | Import → search → export cycle     | Could be tested, lower priority   |

---

## Recommendations

### Immediate Actions

1. **Add migration integration test**
   - Test: Create database with old schema version, run migrations, verify new schema
   - Location: `lib.rs` tests module
   - Effort: Medium

2. **Add import integration test**
   - Test: Import known LOD file, verify word count and content
   - Location: `lib.rs` tests module
   - Effort: Medium

3. **Add export integration test**
   - Test: Export HTML, validate against known structure
   - Location: `lib.rs` tests module
   - Effort: Low

### Future Considerations

4. **FTS rebuild performance test**
   - Test: Create 100k+ definitions, run rebuild_fts, measure memory/time
   - Notes: Documented in STABILITY.md as high-risk

5. **Frontend test framework (optional)**
   - Could add Vitest for component testing
   - ROI: Lower for desktop app - manual testing acceptable
   - Recommendation: Skip for now

---

## Test Strategy

### Recommended Approach

For a dictionary manager of this size (~28 commands, ~10k LOC), prioritize:

1. **Integration tests for data flows** (import, export, migrations) — highest value
2. **Unit tests for complex logic** (search, filtering, FTS query building)
3. **Skip UI testing** — manual testing sufficient for desktop app

### Testing Pyramid

```
       /\
      /  \     E2E: Full import→export flow (1-2 tests)
     /----\
    /      \   Integration: Migrations, import, export (3-5 tests)
   /        \
  /__________\ Unit: db.rs functions, search logic (existing 10 tests)
```

### Current State

- Unit tests: 10 ✓ Good
- Integration tests: 0 ✗ Gap
- E2E tests: 0 ✗ Gap

**Target:** Add 5-8 integration tests covering critical paths.

---

## Summary

| Category          | Current | Gap                    |
| ----------------- | ------- | ---------------------- |
| Unit tests        | 10      | Good                   |
| Integration tests | 0       | Critical gap           |
| Frontend tests    | 0       | Not recommended        |
| **Total**         | 10      | Need integration tests |

**Recommendation:** Add integration tests for migrations, import, and export. These are the highest-value additions for stability confidence.
