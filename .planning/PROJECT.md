# Project: LOD Manager

A desktop + mobile editor for the Loglan Online Dictionary (LOD).

## Overview

| Attribute      | Value                                           |
| -------------- | ----------------------------------------------- |
| **Type**       | Cross-platform desktop/mobile application       |
| **Tech Stack** | Tauri v2, Svelte 5, Vite 8, Rust, SQLite (FTS5) |
| **Version**    | 1.6.10                                          |
| **Repository** | Tauri + Svelte + Rust                           |
| **Mode**       | Analysis & Stabilization                        |

## Purpose

The Loglan Online Dictionary (LOD) is a comprehensive dictionary for Loglan, a constructed logical language. This application provides:

- A dictionary browser (Loglan → English)
- Full-text search (English → Loglan)
- Data import from LOD text files and legacy SQLite DB conversion
- HTML export functionality
- Cross-platform support (Windows, Android)

## Key Features

- Import LOD text files (@ delimited format) & convert legacy DB schemas
- Virtual scrolling for large word lists
- Full-text search with FTS5 and LIKE fallback
- CRUD operations for words, definitions, events, types, authors
- Dark/light theme
- Responsive layout (mobile/desktop)

## Tech Stack

| Component  | Version       |
| ---------- | ------------- |
| Tauri      | v2            |
| Svelte     | 5.x           |
| Vite       | 8.x           |
| Rust       | stable        |
| SQLite     | 3.x with FTS5 |
| TypeScript | 6.x           |
| ESLint     | 10.x          |

## Architecture

- **Frontend**: Svelte 5 with runes, TypeScript, Vite 8
- **Backend**: Rust with Tauri 2.0 commands (`commands/*`, `db.rs`, `converter/*`)
- **Database**: SQLite with FTS5 for full-text search
- **State**: Rust-side Mutex for database connection
- **Build**: Vite for frontend, Cargo for Rust

## Files of Interest

- `PROJECT.md` - Root project overview and IPC command reference (30 commands)
- `ARCHITECTURE.md` - C4 diagrams, DB schema, data flows, ADRs
- `AGENTS.md` / `CLAUDE.md` - Developer and AI agent guides
- `AUDIT_REPORT.md` - Comprehensive Senior Architect audit report
- `TODO.md` - Prioritized P0–P3 stabilization & optimization roadmap
- `src-tauri/src/` - Rust backend (`lib.rs`, `db.rs`, `commands/*`, `converter/*`)
- `src/` - Svelte 5 frontend components

## Analysis Goal

Understand current state, capabilities, and execute P0–P3 stabilization and optimization steps.
