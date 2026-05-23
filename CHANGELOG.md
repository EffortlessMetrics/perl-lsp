# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- 0.15.1 Neovim latency lane:
  - `--runtime-mode e2e` / `PERL_LSP_E2E=1` for latency-focused editor
    harnesses. Defaults: zero diagnostic debounce, syntax-only
    diagnostics, no eager workspace indexing, no file watchers.
  - `--diagnostic-mode syntax-only` /
    `PERL_LSP_DIAGNOSTIC_MODE=syntax-only` restricts diagnostics to
    parse errors only; skips semantic / native critic / external
    perlcritic / module-resolution / workspace dead-code passes. Both
    push and pull diagnostic paths honour the gate.
  - `--diagnostic-debounce-ms <ms>` /
    `PERL_LSP_DIAGNOSTIC_DEBOUNCE_MS=<ms>` configurable diagnostic
    publish debounce window. `0` bypasses the debouncer entirely.
  - E2E startup gate: `initialized` no longer kicks off eager workspace
    indexing under e2e mode (override with `--eager-workspace-indexing
    true`).
  - Generation-aware stale read cancellation in the scheduler. Hover,
    completion, definition, declaration, typeDefinition, implementation,
    and references are cancelled with `RequestCancelled` when the
    document generation advances between ingress and dispatch. Turns
    typing storms into "latest request wins" instead of "every cursor
    position gets its own work item."
  - Raw-RPC latency receipts in `perl-lsp-ux-tests::ux_latency_raw_rpc`
    and a Neovim lean smoke script at
    `scripts/ux/neovim_lean_smoke.sh`.
- Added the conservative `perl.explainProviderDecision` LSP execute-command
  surface. It returns the structured provider decision explanation payload and
  reports a low-confidence `missing_fact` / `no_result` fallback when no
  provider-specific receipt is attached, avoiding false certainty while the
  live provider receipt wiring lands.

### Notes (0.15.1)

- This release does not implement true incremental AST reuse. The live
  LSP path still full-parses after text changes; latency improvements
  come from skipping avoidable background work and cancelling stale
  reads earlier.
- For latency testing, use a release binary with
  `--runtime-mode e2e --diagnostic-mode syntax-only
  --diagnostic-debounce-ms 0`, disable file watchers, and disable
  semantic tokens at the client unless you are explicitly testing
  semantic highlighting.

### Planned

- Documented the Rust 1.95 / 0.14.0 rollout sequence before implementation: compatibility spike first, then MSRV/toolchain, lint, no-panic, file-policy, CI routing, and release-prep lanes.
- Added the proactive CI integrity guards rail ([`docs/development/RUST_1_95_PROACTIVE_GUARDS.md`](docs/development/RUST_1_95_PROACTIVE_GUARDS.md)) as a sibling rollout. Six guard PRs (PG-1 through PG-6) covering label enforcement, risk-pack referential integrity, lane mapping with matrix expansion, net-new workflow-allowlist ledger, CI Actuals emitter + subscription coverage check, and broad-glob justification tightening. Each row mirrors a sibling-repo proven shape.
- Consolidated the remaining Rust 1.95 → 0.14.0 work into a single canonical roadmap: rewrote [`docs/development/RUST_1_95_ROLLOUT.md`](docs/development/RUST_1_95_ROLLOUT.md) into a post-landing source of truth (already landed / remaining implementation ladder / per-rail acceptance contracts / Claude-Codex operating contract); slimmed [`docs/ci/perl-lsp-rust-1.95-rollout.md`](docs/ci/perl-lsp-rust-1.95-rollout.md) to a historical pointer; added [`docs/ci/test-evidence-lanes.md`](docs/ci/test-evidence-lanes.md) defining the five evidence-lane shapes (PR-fast required / PR-targeted / nightly cron / release-only / advisory) with risk-pack auto-routing, skipped-by-policy receipts, and LEM cost framing. Umbrella tracking: **#8663**.

## [0.15.0] - 2026-05-22

Release notes: [v0.15.0](docs/releases/v0.15.0.md)

Minor release focused on JSON-RPC type safety and fixing the LSP4IJ
file-watcher registration crash. Breaking change in the public
`perl-lsp-rs-core::protocol` API (request/response ID field type) lifts
this to a minor version under 0.x semver.

### Fixed

- **LSP4IJ file-watcher registration crash** - Server no longer emits
  wall-clock millisecond IDs for `client/registerCapability`
  (~1.7e12 overflows i32 in strict clients including LSP4IJ). All
  server-to-client requests now route through a bounded `AtomicI32`
  allocator that emits values in `1..=i32::MAX` and wraps cleanly.
  This unblocks JetBrains users on the LSP4IJ plugin. (#221, #224)

### Added

- **Typed JSON-RPC request IDs** - `JsonRpcId` (strict-shape enum:
  integer | string; rejects null/fractional/object/array at the serde
  boundary) and `ServerRequestId` (positive-i32 newtype with no
  out-of-range constructor) added to `perl-lsp-rs-core::protocol`.
  The type system now makes the file-watcher crash structurally
  impossible to reintroduce. (#221, #224)
- **Strict inbound ID validation** - Invalid request-ID shapes
  (null, fractional, object, array) are rejected at the transport
  boundary instead of producing undefined behavior deep in the
  dispatcher. (#221)
- **LSP4IJ regression test** - File-watcher registration request ID
  asserted to be a bounded integer in `1..=i32::MAX`. Source-guard
  tests pin the fix against `lifecycle/watchers.rs` re-introducing
  wall-clock-derived IDs. (#221)

### Changed

- **BREAKING:** `JsonRpcRequest.id` and `JsonRpcResponse.id` are now
  `Option<JsonRpcId>` instead of `Option<serde_json::Value>`.
  Consumers of the published `perl-lsp-rs-core::protocol` crate must
  use `JsonRpcId::Integer(N)` / `JsonRpcId::String(...)` in tests and
  any external construction. `Value` round-trips via `to_value()` /
  `from_value()`. (#221)
- **BREAKING:** `outbound::OutboundSender::send_request` now takes
  `ServerRequestId` instead of raw `i64`. (#221)
- **Cancellation registry typed end-to-end** - `CancellationRegistry`
  tokens, cleanup contexts, and cache are keyed by `JsonRpcId`
  instead of `format!("{:?}", value)` strings.
  `PerlLspCancellationToken.request_id`,
  `RequestCleanupGuard.request_id`, `cancel_mark` / `is_cancelled` /
  `register_progress_request`, and the runtime `cancelled` /
  `progress_token_to_request` collections all move from `Value` to
  `JsonRpcId`. Integer and string IDs with the same textual form
  (e.g. `7` vs `"7"`) are now independently cancellable. (#223, #224)
- **`pending_workspace_configuration_requests`** is now keyed by
  `ServerRequestId` rather than raw `i64`. (#221)

### Looking ahead

- LSP interactive latency rollout rail at
  [`docs/development/LSP_INTERACTIVE_LATENCY_ROLLOUT.md`](docs/development/LSP_INTERACTIVE_LATENCY_ROLLOUT.md).
  Workload-profile and stale-work-cancellation work that benefits
  Neovim and LSP4IJ equally; targets 0.15.1. Umbrella tracking: **#229**.

## [0.14.0] - 2026-05-12

Release notes: [v0.14.0](docs/releases/v0.14.0.md)

### Added

- **Rust 1.95 MSRV** — Minimum supported Rust version raised to 1.95.
  Consumers must use `rustup update stable` (Rust 1.95+ ships stable).
- **Runtime-owned TTL completion cache** — Prefix module scan results are
  now cached with a bounded TTL, eliminating redundant lookups across
  successive completion requests. (#8514 → PR #8667)
- **Literal `require`/`use` symbol tracking** — Symbols from `require
  'Module.pm'` and explicit `use Module` imports are now tracked and offered
  in completions. (#8623 → PR #8678)
- **Real-workspace provider baseline** — Integration tests against a real
  Perl project fixture give confidence that LSP providers work beyond
  synthetic test data. (#8637 → PRs #8682, #8694)
- **DAP module-resolution smoke tests** — Catches regressions in debug
  adapter module loading before they ship. (#8621 → PR #8677)
- **PerlOracleEnv v1 subprocess contract** — Replaces ambient
  `$ENV{PERL5LIB}` injection with an explicit typed contract; subprocess
  environment is now auditable. (#8622 → PRs #8675, #8679)
- **Non-Rust file policy advisory checker** — `cargo xtask check-file-policy
  --mode advisory` documents non-Rust file ownership with an enforced
  allowlist, removing a class of accidental scope drift. (#8566 → PR #8708;
  #8568 → PR #8711)
- **PR sticky CI summary and `ci-doctor`** — `cargo xtask ci-doctor` gives
  clear in-PR feedback without reading raw CI logs. (#4825 → PR #8697;
  #4826 → PR #8693)
- **PR title validation** — `cargo xtask pr title-check` catches malformed
  PR titles before CI. (#8614 → PR #8700)
- **Freshness-check xtask** — Prevents stale-binary false passes locally.
  (#8619 → PR #8683)

### Changed

- **Clippy temporary-allow burndown** — 4 of 5 workspace-level
  `#[allow(clippy::...)]` suppression annotations removed. Only
  `collapsible_match` remains, tracked in #8561. (PR #8712)
- **Clippy lint policy MSRV reconcile** — Clippy lint set aligned with
  actual Rust 1.95 availability. (PR #8707)

### Infrastructure

- All deferred items have verified-open successor issues — nothing dropped,
  everything tracked. See `docs/releases/0.14.0-readiness.md` for the full
  queue.

## [0.13.4] - 2026-05-07

Release notes: [v0.13.4](docs/releases/v0.13.4.md)

### Fixed

- **Known session-creep leaks across LSP caches** — Hover, text-sync,
  workspace, stream-session, and workspace-index caches were retaining
  per-session state past document close/delete lifecycle paths. Eviction is
  now wired through, and retained-state regression tests lock the behavior.
  (#8064)
- **Stream sessions cancel across URI variants** — Stale inline-completion
  stream sessions on `didChange` are cancelled even when the client mixes
  canonical `file://` and `file://localhost` spellings for the same
  document. Regression covers both spellings.
- **Regex embedded code annotated, not rejected** — The parser now
  annotates embedded code inside regex constructs instead of failing the
  parse. (#8056)
- **Status pipeline regeneration** — Parser-accuracy artifact is now
  bootstrapped before regeneration (#8069) and quality counts are parsed
  from external target dirs (#8068), so `docs/project/status/*.md`
  reflects reality after out-of-tree CI runs.
- **Stricter VS Code extension lint** — Cleared lint failures from the
  upgraded TypeScript and `@types/vscode` toolchain. (#8065)

### Added

- **Class::Tiny and Class::Tiny::RW OO framework support** — Full
  semantic analysis for the Class::Tiny family across both the
  `ClassModelBuilder` and `SymbolExtractor` pipelines. `use Class::Tiny
  qw(name email)` and bare `has 'name';` declarations now produce
  accessor symbols so go-to-definition, hover, and workspace symbol
  search work for Class::Tiny accessors. (#8062)
- **LSP churn plateau guardrails** — New `memory_plateau.json` receipt,
  nightly + PR CI gates, `scripts/repro_lsp_storm.py` reproducer, and
  `scripts/assert_rss_plateau.py` plateau assertion. Documented in
  `docs/large-workspaces/LSP_CHURN_REPRO.md` and a new
  `RETAINED_STATE_INVENTORY.md` cataloguing every retained cache, its
  owner, and its eviction rule. Runtime pressure counters expose async
  task/debounce/session pressure, and diagnostics churn now has direct
  retained-state coverage. (#8072, #8076, #8088, #8115)
- **Memory-control closeout** — Long-session retained-state memory behavior
  is now covered by lifecycle cleanup, runtime pressure counters, plateau
  receipts, trend rendering, focused subsystem regressions, and
  retained-state inventory policy. This closes the known retained-state /
  session-creep class and adds guardrails against recurrence, without
  claiming every possible memory issue is fixed. See
  [MEMORY_CONTROL_CLOSEOUT.md](docs/large-workspaces/MEMORY_CONTROL_CLOSEOUT.md).
- **Governed clippy lint policy gate** — New CI gate enforcing the
  `policy/clippy-lints.toml` allowlist. (#8066)
- **Parser coverage risk map and baseline.** (#8005)

### Changed

- **MSRV bumped to Rust 1.93.1** — Toolchain pins, CI matrix, clippy
  policy, `clippy.toml`, and `rust-toolchain.toml` aligned. (#7832)
- **Decoupled `perl-semantic-analyzer` from `perl-workspace`** —
  Removed direct coupling; analyzer no longer reaches into workspace
  internals. (#7962)
- **Internal refactor wave (non-user-visible)** — Hover receiver
  package resolver extracted (#8045); execute-command test-runner
  fallback split into helpers (#8046); completion provider
  construction helpers extracted (#8044); call-hierarchy subroutine
  item builder extracted (#8043); refactor-plan contract skeleton
  added to `perl-refactoring` (#7983).
- **Centralized VS Marketplace install badge count.** (#8049)
- **Dependency bumps** — `actions/upload-artifact` 4 → 7 (#7914),
  `actions/checkout` 4 → 6 (#7915), `actions/cache` 4 → 5 (#7916),
  `@types/vscode` (#7912), TypeScript group (#7911).
- Prepared the `v0.13.4` public-alpha patch train with workspace, crate,
  feature catalog, and VS Code extension version surfaces aligned.

## [0.13.3] - 2026-05-03

Release notes: [v0.13.3](docs/releases/v0.13.3.md)

### Fixed

- **VS Code managed binary install reliability** — Reinstall now installs
  into a versioned subdirectory and atomically updates a `current` pointer,
  so a forced reinstall while the previous `perllsp.exe` is held by a
  running process lands in a fresh sibling directory instead of failing
  with `EBUSY`.
- **Lifecycle-safe `Perl: Reinstall Server Binary`** — The command stops a
  running language client before installing, restarts with the newly
  installed binary on success, and falls back to the previous binary on
  download or health-check failure so a failed reinstall never leaves the
  user worse off than before.
- **Extended retry budget for transient managed-install file locks** —
  Total retry wait grows from ~4s to ~31s, covering the upper end of
  Windows Defender first-time signature scans on a fresh release artifact.
- **Singleflight managed install** — Activation auto-download, manual
  Reinstall, and the silent update check coalesce so two installs cannot
  race the same destination path.

### Changed

- Strengthened source and published VS Code smokes to reinstall twice
  across Windows, macOS, and Linux, with the binary held by a spawned
  process during the second pass. Smokes upload artifacts under
  `target/receipts/vscode-smoke/<source>/<os>/` on every run.
- Prepared the `v0.13.3` public-alpha patch train with workspace, crate,
  feature catalog, and VS Code extension version surfaces aligned.

## [0.13.2] - 2026-05-02

Release notes: [v0.13.2](docs/releases/v0.13.2.md)

### Changed

- Prepared the `v0.13.2` public-alpha patch train with workspace, crate,
  feature catalog, and VS Code extension version surfaces aligned.
- Made release closeout focus on the real user install surfaces: Homebrew tap,
  GitHub release assets, VS Code Marketplace, and Open VSX.
- Aligned parser scorecard truth semantics for clean-ingestion, salvage, and
  insufficient-data rows.

### Fixed

- Locked Homebrew tap, GNU/musl binary selection, installer target selection,
  and VS Code managed-binary startup paths behind release hygiene checks.
- Added release-note chooser and install-surface checks to keep future
  public-alpha release notes from drifting back to stale install guidance.

## [0.13.1] - 2026-05-01

Release notes: [v0.13.1](docs/releases/v0.13.1.md)

### Changed

- Hardened public-alpha release channels after the `v0.13.0` launch.
- Decoupled Open VSX publishing from VS Code Marketplace publishing.
- Clarified release naming: package versions use normal SemVer while product
  posture remains public alpha.
- Improved CI Gate timeout headroom and diagnostics for release runs.
- Corrected Homebrew/tap naming and formula generation around the `perllsp`
  binary.

## [0.13.0-rc1] - 2026-04-30

Release notes: [v0.13.0-rc1](docs/releases/v0.13.0-rc1.md)

### Fixed

- **CI cancellation cascade fix — label events no longer cancel active runs** —
  `cancel-in-progress` now scopes to `pull_request.synchronize` only, so applying
  labels (e.g. `merge-ready`, `ci-green`) does not abort an in-flight CI run on
  the same PR. Adds a `LABEL_EVENT_CANCELS_PR_RUN` xtask lint to prevent the
  failure mode from re-entering. Resolves a recurring queue-blocker that
  surfaced as `exit 143` SIGTERM aborts on PR Smoke and CI Gate. (#7581)

- **First-run error messaging now surfaces to users instead of silent logging** — When
  workspace root is not detected (e.g., opening a single file without opening a folder),
  perl-lsp now sends a `window/showMessage` notification with actionable guidance:
  "perl-lsp: workspace root not detected — module resolution disabled. To enable: open
  the project folder in your editor (File > Open Folder) rather than individual files.
  This warning appears once per server session." Previously this was logged to the server
  log only and users saw nothing. The warning flag is stored as an `Arc<AtomicBool>` on
  `LspServer`, so each server session shows the warning independently — in multi-root or
  multi-server workspace configurations, each `LspServer` instance tracks its own shown
  state rather than sharing a process-level `Once`. (#4178)

### Migration

- **Microcrate collapse complete — migration guide available** — v0.13.0 drops the
  published crate count from 132 to 32 across 10+ collapse waves. All ~100 retired
  crate names stop appearing on crates.io after this release; their code lives as
  subfolder modules inside the owning published crate. See
  [`docs/MIGRATION_v0.13.md`](docs/MIGRATION_v0.13.md) for the complete
  old-path → new-path mapping for every retired crate, feature flag changes
  (`lsp-ga-lock`, `incremental`, `workspace_refactor`), and the breaking-changes
  summary per wave. (#7292, #4410)

### Internal

- **Release prep: start `v0.13.0-rc1` version staging** — bumped workspace and internal crate dependency versions to `0.13.0-rc1`, updated the feature catalog metadata version, and refreshed the top-level README release line for release-candidate signaling. (#0000)

- **`cargo xtask published-crate-count`** — new ratchet gate that monitors the
  count of entries in `[workspace.metadata.publish.allow]` and prevents accidental
  regression during the microcrate collapse (ADR-0041). Fails if the count exceeds
  the baseline in `xtask/published-crate-baseline.txt`; auto-tightens the baseline
  when count decreases. Run via `just ci-published-crate-count` or directly as
  `cargo xtask published-crate-count`. (#4416)

## [0.12.4] - 2026-04-12

Release notes: [v0.12.4](docs/releases/v0.12.4.md) · [GitHub Release](https://github.com/EffortlessMetrics/perl-lsp/releases/tag/v0.12.4)

<!-- 2026-04-11 session: 46 PRs merged across navigation, pragma scoping, incremental parsing, workspace refactoring, Windows hardening, and CI hygiene -->
<!-- 2026-04-12 session: ~25 PRs merged — DAP scorecard, rename perf, editor UX, diagnostics polish, hover improvements, workspace/config, completion ranking, Windows compat, CI hygiene -->

### Headlines

- **Inherited and role method navigation now works end-to-end** — `goto-definition`,
  hover, and workspace completion all BFS through Moo/Moose `with 'Role'` and
  `extends`/`use parent` chains. Previously only direct-method navigation worked;
  inherited methods from parent classes or composed roles returned nothing.
  AUTOLOAD-backed method calls also resolve through the fallback path, and hover
  surfaces the AUTOLOAD resolution. (#4077, #4091)

- **Pragma tracker correctness sweep** — four independent false-negative paths
  in `PragmaTracker` / `check_strict_warnings` fixed in one wave:
  - eval- and sub-scoped `use strict`/`use warnings` no longer suppress
    file-level PL100/PL101 diagnostics; `state_for_offset` is now the single
    source of truth for top-level pragma state (#4052)
  - conditional `use if` / `use unless` pragmas are tracked via suffix
    matching on the flattened argument list (#4050)
  - explicit `use feature` / `no feature` state (including `qw(...)` and
    `:X.Y` version bundles) drives lint decisions about `switch` and other
    feature-gated constructs (#4038)
  - phase-block (`BEGIN`/`END`/`INIT`/`UNITCHECK`/`CHECK`) pragmas are kept
    lexically scoped instead of leaking to the enclosing file; the bad
    `strict_warnings` phase-block override that suppressed PL100/PL101 is
    gone (#4108)

- **Incremental parsing: segment-based token cache + two-sided checkpoint
  window** — replaces the monolithic `TokenCache` with sorted segment storage
  so edits only invalidate overlapping segments, and `CheckpointCache::find_after()`
  bounds the re-lex region by the nearest left and right checkpoints instead of
  a fixed +100-byte heuristic. New `IncrementalStats` counters surface
  segment reuse, invalidation, and relex distance. 23 new correctness tests
  plus a 13-group Criterion benchmark suite. A follow-up correctness fix
  forces a full reparse when a nonzero checkpoint window has no cached prefix
  tokens, preventing a suffix-only token stream from being fed to the parser.
  (#4029, #4048, #4076)

- **Workspace-wide file-operations refactoring** — `workspace/willRenameFiles`
  now plans edits across unopened files by reading from the open-doc cache,
  the workspace index, and finally the filesystem; `workspace/willDeleteFiles`
  emits a user-visible `Warning` when deletion would break cross-file
  references discovered via `index.file_symbols()` + `find_references()`.
  Multi-file rename batches are merged per-URI via a new `append_workspace_edits`
  helper. (#4056, #4098)

- **`workspace/configuration` reverse-request flow** — the server now parses
  the client's `workspace.configuration` capability and, when advertised,
  issues a `workspace/configuration` reverse request per folder after
  `.perl-lsp.toml` is loaded, merging returned overlays into each folder's
  `effective_workspace_config` (TOML stays the base layer). Re-fetched on
  `workspace/didChangeConfiguration`. JSON-RPC responses without `method`
  are routed as an internal `$/perl-lsp/clientResponse` pseudo-notification
  through the existing dispatch system. Non-`file://` workspace folder URIs
  (`vscode-remote://`, `untitled:`, etc.) are tolerated end-to-end —
  `to_file_path()` calls are routed through a file-only URI helper and
  non-filesystem folders are skipped during indexing scans. (#4093, #4059)

- **Windows extended-length path fix for external commands** — `Path::canonicalize`
  on Windows returns paths with the `\\?\` prefix, which Win32 APIs accept but
  `perl.exe` / `prove` / `yath` do not. `normalize_path_for_external_command`
  strips the prefix before every spawn site in `execute_command::provider`.
  Unblocks Run Tests, Run File, and Run Test Sub on Windows. (#4089)

- **Perl::Critic diagnostics UX overhaul** — Critic policy codes now carry
  `source: "perlcritic"` in the LSP surface and an explicit `data.fixable`
  list; three more policy aliases route to existing quick-fixes
  (`RequireUseStrict`, `RequireUseWarnings`, `ProhibitUnusedVariables`);
  missing `perlcritic` binary and invalid profiles surface as workspace
  warnings and health-check output instead of silently skipping. (#4113)

### Added

- **Workspace `workspace/configuration` reverse-request flow** with client
  capability gating, per-folder scoping, and JSON-RPC response routing via
  `$/perl-lsp/clientResponse` (#4093).

- **`workspace/willRenameFiles` workspace-wide planning** — reads text from
  open-doc cache, workspace index, or filesystem, and merges multi-URI edits
  via a new `append_workspace_edits` helper (#4056).

- **`workspace/willDeleteFiles` safe-delete warnings** — emits `Warning`
  severity diagnostic when the delete would break cross-file references
  discovered via `index.file_symbols()` + `find_references()` (#4056).

- **`HoverExtracted::InheritedMethod` variant** with Phase 2 workspace BFS
  over parents and roles; `collect_all_package_members` BFS for workspace
  completion; `workspace_document_text` exposed as `pub(super)` so hover can
  reuse it (#4077).

- **AUTOLOAD fallback in goto-definition and hover** — explicit method-call
  navigation resolves through `AUTOLOAD` when the named method is absent;
  inherited-method hover surfaces the AUTOLOAD resolution (#4091).

- **`Readonly` and `Const::Fast` wrapper declarations** are now surfaced as
  constant symbols by `perl-symbol-surface` and `perl-semantic-analyzer`;
  declaration tokens are marked readonly in semantic-tokens output, including
  package-qualified `our` constants. Scalar, array, hash, and package
  regression coverage added. (#4040, #4043)

- **LSP semantic token delta support advertised** — `semanticTokens.full`
  switched from the legacy boolean form to the structured delta form.
  The delta handler already existed; the advertised capability is now
  aligned with LSP 3.16+ expectations. Capability-snapshot tests
  regenerated. (#4026, #4041, #4042)

- **VS Code: Run Test at Cursor** — new `perl-lsp.runTestAtCursor` command
  palette / context-menu entry that resolves the active cursor position
  against existing code lenses and runs the nearest test subroutine,
  subtest, or file-level run lens (#4025).

- **VS Code: Gherkin step-definition navigation and stubs** — navigation
  from `.feature` steps to Perl `Given`/`When`/`Then` definitions plus a
  quick-fix command that generates step-definition stubs. Unit coverage
  for matching, generation, and registration. (#4024)

- **VS Code test runner now prefers `yath` for test files** when present
  on PATH, with `prove` and `perl` fallbacks intact. Unit tests pin the
  runner preference order without depending on local tool availability.
  (#4031)

- **Segment-based incremental token cache + two-sided checkpoint window**
  — `TokenSegment` sorted storage, `CheckpointCache::find_after()`, and
  `reparse_from_checkpoint_two_sided`; new `IncrementalStats` metrics
  (`segments_reused_before`, `segments_reused_after`, `segments_invalidated`,
  `full_tail_fallbacks`, `left_checkpoint_distance`,
  `right_checkpoint_distance`, `bytes_relexed`); 23 correctness tests and
  a 13-group Criterion benchmark suite (#4029).

- **Orphaned unclosed-block recovery tests wired into `perl-parser-core`**
  — the `unclosed_block_recovery_tests` module existed on disk with six
  well-written tests but was never registered in `mod.rs` and had never
  compiled. Now wired with six additional edge-case tests covering C-style
  `for`, `foreach`, `unless`, `BEGIN` phase blocks, doubly-nested unclosed
  blocks, and nested blocks inside `sub`. (#4079)

- **Symbol visibility regression tests** for Error-partial recursion in
  `perl-semantic-analyzer`, covering arrow-truncation errors, unclosed-sub
  recovery, and missing-RHS recovery (#4071).

- **Per-edit checkpoint and cache delta assertions** for incremental
  parsing, replacing cumulative-counter assertions with per-edit deltas
  and tree-equivalence checks against a fresh full parse (#4076).

- **`require VERSION` pragma semantics test** guarding that `require 5.x`
  does not enable strict or warnings in `PragmaTracker` (#4023).

### Changed

- **Pre-push hook switched from `ci-gate` to `pr-fast`** (Tier A). The
  generated `hooks/pre-push` file is regenerated from `perl-ci-hygiene` and
  a regression test keeps the generated text and checked-in hook file in
  sync. `ci-gate` remains documented as the explicit full merge gate.
  (#4088, #4110)

- **Doc-only pre-push fast path skips code gates entirely** instead of
  running workspace-wide `cargo fmt --all -- --check`. Avoids a Windows
  long-path rustfmt crash on prose-only pushes. Regression test asserts
  the doc-only branch exits before any workspace-wide rustfmt check.
  (#4061)

- **Docs-only merge fast-track** added to the review/merge policy so
  doc-only PRs no longer falsely require `reviewed-deep`. Enforced by
  `scripts/pre-merge-check.sh` with shell-test coverage; reviewer / ops
  docs and label automation updated to match. (#4103)

- **`PragmaTracker::state_for_offset(&map, usize::MAX)`** is now the
  single authoritative source of truth for top-level pragma state in
  `check_strict_warnings`; the scope-unaware `walk_node` closure arms
  that scanned eval/sub interiors are removed (#4052).

- **`SymbolExtractor::visit_node()` Error arm** now recurses into
  `partial: Option<Box<Node>>` instead of treating Error as an opaque
  leaf, bringing symbol extraction into parity with every other
  traversal in the codebase (`semantic_tokens`, `class_model`,
  `scope_analyzer`, `for_each_child`) (#4071).

- **Metric framing scoped down** across README, VSCode marketplace
  listing, and v0.13.0 announcement draft — capabilities are now framed
  as advertised surface, not claimed conformance; entry-points table
  added to README; known UX gaps listed explicitly (#4045,
  #4046, #4049, #4051).

### Fixed

- **Parser error recovery and symbol extraction under partial `Error` nodes**
  — unclosed block recovery landed in `perl-parser-core` (PR #4079) and
  symbol extraction now descends into partial `Error` nodes (PR #4071),
  closing issue [#3499](https://github.com/EffortlessMetrics/perl-lsp/issues/3499).

- **Navigation: inherited and role methods in goto-def, hover, and
  completion** — BFS traversal now chains `model.roles` alongside
  `model.parents` in `inherited_method_definition_location`; hover
  wires the previously dead-code `resolve_inherited_method_hover` path;
  workspace completion uses a new `collect_all_package_members` BFS that
  replaces the direct `get_package_members` call in
  `add_workspace_method_completions` (#4077).

- **Navigation: AUTOLOAD-backed method calls** resolve through
  `AUTOLOAD` fallback in both goto-definition and hover when the named
  method is absent (#4091).

- **Diagnostics: eval- and sub-scoped pragmas no longer suppress
  file-level PL100/PL101** — `pragma_map.iter().any()` replaced with
  `PragmaTracker::state_for_offset` at `usize::MAX`; `walk_node` closure
  arms that descended into `NodeKind::Eval` / `NodeKind::Subroutine`
  bodies and falsely set `has_strict = true` are removed. 4 new tests
  cover eval-scoped and sub-scoped false-negative paths. (#4052)

- **Pragma: phase-block pragmas kept lexically scoped** — `BEGIN`, `END`,
  `INIT`, `UNITCHECK`, and `CHECK` block pragmas no longer leak to file
  scope; the bad `strict_warnings` phase-block override that suppressed
  PL100/PL101 is removed. Replaced with behavior-spec and integration
  coverage for block-local semantics. (#4108)

- **Pragma: conditional `use if` / `use unless`** — `PragmaTracker`
  recognises `use if CONDITION, MODULE, ...` forms and conservatively
  applies the tracked pragma semantics from the suffix target. Lint
  pipeline regressions confirm conditional strict/warnings suppress the
  missing-pragma hints. (#4050)

- **Pragma: explicit `use feature qw(...)` and `:X.Y` bundles** tracked
  in `PragmaTracker`; `version_compat` understands feature bundles; 
  `no feature` lexical disablement is honored. Regressions cover
  `switch` enablement via bundles and lexical disablement. (#4038)

- **Incremental parsing: prefix correctness at checkpoint boundaries** —
  when a nonzero checkpoint window has no cached prefix tokens, fall
  back to a full reparse instead of assembling a suffix-only token
  stream for `Parser::from_tokens`. Regression compares incremental vs
  full parse for an edit past the first checkpoint boundary. (#4048)

- **Semantic analyzer: symbol extraction descends into Error partial
  nodes** (#4071). The arrow-truncation recovery path will start
  producing new symbols if the parser begins wrapping declarations in
  `Error { partial: Some(...) }`.

- **Workspace: non-`file://` URIs tolerated as workspace roots** —
  `vscode-remote://`, `untitled:`, and other virtual schemes are kept
  in LSP string form; direct `to_file_path()` calls are routed through
  a file-only URI helper; workspace folder matching normalises trailing
  slashes; non-filesystem folders are skipped during indexing scans.
  (#4059)

- **Execute-command: Windows extended-length path prefix stripped** —
  `normalize_path_for_external_command` removes the `\\?\` prefix on
  Windows at every spawn site in `execute_command::provider` (yath,
  prove, perl primary and fallback paths, `run_test_sub`, `run_file`).
  Non-Windows is a zero-cost identity via `#[cfg(windows)]`. Closes
  the Windows Run Tests regression. (#4089)

- **DAP types: basename derivation for Windows-style source paths** —
  `Source::new()` adds a narrow fallback when `Path::file_name()` returns
  the entire input string (as it does for backslash-separated paths on
  Unix hosts), so `C:\Users\dev\project\lib\Module.pm` now derives
  `Module.pm` correctly. Unblocks the previously-failing
  `source_with_windows_path` regression. (#4028)

- **Xtask `features verify` repaired** — catalog test paths are now
  resolved from repo root, the advertised-vs-caps snapshot is read from
  `crates/perl-lsp-rs/tests/snapshots/...`, the two-document Insta snapshot
  format is parsed correctly, and the verifier compares against the
  capability-backed advertised LSP subset (#4033).

- **Clippy hygiene: hover/navigation `let_and_return` and
  `needless_borrow`** warnings cleared, plus a follow-up
  `needless_borrow` fix in workspace file-ops after #4052 and #4088
  landed (#4037, #4098).

- **Plan-review hook IO hardened** — `subagent-stop.sh` binds issue
  context from `ISSUE_NUMBER`, payload `issue_number`, or the canonical
  `plan-review-NNN` agent name (in that precedence), instead of the
  broken branch-digit scan that silently labeled random historical
  issues. Fail-loud exit 3 when no valid issue context exists. Both
  `subagent-stop.sh` and `task-completed.sh` normalise payload / receipt
  fields with `tr -d '\r'` so JSONL metrics and CRLF-sensitive receipt
  parsing no longer reject valid UTC timestamps. Canonical
  `plan-review-NNN` naming documented in `.claude/commands/swarm.md`.
  Regression coverage added in both `.ci/scripts/test-hooks.sh` and
  `cargo xtask hook-tests`. (#4064)

- **CI hygiene: allowlisted production panic paths normalised** before
  matching so both Unix and Windows-style relative paths are recognised
  (#4081).

- **CI hygiene: doc-only pre-push fast path skips code gates** entirely
  instead of running workspace-wide rustfmt (#4061).

- **Semantic-analyzer: stale method attribute assertion** fixed to look
  up extracted methods via the current symbol contract while keeping the
  attribute-preservation assertion intact (#4082).

- **Agent definitions: terminal skills made explicit** — `scout-lsp.md`
  gains the missing step 9 `/agent-wrapup` (every other scout variant
  already had it); `reviewer-deep.md` restructures step 4 and adds a
  new step 5 making `/pr-ready` an explicit required follow-up when
  the deep-review decision is "approve". Root cause for the earlier
  incidents where deep reviewers set `reviewed-deep` but forgot to
  call `/pr-ready`, leaving PRs stuck in draft. (#4087)

### Docs

- **Metric framing scoped down** — README, VSCode marketplace listing,
  and v0.13.0 announcement draft now frame capabilities as advertised
  surface, not claimed conformance. README gains an entry-points table
  and an explicit list of known UX gaps. (#4045, #4046, #4049, #4051)

### Tests / Quality

- **Test-side P0 idiom + dependency findings burned down** — three
  `.or_insert_with(Vec::new)` occurrences migrated to `.or_default()`
  in `lsp_cancellation_performance_tests.rs`,
  `test_infrastructure_mocks.rs`, and
  `documentation_validation_mutation_hardening.rs`; explicit
  `HashMap<String, Vec<Duration>>` / `HashMap<String, Vec<usize>>` type
  annotations added where inference became ambiguous;
  `perl-tdd-support` added to `[dev-dependencies]` in `perl-dap-types`,
  `perl-symbol-surface`, and `perl-ast-utils`. (#4002)

- **Match-arm panic asserts burned down** in test code across
  `perl-dap-variables`, `perl-lexer` interpolation tests, and several
  other test suites, continuing the long-running #3258 burn-down
  (#4030, #4032, #4035).

<!-- 2026-04-11 session addendum: PRs merged after the initial changelog entry was written -->

### Added (addendum)

- **Phase-scoped pragma diagnostics** (`PL502`, `PL503`) — new diagnostics flag
  `use strict` / `use warnings` placed inside phase blocks (`BEGIN`, `END`, `INIT`,
  `CHECK`, `UNITCHECK`) where they have lexical block scope rather than file scope;
  quick fixes move the pragmas to file scope preserving shebangs. (#4131)

- **`cargo xtask check-test-wiring` CLI command wired** — PR #4119 added the
  `check_test_wiring` module but omitted the `use` import in `main.rs`; the
  subcommand was returning "unrecognized subcommand". Now fully wired; also fixes
  one genuine orphan discovered by the guard: `crates/perl-lsp-rs/tests/fixtures/integration_example.rs`.
  (#4151)

- **Cross-file `use constant` and parenthesized import lists** — `find_import_source()`
  strips quotes from string args before comparison so `use Foo ('bar', 'baz')` resolves
  via goto-def; `use constant` re-exports are followed across file boundaries. (#4133)

### Changed (addendum)

- **Multi-root workspace integration tests activated in nightly gate** — the 8
  integration tests in `multi_root_workspace_tests.rs` (added in #3984, never run in CI)
  are now wired via a new `ci-workspace-multiroot` justfile recipe, placed in the
  nightly gate only until proven stable. (#4137)

### Fixed (addendum)

- **Hotfix: red master from `check_test_wiring` regex and clippy** — two runtime
  `Regex::new(...).expect(...)` calls in `check_test_wiring.rs` migrated to
  `LazyLock<Regex>` statics; `let_and_return` clippy warning in `parser_corpus_sweep`
  removed; `RUSTSEC-2026-0097` suppressed in audit paths with follow-up in #4149.
  (#4150)

- **Status: feature maturity metadata restored** — valid `maturity` value reinstated
  for the phase-scoped pragma diagnostic capability after #4131 introduced an
  invalid value; `xtask update-status --check` green again. (#4148)

- **DevEx: detect stale installed pre-push hooks** — `just status-check` now compares
  the installed `.git/hooks/pre-push` against the canonical checked-in `hooks/pre-push`,
  normalising CRLF and trailing-blank-line noise. (#4144)

### Tests / Quality (addendum)

- **Perl::Critic missing profile path test** — regression test for an explicitly
  configured but missing Perl::Critic profile path; asserts subprocess is skipped and
  no policy diagnostics are returned. (#4139)

### Added (2026-04-12)

- **DAP launch-success scorecard** — new integration harness measures DAP cold-launch
  pass rate across 5 fixture debuggees (hello, loops, eval, args, begin_end) with
  P50/P95 latency metrics; a new `docs/project/status/dap.md` page surfaces DAP
  coverage alongside the existing LSP status pages. (#4237)

- **Editor UX receipt** — machine-readable `docs/project/status/editor_ux.json`
  receipt generated by `xtask update-status` tracks the editor UX fixture matrix
  pass rate, wired into `quality.md` and the status index. (#4233, #4234)

### Fixed (2026-04-12)

- **Rename operations no longer lag on large files** — `collect_descendant_scopes`
  replaced O(n×d) parent-chain walk with a single O(n) map build + iterative BFS,
  with a cycle guard preventing hangs on pathological self-referential parent links.
  (#4240)

### Changed (2026-04-12)

- **Research-verifier is now the default for claim-heavy PRs** — agent skill
  definitions encode the research-verifier dispatch policy so orchestrators no longer
  need a reminder; claim-heavy criteria defined in three skill files. (#4235)

### Documentation (2026-04-12)

- **`perl-lsp-semantic-tokens` crate docs corrected** — CLAUDE.md updated from
  stale 15 types/7 modifiers to the actual 23 types/13 modifiers, with all token
  types and modifiers listed in index order and Perl-specific extensions called out.
  (#4239)

### Added (2026-04-12 session 2)

- **Compile-time constants hover** — `__FILE__`, `__LINE__`, `__PACKAGE__`, and
  `__SUB__` now show rich hover documentation with descriptions and caveats
  (e.g. `__SUB__` in named subs vs anonymous subs). (#4270, #4294)

- **Fast/slow diagnostic split** — parse errors are now published immediately
  (~440ms sooner) via `publish_parse_errors_fast()`, then replaced by the full
  diagnostic set on the 250ms debounce. Users see red squiggles while typing
  without waiting for scope analysis or perlcritic. (#4279, #4305)

- **Generation-aware staleness guard** — if a `didChange` arrives during slow
  computation (scope analysis, perlcritic, dead-code), the stale diagnostic
  result is suppressed and the debouncer re-fires for the latest version. (#4295)

- **`require Module; Module->import('sym')` completion** — the two-statement
  require+import pattern is now recognised for completion ranking alongside
  `use Module` imports. (#3476, #4296)

- **Module ranking tiers for completion** — completion candidates are ranked
  by import tier (direct import > workspace > CPAN) with string-context
  suppression and open-snippet triggers for module paths. (#4263, #4277)

- **`workspace/configuration` folder propagation** — `didChangeConfiguration`
  now eagerly propagates settings to each folder's `effective_workspace_config`,
  closing a stale-settings window between notification and async pull response.
  (#3515, #4289, #4307)

- **Safe-delete widened dependent detection** — `workspace/willDeleteFiles`
  now detects dependents via both `use` imports and `require` statements,
  surfacing warnings for a broader set of cross-file references. (#3513, #4293)

- **Package declaration rewrite during module rename** — `workspace/willRenameFiles`
  now rewrites `package Foo::Bar` declarations inside the renamed file to match
  the new module path. (#3522, #4291)

### Fixed (2026-04-12 session 2)

- **Package name hover** — hovering a qualified package name like `File::Path`
  previously showed broken hover text because the tokenizer stopped at `:`.
  New `get_package_name_at_position` scans across `::` separators to produce
  correct rich hover with file path, POD, and MetaCPAN link. (#4282, #4306)

- **Signature/Prototype AST byte-span** — `Signature` and `Prototype` nodes
  now carry the correct byte span from the parser, fixing off-by-one ranges
  in hover and semantic tokens. (#4243, #4281)

- **Windows compatibility** — `/proc` reads guarded behind `cfg(target_os = "linux")`;
  hardcoded `/tmp` paths replaced with `std::env::temp_dir()` for cross-platform
  correctness. (#4229, #4278)

### Tests / Quality (2026-04-12 session 2)

- **Cross-folder rename verification** — integration tests verify rename operations
  span both workspace roots correctly. (#3522, #4273, #4292)

- **Import visibility regression tests** — unit tests for `require`+`import`
  symbol resolution patterns. (#3476, #4286)

- **Heredoc `unreachable!` ratchet coverage** — tests cover all 7 heredoc
  unreachable patterns on both path separators. (#4245, #4274)

- **Unused dev-dependencies removed** from 5 crates. (#4183, #4255)


## [0.12.3] - 2026-04-09

Release notes: [v0.12.3](docs/releases/v0.12.3.md) · [GitHub Release](https://github.com/EffortlessMetrics/perl-lsp/releases/tag/v0.12.3)

<!-- Pipeline rehearsal release — validates the full publish + extension + Docker cycle before v0.13.0 public alpha -->
<!-- Rolls up publish pipeline fixes, UX P0 improvements, and CI hardening from Waves 10/11/12 -->

### Headlines

- **`tree-sitter-perl-c` first published to crates.io** — the conventional C grammar
  binding (tree-sitter FFI over the C parser) is now a proper published leaf crate,
  shedding its `libclang`/bindgen dependency in favour of vendored C sources compiled
  via `cc`. Framed as a compatibility and comparison surface alongside the native v3
  parser stack. (#3234)

- **Publish pipeline overhauled** — three layered fixes make the pipeline correct
  and fast: Tarjan SCC topological sort properly handles dev-dependency cycles (#3236);
  dev-dependencies are stripped from each manifest before publishing so circular
  workspace dev-deps no longer block `cargo publish` (#3254); and the registry
  indexing wait is replaced with progressive sparse-index probes that catch silent
  upload failures instead of proceeding on false success (#3230).

- **Archive: 7 dead tree-sitter harness crates removed from the workspace** — the
  old Pest-based `tree-sitter-perl-rs` harness and 6 `perl-ts-*` compatibility shims
  are moved to `archive/`, clearing the `tree-sitter-perl-rs` name for a planned
  Rust-native tree-sitter-style facade over the v3 parser. (#3244, #3250)

- **DevEx polish** — `just doctor` auto-detects and self-heals recurring worktree
  state-corruption bugs; the pre-push hook gains a doc-only fast path and
  self-heals `core.bare=true` corruption; `just bump-version` centralises
  version sync across 191 sites. (#3249, #3238, #3228)

- **Quality burn-down** — ~210 `eprintln!` calls in library crates migrated to
  structured `tracing` macros; three waves of `unwrap`/`expect` eliminations across
  test code; two dead `build.rs` files removed that were causing unnecessary
  recompiles. (#3245, #3229, #3241)

### Added

- **`just doctor`**: one-stop workspace health-check that auto-detects and
  (where safe) auto-fixes recurring state-corruption bugs — `core.bare=true`,
  stale branches, worktree file leaks, orphaned worktree directories, and missing
  pre-push hook. (#3249)

- **`just bump-version`**: centralised version-sync command covering all 191
  version sites (workspace Cargo.toml, every crate manifest, VS Code extension
  manifest and lockfile, `features.toml`, README, CLAUDE.md, ROADMAP). Paired with
  an updated `check-version-sync` gate that now covers all the same sites, so drift
  cannot go undetected. (#3228)

- **`perl-heredoc-anti-patterns` microcrate**: SRP extraction of
  `anti_pattern_detector` from the larger `perl-ts-heredoc-analysis` crate, which
  is now archived. The only part that production code consumed is now a clean
  publishable leaf crate. (#3199)

- **`perl-parser-bench` microcrate**: SRP extraction of the `bench_parser` binary
  that was misplaced inside the tree-sitter-perl-rs harness. Uses `perl-parser`
  (v3 native) directly. (#3198)

- **`perl-parser-pest` published to crates.io**: the legacy v2 Pest-based Perl
  parser is now a published crate, available as a learning tool and Pest reference
  implementation for the broader Perl-in-Rust ecosystem. (#3195)

- **`perl-lsp-ai-provider` published to crates.io**: filled out crates.io metadata
  and added to the publish allow-list. This was a blocker for `perl-lsp-rs`
  publication. (#3196)

- **4 orphaned workspace members registered**: `perl-workspace-folder`,
  `perl-dap-stack`, `perl-lsp-feature-policy`, and `perl-lsp-formatting-types`
  were referenced throughout the workspace but missing from `[workspace] members`,
  causing them to be silently skipped by every workspace-wide CI gate. (#3232)

- **AI streaming tests**: mock streaming-backend coverage for progress, cancel,
  and error paths; final stream sequence field assertion; relaxed error-path
  assertion for terminal final event. (#3170, #3172, #3174, #3175)

- **CPAN corpus caching in CI**: CPAN corpus is now installed and cached before
  the ratchet step, preventing spurious corpus-ratchet failures on clean CI runs.
  (#3173)

### Changed

- **`tree-sitter-perl-c` is now publishable**: vendored C sources compiled via
  `cc` replace the `libclang`/bindgen build step entirely; the single hand-written
  FFI symbol was already sufficient. Crate brought into the workspace as a proper
  member. (#3234)

- **xtask now depends on standalone crates directly**: dev tooling in `xtask` and
  `scripts/test_recursion.rs` was swapped off the archived tree-sitter-perl harness
  onto `perl-parser-pest` (Rust parser) and `tree-sitter-perl-c` (C FFI) directly,
  removing the harness's last consumers before archival. (#3206)

- **`just quick-bench` fixed to actually compare C vs Rust parsers**: previously
  both columns invoked the same `perl-parser-bench` binary (comparing a warm vs
  cold run of the native parser). The C column now invokes `bench_parser_c` from
  `tree-sitter-perl-c`, so the speedup column reflects a real C vs Rust comparison.
  (#3204, #3253)

- **Pre-push hook smarter**: doc-only fast path (markdown/text/license/docs changes
  run `cargo fmt --check` only, skip the full ci-gate); self-heals `core.bare=true`
  corruption before any git operation. (#3238)

- **Publish workflow indexing wait replaced with sparse-index probes**: progressive
  probe at 5s/15s/45s/90s elapsed replaces a fixed 5-minute wait; each crate is
  verified via the crates.io sparse index after publish; the final verify job runs
  unconditionally (`if: always()`) and lists exactly which crates failed. (#3230)

- **`eprintln!` → `tracing` in library code**: ~210 `eprintln!` calls across
  library crates replaced with structured `tracing` macros at appropriate levels
  (warn/error for failures, info for lifecycle, debug/trace for routine output).
  `tracing` added to 6 crates that lacked it. (#3224, #3245)

- **Documentation framing updated**: README Architecture section names the native
  parser/lexer/analysis stack as the architectural centre, distinguishes
  `tree-sitter-perl-c` (C FFI reference, maintained for compatibility) from the
  planned `tree-sitter-perl-rs` facade (Rust-native, in development), and frames
  tree-sitter compatibility as an interoperability surface. (#3247)

- **Per-crate CLAUDE.md headers refreshed** post-archive of tree-sitter harness
  crates. Stale references to archived crates removed. (#3240)

### Fixed

- **Publish: dev-dependency cycles no longer block `cargo publish`** — dev-deps
  are stripped from each crate's `Cargo.toml` before publishing (and restored
  afterward via a `trap` on EXIT). Fixes the 3-crate dev-dep cycle
  (`perl-parser-core` / `perl-tdd-support` / `perl-corpus`) that caused publish
  order failures. (#3254, #3256)

- **Publish: Tarjan SCC topological sort for dev-dep edges** — the previous sort
  excluded dev-dep edges, causing crates that dev-depend on later-published siblings
  to be ordered before them. The fix includes dev-dep edges in the graph, uses
  Tarjan SCC to find strongly-connected components, and retains only inter-SCC
  dev-dep edges (intra-SCC edges are the only ones that can close a cycle).
  (#3236, #3242)

- **Publish: `perl-test-must` published before `perl-tdd-support`** — ordering
  fix for the initial publish sequence that caused `perl-tdd-support` to land
  before its dependency. (#3176, #3177)

- **Corpus ratchet path mismatch** (#3189 / #3257): xtask's CPAN corpus paths are
  now anchored at the workspace root (via `env!("CARGO_MANIFEST_DIR")` at build
  time) rather than resolved against `std::env::current_dir()`. The workflow's
  `test -d` step is aligned to the same absolute path. Regression-guarded by a
  unit test that asserts `workspace_root()` contains a top-level `Cargo.toml`.

- **`hook-tests` workspace scribble** (#3203 / #3246): the hook-test scaffold's
  throwaway git repo inherited `core.hooksPath` from the parent environment,
  causing the parent pre-commit hook to fire inside the temp repo. In one observed
  run the temp repo's `README.md` write landed on the real workspace `README.md`.
  The temp repo is now explicitly isolated with `GIT_CONFIG_NOSYSTEM=1` and
  `core.hooksPath` cleared; temp dirs are created under `$TMPDIR` not the
  workspace root.

- **Windows xtask file-lock** (#3202 / #3241): two dead `build.rs` files removed —
  the root `build.rs` (workspace-only manifest, never run by cargo) and
  `crates/perl-parser/build.rs` (set environment variables that nothing read, and
  marked `perl-parser` dirty on every commit via `.git/HEAD` rerun-if-changed
  directives, propagating unnecessary rebuilds to all 50+ dependents).

- **Windows xtask: recursive subprocess eliminated** (#3221): `cmd_check_parse_errors`
  was spawning xtask as a subprocess of itself, which caused `Access is denied` (os
  error 5) on Windows due to the write-lock on the running executable. The inner
  call is now replaced with a direct function call.

- **Windows xtask: backslash mangling in `smoke-test-release.sh`** (#3214): absolute
  Windows `PathBuf` paths passed to `bash` as arguments caused backslash-escape
  collapse. Fixed by using a relative path instead.

- **Triage workflow silently aborting** (#3235): the `triage-issues` workflow was
  failing on every run that encountered an issue needing labels, silently aborting
  at the first `add_labels` call.

- **`features.toml` dead test paths repaired**: 43 dead test paths corrected to
  match the current `crates/perl-lsp-rs/tests/` layout; the
  `experimental.perlInlineCompletionStream` feature row added (shipped in v0.12.2).
  (#3222, #3251)

- **`unsafe` block documented**: `GenerateConsoleCtrlEvent` FFI call in
  `perl-dap` now carries a SAFETY comment explaining why the call is sound.
  (#3232)

### Removed

- **Archived 7 dead tree-sitter harness crates** to `archive/crates/`:
  `tree-sitter-perl-rs` (old Pest-based harness), `perl-ts-heredoc-analysis`,
  `perl-ts-statement-tracker`, `perl-ts-logos-lexer`, `perl-ts-heredoc-parser`,
  `perl-ts-partial-ast`, `perl-ts-advanced-parsers`. All workspace references,
  CI exclusion lists, and benchmark function paths updated. (#3244, #3250)

- **Dead stray LICENSE files** in `crates/perl-corpus/`, `crates/perl-lexer/`,
  `crates/perl-parser/`: byte-identical orphan files not referenced by any
  `Cargo.toml` `license-file` field. (#3196)

### Dependencies

- `similar` 2.7.0 → 3.0.0 (#3184) — only consumer is xtask; breaking changes do
  not intersect our usage
- `actions/cache` v4 → v5 (#3181) — Node 24 runtime bump; existing caches remain
  readable
- `eslint` 9.39.4 → 10.2.0 (#3179) — flat config already in use; lint passes clean
- `tokio` 1.50.0 → 1.51.0 (#3180)
- `tree-sitter` 0.26.7 → 0.26.8 (#3182)
- dependencies group with 3 updates (#3183)
- npm group in vscode-extension (#3178)

### Publish pipeline fixes (post-v0.12.2 publish run lessons)

These fixes landed after the initial v0.12.2 publish run and directly address the
partial-publish (108/129) and cascading-failure patterns observed in production:

- **HTTP 429 throttle** (#3307): publish workflow detects crates.io rate-limit
  responses and retries with exponential back-off; the 21 crates that failed in
  the v0.12.2 publish run were blocked by 429s from rapid-fire publish attempts.

- **Publish allowlist extended** (#3296): `perl-workspace-index-monitoring` and
  `perl-test-generators` added to the publish allow-list after they were found
  missing from the v0.12.2 publish set.

- **LICENSE files corrected** (#3304): missing or incorrect `LICENSE` files added
  to 4 publishable crates (`perl-lsp-ai-provider`, `perl-workspace-index`,
  `tree-sitter-perl-rs`, `tree-sitter-perl-c`); crates.io rejects publishes with
  license-file fields pointing to absent files.

- **Duplicate `[package.metadata.docs.rs]` key** (#3315): `tree-sitter-perl-c`
  had two `[package.metadata.docs.rs]` tables in `Cargo.toml`; the duplicate key
  caused `cargo publish` to emit a parse warning and was silently dropped, causing
  docs.rs to build without the intended features. Resolved by merging the two
  tables.

- **Continue-on-failure** (#3316): publish loop now tracks failures in a
  `FAILED_CRATES` array instead of `exit 1` immediately; all topologically-ready
  crates are attempted even when an earlier crate fails. On v0.12.2 run
  24126423987, 19 crates were blocked by a single cascade; on run 24133403944,
  22 crates were blocked. Re-runs safely skip already-published crates via the
  sparse-index check.

- **`tree-sitter-perl-c` polish for first publish** (#3273): vendored sources and
  FFI bindings verified clean for crates.io submission; duplicate metadata resolved
  (#3315 above).

- **docs.rs metadata** (#3299): `[package.metadata.docs.rs]` blocks added or
  corrected for feature-gated crates across the workspace; enables docs.rs to
  build documentation with the correct feature flags set.

- **Publish dry-run gate** (#3301): new CI check runs `cargo publish --dry-run` on
  every PR that modifies a `Cargo.toml`, catching publish-time errors (missing
  files, bad metadata, syntax) before they reach the release pipeline.

### UX fixes (P0 launch blockers)

Five actionability fixes for user-visible error paths that surfaced during the
v0.12.2 publish run and post-publish testing:

- **Actionable binary download errors** (#3306): extension now shows a specific
  message with platform, arch, and download URL when the LSP server binary cannot
  be fetched, instead of a generic network failure.

- **LSP startup error diagnosis** (#3308): `classifyStartupError()` maps stderr
  signatures (GLIBC version mismatch, missing shared library, Exec format error,
  permission denied) to actionable hints and remediation steps; reorders error
  dialog actions so "View Logs" appears before "Reinstall".

- **Workspace root detection warning** (#3309): when the workspace root cannot be
  determined, the server now emits a `window/showMessage` warning with the detected
  state instead of failing silently. Previously users had no indication of why
  features were degraded.

- **Enterprise binary distribution note** (#3310): documentation updated to
  explain that `perllsp` is distributed as a pre-compiled binary via `cargo
  install`, with offline-install guidance for air-gapped enterprise environments.

- **Perl interpreter missing error** (#3312): when `perl` is not found on `$PATH`,
  the extension shows the exact binary name searched and a platform-specific
  installation suggestion, replacing the previous "Perl not found" dead end.

### CI hardening

- **SHA-pinned third-party Actions** (#3294): all `uses:` references to third-party
  GitHub Actions pinned to immutable commit SHAs with version comments (e.g.,
  `uses: actions/checkout@11bd71901bbe5b1630ceea73d27597364c9af683 # v4.2.2`).
  Prevents supply-chain attacks via tag mutation.

- **GIT_DIR cleared in hook-tests** (#3318): xtask hook-test scaffold now runs
  with `GIT_DIR` unset, preventing the worktree's inherited `GIT_DIR` value from
  causing git commands inside the temp repo to resolve against the wrong object
  store. Observed contamination: test-repo commits were silently landing in the
  agent worktree.

- **UX regression gate** (#3293): new CI check detects regressions in user-visible
  LSP, DAP, and extension behaviour on every PR that touches those surfaces.
  Backed by the UX test harness framework (#3297).

- **UX test harness framework** (#3297): systematic framework for UX regression
  tests with helpers for LSP, DAP, and extension surface validation.

## [0.12.2] - 2026-04-04

Release notes: [v0.12.2](docs/releases/v0.12.2.md) · [GitHub Release](https://github.com/EffortlessMetrics/perl-lsp/releases/tag/v0.12.2)

`v0.12.2` is the confidence-building release for the 0.12.x series. 89 commits
across 59 PRs spanning new features, performance, testing, distribution, and
documentation. The entire 0.12.x roadmap from v0.12.2 through v0.12.8 milestones
is consolidated into this single release.

The v0.12.2 publish run extended the original GitHub Release with a wave of
quality, distribution, and CI infrastructure work needed to land the full crate
set on crates.io. 108 of 129 crates published successfully in the first attempt;
the remaining 21 (including `tree-sitter-perl-c`, `tree-sitter-perl-rs`,
`perl-parser`, `perl-lsp-rs`, `perllsp`, `perl-dap`) will retry after the HTTP
429 throttle fix lands.

### New Crates (first publish)

- **`tree-sitter-perl-rs`**: v3 ergonomic facade over the native parser stack,
  published alongside `tree-sitter-perl-c` for projects that want tree-sitter
  call ergonomics on top of the Rust-native parser (#3255)
- **`tree-sitter-perl-c`**: conventional C-binding crate for the tree-sitter
  grammar, now publishable on crates.io (#3234)

### Added

- **AI inline completion**: opt-in OpenAI-compatible provider with SSE streaming,
  session management, cancellation, and deterministic fallback when AI is off
  (#3157–#3168)
- **heredoc language injection**: SQL keyword and JSON key detection in heredocs
  with multi-heredoc-per-line support (#3134)
- **type inference in hover**: `TypeInferenceEngine` wired to show inferred types
  on hover (#3150)
- **dead code highlighting**: `DiagnosticTag::Unnecessary` for unreachable code
  (#3092)
- **extract variable/subroutine**: AST-aware code action for extracting
  expressions and blocks (#3090)
- **subroutine inlining**: code action to inline simple subroutines (#3083)
- **POD preview panel**: VS Code command `Perl: Preview POD` (#3131)
- **AST explorer debug panel**: `perl/showAst` custom LSP handler (#3124)
- **Docker image**: `effortlessmetrics/perl-lsp` with perllsp + Perl runtime
  (#3113)
- **DAP cross-platform signals**: continue and interrupt signal handling on
  Linux/macOS/Windows (#3117)
- **context-sensitive quote parsing**: `qw`, `s///`, `tr///` disambiguation in
  complex expressions (#3105)
- **semantic framework coverage**: inheritance and export analysis for Moo/Moose
  patterns (#3103)
- **Linux/macOS installer**: fixed and improved install script (#3122)
- **streaming inline completion controller**: VS Code gating on AI config flags
  (#3161, #3164)

### Performance

- **incremental parsing pipeline**: token caching (#3116), checkpoint recovery
  (#3114), and `Parser::from_tokens` (#3128) complete the incremental path
- **CPAN-scale benchmarks**: 10K files indexed in 672ms, 500K symbol lookup in
  10.6µs (#3121, #3132)
- **large-workspace HashMap optimization**: faster startup for big projects
  (#3112)
- **memory profiling infrastructure**: heap tracking for workspace indexing
  (#3125)
- **completion latency benchmarks**: baseline for regression detection (#3104)

### Fixed

- **DAP attach cleanup**: removed stale mock stub and updated tests (#3135)
- **perlcritic integration**: hardened diagnostic pipeline (#3097)
- **silent error handling**: 23+ silently swallowed errors now emit trace logs
  (#3087, #3151)
- **distribution binary name**: Linux packaging templates and Windows bump
  workflows aligned with `perllsp` (#3106, #3144)
- **Homebrew asset names**: brew-bump workflow aligned (#3120)
- **CI efficiency**: 10 improvements reducing CI minutes (#3156)
- **VS Code type safety**: replaced `any` types with proper TypeScript types
  (#3154)
- **LSP capability snapshots**: regenerated stale snapshots (#3142, #3147)
- **inline completion**: removed duplicate backend type definitions (#3162)
- **pipeline-labels race**: fixed race condition on `reviewed-deep` label (#3100)

### Testing

- **147 DAP tests**: serde, edge cases, and error paths across 4 DAP crates
  (#3152)
- **AI inline completion tests**: integration tests for streaming and
  deterministic paths (#3165, #3168)
- **error builder/lexer mode tests**: missing coverage for error paths (#3091)

### Documentation

- **AI inline completion config reference** (#3167)
- **end-to-end LSP feature development guide** (#3115)
- **large-workspace testing and profiling guide** (#3126)
- **GIF recording guide** for marketing assets (#3130)
- **problem-first README rewrite** (#3119)

### Dependencies

- unified 16 scattered dependency versions via workspace deps (#3153)
- removed 8 unused dependencies across 6 crates (#3146)
- dependabot: insta 1.47.1, proptest, tar, toml 1.1.0, uuid 1.23.0,
  actions/deploy-pages 5, codecov/codecov-action 6

### Quality (publish-run additions)

- **`eprintln!` → `tracing`**: migrated all `eprintln!` / `println!` calls in
  library code to structured `tracing` spans/events; `eprintln!` now banned in
  non-binary crates (#3224, #3245)
- **unwrap burn-down**: Wave 2 (`perl-dap-security`) and Wave 3 (5 crates, 9
  eliminations) converted `unwrap()`/`expect()` calls to `?` and pattern
  matching (#3246 area)
- **error message actionability**: user-visible LSP/DAP error messages rewritten
  to be actionable — what failed, why, what to do next — ahead of v0.13.0
  launch (#3291)
- **crates.io metadata**: `description`, `keywords`, `categories`, `repository`,
  `documentation`, `readme` fields polished across all publishable crates (#3234)
- **docs.rs metadata**: `[package.metadata.docs.rs]` blocks added for
  feature-gated crates (#3234)
- **dead build.rs files removed**: stale `build.rs` files that caused publish
  errors removed from 3 crates (#3217, #3241)
- **stale harness crates archived**: dead tree-sitter harness crates moved to
  `archive/` to reduce workspace noise (#3250, #3244)

### CI (publish-run additions)

- **publish topological sort**: dev-dependencies now included in the publish
  order graph so crates publish in the correct dependency order (#3236, #3242)
- **dev-dependency stripping**: `cargo publish` now strips `[dev-dependencies]`
  before publishing to avoid version conflicts (#3254, #3256)
- **`--allow-dirty` for publish**: added after dev-dep strip leaves the working
  tree dirty (#3300)
- **HTTP 429 throttle handling**: publish workflow detects crates.io rate-limit
  responses and retries with back-off (pending)
- **sparse index wait replaced**: replaced fixed-duration index wait with
  sparse-index polling for faster, more reliable publish verification
- **UX regression gate**: PR check that detects regressions in user-visible LSP,
  DAP, and extension behavior on every PR touching those surfaces (#3293)
- **post-publish smoke test**: automated verification that published crates
  install and the binary starts correctly after each publish run (#3288)
- **version-bump automation centralized**: `just bump-version` now handles
  Cargo.toml, extension package.json, and docs in one command (#3289)
- **`just doctor`**: new workspace health-check recipe that validates the full
  workspace is in a buildable state before starting a session (#3249)
- **`vsce publish` idempotency**: marketplace publish step no longer fails on
  re-run when the version already exists (#3187, #3267)

### UX (publish-run additions)

- **Settings schema polish**: VS Code extension settings schema updated for
  launch-readiness — correct types, descriptions, and defaults (#3278)
- **VS Code Marketplace punch list**: README badges, Open VSX registration,
  extension icon, and feature highlights aligned for marketplace discovery
  (#3284)
- **test de-flake**: `empty_timer_reports_total` race condition fixed (#3278)

## [0.12.1] - 2026-03-31

Release notes: [v0.12.1](docs/releases/v0.12.1.md) · [GitHub Release](https://github.com/EffortlessMetrics/perl-lsp/releases/tag/v0.12.1)

`v0.12.1` is the fix-forward cut after the initial public alpha release. It does
not reopen the wider alpha scope; it closes the release-surface regressions that
slipped into the first `v0.12.0` tag and keeps the install and publish story
aligned.

### Fixed

- restored the top-level README and release-facing docs so the source snapshot
  no longer presents hook-test fixture content as the project front page
- hardened hook-test fixture setup so temporary repos must live outside the real
  checkout and seed commits no longer write placeholder git identities into repo
  config
- fixed local git-hook installation for worktrees and added pre-commit blocking
  for the known placeholder identities used by release and hook tests

### Changed

- workspace, feature-catalog, VS Code extension, and operator release surfaces
  now target `0.12.1`
- status and roadmap docs now treat `v0.12.0` as the latest published GitHub
  release and `v0.12.1` as the active fix-forward cut

## [0.12.0] - 2026-03-30

Release notes: [v0.12.0](docs/releases/v0.12.0.md) · [GitHub Release](https://github.com/EffortlessMetrics/perl-lsp/releases/tag/v0.12.0)

`v0.12.0` is the initial public alpha for the native Rust Perl 5 toolchain. The
headline change is not one feature in isolation; it is that the parser, language
server, debugger, install surface, and release process now line up well enough
for normal editor use.

### Highlights

#### Native editor path

- `perllsp` and `perl-dap` are now treated as first-class native binaries for editor integration and debugging.
- VS Code, manual binary install, and release surfaces were tightened for first-run setup, health checks, and issue reporting.
- `.perl-lsp.toml` gives teams a shared, editor-agnostic project configuration layer.

#### Better day-to-day language tooling

- Completion, hover, diagnostics, formatting, semantic tokens, workspace symbols, code lens, and code actions all received broad hardening.
- Hover and completion coverage expanded for Perl built-ins, special variables, module flows, and workspace-aware suggestions.
- Diagnostic wiring now consistently surfaces parser, project, and optional Perl::Critic signals through the LSP pipeline.

#### Better real-world Perl coverage

- The native recursive-descent parser was hardened against curated common-corpus and CPAN-facing receipts instead of toy examples alone.
- Semantic and workspace layers improved cross-file navigation, rename, inheritance-aware lookups, and framework-aware behavior for Moo and Moose patterns.
- Workspace indexing, cancellation, timeouts, and runtime concurrency all received reliability work aimed at larger real projects.

#### Release and contributor surface

- Release prep, package-manager manifests, docs, validation receipts, and status pages were aligned for the public-alpha launch.
- The workspace continued its crate-boundary cleanup so parser, runtime, LSP, DAP, and release tooling are easier to reason about independently.

### Notable user-facing additions

- project config via `.perl-lsp.toml`
- richer hover coverage for special variables, built-ins, and framework-aware symbols
- broader completion coverage and improved ranking
- native DAP improvements for stepping, variables, and editor integration
- stronger workspace symbol, formatting, code action, and code lens support

### Notable fixes

- parser recovery and disambiguation across real Perl edge cases such as quote operators, slash parsing, prototypes, and framework-heavy code
- deadlock, contention, and stale-state fixes in the LSP runtime and workspace index
- safer handling for empty files, binary files, Windows and macOS path quirks, and shell-launch edge cases
- stale capability drift, unwired command paths, and release-surface documentation mismatches

For the detailed receipts behind this release, see [docs/project/CURRENT_STATUS.md](docs/project/CURRENT_STATUS.md) and [docs/project/status/index.md](docs/project/status/index.md).

## [0.11.0] - 2026-03-12

Release notes: [v0.11.0](docs/releases/v0.11.0.md) · [GitHub Release](https://github.com/EffortlessMetrics/perl-lsp/releases/tag/v0.11.0)

This release finalizes the 0.11.0 distribution pipeline across GitHub releases,
crates.io, and the VS Code extension so the workspace can ship from a single,
repeatable release flow.

### Added
- **Turnkey Release Orchestration**: A PR-driven release path now covers version
  bumping, changelog generation, tagging, GitHub release creation, crates.io
  publishing, extension publishing, and downstream package manager automation.
- **Topological crates.io Publishing**: Workspace publish automation computes
  dependency order from `cargo metadata` and publishes only the crates in the
  workspace allowlist.
- **Release Guardrails**: Release helper scripts now validate semver inputs and
  align manual operator flows with the automated `0.11.0` release path.

### Changed
- **Workspace Release Alignment**: Workspace packages, extension metadata, and
  release workflows now target `0.11.0`.
- **Release Tooling**: Legacy release helper scripts now delegate to the current
  GitHub workflow-based release flow instead of relying on stale one-off cargo
  publish steps and outdated examples.
- **Operator Documentation in Scripts**: Manual publish and smoke-test helpers
  now accept an explicit version argument and default to the matching `vX.Y.Z`
  release ref when dispatching workflows or validating published artifacts.

### Fixed
- **Stale Release Examples**: Removed hardcoded `0.8.3` release references from
  publish and smoke-test scripts that could misdirect manual release operations.
- **Publish Version Safety**: crates.io publishing now fails early when the
  workflow target version does not match the versions resolved for workspace
  crates scheduled for publication.

## [0.10.0] - 2026-02-28

Release notes: [v0.10.0](docs/releases/v0.10.0.md) (internal milestone — no GitHub Release)

A major release campaign spanning 60+ PRs (#845-#911) focused on build reliability,
security hardening, crates.io publishing readiness, documentation, and code quality.

### Added
- **Document Highlight for Modern Perl**: try/catch parameters, method/sub signatures, and string interpolation (#882, #896).
- **Feature Governance Microcrates**: Extracted feature governance into 9 dedicated crates for modularity (#848).
- **Module Infrastructure Crates**: Content-Length framing and LSP transport hardening (#857).
- **Context-Aware Status Menu**: Perl LSP status menu with workspace-aware states (#646).
- **InlineValues Lifecycle Coverage**: Test coverage for inlineValues support (#729).
- **Tie-Interface Corpus Tests**: New corpus test fixtures for Perl tie interface syntax (#900).
- **Public API Documentation**: Comprehensive rustdoc for `perl-parser` (#904) and leaf crates (#903).
- **Copilot Instructions**: `.github/copilot-instructions.md` for AI-assisted development (#886).
- **Merge-Gate Commit Status**: CI now publishes merge-gate status checks (#880).
- **Benchmark Test Enablement**: Previously-ignored workspace benchmark test enabled with real assertions (#908).

### Changed
- **Version Bump to 0.10.0**: All 80+ workspace crates, documentation, VS Code extension, and feature catalogs updated (77+ files) (#879, #884).
- **crates.io Publishing Readiness**: All crate metadata verified, publish-ignore lists normalized, crate badges added, publish allowlist expanded (#865, #867, #871, #897).
- **VS Code Extension Polish**: Marketplace readiness with packaging fixes, runtime deps, npm lockfile (#863, #866, #869, #906).
- **Documentation Overhaul**: CONTRIBUTING.md polished for public release (#909), README.md and ROADMAP.md updated (#888), FrameworkKind/FrameworkFlags docs (#887), cargo doc warnings resolved (#894).
- **features.toml**: Version bumped to 0.10.0 with 100% LSP coverage maintained (53/53 user-visible, 97/97 protocol).
- **LSP Harness**: Replaced sleep-poll with condvar+drain-bytes pattern for deterministic testing (#846).
- **xtask Gates**: Fail closed for required timeout/error statuses (#868).
- **Unused Dependencies Removed**: cargo-machete sweep across workspace (#895).
- **Debt Ledger Updated**: Refreshed after cleanup campaign (#898).
- **Stale Files Cleaned**: Removed stale tracked files, hardened .gitignore (#889).
- **Semver-Aware Benchmark Sorting**: Correct version comparison for baseline selection (#885).

### Fixed
- **Build**: Resolved 4 compilation errors in the release candidate build (#881).
- **Clippy**: Resolved warnings across all targets (#901).
- **Document Highlight Regressions**: Fixed test regressions from modern syntax support (#896).
- **LSP Error Logging**: Improved error logging in LSP providers (#905).
- **Unresolved Review Comments**: Addressed outstanding comments from PRs #881 and #882 (#892).
- **Version Drift**: Fixed remaining v0.9.x references in satellite files (#884).
- **Checksum Verification**: Hardened verification and stabilized incremental parsing CI (#858).
- **Installer Scripts**: Hardened for security and reliability (#910).
- **Refactoring Test Isolation**: Isolated `cleanup_no_backups` backup root (#864).
- **CI Receipt Parsing**: Aligned receipt parsing and serialized BDD tests (#845).
- **CI BDD Gate**: Added `--locked` flag and timing receipts (#847).
- **CI Docs Deploy**: Skip when GitHub Pages is disabled (#859).
- **Release Workflow**: Asset naming alignment across chain (#890, #902), concurrency groups (#890).
- **Release Tooling**: git-cliff installation fixes (#873, #874, #875), cargo-release installs (#876, #877), PR-driven 0.x.y flow (#872).
- **Publish Workflow**: Dry-run quoting fix (#870), `--no-verify` for dev-dep cycles (#867).

### Security
- **[HIGH] Path Traversal in DAP Launch**: Fixed path traversal vulnerability in debug adapter (#640).
- **[HIGH] Argument Injection in TestRunner**: Fixed argument injection vulnerability (#633).
- **[MEDIUM] Safe Evaluation Bypass**: Fixed bypass for iterator/IO operations (#647).
- **GitHub Actions Hardening**: SHA-pinned all workflow action references (#911).
- **Installer Hardening**: Hardened install scripts for security and reliability (#910).
- **VS Code Extension**: Pinned minimatch to 10.2.3 to remediate CVEs (#861).

### Performance
- **Symbol Extraction**: Optimized regex compilation for faster workspace indexing (#645).
- **Semantic Analyzer**: Eliminated deep cloning of AST nodes in subroutine analysis (#632).
- **Scope Analyzer**: Optimized unused parameter detection, fixed double reporting (#638).

### Infrastructure
- **Nightly CI Stabilization**: Fuzz harness panic hardening, coverage test resilience, clippy cleanup (#860).
- **Release Orchestration**: Turnkey PR-driven 0.x.y release workflow (#872).
- **Release Tool Installs**: Deterministic git-cliff and cargo-release installation (#873-#877).
- **crates.io Dry-Run**: Unblocked dry-run packaging for all workspace crates (#865).
- **Lockfile Maintenance**: Refreshed lockfile for CI deny checks, fuzz lockfile exclusion (#885).

### Dependencies
- `rand` 0.9.2 -> 0.10.0 (#855).
- `serial_test` 3.3.1 -> 3.4.0 (#854).
- `uuid` 1.20.0 -> 1.21.0 (#856).
- `toml` 0.9.12 -> 1.0.3 (#853).
- `aquasecurity/trivy-action` 0.34.0 -> 0.34.1 (#851).
- `@types/node` 25.1.0 -> 25.3.0 (#849).
- `@types/tar` 6.1.13 -> 7.0.87 (#850).
- Additional dependency group updates (#852).

## [0.9.1] - 2026-02-20

Release notes: [v0.9.1](docs/releases/v0.9.1.md) (tag only — no GitHub Release)

### Added
- **Initial Public Alpha Release**: Substantially complete feature set for early testing.
- **Enhanced LSP Features**: 99% coverage of LSP 3.18 methods (alpha-validated).
- **Complete Semantic Analyzer**: All NodeKind handlers implemented (Phases 1, 2, 3) with 100% AST node coverage.
- **Debug Adapter Protocol (DAP) Support**: Phase 1 bridge to Perl::LanguageServer.
- **Enhanced LSP Cancellation System**: Thread-safe infrastructure for minimal latency.
- **Advanced Code Actions**: AST-aware refactoring including extraction and import optimization.
- **Security Hardening**: UTF-16 boundary fixes and path traversal prevention.
- **Comprehensive API Documentation**: Infrastructure for documentation enforcement.
- **Optimized Test Suite**: 0.31s full test suite execution via adaptive threading.

### Changed
- **Project Origins Documented**: Origins in Q2 2025, forked July 15, 2025 from `tree-sitter-perl-better`.
- **Stability Roadmap Refined**: Formal Stability Contract (contract-locked APIs) pushed to v0.15.0.
- **MSRV Updated**: Minimum Supported Rust Version bumped to 1.92 (Rust 2024 edition).
- **Parser Architecture**: Native recursive descent parser as the primary implementation.

### Fixed
- **v0.9.1 close-out receipts captured**: Workspace index state-machine transitions and early-exit behavior verified.
- **Security boundary fixes**: Resolved multi-root workspace path traversal issues.

## [0.9.0] - 2026-01-18

Release notes: [v0.9.0](docs/releases/v0.9.0.md) (internal milestone — no tag or GitHub Release)

### Added
- **Semantic Analyzer Phase 1**: 12/12 critical node handlers implemented.
- **LSP textDocument/definition Integration**: Semantic-aware definition resolution.
- **Enhanced Cross-File Navigation**: Dual indexing strategy for improved reference coverage.

### Changed
- **LSP Coverage**: Increased to 82% of trackable features.

## [0.8.8] - 2025-12-01

Release notes: [v0.8.8](docs/releases/v0.8.8.md) (internal milestone — no tag or GitHub Release)

### Added
- **Initial Workspace Configuration Support**.
- **Enhanced Formatting Fallback**: Always-available capabilities with perltidy integration.

---

## Future Milestones

### Next Release
- Enhanced DAP native implementation (Phase 2).
- Semantic depth improvements for Moo/Moose.

### v0.15.0 (Stability Contract Milestone)
- **Formal Stability Contract**: Contract-locked APIs and wire protocol invariants.
- Full protocol compliance audit.
- Multi-release deprecation cycles.

---

## Version Support Policy (Alpha Phase)

During the alpha phase (pre-v0.15.0):
- **Current Alpha (0.x.y)**: Active development and bug fixes.
- **Breaking Changes**: Allowed in minor (0.x) releases.
- **Security**: Critical patches prioritized for the latest alpha version.

---

## Links

For the full cross-channel release history, see [RELEASE_HISTORY.md](RELEASE_HISTORY.md).

<!-- Compare ranges -->
[0.13.2]: https://github.com/EffortlessMetrics/perl-lsp/compare/v0.13.1...v0.13.2
[0.13.1]: https://github.com/EffortlessMetrics/perl-lsp/compare/v0.13.0...v0.13.1
[0.12.4]: https://github.com/EffortlessMetrics/perl-lsp/compare/v0.12.3...v0.12.4
[0.12.3]: https://github.com/EffortlessMetrics/perl-lsp/compare/v0.12.2...v0.12.3
[0.12.2]: https://github.com/EffortlessMetrics/perl-lsp/compare/v0.12.1...v0.12.2
[0.12.1]: https://github.com/EffortlessMetrics/perl-lsp/compare/v0.12.0...v0.12.1
[0.12.0]: https://github.com/EffortlessMetrics/perl-lsp/compare/v0.11.0...v0.12.0
[0.11.0]: https://github.com/EffortlessMetrics/perl-lsp/compare/v0.8.5...v0.11.0
[0.10.0]: https://github.com/EffortlessMetrics/perl-lsp/compare/v0.9.1...v0.10.0
[0.9.1]: https://github.com/EffortlessMetrics/perl-lsp/compare/v0.8.5...v0.9.1
[0.9.0]: https://github.com/EffortlessMetrics/perl-lsp/compare/v0.8.5...v0.9.0
[0.8.8]: https://github.com/EffortlessMetrics/perl-lsp/compare/v0.8.5...v0.8.8
[0.13.0-rc1]: https://github.com/EffortlessMetrics/perl-lsp/compare/v0.12.4...v0.13.0-rc1
[Unreleased]: https://github.com/EffortlessMetrics/perl-lsp/compare/v0.13.2...HEAD
[0.15.0]: https://github.com/EffortlessMetrics/perl-lsp/compare/v0.14.0...v0.15.0
