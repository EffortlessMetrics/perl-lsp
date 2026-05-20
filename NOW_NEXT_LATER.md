# NOW / NEXT / LATER

This file is the short planning snapshot for sequencing work. Use
[docs/project/ROADMAP.md](docs/project/ROADMAP.md) for the canonical milestone
plan and [docs/project/CURRENT_STATUS.md](docs/project/CURRENT_STATUS.md) for
evidence-backed status and release facts.

## DONE — v0.12.x and v0.13.x public-alpha foundation

- The 0.12.x line built confidence across parser corpus, diagnostics, refactoring, distribution, packaging, and announcement polish
- The 0.13.x line moved the public-alpha posture forward while preserving release-channel discipline and evidence-backed status docs
- Earlier release facts are historical; verify current workspace version, crate surface, and channel state against the truth sources before quoting them

## NOW — v0.14.0 channel closeout and post-release proof

- `v0.14.0` is the current public-alpha release line; GitHub Release and crates.io surfaces show it live, but full channel closeout still needs explicit receipts
- Keep release proof explicit across GitHub Releases, crates.io, Docker, VS Code Marketplace, Open VSX, and the owned Homebrew tap path
- Keep package-version language separate from product-posture language: SemVer package version, public-alpha product promise
- Land CI/control-plane follow-up as reviewable lanes, with #7404 (`update-status --write` streaming) first
- Keep parser corpus lanes, compiler-backed provider dashboards, and install-surface receipts linked rather than duplicated in this short snapshot

## NEXT — post-v0.14.0

- Close channel receipts before broad cleanup
- Resume parser, corpus, semantic, DAP, and editor-trust hardening after release proof is complete
- Continue compiler-backed provider cutovers through provenance-backed, live-with-fallback slices
- Burn down deferred `v0.14.0` successor issues by ledger rather than by undocumented cleanup

## LATER — beyond v0.14.0

- Stability contract for APIs and advertised wire behavior
- Performance hardening for larger workspaces
- Security and supply-chain posture hardening
- Path to `v1.0.0`

## Working Rules

- Last updated: `2026-05-19`
- Keep “current release line” separate from “next milestone”.
- Put receipts and computed metrics in [docs/project/CURRENT_STATUS.md](docs/project/CURRENT_STATUS.md), not here.
- Put detailed milestone criteria in [docs/project/ROADMAP.md](docs/project/ROADMAP.md), not here.
