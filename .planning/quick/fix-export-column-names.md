# Quick Task: Fix Export Column Names

**Date**: 2026-04-07
**Status**: Complete
**Commit**: dd291da

## Task

Fix export.rs SQL column names to match the current schema after developer's schema refactor.

## Changes Made

### export.rs
- `definitions` SELECT: `grammar` → `grammar_code`, `tags` → `case_tags`
- `types` JOIN select: `t.name` → `t.type`
- `words` JOIN: `w.type_id` → `w.type`
- `words` column: `w.source` → `NULL` (column removed from schema)
- Event filter: `event_start_id` → `event_start`, `event_end_id` → `event_end`
- Event subquery: `SELECT id FROM events` → `SELECT event_id FROM events`

### lib.rs (test fixes)
- `definitions` INSERT columns: `grammar` → `grammar_code`, `tags` → `case_tags`
- `SaveDefinition` struct fields: `grammar` → `grammar_code`, `tags` → `case_tags`
- `test_export_html_with_data`: `types (name)` → `types (type)`, words INSERT updated for new schema

## Tests
- `test_export_html_empty`: PASS
- `test_export_html_with_data`: PASS
