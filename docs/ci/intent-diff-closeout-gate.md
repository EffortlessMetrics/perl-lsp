# intent-diff-closeout-gate

`cargo xtask intent-diff-gate` validates whether PR title/body intent matches diff evidence, and blocks premature issue closeouts.

## Motivation

This gate prevents regressions like the #6780 pattern where a PR claimed a VS Code activation fix/closeout but shipped docs-only changes.

## Commands

```bash
cargo xtask intent-diff-gate --pr <N> --receipt target/receipts/intent-diff-gate.json
cargo xtask intent-diff-gate --fixture xtask/tests/fixtures/intent-diff/<name>.json
```

## Rules enforced

1. Code-fix claim with docs-only diff => warn/fail by policy.
2. `Closes`/`Fixes`/`Resolves` with known issue target paths requires one of:
   - target path touched,
   - tests updated,
   - behavior receipt present,
   - explicit override marker.
3. Scaffold/partial PRs using closing keywords => warn/fail by policy.
4. Docs-scoped titles with production code edits => warn/fail by policy.
5. VS Code activation fix claims require evidence against expected activation paths/tests unless override.

## Policy

Policy is read from `.ci/policies/intent-diff-rules.toml`.

## Receipt

Receipt schema: `.ci/receipts/schemas/intent-diff-gate.schema.json`

Required fields:

- `claimed_component`
- `claimed_closeout_issues`
- `expected_paths`
- `actual_paths`
- `evidence`
- `verdict`
- `violations`
