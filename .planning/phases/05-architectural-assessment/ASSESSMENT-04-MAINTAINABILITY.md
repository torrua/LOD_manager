# Maintainability Scorecard

**Assessment Date:** 2026-04-04

---

## Dimension Scores

| Dimension         | Score (1-10) | Justification                                                                                                                                                            |
| ----------------- | ------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| **Organization**  | **8**        | Clear module separation (lib.rs, db.rs, models.rs, import.rs, export.rs). Each has single responsibility. lib.rs is large (~903 lines) but justified as command gateway. |
| **Coupling**      | **8**        | Loose coupling between frontend and backend. db.rs is the only backend module called by commands. AppState is accessed through helpers (`with_db`, `with_db_mut`).       |
| **Documentation** | **6**        | Module-level docs present. Inline comments for complex logic. AGENTS.md exists. Missing: function-level docs for complex operations.                                     |
| **Extensibility** | **7**        | Adding new word fields requires changes to models.rs, db.rs, and UI. Import/export have clear entry points. Reasonable for app size.                                     |
| **Overall**       | **7.25**     | Simple architecture, proportionate to app size                                                                                                                           |

---

## Improvement Areas

### High Priority

1. **Error type system**
   - Location: `lib.rs:28-30`
   - Issue: Generic `String` errors make debugging harder
   - Recommendation: Add error enum for common cases
   - Effort: Medium

2. **Test coverage**
   - Location: `lib.rs` under `#[cfg(test)]`
   - Issue: Only 2 tests, no coverage for import/export/migrations
   - Recommendation: Add integration tests for key flows
   - Effort: Medium

### Medium Priority

3. **Module documentation**
   - Location: `db.rs`, `models.rs`, `import.rs`, `export.rs`
   - Issue: Complex functions lack `///` docs
   - Recommendation: Add docs to public APIs
   - Effort: Low

4. **Command organization**
   - Location: `lib.rs` (~903 lines)
   - Issue: Large file could be split by domain
   - Recommendation: Consider `commands/*.rs` modules if file grows
   - Effort: Low (not urgent)

### Low Priority

5. **Magic number extraction**
   - Location: Various (search limit: 300 in store.svelte.ts:534)
   - Issue: Constants scattered
   - Recommendation: Centralize constants
   - Effort: Low

---

## Strengths

### Architecture

- **Clean separation of concerns:** Commands (lib.rs) delegate to data layer (db.rs), keeping lib.rs thin
- **Consistent patterns:** All commands follow same structure: validate → delegate → transform → return
- **Proper error handling:** `Result<T, String>` pattern used consistently

### Code Quality

- **Type safety:** Strong TypeScript types in frontend, Rust structs in backend
- **No obvious code smells:** No god objects, tight coupling, or primitive obsession
- **Proper use of Rust features:** Uses `?` operator, proper error propagation, thread-safe state

### Developer Experience

- **AGENTS.md exists:** Clear project guidelines
- **Codebase docs available:** ARCHITECTURE.md, CONVENTIONS.md, CONCERNS.md
- **Consistent conventions:** Follows defined style guides (lint, format)

---

## Module Breakdown

| Module    | Lines | Responsibility      | Assessment                        |
| --------- | ----- | ------------------- | --------------------------------- |
| lib.rs    | ~903  | Tauri commands      | Well-organized, commands are thin |
| db.rs     | ~900  | Database operations | Complex but readable              |
| models.rs | ~200  | Data structures     | Clean, well-typed                 |
| import.rs | ~300  | LOD file import     | Focus on parsing, reasonable      |
| export.rs | ~200  | HTML export         | Simple, focused                   |

---

## Recommendations Summary

### Should Do Now

1. **Add error categorization** — Current generic errors make debugging harder
2. **Add integration tests** — Critical paths: import, export, FTS rebuild

### Should Do Soon

3. **Document complex functions** — Add `///` docs to db.rs public functions
4. **Extract constants** — Centralize magic numbers like search limits

### Can Defer

5. **Split lib.rs** — Not urgent, would add complexity without clear benefit
6. **Refactor builder functions** — FTS query builders work, can simplify later

---

## Final Assessment

**Overall Score: 7.25/10**

This is a well-maintained codebase for its size. The architecture is appropriate for a dictionary manager - not over-engineered, not under-engineered. Main areas for improvement are around error handling (make it more debuggable) and test coverage (add critical path tests). The codebase is extensible and follows consistent patterns.
