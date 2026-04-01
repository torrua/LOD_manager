# Testing Patterns

**Analysis Date:** 2026-04-01

## Test Framework

### Rust Tests

- **Location:** Inline in `src-tauri/src/lib.rs` under `#[cfg(test)]`
- **Runner:** `cargo test`
- **Config:** `src-tauri/Cargo.toml`

Run all tests:

```bash
cargo test --manifest-path src-tauri/Cargo.toml
```

Run single test:

```bash
cargo test --manifest-path src-tauri/Cargo.toml -- test_name
```

### Frontend Tests

- **Framework:** None configured
- **Validation:** Use `npm run check` for TypeScript type checking

---

## Test Structure

### Rust Unit Tests

Tests are located in `src-tauri/src/lib.rs` under the `mod tests` block (lines 346-447).

Example test structure:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn test_in_memory_db_init() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        db::init_schema(&conn).unwrap();
        db::init_fts(&conn).unwrap();
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM events WHERE name='Start'", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_get_word_performance_optimal() {
        // Test setup with in-memory database
        // Performance assertion
        assert!(duration.as_millis() < 1000, "get_word should be very fast");
    }
}
```

### Test Patterns

**Setup:** In-memory SQLite database with schema initialization
**Assertions:** `assert_eq!`, `assert!` for conditions
**Error testing:** `assert!(result.is_err(), "message")` for expected failures

---

## CI Requirements

All checks must pass before merging:

```bash
# TypeScript checks
npm run format:check      # Prettier
npm run lint             # ESLint
npm run check            # svelte-check (TypeScript)

# Rust checks
npm run rust:fmt         # rustfmt
npm run rust:lint        # Clippy (denies warnings)

# Tests
cargo test --manifest-path src-tauri/Cargo.toml
```

Full CI pipeline:

```bash
npm run ci:check
cargo test --manifest-path src-tauri/Cargo.toml
```

---

## Coverage

No coverage requirements currently enforced. Run tests locally to verify correctness.

---

_Testing analysis: 2026-04-01_
