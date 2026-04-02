# NOW / NEXT / LATER

This file is the short planning snapshot for sequencing work. Use
[docs/project/ROADMAP.md](docs/project/ROADMAP.md) for the canonical milestone
plan and [docs/project/CURRENT_STATUS.md](docs/project/CURRENT_STATUS.md) for
evidence-backed status and release facts.

## DONE — v0.12.2 stability + v0.12.3 diagnostics & refactoring (shipped 2026-04-02)

- 27 PRs merged: CI gates, hook fix, error handling, test coverage, dep bumps
- Diagnostics: dead code (PL406), perlcritic hardening, strict/warnings already done
- Refactoring: subroutine inlining, extract var/sub, scoped rename already done
- Moose/Moo, DAP Phase 3 all confirmed already implemented

## NOW — v0.12.4 finishing + v0.12.5 parser confidence

- Semantic framework coverage (#3077) — builder active
- Parser: quote-like operators (#3020), state keyword (#3033) — builders active
- Corpus ratchet needed to get true post-fix baseline (all Tier 1 blockers fixed)
- DAP: cross-platform signals (#3028), attach command (#3025)

## NEXT — v0.12.6 performance + v0.12.7 distribution

- Performance: workspace startup (#2078), completion latency (#2077), CPAN-scale (#1664)
- Distribution: Docker (#2083), Nix (#2081), Homebrew (#2086), Windows (#2596), Linux (#2095)
- Supply chain: SBOM and SLSA provenance (#281)

## LATER — v0.13.0 public alpha announcement

- 0.12.x builds confidence; 0.13.0 is the initial public alpha announcement
- Seamless install across all distribution channels
- Performance, security, and API-stability work on the path to `v1.0.0`

## Working Rules

- Last updated: `2026-04-02`
- Keep “current release line” separate from “next milestone”.
- Put receipts and computed metrics in [docs/project/CURRENT_STATUS.md](docs/project/CURRENT_STATUS.md), not here.
- Put detailed milestone criteria in [docs/project/ROADMAP.md](docs/project/ROADMAP.md), not here.
