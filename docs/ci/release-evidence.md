# Release Evidence Bundle Scaffolding

Release readiness for a version is represented by a concrete evidence bundle in
`target/release-evidence/v<version>/`.

## Commands

```bash
cargo xtask release evidence --version 0.13.0 --out target/release-evidence/v0.13.0
cargo xtask release verify-evidence --version 0.13.0 --receipt target/receipts/release-evidence.json
```

## Required evidence files

- `ci-gate.json`
- `parser-ratchet-release.json`
- `vscode-extension-smoke.json`
- `lsp-scenario.json`
- `real-workspace-baseline.json`
- `ai-completion-e2e.json`
- `advisory-status.json`
- `unresolved-risk-register.json`

`verify-evidence` enforces:

- required receipts exist
- required receipts report pass
- advisory failures are classified as warnings
- advisory failures do not block release unless policy marks them release-blocking
- a summary receipt is written to the `--receipt` path

Policy is configured in `.ci/release/evidence.toml`.
