# Changelog

All notable changes to LOD Manager are documented here.

## [1.1.0] — 2026-03-09

- Add GitHub download functionality for LOD files
- Improve TypeScript typing for GitHub API responses
- Auto-create database in app directory on Android first launch
- Fix GitHub download list duplication on repeated clicks
- Add UNIQUE constraint on (word_id, position) for definitions to prevent duplicates on re-import

## [1.0.7] — 2026-03-09

- Bump version to 1.0.7

## [1.0.6] — 2026-03-09

- Bump version to 1.0.6

## [1.0.0] — Initial release

- Import LOD text files (Words, Definitions, Events, Types, Authors)
- Loglan → English dictionary browser with virtual scroll
- English → Loglan full-text search (FTS5) with LIKE fallback
- Edit words, definitions, events, types, authors
- Export to HTML (Loglan → English)
- Collapsible "Used In" and "Words Added / Removed" sections
- Dark / light theme
- Narrow (mobile) layout
