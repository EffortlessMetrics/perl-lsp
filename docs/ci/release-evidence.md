# Release Evidence Bundle

Release readiness is proven with a receipt bundle under:

- `target/release-evidence/v<version>/...`

The `xtask release evidence` flow verifies that required receipts exist, required receipts are passing, and advisory failures are classified by policy (warning by default, release-blocking only when policy says so).

## Policy

Policy lives at `.ci/release/evidence.toml`:

- `release_evidence.required`: required receipt filenames.
- `release_evidence.summary_receipt`: emitted summary receipt path.
- `advisory.receipt`: advisory receipt filename.
- `advisory.release_blocking`: whether advisory failures are release-blocking.

## Commands

```bash
cargo xtask release evidence --version 0.13.0 --out target/release-evidence/v0.13.0
cargo xtask release verify-evidence --version 0.13.0 --receipt target/receipts/release-evidence.json
```

## Required evidence paths

- `ci-gate.json`
- `parser-ratchet-release.json`
- `vscode-extension-smoke.json`
- `lsp-scenario.json`
- `real-workspace-baseline.json`
- `ai-completion-e2e.json`
- `advisory-status.json`
- `unresolved-risk-register.json`

Summary receipts conform to `.ci/receipts/schemas/release-evidence.schema.json`.
