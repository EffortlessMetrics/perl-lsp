# perl-lsp Roadmap

> Canonical planning document.
> Evidence and computed metrics belong in [CURRENT_STATUS.md](CURRENT_STATUS.md).
> Current workspace version is taken from [`../../Cargo.toml`](../../Cargo.toml);
> published release state must be verified against GitHub Releases;
> current capability truth is taken from [`../../features.toml`](../../features.toml).

## Current Framing

- Workspace version line: `v0.12.4`
- Latest published GitHub/editor release: `v0.12.3` (GitHub Releases, VS Code Marketplace, and Open VSX public line, shipped 2026-04-09)
- crates.io published line: `v0.12.2` (registry line, shipped 2026-04-07)
- Active work: finish the `v0.13.0` public alpha announcement pass (demo assets, distribution-truth cleanup, post-release docs/automation cleanup) while keeping the shipped `v0.12.3` line stable across GitHub Releases and the editor marketplaces
- Canonical local receipt: `nix develop -c just ci-gate`

Publication discipline: public release truth is intentionally split right now. GitHub Releases and the editor marketplaces are on `v0.12.3`; crates.io remains on `v0.12.2` until the registry window reopens. Milestone sections below can describe the intended `0.12.x` breakdown, but they must not blur that channel split.

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

## Completed: v0.12.3 Diagnostic & Refactoring Hardening (GitHub/editor release shipped 2026-04-09)

- Dead code highlighting with DiagnosticTag::Unnecessary (#2060, PR #3092)
- Perlcritic integration hardened: cached analyzer, walk-up discovery (#2018, PR #3097)
- Strict/warnings diagnostics already implemented (PL100/PL101), catalogued in features.toml (#3095)
- Subroutine inlining (#3040, PR #3083) — 4 bugs caught and fixed by deep review
- Extract variable/subroutine (#3031, PR #3090)
- Scoped rename already complete (#3037)
- Moose/Moo method modifiers (#2328) and role composition (#2325) already implemented
- DAP Phase 3 test suite (#435) already complete (20 tests, all AC criteria met)
- 12 PRs merged + 6 issues discovered already-done

## Prepared Scope: v0.12.4 Diagnostics & Semantics

- Semantic framework coverage: inheritance, exports (#3077, PR #3098)
- Cross-platform DAP continue/interrupt signal handling (#3028, PR #3117)
- DAP attach command: stale mock stub removed, tests updated (#3025, PR #3135)

## Prepared Scope: v0.12.5 Parser Confidence

- All Tier 1 parser blockers confirmed fixed
- Incremental parser checkpoint recovery (#2080, PR #3114)
- Token caching for incremental parsing (#3021, PR #3116)
- Corpus ratchet automation (#2026, PR #3110)
- 90% CPAN clean rate target documented (#3076, PR #3123)

## Prepared Scope: v0.12.6 Performance

- Large-workspace HashMap optimization (#2078, PR #3112)
- Memory profiling infrastructure (#2085, PR #3125)
- CPAN-scale benchmarks: 10K files, 500K symbols (#1664, PR #3121/3132)
- Large-workspace testing and profiling guide (#3022, PR #3126)

## Prepared Scope: v0.12.7 Distribution & Packaging

- Docker image with perllsp + Perl runtime (#2083, PR #3113)
- Linux/macOS installer script (#2095, PR #3122)
- Homebrew bump workflow + install docs (#2086, PR #3120)
- Windows bump workflows aligned (#2596, PR #3106)

## Prepared Scope: v0.12.8 Announcement Polish

- Heredoc language injection for SQL/JSON (#2059, PR #3134)
- POD preview panel (#2062, PR #3131)
- AST explorer debug panel (#2065, PR #3124)
- Problem-first README rewrite (#3119)
- End-to-end LSP feature development guide (#3027, PR #3115)
- GIF recording guide and asset structure (#2336, PR #3130)

## Active: Quality Cleanup (post-v0.12.3 / pre-v0.13.0)

- Debug println removal from library code (in progress)
- Unused dependency removal across 6 crates (in progress)
- Banned unwrap()/expect() replacement in production code (in progress)
- Clippy zero-warning enforcement (done, PR #3138)

## Now / Next / Later

### Now (post-v0.12.3 / pre-v0.13.0)

- `v0.12.3` shipped to GitHub Releases, VS Code Marketplace, and Open VSX on 2026-04-09; crates.io remains on `v0.12.2`
- Pre-announcement license badge fix (PR #3193): canonical SPDX text in all 126 LICENSE files
- Pre-announcement Docker arm64 timeout fix (#3188 → PR #3191, merged)
- Per-release dependency triage: 7 dependabot PRs merged 2026-04-07 (#3178–#3184)
- Code quality cleanup: debug prints (only `crates/perl-corpus/src/bin/main.rs` CLI output remains, library code clean), unused deps, remaining `unwrap()`/`expect()` audit in production code
- Test coverage gaps and broken integration tests
- VSCode extension lint/quality audit (eslint v10 landed in #3179)
- AI inline completion (#3018) shipped in the live 0.12.x line — feature wired end-to-end via #3157–#3168, awaiting E2E user validation
- Coroutine support issue #3539 is re-scoped: defer hypothetical core syntax, split upstream-tracking from CPAN-library IDE support planning

### Next (v0.13.0 — public alpha announcement)

- The 0.12.x line has built confidence across parser, diagnostics, refactoring, and distribution
- Quality cleanup PRs land, version bump to 0.13.0
- Seamless install story verified across all distribution channels
- Announcement blog post / release notes

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

GitHub/editor release line: status regeneration, corpus receipts, version-surface alignment,
and readiness verification shipped on 2026-04-09 ahead of the public alpha announcement.

### v0.12.4

Follow-on diagnostics and semantics scope retained on the prep track, not yet a separately published GitHub release.

### v0.12.5–v0.12.8

Parser confidence, performance, distribution, and announcement-polish scopes retained on the prep track.
Treat these as internal milestone slices until the next public GitHub release beyond `v0.12.3` is actually cut.

### v0.13.0

Initial public alpha announcement. The 0.12.x line built confidence
across parser corpus, diagnostics, refactoring, and distribution.
0.13.0 is the announcement version.

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
| debug | 24 | 24 | 100% |
| notebook | 2 | 2 | 100% |
| protocol | 9 | 9 | 100% |
| text_document | 49 | 49 | 100% |
| window | 9 | 9 | 100% |
| workspace | 26 | 26 | 100% |
| **Overall** | **119** | **119** | **100%** |
<!-- END: COMPLIANCE_TABLE -->

For live capability posture, run `just status-check` or read [CURRENT_STATUS.md](CURRENT_STATUS.md).

## Truth Sources

| Topic | Source |
| --- | --- |
| Workspace version line | [`../../Cargo.toml`](../../Cargo.toml) |
| Latest published release | GitHub Releases (`v0.12.3`) + crates.io API (`0.12.2` when channel split matters) |
| Capability catalog | [`../../features.toml`](../../features.toml) |
| Evidence-backed metrics | [CURRENT_STATUS.md](CURRENT_STATUS.md) |
| Top-level summary docs | [../../ROADMAP.md](../../ROADMAP.md), [../../NOW_NEXT_LATER.md](../../NOW_NEXT_LATER.md) |

<!-- Last Updated: 2026-04-09 -->
