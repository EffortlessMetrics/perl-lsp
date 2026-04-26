# Intent/diff closeout evidence gate

`cargo xtask intent-diff-gate` validates that PR claims (title/body) match the changed paths and closeout evidence.

## Commands

- `cargo xtask intent-diff-gate --pr <N> --receipt target/receipts/intent-diff-gate.json`
- `cargo xtask intent-diff-gate --fixture <json>`

## What it enforces

1. A fix-style claim with docs-only diff is flagged (policy decides fail/warn).
2. `Closes/Fixes/Resolves #NNNN` claims for issues with known target paths require evidence:
   - target path touched, or
   - test updated, or
   - behavior receipt path touched, or
   - explicit override marker in title/body.
3. Scaffold/partial/WIP PRs must not use closing keywords.
4. Docs-scoped titles with production code changes are flagged.
5. VS Code activation claims require `vscode-extension/package.json` or relevant tests, unless overridden.

## Policy and receipt files

- Policy: `.ci/policies/intent-diff-rules.toml`
- Receipt schema: `.ci/receipts/schemas/intent-diff-gate.schema.json`
- Receipt output fields:
  - `claimed_component`
  - `claimed_closeout_issues`
  - `expected_paths`
  - `actual_paths`
  - `evidence`
  - `verdict`
  - `violations`

## Motivation

This gate is intended to prevent intent/diff mismatches like the #6780-style case where a PR claimed a VS Code activation fix and closeout but only changed documentation.
