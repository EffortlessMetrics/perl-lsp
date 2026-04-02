# NOW / NEXT / LATER

This file is the short planning snapshot for sequencing work. Use
[docs/project/ROADMAP.md](docs/project/ROADMAP.md) for the canonical milestone
plan and [docs/project/CURRENT_STATUS.md](docs/project/CURRENT_STATUS.md) for
evidence-backed status and release facts.

## NOW — v0.12.2 stability hardening

- CI infrastructure: merge 3 CI improvement PRs (#3078–#3080) and 7 Dependabot PRs
- Hook fix: pre-push hook skip CI gate on branch deletions (#3081)
- Parser: scout top 3 uninvestigated blockers (206+99+84 affected CPAN files), target 80% clean rate
- Hygiene: close error-handling and test-gap batch (#3029, #3032, #3036, #3038, #3039)

## NEXT — v0.12.3 diagnostic & refactoring hardening

- Diagnostics: dead code highlighting (#2060), perlcritic integration (#2018), `strict`/`warnings` enforcement
- Refactoring: workspace-scoped rename (#3037), extract variable/subroutine (#3031), subroutine inlining (#3040)
- Frameworks: Moose/Moo method modifiers (#2328) and role composition (#2325)
- DAP: Phase 3 test suite (#435), cross-platform signal handling (#3028), attach command (#3025)

## THEN — v0.12.4 parser & performance, v0.12.5 distribution

- Parser corpus confidence: close Tier 1 blockers, target ≥85% CPAN clean rate
- Performance: large-workspace startup (#2078), completion latency (#2077), memory profiling (#2085)
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
