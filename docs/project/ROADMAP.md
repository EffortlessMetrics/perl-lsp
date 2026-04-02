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

## Active Milestone: v0.12.2 Stability Hardening

Focus: CI infrastructure improvements, dependency updates, and parser corpus
confidence — close the gaps that make day-to-day development and release
operations smoother without adding new user-facing features.

### Track 1: CI & Infrastructure

| Issue | Title | Status |
|-------|-------|--------|
| #3081 | Pre-push hook skip CI on branch deletion | filed |
| #3078 | Version sync enforcement in merge gate | PR open |
| #3079 | Benchmark regression alerts with real baselines | PR open |
| #3080 | Parser branch coverage gate and baseline refresh | PR open |
| #2027 | Version-check CI gate to prevent drift | research-verified |
| #2026 | Automate corpus ratchet after parser fix merges | research-verified |

### Track 2: Dependency Freshness

7 Dependabot PRs pending review (#3064–#3071): `toml`, `insta`, `uuid`,
`proptest`, `actions/deploy-pages`, `codecov/codecov-action`, and a grouped
cargo update.

### Track 3: Parser Corpus Confidence

Target: raise CPAN clean rate from 72.1% toward 80%.

| Blocker | Files | Issue | Status |
|---------|-------|-------|--------|
| `unexpected_token_in_expr` | 206 | — | needs investigation |
| `unexpected_rbrace_expr` | 105 | #2189 | filed |
| `unclosed_paren` | 99 | — | needs investigation |
| `unexpected_comma_expr` | 98 | #2140 | filed |
| `unexpected_rparen_expr` | 84 | — | needs investigation |
| `unclosed_paren_identifier` | 70 | #2391 | filed |
| `expected_module_name` | 69 | — | needs investigation |

Focus for v0.12.2: scout and triage the top 3 uninvestigated blockers, land
fixes for any that have straightforward root causes.

### Track 4: Test & Error Handling Hygiene

| Issue | Title |
|-------|-------|
| #3039 | Missing error path tests (should_panic) in critical runtime modules |
| #3038 | Improve test error handling patterns to match production |
| #3036 | Add trace logging for URI parsing failures |
| #3032 | Log file re-indexing failures in workspace watcher |
| #3029 | Log incremental parsing failures instead of silent fallback |
| #3030 | Missing tests for lexer mode tracking |
| #3024 | Missing tests for LSP error response builders |

### Exit criteria

- [ ] All 3 CI improvement PRs (#3078–#3080) merged
- [ ] Pre-push hook branch-deletion fix (#3081) merged
- [ ] Dependabot PRs reviewed and merged or closed
- [ ] Top 3 uninvestigated parser blockers scouted with issues filed
- [ ] CPAN clean rate holds or improves (baseline: 72.1%)
- [ ] `nix develop -c just ci-gate` green
- [ ] At least 3 error-handling hygiene issues closed

## Next Milestone: v0.12.3 Diagnostic & Refactoring Hardening

Focus: ship the diagnostic and refactoring features that make the editor
experience materially better for daily Perl development.

### Track 1: Diagnostics

| Issue | Title | Status |
|-------|-------|--------|
| #2060 | Dead code highlighting with `DiagnosticTag::Unnecessary` | accuracy-reviewed |
| #2018 | Integrate perlcritic for code quality diagnostics | accuracy-reviewed |
| — | `strict` pragma enforcement diagnostics | roadmap (not filed) |
| — | `warnings` pragma signal diagnostics | roadmap (not filed) |

### Track 2: Refactoring

| Issue | Title | Status |
|-------|-------|--------|
| #3037 | Complete scoped rename with scope filtering | plan-reviewed |
| #3040 | Implement subroutine inlining for code actions | plan-reviewed |
| #3031 | Implement extract variable/subroutine code actions | plan-reviewed |
| #349 | Generate edits for extract variable/subroutine | plan-reviewed |
| #1663 | Complete scoped rename + subroutine inlining | research-verified |

### Track 3: Semantic Framework Coverage

| Issue | Title | Status |
|-------|-------|--------|
| #3077 | Complete semantic framework coverage (inheritance, exports) | open |
| #2328 | Moose/Moo method modifiers IDE support | known blocker |
| #2325 | Moose/Moo role composition detection | known blocker |

### Track 4: DAP Hardening

| Issue | Title | Status |
|-------|-------|--------|
| #435 | DAP non-regression test suite (Phase 3) | in-build |
| #3028 | Cross-platform DAP continue/interrupt signal handling | open |
| #3025 | DAP attach command for debugging existing processes | open |

### Exit criteria

- [ ] Dead code highlighting and perlcritic integration shipped
- [ ] Workspace-scoped rename at GA
- [ ] Extract variable/subroutine code actions at GA
- [ ] Moose/Moo framework support (method modifiers + role composition) landed
- [ ] DAP Phase 3 test suite complete
- [ ] Parser CPAN clean rate ≥ 80%

### Supporting docs

- [CURRENT_STATUS.md](CURRENT_STATUS.md)
- [PARSER_EDGE_CASE_ROADMAP.md](PARSER_EDGE_CASE_ROADMAP.md)
- [CPAN_CORPUS_STRATEGY.md](CPAN_CORPUS_STRATEGY.md)

## Now / Next / Later

### Now (v0.12.2)

- Merge 3 CI improvement PRs and 7 Dependabot PRs
- Fix pre-push hook branch-deletion regression (#3081)
- Scout top 3 uninvestigated parser blockers (206+99+84 affected files)
- Close error-handling hygiene batch (#3029, #3032, #3036, #3038, #3039)

### Next (v0.12.3)

- Diagnostic hardening: dead code highlighting, perlcritic, `strict`, `warnings`
- Refactoring reliability: workspace-scoped rename, extract variable/subroutine, subroutine inlining
- Semantic framework coverage: Moose/Moo method modifiers and role composition
- DAP Phase 3 test suite and cross-platform signal handling

### Then (v0.12.4 — diagnostics & semantics)

- Dead code highlighting (#2060), perlcritic integration (#2018)
- `strict`/`warnings` enforcement diagnostics
- Moose/Moo method modifiers (#2328), role composition (#2325)
- Semantic framework coverage for inheritance and exports (#3077)

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
