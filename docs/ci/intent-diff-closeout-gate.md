# Intent/Diff Closeout Evidence Gate

`cargo xtask intent-diff-gate` checks whether PR intent (title/body) matches the actual diff,
and whether closeout keywords include enough evidence to safely close issues.

Motivation: PR #6780 showed how a mismatch can slip through when a PR claims a VS Code
activation fix but changes only docs. This gate is designed to catch that pattern early
without assigning blame.

## Commands

```bash
cargo xtask intent-diff-gate --pr <N> --receipt target/receipts/intent-diff-gate.json
cargo xtask intent-diff-gate --fixture <json>
```

## Rules

1. Code-fix claim with docs-only diff is flagged (warn/fail from policy).
2. `Closes/Fixes/Resolves #NNNN` requires at least one of:
   - expected target path touched,
   - test update,
   - behavior receipt,
   - explicit override.
3. Scaffold/partial PRs that use closing keywords are flagged.
4. Docs-claimed PRs touching production code are flagged.
5. VS Code activation fix claims map to expected paths:
   - `vscode-extension/package.json`, or
   - relevant tests under `crates/perl-lsp-rs/tests/`.

## Inputs and policy

- Policy file: `.ci/policies/intent-diff-rules.toml`
- Receipt schema: `.ci/receipts/schemas/intent-diff-gate.schema.json`
- Local fixtures: `xtask/tests/fixtures/intent-diff/*.json`

## Receipt shape

The gate writes a receipt containing:

- `claimed_component`
- `claimed_closeout_issues`
- `expected_paths`
- `actual_paths`
- `evidence`
- `verdict`
- `violations`
