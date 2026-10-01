# Phase 6: Recommendations & Action Plan - Executive Summary

## Overview

Phase 6 synthesizes all findings from Phase 5 (Architectural Assessment) into a prioritized, actionable plan. The LOD Manager codebase is fundamentally sound — 0 critical issues, 19/24 compliance checks passed, and patterns are well-chosen for the app's size. However, there are 2 high-severity stability risks and significant test coverage gaps that should be addressed before the app handles larger datasets or gains more users.

The recommendations are organized into 4 priority levels (P0-P3) with 15 total items. Seven of these are **quick wins** — changes that take under 2 hours each with high or medium impact and minimal risk. These can all be completed in a single work session (~5 hours total).

## Final Action Plan

### Immediate (Week 1) — Quick Wins + P0 Fixes

**Estimated effort: ~8 hours**

1. **Replace `.unwrap()` in import** (15 min) — eliminates app crash risk
2. **Enable WAL mode by default** (10 min) — better concurrent access
3. **Add file size validation for Android** (30 min) — prevents OOM
4. **Document `get_word` SQL** (20 min) — protects against performance regression
5. **Add module-level documentation** (1 hour) — improves onboarding
6. **Add migration tests** (2 hours) — confidence for data integrity
7. **Add export tests** (1 hour) — confidence for HTML output
8. **Handle partial state on open failure** (2 hours) — prevents inconsistent DB state

### Short-term (Weeks 2-3) — P1 Reliability

**Estimated effort: ~6 hours**

9. **Add import integration tests** (3 hours) — core feature coverage
10. **Add FTS rebuild and search tests** (2 hours) — search reliability
11. **Create typed error enum** (2 hours) — structured error handling
12. **Report skipped import rows** (1 hour) — user visibility

### Medium-term (Month 2) — P2 Maintainability

**Estimated effort: ~3 hours**

13. **Add migration rollback** (2 hours) — data safety during schema changes
14. **Batch FTS rebuild** (1 hour) — memory efficiency for large DBs

### Future (When app grows) — P3 Strategic

**Estimated effort: 2+ days**

15. **Split lib.rs into submodules** (when >35 commands)
16. **Add typed wrappers for raw strings** (if validation needs grow)
17. **Consider frontend test framework** (if UI bugs justify it)

## Risk Mitigation Summary

| Risk                          | Severity | Mitigation                    | Status          |
| ----------------------------- | -------- | ----------------------------- | --------------- |
| Transaction panic in import   | High     | Replace `.unwrap()` with `?`  | **Immediate**   |
| Partial state on open failure | High     | Migration flag or transaction | **Immediate**   |
| FTS rebuild memory            | Medium   | Batch processing              | **Medium-term** |
| No migration rollback         | Medium   | Transaction + backup table    | **Medium-term** |
| Android file size             | Medium   | Size validation (100MB limit) | **Immediate**   |
| Error categorization          | Medium   | Typed error enum              | **Short-term**  |

## Expected Outcomes

After implementing Immediate + Short-term recommendations:

- **Stability:** 0 high-severity risks (down from 2)
- **Test coverage:** ~25 tests (up from 11), covering migrations, import, export, FTS
- **Error handling:** Structured error types instead of generic strings
- **Documentation:** All modules documented, complex SQL explained
- **Performance:** WAL mode enabled, better concurrent access
- **Maintainability score:** ~8/10 (up from 6.8/10)

## Next Steps

Phase 7 (Implementation) should focus on:

1. **Execute quick wins first** — maximum impact for minimum effort
2. **Add tests before refactoring** — safety net for any future changes
3. **Implement typed errors** — foundation for better error handling across the app
4. **Monitor after each change** — verify no regressions in existing functionality

The codebase is in good health. These recommendations are incremental improvements, not a rewrite. Each change is isolated and can be implemented independently.

---

_Assessment Date: 2026-04-04_
_Phase: 06 - Recommendations & Action Plan_
_Requirements: REC-01, REC-02, REC-03_
