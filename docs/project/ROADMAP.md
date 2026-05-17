# perl-lsp Roadmap

> Canonical planning document.
> Evidence and computed metrics belong in [CURRENT_STATUS.md](CURRENT_STATUS.md).
> Current workspace version is taken from [`../../Cargo.toml`](../../Cargo.toml);
> published release state must be verified against GitHub Releases;
> current capability truth is taken from [`../../features.toml`](../../features.toml).

## Current Framing

- Workspace version line: `v0.14.0`
- Current release train: `v0.14.0` public-alpha patch prep, with release dispatch intentionally pending
- Published crate surface target: 31 crates from `[workspace.metadata.publish.allow]`
- Active work: finish release-prep verification, keep install-surface receipts wired into the runbook, and keep release language public-alpha rather than stable/GA
- Canonical local receipt: `nix develop -c just ci-gate`

Publication discipline: `v0.14.0` uses a normal SemVer package version for release channels while the human-facing product posture remains public alpha. See [RELEASE_HISTORY.md](../../RELEASE_HISTORY.md) for the cross-channel ledger, and do not dispatch the release until the prep checks pass.

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

## Active: Public-Alpha Release Prep (v0.14.0)

The active milestone is a public-alpha patch train, not a stable/GA launch.
Release dispatch remains intentionally pending until the prep checks and channel
receipts prove the train is ready.

### Release-prep exit criteria

- GitHub Release, crates.io, Docker, VS Code Marketplace, Open VSX, and Homebrew tap receipts are captured in the release ledger.
- The owned Homebrew path remains `brew install effortlessmetrics/tap/perllsp`.
- Public install language says public alpha everywhere; avoid stable/GA language until a separate stability-contract milestone lands.
- Release notes stay concise and point to concrete receipts instead of copying generated status tables.
- The canonical local merge receipt remains `nix develop -c just ci-gate`; release operators may add channel-specific smoke receipts in the release issue or release notes.
- Follow-on parser, semantic, DAP, and quality cleanup resumes after the release-channel receipts are closed.

### Workstreams currently in bounds

| Workstream | Next useful slice | Guardrail | Primary reference |
| --- | --- | --- | --- |
| Release channels | Finish `v0.14.0` prep verification and record final channel receipts | Do not dispatch while prep checks are red or missing | [status/release.md](status/release.md), [RELEASE_HISTORY.md](../../RELEASE_HISTORY.md) |
| CI/control plane | Land the seven post-substrate lanes as independent PRs | No bulk stale closure, full merge bot, global pre-push hook, or broad CI rewrite | [CI_WAVE_EXECUTION_PLAN.md](CI_WAVE_EXECUTION_PLAN.md) |
| Editor trust | Keep reliability, conservative answers, recovery, and scorecards ahead of broad new capability | Do not promote flaky or expensive scorecard rows directly to merge-blocking | [EDITOR_TRUST_WAVE.md](EDITOR_TRUST_WAVE.md) |
| Compiler-backed LSP | Continue fact-source-traced proof lanes and provider cutovers with live fallback | No provider behavior change without provenance, shadow receipt, and rollback story | [COMPILER_BACKED_LSP_ROADMAP.md](COMPILER_BACKED_LSP_ROADMAP.md), [status/provider_cutover.md](status/provider_cutover.md) |
| Module resolution / `@INC` | Keep completion, definition, hover, diagnostics, and symbols on one include-path policy | System `@INC` and `PERL5LIB` stay opt-in unless a separate policy changes that | [EDITOR_TRUST_WAVE.md](EDITOR_TRUST_WAVE.md) |

## Now / Next / Later

### Now (v0.14.0 public-alpha patch prep)

- Run release-prep checks before dispatching the `v0.14.0` train; keep the release ledger as the channel truth.
- Keep public-alpha wording consistent across README, release notes, package metadata, marketplace text, and install docs.
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
- Semantic substrate migration status now tracks Wave 2/Wave 3 reality in [SEMANTIC_SUBSTRATE_FIRST_WAVE_PLAN.md](SEMANTIC_SUBSTRATE_FIRST_WAVE_PLAN.md): core semantic facts, HIR-backed `ImportSpec` / `ExportSet`, `visible_symbols_at`, and shadow receipts have fixture evidence; fact-source trace receipts are in place; and provider cutover now has narrow diagnostics, hover, definition, and references live-with-fallback behavior plus shadow/provenance receipts for completion, rename, safe-delete, workspace symbols, document symbols, and semantic tokens.
- The longer compiler-backed LSP direction is tracked in [COMPILER_BACKED_LSP_ROADMAP.md](COMPILER_BACKED_LSP_ROADMAP.md), with lane status in [COMPILER_CAPABILITY_STATUS.md](COMPILER_CAPABILITY_STATUS.md), fact-layer state in [compiler_facts.md](status/compiler_facts.md), and provider staging plus the navigation live quality dashboard in [provider_cutover.md](status/provider_cutover.md).
- Completed compiler-backed proof lanes include import/export [#8264](https://github.com/EffortlessMetrics/perl-lsp/issues/8264), compile-environment state [#8280](https://github.com/EffortlessMetrics/perl-lsp/issues/8280), Exporter adapter registry [#8245](https://github.com/EffortlessMetrics/perl-lsp/issues/8245), compile-effect log [#8291](https://github.com/EffortlessMetrics/perl-lsp/pull/8291), symbolic-ref boundaries [#8297](https://github.com/EffortlessMetrics/perl-lsp/pull/8297), differential oracle proof [#8300](https://github.com/EffortlessMetrics/perl-lsp/pull/8300), provider fact-source trace receipts [#8305](https://github.com/EffortlessMetrics/perl-lsp/pull/8305), diagnostics proof/cutover [#8319](https://github.com/EffortlessMetrics/perl-lsp/issues/8319) / [#8327](https://github.com/EffortlessMetrics/perl-lsp/issues/8327), completion proof [#8342](https://github.com/EffortlessMetrics/perl-lsp/pull/8342), hover proof/provenance [#8344](https://github.com/EffortlessMetrics/perl-lsp/pull/8344) / [#8369](https://github.com/EffortlessMetrics/perl-lsp/issues/8369), definition/reference proof and cutovers [#8349](https://github.com/EffortlessMetrics/perl-lsp/pull/8349) / [#8382](https://github.com/EffortlessMetrics/perl-lsp/issues/8382) / [#8462](https://github.com/EffortlessMetrics/perl-lsp/issues/8462) / [#8803](https://github.com/EffortlessMetrics/perl-lsp/issues/8803) / [#8828](https://github.com/EffortlessMetrics/perl-lsp/issues/8828) / [#8836](https://github.com/EffortlessMetrics/perl-lsp/issues/8836), rename/safe-delete proof [#8351](https://github.com/EffortlessMetrics/perl-lsp/pull/8351), workspace-symbol source/freshness proof [#8353](https://github.com/EffortlessMetrics/perl-lsp/issues/8353), document-symbol source/freshness proof [#8359](https://github.com/EffortlessMetrics/perl-lsp/issues/8359), and semantic-token source/freshness proof [#8360](https://github.com/EffortlessMetrics/perl-lsp/issues/8360). Broader real-Perl conformance expansion remains tracked under [#8199](https://github.com/EffortlessMetrics/perl-lsp/issues/8199).

### Next (post v0.14.0)

- Resume parser, corpus, semantic, and DAP hardening after the release-channel receipts close.
- Run the editor-trust wave through [EDITOR_TRUST_WAVE.md](EDITOR_TRUST_WAVE.md): one lane, one canonical PR, one acceptance checklist, one verification receipt.
- Consolidate editor-facing verification through the UX fixture schema, shared harness, normalized responses, scorecard JSON, dashboard, and ratchet checks before promoting merge-blocking floors.
- Keep the install story verified across all distribution channels, including explicit public-alpha language and the owned Homebrew tap path.
- Keep public-alpha release notes concise and tied to concrete channel receipts.

### Later

- Define the stability contract for public APIs and advertised wire behavior.
- Harden large-workspace performance and memory behavior with receipts, not anecdotes.
- Harden security posture, repository trust, and documentation.
- Continue the path to `v1.0.0` only after public-alpha claims, provider cutovers, release channels, and quality gates have durable evidence.

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

### v0.14.0

Public-alpha minor release train in progress for the Rust 1.95 MSRV line.
RP-1 (readiness queue) is complete and RP-2 (dry-run publish readiness)
is open on `master` readiness tracking.

### Beyond v0.14.0

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

<!-- Last Updated: 2026-05-16 -->
