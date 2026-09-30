# CLAUDE.md — AI Assistant Context for LOD Manager

Read **[`AGENTS.md`](file:///c:/Users/User/OneDrive/JS/LOD%20Manager/AGENTS.md)** first for full coding rules, lint requirements, and conventions.

---

## Quick Reference

- **Project**: LOD Manager (`v1.7.0`) — Cross-platform desktop (Windows/Linux/macOS) & mobile (Android) dictionary editor for the Loglan Online Dictionary (LOD).
- **Stack**: Tauri v2 · Svelte 5 (Runes) · TypeScript 6 · Vite 8 · Rust (Edition 2024) · `rusqlite` 0.31 (SQLite 3 + FTS5).
- **Architecture**:
  - Frontend global reactive state: [`src/lib/store.svelte.ts`](file:///c:/Users/User/OneDrive/JS/LOD%20Manager/src/lib/store.svelte.ts)
  - Frontend components: [`src/lib/components/`](file:///c:/Users/User/OneDrive/JS/LOD%20Manager/src/lib/components)
  - Shared TS types: [`src/types.ts`](file:///c:/Users/User/OneDrive/JS/LOD%20Manager/src/types.ts)
  - Tauri command submodules: [`src-tauri/src/commands/`](file:///c:/Users/User/OneDrive/JS/LOD%20Manager/src-tauri/src/commands)
  - SQLite data access & FTS5: [`src-tauri/src/db.rs`](file:///c:/Users/User/OneDrive/JS/LOD%20Manager/src-tauri/src/db.rs)
  - Serde IPC models: [`src-tauri/src/models.rs`](file:///c:/Users/User/OneDrive/JS/LOD%20Manager/src-tauri/src/models.rs)
  - Import / Converter / Export: [`src-tauri/src/import.rs`](file:///c:/Users/User/OneDrive/JS/LOD%20Manager/src-tauri/src/import.rs), [`src-tauri/src/converter/converter.rs`](file:///c:/Users/User/OneDrive/JS/LOD%20Manager/src-tauri/src/converter/converter.rs), [`src-tauri/src/export.rs`](file:///c:/Users/User/OneDrive/JS/LOD%20Manager/src-tauri/src/export.rs)

---

## Core Documentation Map

- **[`PROJECT.md`](file:///c:/Users/User/OneDrive/JS/LOD%20Manager/PROJECT.md)** — Features, repository tree, and all 30 Tauri IPC commands.
- **[`ARCHITECTURE.md`](file:///c:/Users/User/OneDrive/JS/LOD%20Manager/ARCHITECTURE.md)** — C4 diagrams, database schema, 5-step `get_word` pipeline, 4-query `export_html` pipeline, and ADRs.
- **[`AGENTS.md`](file:///c:/Users/User/OneDrive/JS/LOD%20Manager/AGENTS.md)** — Build/lint/test commands, Prettier/ESLint/Clippy/rustfmt rules.
- **[`AUDIT_REPORT.md`](file:///c:/Users/User/OneDrive/JS/LOD%20Manager/AUDIT_REPORT.md)** — Comprehensive Senior Architect audit report, verified bugs, and optimization steps.
- **[`TODO.md`](file:///c:/Users/User/OneDrive/JS/LOD%20Manager/TODO.md)** — Actionable P0–P3 bug-fix & optimization checklist.

---

## Critical Guardrails

1. **Line Endings in Rust**: `src-tauri/rustfmt.toml` enforces `newline_style = "Unix"`. Always save `.rs` files with `LF` line endings.
2. **Svelte 5 Runes**: Use `$state`, `$derived`, `$effect`, and `$props()`. Never mutate `$state` arrays in-place via `.sort()` when reading derived values.
3. **IPC Serialization Parity**: Keep `src-tauri/src/models.rs`, `src-tauri/src/db.rs` (`json_object`), and `src/types.ts` strictly aligned.
4. **Verification Before Commit**:
   - Frontend: `npm run ci:check`
   - Backend: `npm run rust:fmt:check && npm run rust:lint && cargo test --manifest-path src-tauri/Cargo.toml`
