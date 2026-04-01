# Coding Conventions

**Analysis Date:** 2026-04-01

## TypeScript / Svelte

### ESLint Rules

From `eslint.config.js`:

| Rule                                         | Setting | Enforced                                           |
| -------------------------------------------- | ------- | -------------------------------------------------- |
| `prefer-const`                               | error   | Variables that are not reassigned must use `const` |
| `no-var`                                     | error   | No legacy `var` keyword                            |
| `eqeqeq`                                     | error   | Always use `===` / `!==`                           |
| `object-shorthand`                           | error   | Use `{ x }` not `{ x: x }`                         |
| `prefer-template`                            | error   | Use backtick strings over concatenation            |
| `@typescript-eslint/consistent-type-imports` | error   | Use `import type { Foo }`                          |
| `@typescript-eslint/no-explicit-any`         | warn    | Avoid `any` type                                   |
| `@typescript-eslint/no-unused-vars`          | warn    | Allow `_` prefix for unused args                   |

### Prettier

From `.prettierrc`:

| Option          | Value |
| --------------- | ----- |
| `semi`          | true  |
| `singleQuote`   | true  |
| `tabWidth`      | 2     |
| `useTabs`       | false |
| `printWidth`    | 100   |
| `trailingComma` | es5   |

### TypeScript Strict

From `tsconfig.json`:

| Option                       | Behavior                                     |
| ---------------------------- | -------------------------------------------- |
| `verbatimModuleSyntax`       | Must use `import type` for type-only imports |
| `noUncheckedIndexedAccess`   | Array access returns `T \| undefined`        |
| `exactOptionalPropertyTypes` | Optional properties are truly optional       |
| `strict`                     | All strict checks enabled                    |

### Svelte 5

- Use **runes** for reactivity: `$state`, `$derived`, `$effect`
- Use `$props()` for component props
- Use `import { mount } from 'svelte'` (not `new App()`)

Example:

```svelte
<script lang="ts">
  let { name, onSave }: { name: string; onSave: () => void } = $props();
  let count = $state(0);
  let doubled = $derived(count * 2);
</script>
```

### Import Style

```typescript
import type { Foo } from './types';
import { bar } from './utils';
```

Use `import type` for type-only imports (required by `verbatimModuleSyntax`).

---

## Rust

### rustfmt

From `src-tauri/rustfmt.toml`:

| Option          | Value |
| --------------- | ----- |
| `edition`       | 2021  |
| `max_width`     | 100   |
| `tab_spaces`    | 4     |
| `newline_style` | Unix  |

### Clippy

From `src-tauri/.clippy.toml`:

| Option                           | Threshold |
| -------------------------------- | --------- |
| `cognitive-complexity-threshold` | 20        |
| `too-many-lines-threshold`       | 60        |

### Project-wide Lint Attributes

From `src-tauri/src/lib.rs`:

```rust
#![warn(clippy::all)]
#![warn(clippy::pedantic)]
#![allow(clippy::needless_pass_by_value)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::wildcard_imports)]
#![allow(clippy::too_many_lines)]
```

### Naming Conventions

| Category            | Convention           | Example                     |
| ------------------- | -------------------- | --------------------------- |
| Types               | PascalCase           | `WordDetail`, `SaveWord`    |
| Functions/variables | snake_case           | `open_database`, `get_word` |
| Modules             | snake_case           | `db`, `import`, `export`    |
| Constants           | SCREAMING_SNAKE_CASE | `MAX_RESULTS`               |

### Error Handling

- Use `Result<T, String>` for Tauri commands
- Use `map_err(err)` helper for error conversion
- Use `?` operator for propagation
- Frontend errors: throw exceptions

Pattern in `src-tauri/src/lib.rs`:

```rust
type Res<T> = Result<T, String>;

fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}

#[tauri::command]
fn open_database(state: Db, path: String) -> Res<AppInfo> {
    let conn = Connection::open(&path).map_err(err)?;
    // ...
}
```

---

## Commit Style

Use [Conventional Commits](https://www.conventionalcommits.org/):

| Type     | Purpose                    |
| -------- | -------------------------- |
| `feat:`  | New feature                |
| `fix:`   | Bug fix                    |
| `chore:` | Maintenance, version bumps |
| `docs:`  | Documentation              |

Examples:

```
feat: add event filter to word list
fix: preserve layout in read-only mode
chore: bump version to 1.1.0
docs: update README build instructions
```

---

_Convention analysis: 2026-04-01_
