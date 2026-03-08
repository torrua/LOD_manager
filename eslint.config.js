import js from '@eslint/js';
import ts from 'typescript-eslint';
import svelte from 'eslint-plugin-svelte';
import svelteParser from 'svelte-eslint-parser';
import globals from 'globals';

export default [
  // ── Base JS recommended ────────────────────────────────────────────────────
  js.configs.recommended,

  // ── TypeScript ─────────────────────────────────────────────────────────────
  ...ts.configs.recommended,

  // ── Svelte files ──────────────────────────────────────────────────────────
  ...svelte.configs['flat/recommended'],
  {
    files: ['**/*.svelte'],
    languageOptions: {
      parser: svelteParser,
      parserOptions: {
        parser: ts.parser,
      },
      globals: { ...globals.browser },
    },
  },

  // ── TypeScript + plain TS files ───────────────────────────────────────────
  {
    files: ['**/*.ts'],
    languageOptions: { globals: { ...globals.browser } },
  },

  // ── Project-wide rule overrides ───────────────────────────────────────────
  {
    rules: {
      // TypeScript
      '@typescript-eslint/no-explicit-any': 'warn',
      '@typescript-eslint/no-unused-vars': ['warn', { argsIgnorePattern: '^_' }],
      '@typescript-eslint/no-non-null-assertion': 'warn',
      '@typescript-eslint/consistent-type-imports': ['error', { prefer: 'type-imports' }],

      // General code quality
      'no-console': ['warn', { allow: ['warn', 'error'] }],
      eqeqeq: ['error', 'always'],
      'prefer-const': 'error',
      'no-var': 'error',
      'object-shorthand': 'error',
      'prefer-template': 'error',

      // Svelte-specific
      'svelte/no-unused-svelte-ignore': 'warn',
      'svelte/valid-compile': 'error',
    },
  },

  // ── Ignore generated / build output ───────────────────────────────────────
  {
    ignores: ['dist/**', 'src-tauri/gen/**', 'src-tauri/target/**', 'node_modules/**'],
  },
];
