# Oxlint warning baseline

This is the ratchet for the expanded Oxlint scope. It covers `src/`,
`src/test/`, `scripts/`, `jest.config.js`, and `rolldown.config.mjs` with
type-aware checking enabled.

The machine-readable inventory is [`oxlint-warning-baseline.json`](./oxlint-warning-baseline.json).

- command: `npm run lint`
- Oxlint: 1.73.0
- oxlint-tsgolint: 0.24.0
- result: 0 errors, 352 warnings
- breakdown by rule: 338 `typescript/no-explicit-any`, 11 `no-unused-vars`, 3
  `typescript/consistent-type-imports`
- breakdown by surface: 352 tests, 0 scripts, 0 production, 0 build-config

The Node/npm/TypeScript authority slice added `scripts/toolchain-doctor.js`;
its two existing `no-console` warnings remain recorded explicitly rather than
silently absorbed.

The shared VS Code test mock now uses typed unknown-based seams for command,
progress, workspace, and diagnostic values. That cleanup removed its 17
warnings without changing the warning baseline outside the affected file.

The command contract suite now uses typed manifest adapters for command,
palette, context-menu, and keybinding entries. That cleanup removed its 54
warnings while preserving the existing 70 contract tests.

Script output now flows through the injectable `scripts/reporter.js` seam.
`bundle-lsp.js`, `lint-canary.js`, and `toolchain-doctor.js` therefore carry no
`no-console` debt, and the reporter has a direct stream-routing contract test.

The arrow-completion tests now use explicit VS Code document, editor, and
change-event adapters instead of untyped casts, removing five warnings while
preserving their four behavior tests.

`scripts/check-oxlint-warning-budget.js` consumes Oxlint JSON diagnostics and
enforces a non-increasing total, per-rule, per-surface, rule-by-surface, and
per-file inventory. New errors fail immediately; warning cleanup may lower the
budget, but increasing any baseline bucket requires an intentional baseline
review.
