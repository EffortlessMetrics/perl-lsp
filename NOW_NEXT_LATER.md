# NOW / NEXT / LATER

This file is the short planning snapshot for sequencing work. Use
[docs/project/ROADMAP.md](docs/project/ROADMAP.md) for the canonical milestone
plan and [docs/project/CURRENT_STATUS.md](docs/project/CURRENT_STATUS.md) for
evidence-backed status and release facts.

## DONE — v0.12.2 through v0.12.8 (consolidated and shipped)

- Work consolidated and merged 2026-04-02 (~70 PRs): CI gates, error handling, test coverage, parser confidence, performance, distribution, AI inline completion, packaging, announcement polish
- `v0.12.2` cut as a GitHub Release on 2026-04-04 with binaries, SBOM, and SHA256SUMS
- Editor channels (VSCode Marketplace, Open VSX) and Docker Hub: published 2026-04-04

## NOW — pre-announcement cleanup

- License badge fixed (canonical SPDX text in all 126 LICENSE files), GitHub now reports `Apache-2.0` instead of `NOASSERTION`
- Docker arm64 timeout fix landed (Dockerfile MSRV pin + workflow timeout bump)
- Dependency triage complete: 7 dependabot PRs merged including 3 majors (eslint v10, actions/cache v5, similar 3.0.0)
- SRP microcrate extractions in flight (anti_pattern_detector, bench_parser) to free the dead `tree-sitter-perl-rs` harness for archival
- Publishing the modern parsers as `tree-sitter-perl-c` (C tree-sitter FFI) and a new `tree-sitter-perl-rs` (Rust v3 facade with tree-sitter-compatible output)
- Per-crate publish blockers cleared (perl-lsp-ai-provider unblocked, perl-heredoc-anti-patterns extraction in progress)

## NEXT — v0.13.0 public alpha announcement

- Re-trigger crates.io publish after the SRP extractions and harness archival land
- Final smoke test across all distribution channels
- Bump workspace to `0.13.0`
- Announcement blog post / release notes

## LATER — beyond v0.13.0

- Stability contract for APIs and advertised wire behavior
- Performance hardening for larger workspaces
- Security posture and documentation hardening
- Path to `v1.0.0`

## Working Rules

- Last updated: `2026-04-07`
- Keep “current release line” separate from “next milestone”.
- Put receipts and computed metrics in [docs/project/CURRENT_STATUS.md](docs/project/CURRENT_STATUS.md), not here.
- Put detailed milestone criteria in [docs/project/ROADMAP.md](docs/project/ROADMAP.md), not here.
