# Changelog

All notable changes to LOD Manager are documented here.

## [1.6.0] — 2026-03-12

### 🚀 Performance Improvements
- **Search Speed**: Significantly improved English-to-Loglan search performance with optimized database queries
- **Database Indexes**: Added missing indexes for better query performance across all tables
- **FTS Rebuild**: Enhanced full-text search index rebuilding process with better error handling

### 🐛 Bug Fixes
- **Word Navigation**: Fixed jump to word functionality for smoother navigation between search results
- **Unicode Handling**: Fixed unicode escape sequences in keyword search patterns
- **Code Quality**: Resolved Clippy linting warnings and enforced stricter code standards
- **Formatting**: Fixed prettier formatting issues across frontend components

### 🔧 Technical Improvements
- **Database Schema**: Improved database consistency with proper indexing strategy
- **Error Handling**: Enhanced error handling in database operations
- **Build Process**: Improved CI/CD pipeline with comprehensive formatting checks
- **Documentation**: Updated inline documentation with proper markdown formatting

### 📝 Code Quality
- Enforced stricter Rust linting rules with Clippy
- Improved code comments for better maintainability
- Standardized error handling patterns

## [1.5.0] — 2026-03-11

- Add Settings button to desktop header
- Fix mobile "+" button to directly create new elements for current tab
- Remove sidebar completely for Types and Authors sections in desktop mode
- Improve button height consistency in header
- Fix linting issues and code quality
- Update version to 1.5.0

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
