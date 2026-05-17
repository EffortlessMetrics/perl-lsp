# NOW / NEXT / LATER

This file is the short planning snapshot for sequencing work. Use
[docs/project/ROADMAP.md](docs/project/ROADMAP.md) for the canonical milestone
plan and [docs/project/CURRENT_STATUS.md](docs/project/CURRENT_STATUS.md) for
evidence-backed status and release facts.

## DONE — public-alpha foundation through v0.13.x

- v0.12.x built confidence across parser corpus, diagnostics, refactoring, distribution, AI inline completion, packaging, and announcement polish
- v0.13.x carried that public-alpha foundation forward into the current release-prep line
- CI/control-plane Wave 2 substrate landed: gate timeout receipts, bounded build storage contracts, UX receipt upload path, PR-fast planner coverage, and tokmd advisory instrumentation
- Compiler-backed LSP substrate now has fixture-backed semantic facts, fact-source traces, and live-with-fallback slices for narrow diagnostics, hover, definition, and references

## NOW — v0.14.0 public-alpha patch prep

- Treat `v0.14.0` as the active public-alpha release train; do not describe it as stable/GA
- Complete RP-2 dry-run publish readiness before tag, channel dispatch, or announcement operations
- Keep install-channel receipts explicit for crates.io, GitHub Release assets, Docker, VS Code Marketplace, Open VSX, and Homebrew (`brew install effortlessmetrics/tap/perllsp`)
- Run the next CI/control-plane slice first: `update-status --write` progress streaming and failure attribution
- Keep compiler-backed provider work evidence-gated: live paths need provenance and fallback receipts; uncertain paths remain shadow-only

## NEXT — after v0.14.0 receipts close

- Resume parser, corpus, semantic, and DAP hardening in narrow reviewable slices
- Promote compiler-backed completion, rename, safe-delete, workspace symbols, document symbols, and semantic tokens only after source/freshness receipts are strong enough
- Execute the editor-trust wave with one canonical lane per UX area and deterministic real-project receipts before CI ratchets
- Continue quality cleanup: production-code `unwrap()` / `expect()` audit, temporary allow burndown, dependency triage, and install-surface checks

## LATER — v1.0 runway

- Define the stable Rust API surface and the internal crate boundary
- Define advertised LSP/DAP wire-behavior compatibility and fallback semantics
- Establish large-workspace performance and memory budgets that respect agent build-storage limits
- Document subprocess, Perl environment, and extension-distribution security expectations
- Pick a `v1.0.0` date only after those contracts have reviewable acceptance criteria

## Working Rules

- Last updated: `2026-05-16`
- Keep “current release line” separate from “next milestone”.
- Put receipts and computed metrics in [docs/project/CURRENT_STATUS.md](docs/project/CURRENT_STATUS.md), not here.
- Put detailed milestone criteria in [docs/project/ROADMAP.md](docs/project/ROADMAP.md), not here.
