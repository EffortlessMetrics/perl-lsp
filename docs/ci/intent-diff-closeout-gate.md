# Intent/Diff Closeout Evidence Gate

`cargo xtask intent-diff-gate` validates whether PR intent claims (title/body) match the diff and whether closeout keywords have supporting evidence.

## Commands

```bash
cargo xtask intent-diff-gate --pr <N> --receipt target/receipts/intent-diff-gate.json
cargo xtask intent-diff-gate --fixture xtask/tests/fixtures/intent-diff/<fixture>.json
```

## Rules

1. Code-fix intent with docs-only diffs is a policy violation (`warn`/`fail` by policy).
2. `Closes/Fixes/Resolves #<issue>` requires one of:
   - expected target path changed,
   - tests updated,
   - behavior receipt present,
   - explicit override reason.
3. Scaffold/partial PRs must not use closing keywords.
4. Docs-scoped titles that modify production code are flagged.
5. VS Code activation fix claims require `vscode-extension/package.json` (or relevant tests) unless explicitly overridden.

Motivation: we want to prevent intent/diff/closeout drift like the #6780-style mismatch while keeping `Refs #...` usable for partial work.
