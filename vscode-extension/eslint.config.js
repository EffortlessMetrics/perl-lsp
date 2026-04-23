// ESLint flat config (ESLint v9+)
// Lints all TypeScript source files; excludes compiled output, tests, and node_modules.

const tsPlugin = require('@typescript-eslint/eslint-plugin');
const tsParser = require('@typescript-eslint/parser');

/** @type {import('eslint').Linter.Config[]} */
module.exports = [
  // Global ignores
  {
    ignores: ['out/**', 'node_modules/**', 'src/test/**', '*.js'],
  },

  // TypeScript source files
  {
    files: ['src/**/*.ts'],
    languageOptions: {
      parser: tsParser,
      parserOptions: {
        project: './tsconfig.json',
        ecmaVersion: 2022,
        sourceType: 'module',
      },
    },
    plugins: {
      '@typescript-eslint': tsPlugin,
    },
    rules: {
      // Disallow the `any` escape hatch in catch clauses — use `unknown` instead
      // and narrow the type before accessing properties.
      '@typescript-eslint/no-explicit-any': 'warn',

      // Enforce consistent use of type imports where possible.
      '@typescript-eslint/consistent-type-imports': ['warn', { prefer: 'type-imports' }],

      // Prevent floating (unhandled) promises — all async calls must be awaited
      // or explicitly discarded with `void`.
      '@typescript-eslint/no-floating-promises': 'error',

      // Disallow unused variables (mirrors TypeScript's noUnusedLocals).
      '@typescript-eslint/no-unused-vars': ['warn', { argsIgnorePattern: '^_' }],

      // Standard JS rules
      'no-console': 'warn',
      'eqeqeq': ['error', 'always'],
    },
  },
];
