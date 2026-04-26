# Release evidence bundle scaffolding

Release readiness is tracked as an evidence bundle receipt, not as a label.

## Commands

```bash
cargo xtask release evidence --version 0.13.0 --out target/release-evidence/v0.13.0
cargo xtask release verify-evidence --version 0.13.0 --receipt target/receipts/release-evidence.json
```

## Required evidence bundle paths

The release evidence generator expects these JSON receipts in the `--out` directory:

- `ci-gate.json`
- `parser-ratchet-release.json`
- `vscode-extension-smoke.json`
- `lsp-scenario.json`
- `real-workspace-baseline.json`
- `ai-completion-e2e.json`
- `advisory-status.json`
- `unresolved-risk-register.json`

The canonical policy file is `.ci/release/evidence.toml`.

## Behavior

- Verifies required receipts exist.
- Verifies required receipts report `"status": "pass"`.
- Classifies advisory failures as warnings when policy marks them as non-blocking.
- Does not fail release evidence verification for advisory-only failures unless policy says
  `release_blocking = true`.
- Writes summary receipt to `target/receipts/release-evidence.json`.
