# NOW / NEXT / LATER

This file is the short planning snapshot for sequencing work. Use
[docs/project/ROADMAP.md](docs/project/ROADMAP.md) for the canonical milestone
plan and [docs/project/CURRENT_STATUS.md](docs/project/CURRENT_STATUS.md) for
evidence-backed status and release facts.

## NOW

- Release execution: keep `nix develop -c just ci-gate` and release receipts green while `v0.12.1` closes the launch regressions found after `v0.12.0`
- Packaging truth: keep `perllsp` and `perl-lsp-rs` aligned across Cargo, docs, release assets, and operator runbooks
- Parser hardening: keep corpus ratchets honest and finish the highest-value edge-case fixes
- Semantic coverage: land the framework-aware resolution work that blocks real project navigation
- Docs alignment: keep README, roadmap, changelog, status, and install guidance consistent about what is already published versus what is still on `main`

## NEXT

- Diagnostic hardening around `strict`, `warnings`, safe static analysis, and dead-code signals
- Refactoring and debugger reliability beyond the current alpha posture
- Post-release cleanup for package-manager, docs, and distribution surfaces

## LATER

- `v0.15.0` stability contract for APIs and advertised behavior
- Platform certification and packaging expansion
- Performance, security, and API-stability work on the path to `v1.0.0`

## Working Rules

- Last updated: `2026-03-30`
- Keep “current release line” separate from “next milestone”.
- Put receipts and computed metrics in [docs/project/CURRENT_STATUS.md](docs/project/CURRENT_STATUS.md), not here.
- Put detailed milestone criteria in [docs/project/ROADMAP.md](docs/project/ROADMAP.md), not here.
