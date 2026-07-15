# Oxlint warning baseline

This is the ratchet for the expanded Oxlint scope. It covers `src/`,
`src/test/`, `scripts/`, `jest.config.js`, and `rolldown.config.mjs` with
type-aware checking enabled.

The machine-readable inventory is [`oxlint-warning-baseline.json`](./oxlint-warning-baseline.json).

- command: `npm run lint`
- Oxlint: 1.73.0
- oxlint-tsgolint: 0.24.0
- result: 0 errors, 70 warnings
- breakdown by rule: 69 `typescript/no-explicit-any`, 1 `no-unused-vars`, 0
  `typescript/consistent-type-imports`
- breakdown by surface: 70 tests, 0 scripts, 0 production, 0 build-config

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

The walkthrough and “What’s New” contract tests now use typed manifest and
VS Code context adapters, removing 19 warnings while preserving their static
UX and lifecycle assertions.

The health-widget, test-at-cursor, and streaming-completion tests now use
typed VS Code and language-client seams, removing five warnings while
preserving their 49 behavior tests.

The formatting-error tests now use one typed OutputChannel fixture instead of
repeated untyped casts, removing 15 warnings while preserving their 12
notification and cooldown tests.

The onboarding tests now use typed VS Code context, workspace, process-check,
and package-manifest adapters, removing 29 warnings while preserving their 33
health, guidance, and startup-diagnostic tests.

The POD preview contract tests now use typed command and command-palette
manifest entries, removing eight warnings while preserving their 22 static
registration and conversion tests.

The configuration contract tests now use typed JSON schemas for language
configuration, package contributions, grammars, and snippets. A required-value
helper keeps missing manifest entries as explicit test failures, removing 77
warnings while preserving their 106 static configuration tests.

The debug adapter tests now use typed VS Code contexts, debug configurations,
sessions, descriptors, launch JSON, and test adapters. This removes 58 warnings
while preserving their 48 provider, descriptor, launch, and wizard tests.

The downloader tests now use typed VS Code fixtures and an explicit test-only
surface for private installation, transport, checksum, and update-check seams.
This removes 71 warnings while preserving their 113 security, lifecycle, and
platform tests.

`scripts/check-oxlint-warning-budget.js` consumes Oxlint JSON diagnostics and
enforces a non-increasing total, per-rule, per-surface, rule-by-surface, and
per-file inventory. New errors fail immediately; warning cleanup may lower the
budget, but increasing any baseline bucket requires an intentional baseline
review.
