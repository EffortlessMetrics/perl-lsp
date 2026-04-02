# perl-lsp Roadmap

> Canonical planning document.
> Evidence and computed metrics belong in [CURRENT_STATUS.md](CURRENT_STATUS.md).
> Current workspace version is taken from [`../../Cargo.toml`](../../Cargo.toml);
> published release state must be verified against GitHub Releases;
> current capability truth is taken from [`../../features.toml`](../../features.toml).

## Current Framing

- Workspace version line: `v0.12.1`
- Latest published release: `v0.12.1` (tagged 2026-03-30, cleanup completed 2026-04-02)
- Active release target: `v0.12.2` stability hardening
- Canonical local receipt: `nix develop -c just ci-gate`

## How To Read This File

- [CURRENT_STATUS.md](CURRENT_STATUS.md) tells you what is true right now.
- This roadmap tells you what we are trying to land next.
- [../../ROADMAP.md](../../ROADMAP.md) and [../../NOW_NEXT_LATER.md](../../NOW_NEXT_LATER.md) are summaries, not the canonical plan.

## Completed: v0.12.1 Fix-Forward

Released 2026-03-30. Cleanup completed 2026-04-02.

- Fixed README drift, hook-fixture isolation, and git-identity injection
- Aligned all version surfaces (`Cargo.toml`, `features.toml`, `package.json`)
- Cleaned 11 stale release branches, closed tracking issue #2936
- Found and filed: pre-push hook fires CI gate on branch deletions (#3081)
- Found and fixed: `core.bare = true` corruption in `.git/config` (stale worktree interaction)

## Completed: v0.12.2 Stability Hardening (shipped 2026-04-02)

- CI improvements: version sync gate (#3078), benchmark alerts (#3079), coverage baseline (#3080)
- Pre-push hook fix (#3081, #3086), enforcement gaps (#3088), pipeline-labels race (#3100)
- Error handling logging (#3087), test coverage batch (#3091)
- 8 Dependabot PRs merged, perl-uri CI fix (#3084)
- All 7 Tier 1 parser blockers confirmed fixed via scouts (#3085, #3096)
- 10 PRs merged total

## Completed: v0.12.3 Diagnostic & Refactoring Hardening (shipped 2026-04-02)

- Dead code highlighting with DiagnosticTag::Unnecessary (#2060, PR #3092)
- Perlcritic integration hardened: cached analyzer, walk-up discovery (#2018, PR #3097)
- Strict/warnings diagnostics already implemented (PL100/PL101), catalogued in features.toml (#3095)
- Subroutine inlining (#3040, PR #3083) — 4 bugs caught and fixed by deep review
- Extract variable/subroutine (#3031, PR #3090)
- Scoped rename already complete (#3037)
- Moose/Moo method modifiers (#2328) and role composition (#2325) already implemented
- DAP Phase 3 test suite (#435) already complete (20 tests, all AC criteria met)
- 12 PRs merged + 6 issues discovered already-done

## Active Milestone: v0.12.4 Diagnostics & Semantics

Remaining work:

| Issue | Title | Status |
|-------|-------|--------|
| #3077 | Semantic framework coverage (inheritance, exports) | builder active |
| #3028 | Cross-platform DAP continue/interrupt signal handling | open |
| #3025 | DAP attach command for debugging existing processes | open |

### Supporting docs

- [CURRENT_STATUS.md](CURRENT_STATUS.md)
- [PARSER_EDGE_CASE_ROADMAP.md](PARSER_EDGE_CASE_ROADMAP.md)
- [CPAN_CORPUS_STRATEGY.md](CPAN_CORPUS_STRATEGY.md)

## Now / Next / Later

### Now (v0.12.4 — finishing)

- Semantic framework coverage (#3077) — builder active
- DAP cross-platform signal handling (#3028) and attach command (#3025)

### Next (v0.12.5 — parser confidence)

- Corpus ratchet to get true post-fix baseline (all Tier 1 blockers fixed)
- Quote-like operator parsing (#3020) and `state` keyword (#3033) — builders active
- Remaining Tier 2/3 blocker fixes

### Then (v0.12.5 — parser confidence)

- Corpus ratchet toward 90%+ CPAN clean rate (#3076)
- Quote-like operator parsing (#3020), `state` keyword (#3033)
- Top Tier 1 blocker fixes (206+99+84 affected files)
- Incremental parser checkpoint recovery (#2080)

### Then (v0.12.6 — performance)

- Large-workspace startup (#2078), completion latency (#2077), memory (#2085)
- Async task spawning overhead (#2082), CPAN-scale tuning (#1664)
- Benchmark regression detection (#2087)

### Then (v0.12.7 — distribution & packaging)

- Docker image (#2083), Nix flake (#2081), Homebrew verification (#2086)
- Windows package managers (#2596, #2089), Linux installers (#2095)
- Supply chain: SBOM generation and SLSA provenance (#281)

### Then (v0.12.8 — announcement polish)

- Rich hover docs with type info and POD excerpts (#1657)
- CPAN module docs on hover via MetaCPAN (#2061)
- Heredoc language injection for SQL/HTML/JSON (#2059)
- POD preview panel (#2062), debug launch templates (#2020)
- Animated GIFs showcasing features (#2336, #3026)

### Later (v0.13.0 — public alpha announcement)

- The 0.12.x line builds confidence; 0.13.0 is the initial public alpha announcement
- Seamless install story across all distribution channels
- Performance, security, and API stabilization work toward `v1.0.0`

## Milestone Ladder

### v0.11.0

Initial marketplace distribution.

### v0.12.0

Public alpha configuration: crates.io build-out, CPAN corpus testing, release
infrastructure, and packaging surfaces.

### v0.12.1

Fix-forward release (shipped 2026-03-30): README restoration, hook-fixture isolation,
git-hook installation, and release-surface alignment after the initial public alpha cut.

### v0.12.2

Stability hardening: CI infrastructure improvements, dependency freshness, parser
corpus confidence ratchet, and error-handling hygiene.

### v0.12.3

Diagnostic and refactoring hardening: dead code highlighting, perlcritic integration,
workspace-scoped rename, extract variable/subroutine, and Moose/Moo framework support.

### v0.12.4

Parser corpus confidence and performance profiling: close Tier 1 parser blockers,
large-workspace startup and completion latency work, benchmark regression detection.

### v0.12.5

Distribution and packaging: Docker, Nix, Homebrew, Windows/Linux package managers,
supply chain security (SBOM, SLSA provenance).

### v0.13.0

Initial public alpha announcement. The 0.12.x line builds confidence
across parser corpus, diagnostics, refactoring, and distribution so that
0.13.0 is ready for public announcement.

### Beyond v0.13.0

- Stability contract for APIs and advertised wire behavior
- Performance hardening for larger workspaces
- Security posture and documentation hardening
- Path to `v1.0.0`

## LSP Feature Implementation

The LSP compliance table is auto-generated from `features.toml`.

<!-- BEGIN: COMPLIANCE_TABLE -->
| Area | Implemented | Total | Coverage |
|------|-------------|-------|----------|
| debug | 10 | 10 | 100% |
| notebook | 2 | 2 | 100% |
| protocol | 9 | 9 | 100% |
| text_document | 42 | 42 | 100% |
| window | 9 | 9 | 100% |
| workspace | 26 | 26 | 100% |
| **Overall** | **98** | **98** | **100%** |
<!-- END: COMPLIANCE_TABLE -->

For live capability posture, run `just status-check` or read [CURRENT_STATUS.md](CURRENT_STATUS.md).

## Truth Sources

| Topic | Source |
| --- | --- |
| Workspace version line | [`../../Cargo.toml`](../../Cargo.toml) |
| Latest published release | GitHub Releases |
| Capability catalog | [`../../features.toml`](../../features.toml) |
| Evidence-backed metrics | [CURRENT_STATUS.md](CURRENT_STATUS.md) |
| Top-level summary docs | [../../ROADMAP.md](../../ROADMAP.md), [../../NOW_NEXT_LATER.md](../../NOW_NEXT_LATER.md) |

<!-- Last Updated: 2026-04-02 -->
