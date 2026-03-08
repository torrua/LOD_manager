# Contributing to LOD Manager

## Setup

```bash
# 1. Clone and install
git clone https://github.com/YOUR_USERNAME/lod-manager
cd lod-manager
npm install

# 2. Run in dev mode
npm run tauri
```

## Code quality tools

### All checks at once (same as CI)

```bash
npm run ci:check
```

### Frontend

| Command                | What it does                              |
| ---------------------- | ----------------------------------------- |
| `npm run lint`         | ESLint — catch errors and bad patterns    |
| `npm run lint:fix`     | ESLint — auto-fix what it can             |
| `npm run format`       | Prettier — format all TS/Svelte files     |
| `npm run format:check` | Prettier — check without writing          |
| `npm run check`        | svelte-check — full TypeScript type check |
| `npm run check:watch`  | TypeScript — watch mode                   |

### Rust

| Command                                           | What it does                    |
| ------------------------------------------------- | ------------------------------- |
| `npm run rust:fmt`                                | rustfmt — format all Rust files |
| `npm run rust:fmt:check`                          | rustfmt — check without writing |
| `npm run rust:lint`                               | Clippy — lint, deny warnings    |
| `cargo test --manifest-path src-tauri/Cargo.toml` | Run unit tests                  |

## CI

Every push to `main` or `develop`, and every pull request, runs:

1. Prettier format check
2. ESLint
3. TypeScript / svelte-check
4. rustfmt check
5. Clippy (deny warnings)
6. cargo test

**The CI must pass before merging.**

## Releasing

See [README.md](README.md#releasing-a-new-version).

## Commit style

Use [Conventional Commits](https://www.conventionalcommits.org/):

```
feat: add event filter to word list
fix: preserve layout in read-only mode
chore: bump version to 1.1.0
docs: update README build instructions
```
