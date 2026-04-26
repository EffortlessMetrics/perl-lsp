# Release Evidence Bundle

`cargo xtask release evidence` scaffolds a release evidence bundle directory and
records the required receipt filenames for a versioned release candidate.

`cargo xtask release verify-evidence` validates an existing bundle and writes a
summary receipt (`target/receipts/release-evidence.json` by default in CI jobs).

## Required receipt files

For `--version 0.13.0`, the verifier expects:

- `target/release-evidence/v0.13.0/ci-gate.json`
- `target/release-evidence/v0.13.0/parser-ratchet-release.json`
- `target/release-evidence/v0.13.0/vscode-extension-smoke.json`
- `target/release-evidence/v0.13.0/lsp-scenario.json`
- `target/release-evidence/v0.13.0/real-workspace-baseline.json`
- `target/release-evidence/v0.13.0/ai-completion-e2e.json`
- `target/release-evidence/v0.13.0/advisory-status.json`
- `target/release-evidence/v0.13.0/unresolved-risk-register.json`

## Policy

Policy lives in `.ci/release/evidence.toml`.

- Missing required receipts fail verification.
- Non-advisory required receipts with `status != "pass"` fail verification.
- Advisory failures are classified against
  `release_blocking_advisory_severities`.
- Advisory failures do **not** fail verification unless an advisory matches a
  release-blocking severity.

## Commands

```bash
cargo xtask release evidence --version 0.13.0 --out target/release-evidence/v0.13.0
cargo xtask release verify-evidence --version 0.13.0 --receipt target/receipts/release-evidence.json
```
