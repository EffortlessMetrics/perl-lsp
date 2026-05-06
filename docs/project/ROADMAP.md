# perl-lsp Roadmap

> Canonical planning document.
> Evidence and computed metrics belong in [CURRENT_STATUS.md](CURRENT_STATUS.md).
> Current workspace version is taken from [`../../Cargo.toml`](../../Cargo.toml);
> published release state must be verified against GitHub Releases;
> current capability truth is taken from [`../../features.toml`](../../features.toml).

## Current Framing

- Workspace version line: `v0.13.4`
- Current release train: `v0.13.4` public-alpha patch prep, with release dispatch intentionally pending
- Published crate surface target: 31 crates from `[workspace.metadata.publish.allow]`
- Active work: finish release-prep verification, keep install-surface receipts wired into the runbook, and keep release language public-alpha rather than stable/GA
- Canonical local receipt: `nix develop -c just ci-gate`

Publication discipline: `v0.13.4` uses a normal SemVer package version for release channels while the human-facing product posture remains public alpha. See [RELEASE_HISTORY.md](../../RELEASE_HISTORY.md) for the cross-channel ledger, and do not dispatch the release until the prep checks pass.

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

## Completed: v0.12.4 Diagnostics & Semantics (shipped 2026-04-12)

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

## Active: Public-Alpha Release Prep (v0.13.2)

- GitHub Release, crates.io, Docker, VS Code Marketplace, Open VSX, and Homebrew tap receipts are tracked separately
- The owned Homebrew path is `brew install effortlessmetrics/tap/perllsp`
- Public install language must say public alpha, not stable/GA
- Follow-on quality cleanup resumes after the release-channel receipts are closed

## Now / Next / Later

### Now (v0.13.2 public-alpha patch prep)

- CI/control-plane Wave 2 substrate already landed and should not be re-implemented in parallel follow-up PRs:
  - Per-gate timeout regression coverage in gate receipts (#7525)
  - Bounded build-plane/agent storage contract (`cargo-safe`, `devplane-init`, `storage-doctor`) (#7449)
  - UX receipt command registration + workflow upload path (#7569, #7561)
  - PR-fast planner matrix coverage (#7547)
  - Tokmd advisory workflow staged as non-blocking instrumentation (#7568)
- Next CI/control-plane wave should optimize for reviewable, testable, independent slices and avoid broad redesign:
  1. `update-status --write` progress streaming/failure attribution (#7404)
  2. CI trigger regression lint (`pull_request:labeled|unlabeled` + `cancel-in-progress`)
  3. Expected-skip/stale-check status normalization in merge-ready/reconciler
  4. Review receipt -> reconciler label projection (labels as projected state, not source truth)
  5. PR disposition evidence contract (duplicate/superseded/absorbed/extracted with linked evidence)
  6. Merge-train planner/receipt protocol with stop conditions
  7. Tokmd advisory stabilization (explicitly non-required while calibrating signal)
- Wave guardrails: no bulk stale-closure automation, no full merge bot scope, no global pre-push hooks, no broad CI architecture rewrite in this pass.
- `v0.13.2` is staged as the next public-alpha patch release; run the release-prep checks before dispatching the train
- Pre-announcement license badge fix (PR #3193): canonical SPDX text in all 126 LICENSE files
- Pre-announcement Docker arm64 timeout fix (#3188 → PR #3191, merged)
- Per-release dependency triage: 7 dependabot PRs merged 2026-04-07 (#3178–#3184)
- Code quality cleanup: debug prints (only `crates/perl-corpus/src/bin/main.rs` CLI output remains, library code clean), unused deps, remaining `unwrap()`/`expect()` audit in production code
- Test coverage gaps and broken integration tests
- VSCode extension lint/quality audit (eslint v10 landed in #3179)
- AI inline completion (#3018) shipped in the live 0.12.x line — feature wired end-to-end via #3157–#3168, awaiting E2E user validation
- Workspace-wide rename slice: multi-root support shipped in 0.12.x (#3984); workspace-wide rename/module-move remains roughly 30% complete and only conditionally in scope pending #3522 verification
- Coroutine support issue #3539 is re-scoped: defer hypothetical core syntax, split upstream-tracking from CPAN-library IDE support planning
- Semantic substrate migration status now tracks Wave 2 reality (facts vocabulary, SymbolDecl adapter, FileFactShard write-through, and DefinitionCandidate multimap landed; SymbolRef adapter and typed-reference global index still staged; ExportInfo -> ExportSet landed) in [SEMANTIC_SUBSTRATE_FIRST_WAVE_PLAN.md](SEMANTIC_SUBSTRATE_FIRST_WAVE_PLAN.md); provider cutover remains explicitly deferred until Wave 3 foundations (`ImportSpec` + `visible_symbols_at`) are proven
- CI/control-plane next-wave execution sequencing is tracked in [CI_WAVE_EXECUTION_PLAN.md](CI_WAVE_EXECUTION_PLAN.md), with #7404 (`update-status --write` streaming) as the top urgency lane.

### Next (post v0.13.2)

- The 0.12.x line has built confidence across parser, diagnostics, refactoring, and distribution
- Resume parser, corpus, semantic, and DAP hardening after the release-channel receipts close
- Run the editor-trust wave through [EDITOR_TRUST_WAVE.md](EDITOR_TRUST_WAVE.md): one lane, one canonical PR, one acceptance checklist, one verification receipt
- Keep the install story verified across all distribution channels
- Keep public-alpha release notes concise and tied to concrete channel receipts

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
Treat these as historical prep slices superseded by the `v0.13.x` public-alpha line.

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
| Latest published release | [RELEASE_HISTORY.md](../../RELEASE_HISTORY.md) records the cross-channel ledger; verify live channel state before citing completion |
| Capability catalog | [`../../features.toml`](../../features.toml) |
| Evidence-backed metrics | [CURRENT_STATUS.md](CURRENT_STATUS.md) |
| Top-level summary docs | [../../ROADMAP.md](../../ROADMAP.md), [../../NOW_NEXT_LATER.md](../../NOW_NEXT_LATER.md) |

<!-- Last Updated: 2026-05-01 -->
