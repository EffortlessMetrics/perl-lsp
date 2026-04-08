# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

<!-- To be populated before announcement -->

## [0.12.3] - 2026-04-08

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
  match the current `crates/perl-lsp/tests/` layout; the
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

## [0.12.2] - 2026-04-08

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

## [0.12.1] - 2026-03-30

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

## [0.12.0] - 2026-03-24

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

### Added
- **Semantic Analyzer Phase 1**: 12/12 critical node handlers implemented.
- **LSP textDocument/definition Integration**: Semantic-aware definition resolution.
- **Enhanced Cross-File Navigation**: Dual indexing strategy for improved reference coverage.

### Changed
- **LSP Coverage**: Increased to 82% of trackable features.

## [0.8.8] - 2025-12-01

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
