# perl-lsp Roadmap

> Canonical planning document.
> Evidence and computed metrics belong in [CURRENT_STATUS.md](CURRENT_STATUS.md).
> Current workspace version is taken from [`../../Cargo.toml`](../../Cargo.toml);
> published release state must be verified against GitHub Releases;
> current capability truth is taken from [`../../features.toml`](../../features.toml).

## Current Framing

## Active Swarm Roadmap: Multi-Lane Trust Hardening

This is not a release plan.

`perl-lsp-swarm` is currently in a multi-lane execution phase. The goal is to
review, improve, merge, and stabilize trustworthy work across parallel lanes
without cutting a release, changing public release posture, or moving routine
development back to the release-lineage repository.

Release work is intentionally held. A future release candidate may be selected
after the lanes below produce enough stable, evidence-backed behavior, but no
current lane should treat release, packaging, publishing, marketplace upload,
Homebrew update, signing, or announcement work as in scope unless explicitly
assigned.

### Release Hold

Current state: **no release cut is planned from this roadmap phase.**

During this phase, agents must not:

- bump workspace/package/extension versions for release purposes;
- prepare release notes as if a release is imminent;
- publish crates, containers, extensions, Homebrew formulae, or GitHub releases;
- change signing, release, marketplace, or package-lineage automation;
- claim release readiness from merged swarm PRs alone;
- move routine development back to `perl-lsp`.

Allowed release-adjacent work:

- documenting what would be needed for a future release candidate;
- preserving changelog fragments as unreleased notes;
- keeping install docs accurate if they are already wrong;
- recording release blockers discovered during normal work;
- curated lineage-sync planning only when explicitly assigned.

Done when:

- roadmap readers can distinguish active swarm execution from release work;
- release-lineage work remains parked unless explicitly assigned;
- merged swarm PRs improve future release confidence without implying a release.

### Active Lanes

| Lane | Purpose | Allowed work | Not allowed |
| --- | --- | --- | --- |
| Trust lane | Provider confidence, exact/fallback boundaries, support claims, receipts | Provider-decision receipts, promotion-ledger updates, source/freshness/provenance tests, exact/fallback blocker tests | Broad provider promotion without receipts; generated/dynamic facts as exact |
| Receiver/completion lane | Improve method/package/member completion while preserving confidence tiers | Source-backed receiver facts, fallback receipts, sort-tier checks, package-member boundary hardening | Unbounded data-flow claims; generated accessor promotion without policy |
| Reliability lane | CI, BDD gates, storage/devplane, test organization, docs hygiene | Serialized BDD recipes, flaky-test isolation, bounded CI additions, targeted docs fixes | Bulk stale closure, release automation, broad CI rewrites |
| Substrate lane | Parser, lexer, URI, symbol, position, pragma, oracle/PIR foundations | Property tests, fuzz targets, parser edge-case receipts, deterministic fact substrate | Provider cutover unless trust lane promotes the fact class |
| DAP lane | Debug adapter correctness for public-alpha use | Lifecycle tests, inline values, stack/frame behavior, attach validation, deprecated dispatcher removal proof | Native debugger parity claims; packaging/release work |
| Editor-trust lane | Real editor/user workflow receipts | Raw RPC, Neovim/VS Code-style scenarios, UX fixtures, replayable acceptance receipts | Wall-clock promises without measurement; release claims |
| Release-lineage lane | Future curated sync only | Explicitly assigned sync planning or release-blocker records | Routine feature work; current release cutting |

### Lane Interface Rules

Multiple lanes may be active at the same time. Parallel work is expected.

A PR crosses a lane boundary when it changes files, behavior, claims, or status
owned by another lane. Crossing a boundary is allowed only when the PR explains
the dependency and keeps the foreign-lane change minimal.

Required for every PR:

1. Name the primary lane.
2. Name the claim type:
   - runtime behavior;
   - test/receipt only;
   - docs/status only;
   - CI/control-plane only;
   - refactor only;
   - release-lineage only.
3. State whether provider behavior changes.
4. State whether support-tier, promotion-ledger, parser bucket, semantic-token,
   DAP, packaging, or release claims change.
5. Include replay commands appropriate to the lane.
6. Link or update the relevant status doc only if the PR changes status truth.

When blocked by another lane:

- do not close the other lane's PR;
- do not open a competing broad replacement;
- comment with the dependency;
- split the PR if necessary;
- keep local fixes narrow.

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

## Active: Public-Alpha Channel Closeout (v0.14.0)

- GitHub Release and crates.io surfaces show `v0.14.0` live; Docker, VS Code Marketplace, Open VSX, and Homebrew tap receipts are still tracked separately until verified
- The owned Homebrew path is `brew install effortlessmetrics/tap/perllsp`
- Public install language must say public alpha, not stable/GA
- Follow-on quality cleanup resumes after the remaining release-channel receipts are closed or explicitly deferred

### Release Exit Criteria

The release train is complete only when each criterion has an evidence link in the release closeout or release-runbook issue. Keep the proof in status or release docs; do not paste generated tables here.

| Area | Exit criterion | Evidence source |
| --- | --- | --- |
| Version surface | Workspace package version, `features.toml` metadata, extension packaging, release notes, and changelog all name the same `v0.14.0` train | [`../../Cargo.toml`](../../Cargo.toml), [`../../features.toml`](../../features.toml), [docs/releases/v0.14.0.md](../releases/v0.14.0.md) |
| Publish surface | The 31-crate allowlist has dry-run or publish receipts, and deferred items have successor issues rather than silent drops | [`[workspace.metadata.publish.allow]`](../../Cargo.toml), [docs/releases/v0.14.0.md](../releases/v0.14.0.md) |
| Install channels | GitHub assets, crates.io, Docker, VS Code Marketplace, Open VSX, and Homebrew each have an install/smoke receipt or an explicit pending/deferred state | [status/release.md](status/release.md), [CURRENT_STATUS.md](CURRENT_STATUS.md), [docs/releases/v0.14.0.md](../releases/v0.14.0.md) |
| Local gate | The canonical merge receipt is fresh for the branch being released or the post-release closeout branch | [protocols/verification.md](protocols/verification.md) |
| Public wording | User-facing docs call the release public alpha and avoid stable/GA promises | [docs/releases/v0.14.0.md](../releases/v0.14.0.md), [CURRENT_STATUS.md](CURRENT_STATUS.md) |

### Tracks

### Track A — Queue Stabilization

Goal: make the current swarm output reviewable without stopping useful parallel work.

Allowed:

- merge small, non-draft PRs with narrow scope;
- close only truly superseded duplicates with replacement links;
- batch draft PRs by lane;
- rebase draft PRs after nearby lane work lands;
- add missing proof commands to PR bodies.

Not allowed:

- closing another lane's PR as “cleanup”;
- bulk stale closure;
- merging large draft batches without lane-specific proof;
- treating bot quota-limit comments as review approval.

Exit criteria:

- non-draft queue is small and reviewable;
- draft PRs are grouped by lane;
- every open PR has a primary claim type;
- superseded PRs point to their replacement.

### Track B — Provider Trust Boundaries

Goal: make provider output explainable before making it broader.

Work items:

- package-member completion source-backed boundary;
- receiver exact/fallback classification;
- generated/no-source blocker receipts;
- ambiguous identity blocker receipts;
- provider-decision explanations;
- support-tier and promotion-ledger alignment.

Exit criteria:

- exact answers have source/freshness/provenance receipts;
- fallback answers have blocker receipts;
- generated/dynamic/ambiguous/stale/low-confidence cases are not promoted by accident;
- provider docs match tested behavior.

### Track C — Real Editor Trust

Goal: turn editor-shaped workflows into replayable receipts.

Work items:

- completion UX fixtures;
- navigation workflows;
- type hierarchy and call hierarchy BDD;
- moniker and inlineValue content e2e;
- Neovim/raw RPC latency receipts;
- diagnostics settle/recovery scenarios.

Exit criteria:

- each receipt has a replay command;
- each receipt states expected exact/fallback behavior;
- editor receipts are dashboarded;
- no wall-clock performance promises are made without measurement hardware.

### Track D — Reliability and CI

Goal: make swarm verification stronger without turning CI into a bottleneck.

Work items:

- serialize heavy BDD tests under named recipes;
- isolate slow/flaky tests;
- add property tests with bounded case counts;
- add fuzz targets under bounded/nightly recipes;
- improve storage/devplane hygiene;
- fix docs links with checker receipts.

Exit criteria:

- new tests are assigned to fast, slow, nightly, or manual lanes;
- CI runtime impact is understood;
- large docs/fuzz/property batches have focused proof;
- reliability PRs do not broaden provider behavior.

### Track E — DAP Public-Alpha Hardening

Goal: make DAP behavior predictable without claiming full native-debugger parity.

Work items:

- terminate/disconnect lifecycle body matrix;
- inline-values extraction and formatting;
- stack frame ID/path validation;
- TCP attach validation;
- deprecated dispatcher removal regression net.

Exit criteria:

- production `DebugAdapter` surfaces are covered;
- deprecated/internal behavior is either removed or documented;
- public-alpha DAP docs match tested behavior;
- no release or packaging claims are made.

### Track F — Substrate and Parser Confidence

Goal: improve the fact substrate that providers can eventually trust.

Work items:

- parser edge-case tests for fragile constructs;
- token/span/URI/line-index/position/symbol/pragma property tests;
- bounded fuzz targets;
- oracle-runner and PIR work only when explicitly assigned;
- deterministic receipts before oracle results inform provider promotion.

Exit criteria:

- substrate changes are replayable and deterministic;
- fact classes remain unpromoted until the trust lane promotes them;
- parser confidence improves without inventing release claims.

### Track G — Release-Lineage Parking Lot

Goal: record future release/sync requirements without doing release work now.

Allowed:

- list future sync candidates;
- record release blockers discovered by active lanes;
- keep unreleased changelog fragments;
- document future release-candidate selection criteria.

Not allowed:

- cutting a release;
- version bumps for release;
- publishing;
- marketplace/Homebrew/Docker work;
- signing changes;
- source-over-swarm routine development.

Exit criteria:

- future release candidates can be selected from stable merged swarm work;
- no active PR accidentally performs release work;
- release-lineage sync remains explicit and assigned.

### Lane Capacity Guidance

| Lane | Soft cap | Reason |
| --- | ---: | --- |
| Trust | 2 | Provider/support claims require deep review |
| Receiver/completion | 2 | Easy to over-promote dynamic behavior |
| Substrate | 2 | Fact-layer changes can affect many providers later |
| Reliability | 4 | Usually lower behavioral risk, but can create CI drag |
| DAP | 2 | Protocol behavior needs careful production-surface checks |
| Editor trust | 2 | Receipts are valuable but can become flaky |
| Release-lineage | 0 unless assigned | Release is intentionally held |

Exceeding a cap is not forbidden, but PRs should explain why the extra
parallelism is safe.

### Release-Hold Checklist for Active PRs

Every active PR should answer:

- Does this cut, prepare, publish, or announce a release?
- Does this change version numbers?
- Does this change package/signing/marketplace/Homebrew/Docker behavior?
- Does this move work into `perl-lsp` rather than `perl-lsp-swarm`?
- Does this make user-facing release claims?

Expected answer during this roadmap phase: **no**, unless the maintainer
explicitly assigned release-lineage work.

### Roadmap Anti-Goals

The following are explicitly not progress in this phase:

- cutting or preparing a release;
- version bumping for release;
- publishing crates, images, editor extensions, or Homebrew formulae;
- moving routine feature development back to `perl-lsp`;
- broadening provider behavior from receipt-only PRs;
- treating generated, dynamic, stale, ambiguous, or low-confidence facts as exact;
- closing other lanes' PRs as queue burn-down;
- adding tests that are not assigned to a durable verification lane;
- broad CI rewrites;
- release notes that imply shipped support before a release candidate is chosen.

## Future Release Candidate Selection Criteria

This section is parking-lot guidance only. It does not authorize a release.

A future release candidate can be considered only after:

1. The active non-draft swarm queue is small and reviewed.
2. Trust-boundary PRs are merged and reflected in provider status docs.
3. Editor receipts cover the main user workflows intended for the release.
4. DAP public-alpha claims are matched to production-surface tests.
5. CI reliability changes are stable for multiple mainline runs.
6. Release-lineage sync candidates are listed and scoped.
7. Public-alpha wording remains accurate.

Selecting a release candidate requires a separate maintainer decision and a
dedicated release-lineage plan.

### Compact Track Table

| Track | Active goal | Merge bias | Hold / reject |
| --- | --- | --- | --- |
| Queue stabilization | Make parallel work reviewable | Small non-draft PRs, supersession cleanup, lane labels | Bulk closure, cross-lane interference |
| Provider trust | Exact/fallback/provider-decision clarity | Source-backed receipts, blocker receipts | Unproven support promotion |
| Receiver/completion | Better completion with confidence tiers | Static/source-backed receiver proof | Dynamic/generated exactness claims |
| Editor trust | Replayable editor workflows | BDD/UX/raw RPC receipts | Wall-clock promises without measurement |
| Reliability/CI | Stronger gates, less flake | Serialized recipes, bounded tests | Broad CI rewrites, test flood |
| DAP | Predictable public-alpha debug behavior | Production-surface protocol tests | Native parity claims |
| Substrate/parser | Better facts and invariants | Property/fuzz/parser receipts | Provider cutover without trust-lane promotion |
| Release-lineage | Park future sync requirements | Blocker notes, future candidate criteria | Any release cut or publish work |

## Now / Next / Later

### Now (v0.14.0 public-alpha channel closeout)

- Reconcile the live `v0.14.0` GitHub Release and crates.io surfaces with release notes, release history, generated status, and remaining channel receipts.
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
- `v0.14.0` is the current public-alpha release line; finish receipt closeout before treating the release as fully closed
- Pre-announcement license badge fix (PR #3193): canonical SPDX text in all 126 LICENSE files
- Pre-announcement Docker arm64 timeout fix (#3188 → PR #3191, merged)
- Per-release dependency triage: 7 dependabot PRs merged 2026-04-07 (#3178–#3184)
- Code quality cleanup: debug prints (only `crates/perl-corpus/src/bin/main.rs` CLI output remains, library code clean), unused deps, remaining `unwrap()`/`expect()` audit in production code
- Test coverage gaps and broken integration tests
- VSCode extension lint/quality audit (eslint v10 landed in #3179)
- AI inline completion (#3018) shipped in the live 0.12.x line — feature wired end-to-end via #3157–#3168, awaiting E2E user validation
- Workspace-wide rename slice: multi-root support shipped in 0.12.x (#3984); workspace-wide rename/module-move remains roughly 30% complete and only conditionally in scope pending #3522 verification
- Coroutine support issue #3539 is re-scoped: defer hypothetical core syntax, split upstream-tracking from CPAN-library IDE support planning
- Semantic substrate migration status now tracks Wave 2/Wave 3 reality in [SEMANTIC_SUBSTRATE_FIRST_WAVE_PLAN.md](SEMANTIC_SUBSTRATE_FIRST_WAVE_PLAN.md): core semantic facts, HIR-backed `ImportSpec` / `ExportSet`, `visible_symbols_at`, and shadow receipts have fixture evidence; fact-source trace receipts are in place; and provider cutover now has narrow diagnostics, hover, definition, and references live-with-fallback behavior plus shadow/provenance receipts for completion, rename, safe-delete, workspace symbols, document symbols, and semantic tokens. The longer compiler-backed LSP direction is tracked in [COMPILER_BACKED_LSP_ROADMAP.md](COMPILER_BACKED_LSP_ROADMAP.md), with lane status in [COMPILER_CAPABILITY_STATUS.md](COMPILER_CAPABILITY_STATUS.md), fact-layer state in [compiler_facts.md](status/compiler_facts.md), and provider staging plus the navigation live quality dashboard in [provider_cutover.md](status/provider_cutover.md). The import/export proof lane [#8264](https://github.com/EffortlessMetrics/perl-lsp/issues/8264), compile-environment state lane [#8280](https://github.com/EffortlessMetrics/perl-lsp/issues/8280), Exporter adapter registry lane [#8245](https://github.com/EffortlessMetrics/perl-lsp/issues/8245), first compile-effect log slice [#8291](https://github.com/EffortlessMetrics/perl-lsp/pull/8291), symbolic-ref boundary slice [#8297](https://github.com/EffortlessMetrics/perl-lsp/pull/8297), differential oracle proof [#8300](https://github.com/EffortlessMetrics/perl-lsp/pull/8300), provider fact-source trace receipts [#8305](https://github.com/EffortlessMetrics/perl-lsp/pull/8305), diagnostics proof/cutover [#8319](https://github.com/EffortlessMetrics/perl-lsp/issues/8319) / [#8327](https://github.com/EffortlessMetrics/perl-lsp/issues/8327), completion proof [#8342](https://github.com/EffortlessMetrics/perl-lsp/pull/8342), hover proof [#8344](https://github.com/EffortlessMetrics/perl-lsp/pull/8344), hover live provenance slice [#8369](https://github.com/EffortlessMetrics/perl-lsp/issues/8369), definition/reference proof and runtime receipts [#8349](https://github.com/EffortlessMetrics/perl-lsp/pull/8349) / [#8382](https://github.com/EffortlessMetrics/perl-lsp/issues/8382) / [#8462](https://github.com/EffortlessMetrics/perl-lsp/issues/8462), definition live cutover [#8803](https://github.com/EffortlessMetrics/perl-lsp/issues/8803), references live cutovers [#8828](https://github.com/EffortlessMetrics/perl-lsp/issues/8828) / [#8836](https://github.com/EffortlessMetrics/perl-lsp/issues/8836), rename/safe-delete proof [#8351](https://github.com/EffortlessMetrics/perl-lsp/pull/8351), workspace-symbol source/freshness proof [#8353](https://github.com/EffortlessMetrics/perl-lsp/issues/8353), document-symbol source/freshness proof [#8359](https://github.com/EffortlessMetrics/perl-lsp/issues/8359), and semantic-token source/freshness proof [#8360](https://github.com/EffortlessMetrics/perl-lsp/issues/8360) are complete; broader real-Perl conformance expansion remains tracked under [#8199](https://github.com/EffortlessMetrics/perl-lsp/issues/8199).
- Receiver expression facts are planned in [PLSP-SPEC-0005](../specs/PLSP-SPEC-0005-receiver-expression-facts.md) and sequenced in [RECEIVER_FACTS_IMPLEMENTATION_PLAN.md](RECEIVER_FACTS_IMPLEMENTATION_PLAN.md); the lane must land semantic facts and receipts before provider-visible receiver completion claims.
- CI/control-plane next-wave execution sequencing is tracked in [CI_WAVE_EXECUTION_PLAN.md](CI_WAVE_EXECUTION_PLAN.md), with #7404 (`update-status --write` streaming) as the top urgency lane.

### Next (post v0.14.0 closeout)

- The 0.13.x line has built confidence across parser, diagnostics, refactoring, and distribution
- Resume parser, corpus, semantic, and DAP hardening after the release-channel receipts close
- Run the editor-trust wave through [EDITOR_TRUST_WAVE.md](EDITOR_TRUST_WAVE.md): one lane, one canonical PR, one acceptance checklist, one verification receipt
- Keep the install story verified across all distribution channels
- Keep public-alpha release notes concise and tied to concrete channel receipts

#### Post-Release Sequencing

1. **Close release receipts first.** Do not start broad feature cleanup until the v0.14.0 channel ledger is explicit about what shipped, what is pending, and what users should install.
2. **Stabilize the control plane.** Land the CI wave in narrow, reviewable PRs so follow-on parser/provider work can trust queue state and status receipts.
3. **Promote compiler-backed provider slices.** Prefer source/freshness/provenance proof and live-with-fallback cutovers over blanket rewrites. Retire legacy heuristics only after the dashboard shows reliable real-workspace behavior.
4. **Expand real-Perl acceptance.** Add corpus and editor-trust receipts for workflows that users actually exercise: navigation across generated exports, import-heavy modules, refactoring previews, diagnostics, and DAP launch/attach paths.
5. **Burn down tracked debt by ledger.** Use successor issues from [docs/releases/v0.14.0.md](../releases/v0.14.0.md) for PerlOracleEnv seams, clippy suppressions, coverage claim boundaries, file-policy wiring, and DAP runtime module breakpoints.

#### Later Themes

- **API and wire-behavior stability:** document which facade APIs and LSP responses are compatibility commitments before `v1.0.0`.
- **Large-workspace performance:** keep indexing, completion, and provider latency measured on realistic file and symbol counts before expanding advertised performance claims.
- **Security and supply chain:** tighten subprocess environment seams, publish/install verification, SBOM/signature posture, and dependency freshness policy.
- **Distribution maturity:** make Homebrew, Docker, crates.io, VS Code Marketplace, Open VSX, and GitHub Releases behave like one coherent public-alpha install story.

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
| text_document | 53 | 53 | 100% |
| window | 9 | 9 | 100% |
| workspace | 28 | 28 | 100% |
| **Overall** | **125** | **125** | **100%** |
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

<!-- Last Updated: 2026-05-19 -->
