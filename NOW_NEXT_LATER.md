# NOW / NEXT / LATER

This file is the short planning snapshot for sequencing work. Use
[docs/project/ROADMAP.md](docs/project/ROADMAP.md) for the canonical milestone
plan and [docs/project/CURRENT_STATUS.md](docs/project/CURRENT_STATUS.md) for
evidence-backed status and release facts.

## DONE — public-alpha confidence line through v0.13.x

- The 0.12.x and 0.13.x lines built confidence across parser, diagnostics, refactoring, distribution, AI inline completion, and release automation.
- Release history and channel receipts are tracked in [RELEASE_HISTORY.md](RELEASE_HISTORY.md); do not copy those tables here.
- The current workspace version line has advanced to `v0.14.0` for the Rust 1.95 MSRV public-alpha patch train.

## NOW — v0.14.0 public-alpha patch prep

- Complete release-prep verification before dispatching release orchestration.
- Keep every install surface public-alpha framed: GitHub Release, crates.io, Docker, VS Code Marketplace, Open VSX, and `brew install effortlessmetrics/tap/perllsp`.
- Treat CI/control-plane work as seven independent lanes: status streaming, trigger lint, check-state normalization, review-receipt label projection, disposition receipts, merge-train receipts, and tokmd advisory stabilization.
- Preserve the editor-trust north star: no hangs, no confident lies, no silent regressions, edit-time recovery, and measurable improvements.
- Continue compiler-backed LSP work through fact-source tracing, shadow/provenance receipts, live fallback, and narrow provider cutovers.

## NEXT — after v0.14.0 channel receipts close

- Resume parser, corpus, semantic, and DAP hardening without reopening already-landed release prep.
- Run the editor-trust wave through one canonical harness, fixture schema, scorecard, dashboard, and ratchet path.
- Unify module-resolution and `@INC` behavior across completion, definition, hover, diagnostics, and workspace symbols.
- Promote stable scorecard floors gradually: nightly first, label-gated next, merge-blocking only after deterministic proof.

## LATER — public-alpha maturity toward v1.0.0

- Define the stability contract for public APIs and advertised wire behavior.
- Harden large-workspace performance and memory behavior.
- Harden security posture, repository trust, and documentation.
- Keep release and capability claims tied to receipts rather than aspirational wording.

## Working Rules

- Last updated: `2026-05-16`.
- Keep “current release line” separate from “next milestone”.
- Put receipts and computed metrics in [docs/project/CURRENT_STATUS.md](docs/project/CURRENT_STATUS.md), not here.
- Put detailed milestone criteria in [docs/project/ROADMAP.md](docs/project/ROADMAP.md), not here.
