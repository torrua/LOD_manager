# Project: LOD Manager

A desktop + mobile editor for the Loglan Online Dictionary (LOD).

## Overview

| Attribute      | Value                                     |
| -------------- | ----------------------------------------- |
| **Type**       | Cross-platform desktop/mobile application |
| **Tech Stack** | Tauri v2, Svelte 5, Rust, SQLite (FTS5)   |
| **Version**    | 1.6.1                                     |
| **Repository** | Tauri + Svelte + Rust                     |
| **Mode**       | Analysis                                  |

## Purpose

The Loglan Online Dictionary (LOD) is a comprehensive dictionary for Loglan, a constructed logical language. This application provides:

- A dictionary browser (Loglan → English)
- Full-text search (English → Loglan)
- Data import from LOD text files
- HTML export functionality
- Cross-platform support (Windows, Android)

## Key Features

- Import LOD text files (@ delimited format)
- Virtual scrolling for large word lists
- Full-text search with FTS5 and LIKE fallback
- CRUD operations for words, definitions, events, types, authors
- Dark/light theme
- Responsive layout (mobile/desktop)

## Architecture

- **Frontend**: Svelte 5 with runes, TypeScript, Vite
- **Backend**: Rust with Tauri 2.0 commands
- **Database**: SQLite with FTS5 for full-text search
- **State**: Rust-side Mutex for database connection

## Files of Interest

- `src-tauri/src/lib.rs` - Tauri commands (27 commands)
- `src-tauri/src/db.rs` - Database operations
- `src/` - Svelte frontend components
- `tables/` - SQLite database files
- `AGENTS.md` - Developer guide for agents

## Analysis Goal

Understand current state, capabilities, and identify potential improvements or concerns.
