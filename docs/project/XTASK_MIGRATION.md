# Xtask Migration Tracking

This document tracks the ongoing migration of shell/Python scripts to Rust
xtask subcommands (`cargo xtask <subcommand>`).

## Why Migrate

- **Cross-platform**: Shell scripts do not work on Windows; Rust compiles everywhere.
- **Type safety and error handling**: Rust's `Result`/`Option` replace fragile `set -euo pipefail`.
- **Workspace integration**: Xtask can import workspace crates, reuse shared types, and access `Cargo.toml` metadata directly.
- **Testable**: Each subcommand can have unit tests; shell scripts cannot.
- **Single binary**: `cargo xtask` replaces 75+ scripts scattered across `scripts/`, `scripts/gh/`, `scripts/forensics/`, `.ci/scripts/`, and `benchmarks/scripts/`.

## Current Xtask Subcommands

| Subcommand | Module | Lines | Purpose |
|------------|--------|------:|---------|
| `ci-audit-workflows` | `ci_audit_workflows.rs` | 150 | Audit PR workflows for ungated non-trivial jobs |
| `ci-baseline` | `ci_metrics.rs` | -- | Measure CI baseline from recent workflow runs |
| `ci-cost-monitor` | `ci_metrics.rs` | -- | Analyze GitHub Actions spend over a period |
| `ci` | `ci.rs` | 117 | Lean CI suite (format + clippy + tests) |
| `check-only` | `ci.rs` | -- | Format and clippy checks only |
| `ci-measure` | `ci_measure.rs` | -- | Measure CI lane runtimes and emit timing artifacts |
| `targeted-checks` | `targeted_checks.rs` | 319 | Target changed crates and run fast checks |
| `build` | `build.rs` | 69 | Build with configurable features/mode |
| `test` | `test.rs` | 148 | Run tests with suite/coverage options |
| `bench` | `bench.rs` | 355 | Run benchmarks |
| `bench-alert` | `benchmarks.rs` | -- | Run benchmark alert checks |
| `bench-alert-test` | `benchmarks.rs` | -- | Run benchmark alert regression test suite |
| `bench-extract` | `benchmarks.rs` | -- | Extract and normalize criterion outputs |
| `inject-sha-assets` | `inject_sha_assets.rs` | 206 | Generate Homebrew formula and VS Code asset map |
| `update-homebrew` | `update_homebrew.rs` | 282 | Generate Homebrew formula from release SHA256SUMS |
| `bench-compare` | `benchmarks.rs` | -- | Compare benchmark artifacts against baseline |
| `bench-format` | `benchmarks.rs` | -- | Format benchmark JSON output |
| `bench-run` | `benchmarks.rs` | -- | Wrapper for benchmark runner script |
| `compare` | `compare.rs` | 1174 | C vs Rust benchmark comparison |
| `build-timing-receipt` | `build_timing.rs` | -- | Collect build timing receipts and measurement baselines |
| `compare-build-timing` | `build_timing.rs` | -- | Compare build timing receipts with markdown report |
| `doc` | `doc.rs` | 35 | Generate documentation |
| `check` | `check.rs` | 58 | Code quality checks (clippy, fmt) |
| `fmt` | `fmt.rs` | 43 | Format code |
| `clean` | `clean.rs` | 47 | Clean build artifacts |
| `dev` | `dev.rs` | 178 | Development server with watch |
| `parse-rust` | `parse_rust.rs` | 54 | Run pure Rust parser on a file |
| `parser-matrix` | `parser_matrix.rs` | 338 | Generate `docs/reference/PARSER_FEATURE_MATRIX.md` from parser audit report |
| `validate-workspace-exclusions` | `validate_workspace_exclusions.rs` | -- | Validate workspace exclusion strategy and dependency invariants |
| `release` | `release.rs` | 223 | Prepare a release |
| `security-hardening` | `hardening.rs` | -- | Production security hardening checks |
| `release-turnkey` | `release_turnkey.rs` | -- | PR-driven release orchestration |
| `performance-hardening` | `hardening.rs` | -- | Production performance hardening checks |
| `bump-version` | `bump_version.rs` | 184 | Bump version numbers across project |
| `publish-crates` | `publish.rs` | 203 | Publish crates to crates.io |
| `forbid-fatal-constructs` | `forbid_fatal_constructs.rs` | -- | Run forbidden fatal construct checks via perl-ci-hygiene |
| `forensics-dossier` | `forensics.rs` | -- | Generate complete PR dossier artifacts |
| `forensics-harvest` | `forensics.rs` | -- | Harvest PR forensics metadata |
| `forensics-render` | `forensics.rs` | -- | Render PR dossiers from existing YAML |
| `forensics-telemetry-full` | `forensics.rs` | -- | Run full PR telemetry pipeline |
| `forensics-telemetry-quick` | `forensics.rs` | -- | Run quick PR telemetry pipeline |
| `forensics-temporal` | `forensics.rs` | -- | Analyze PR temporal topology |
| `gh-backfill-prefixed-labels` | `github.rs` | -- | Backfill legacy issue labels into prefixed taxonomy |
| `gh-labels` | `github.rs` | -- | Ensure GitHub label taxonomy is present |
| `gh-triage` | `github.rs` | -- | Show issues missing required label taxonomy |
| `ci-hygiene` | `ci_hygiene.rs` | -- | Pass-through to `perl-ci-hygiene` subcommands |
| `worktree-cleanup` | `worktrees.rs` | 71 | Remove stale `.claude/worktrees` entries |
| `publish-vscode` | `publish.rs` | -- | Publish VSCode extension |
| `populate-book` | `populate_book.rs` | -- | Populate mdBook source directory from docs |
| `e2e-validate` | `e2e_validate.rs` | -- | Run end-to-end validation suite |
| `verify-publication-facts` | `publication_facts.rs` | -- | Verify PUBLICATION_FACTS_LEDGER metrics |
| `publish-receipts` | `publish_receipts.rs` | -- | Publish phase-0 receipt bundle for review |
| `prep-crates-io-launch` | `prep_crates_io_launch.rs` | -- | Launch preflight checks (`core` / `all`) |
| `test-heredoc` | (delegates to `test.rs`) | -- | Heredoc-specific tests |
| `test-edge-cases` | `edge_cases.rs` | 110 | Edge case test suite |
| `corpus-audit` | `corpus_audit.rs` | 325 | Corpus coverage analysis |
| `compare-three` | `compare_parsers.rs` | 318 | Three-way parser comparison (legacy) |
| `test-lsp` | `test_lsp.rs` | 509 | LSP feature tests with demo scripts |
| `parser-corpus-sweep` | `parser_corpus_sweep.rs` | 1097 | System Perl corpus error-rate sweep |
| `features sync-docs` | `features.rs` | 378 | Sync docs from features.toml |
| `features verify` | `features.rs` | -- | Verify features match capabilities |
| `features report` | `features.rs` | -- | Generate compliance report |
| `srp-microcrates` | `srp_microcrates.rs` | 194 | SRP microcrate inventory |
| `validate-memory-profiler` | `compare.rs` | -- | Memory profiling validation |
| `gates` | `gates.rs` | 1370 | CI gates with receipt generation |
| `production-gates-validation` | `hardening.rs` | -- | Validate production gates and SLO posture |
| `corpus` | `corpus.rs` | 625 | Corpus tests (legacy feature) |
| `highlight` | `highlight.rs` | 272 | Highlight tests (parser-tasks feature) |
| `bindings` | `bindings.rs` | 49 | Generate bindings (parser-tasks feature) |

## Migration Status

### scripts/ (top-level)

| Script | Lines | Xtask Equivalent | Status | Priority | Notes |
|--------|------:|-------------------|--------|----------|-------|
| `dead-code-check.sh` | 457 | `cargo xtask dead-code` | **Replaced** | Low | Compatibility shim for `cargo xtask dead-code` |
| `execute-gate.sh` | 112 | `cargo xtask gates` | **Replaced** | -- | Xtask `gates` subsumes single-gate execution with receipts |
| `run-gates.sh` | 165 | `cargo xtask gates` | **Replaced** | -- | Xtask `gates` covers full gate runs |
| `gate-local.sh` | 135 | `cargo xtask gates` / `cargo xtask ci` | **Replaced** | -- | WSL-safe local gate; xtask handles parallelism natively |
| `generate-receipt.sh` | 133 | `cargo xtask gates --receipt` | **Replaced** | -- | Receipt generation built into xtask gates |
| `generate-receipts.sh` | 121 | `cargo xtask gates --receipt` | **Replaced** | -- | Batch receipt generation |
| `list-gates.py` | 21 | `cargo xtask gates --list` | **Replaced** | -- | Gate listing |
| `forbid-fatal-constructs.sh` | 12 | `cargo xtask forbid-fatal-constructs` | **Replaced** | High | Policy gate; canonical xtask command handles this |
| `check-version-sync.sh` | 12 | `cargo xtask check-version-sync` | **Replaced** | Medium | Thin wrapper delegating to `cargo xtask` |
| `update-current-status.py` | 454 | `cargo xtask update-status` | **Replaced** | High | Compatibility shim for `cargo xtask update-status` |
| `debt-report.py` | 436 | `cargo xtask debt-report` | **Replaced** | High | Compatibility shim for `cargo xtask debt-report` |
| `debt-pr-summary.py` | 36 | `cargo xtask debt-report --summary` | **Replaced** | Low | Small PR summary formatter; now delegated to `debt-report --summary` |
| `check-doc-claims.py` | 123 | `cargo xtask doc-claims` | **Replaced** | Medium | Compatibility shim for `cargo xtask doc-claims` |
| `check_features_invariants.py` | 104 | `cargo xtask features invariants` | **Replaced** | Medium | Feature catalog invariants now checked by `cargo xtask features invariants` |
| `ci-audit-workflows.py` | 123 | `cargo xtask ci-audit-workflows` | **Replaced** | Medium | CI spend audit; now delegated to `ci-audit-workflows` task |
| `update-parser-matrix.py` | 255 | `cargo xtask parser-matrix` | **Replaced** | Low | Compatibility shim for `cargo xtask parser-matrix` |
| `release-turnkey-pr.sh` | 431 | `cargo xtask release-turnkey` | **Replaced** | High | Orchestration entrypoint now delegated through `xtask release-turnkey` |
| `prepare-release.sh` | 61 | `cargo xtask release-turnkey` | **Replaced** | -- | Thin wrapper around `release-turnkey` flow |
| `publish-release.sh` | 94 | `cargo xtask publish-release` | **Replaced** | -- | Crate publishing dispatch wrapper |
| `publish-receipts.sh` | 68 | `cargo xtask publish-receipts` | **Replaced** | Low | Archives gate + receipt artifacts with provenance metadata |
| `install.sh` | 236 | -- | **Keep** | -- | Curl-pipe installer; must remain shell for `curl \| bash` UX |
| `install-githooks.sh` | 13 | `cargo xtask ci-hygiene install-githooks` | **Replaced** | Low | Trivial pass-through wrapper |
| `lsp-smoke.sh` | 107 | `cargo xtask test-lsp` (partial) | **Partial** | Medium | LSP smoke test over JSON-RPC; xtask test-lsp covers demo scripts |
| `smoke-test-release.sh` | 74 | `cargo xtask smoke-test-release` | **Replaced** | Medium | Post-release binary smoke test |
| `ci-cost-monitor.sh` | 409 | `cargo xtask ci-cost-monitor` | **Replaced** | Low | CI cost analysis and budget reporting now handled by `cargo xtask ci-cost-monitor` |
| `cleanup-completed-worktrees.sh` | 98 | -- | **Keep** | -- | Mid-cycle cleanup with manual control |
| `close-duplicate-prs.sh` | 63 | -- | **Keep** | -- | One-off GitHub housekeeping |
| `cleanup-worktrees.sh` | 8 | `cargo xtask worktree-cleanup` | **Replaced** | Low | Worktree cleanup wrapper |
| `inject-sha-assets.sh` | 140 | `cargo xtask inject-sha-assets` | **Replaced** | Low | Release asset SHA injection |
| `update-homebrew.sh` | 132 | `cargo xtask update-homebrew` | **Replaced** | Low | Homebrew formula update; release-only |
| `populate-book.sh` | 143 | `cargo xtask populate-book` | **Replaced** | Low | mdBook content assembly |
| `render-docs.sh` | 130 | `cargo xtask doc` (partial) | **Partial** | Low | Full doc rendering pipeline; xtask doc handles cargo doc only |
| `build-timing-receipt.sh` | 197 | `cargo xtask build-timing-receipt` | **Replaced** | Low | Compatibility shim for build timing receipt generation |
| `compare-build-timing.sh` | 214 | `cargo xtask compare-build-timing` | **Replaced** | Low | Compatibility shim for build timing comparison |
| `validate-workspace-exclusions.sh` | 97 | `cargo xtask validate-workspace-exclusions` | **Replaced** | Low | Compatibility shim for `cargo xtask validate-workspace-exclusions` |
| `validate_features.sh` | 71 | `cargo xtask features verify` | **Replaced** | -- | Feature validation |
| `validate-phase1.sh` | 85 | -- | **Keep** | -- | One-time phase validation; historical |
| `validate_issue_146.sh` | 201 | -- | **Keep** | -- | One-time issue validation; historical |
| `verify_stacker.sh` | 11 | `cargo xtask ci-hygiene verify-stacker` | **Replaced** | Low | Trivial one-liner |
| `devex-doctor.sh` | 109 | `cargo xtask devex-doctor` | **Replaced** | Medium | Developer environment diagnostics |
| `swarm-summary.sh` | 25 | `cargo xtask swarm-summary` | **Replaced** | Low | Swarm metrics summary |
| `verify-publication-facts.sh` | 262 | `cargo xtask verify-publication-facts` | **Replaced** | Medium | Publication claims and ledger drift checks |
| `devex-targeted-checks.sh` | 124 | `cargo xtask targeted-checks` | **Replaced** | Medium | Targeted devex checks |
| `test-lsp-cancellation.sh` | 12 | `cargo xtask ci-hygiene test-lsp-cancellation` | **Replaced** | Low | LSP cancellation test |
| `cargo-package-workspace-dry-run.sh` | 47 | `cargo xtask publish-crates --dry-run` | **Replaced** | -- | Dry-run packaging |
| `prep-crates-io-launch.sh` | 79 | `cargo xtask prep-crates-io-launch` | **Replaced** | Low | Pre-publish checklist (`core` and `all` modes) |
| `llvm.sh` | 233 | -- | **Keep** | -- | LLVM toolchain setup; platform-specific by nature |
| `security-hardening.sh` | 292 | `cargo xtask security-hardening` | **Replaced** | Low | Production hardening; Phase 6 one-time |
| `performance-hardening.sh` | 334 | `cargo xtask performance-hardening` | **Replaced** | Low | Production hardening; Phase 6 one-time |
| `e2e-validation.sh` | 463 | `cargo xtask e2e-validate` | **Replaced** | Low | Production hardening E2E validation |
| `e2e-gate.sh` | 11 | `cargo xtask ci-hygiene e2e-gate` | **Replaced** | Low | Trivial wrapper |
| `production-gates-validation.sh` | 333 | `cargo xtask production-gates-validation` | **Replaced** | Low | Production gates; Phase 6 one-time |
| `preflight.sh` | 11 | `cargo xtask ci-hygiene preflight` | **Replaced** | Low | Trivial wrapper |
| `test-capped.sh` | 11 | `cargo xtask ci-hygiene test-capped` | **Replaced** | Low | Trivial test wrapper |
| `test-e2e-capped.sh` | 11 | `cargo xtask ci-hygiene test-e2e-capped` | **Replaced** | Low | Trivial test wrapper |
| `quick-receipts.sh` | 12 | `cargo xtask ci-hygiene quick-receipts` | **Replaced** | Low | Trivial wrapper |
| `ignored-test-count.sh` | 12 | `cargo xtask ci-hygiene ignored-test-count` | **Replaced** | Medium | Simple grep-based counter |

### scripts/ -- Benchmark Scripts (top-level)

| Script | Lines | Xtask Equivalent | Status | Priority | Notes |
|--------|------:|-------------------|--------|----------|-------|
| `benchmark_all.sh` | 165 | `cargo xtask bench` | **Replaced** | -- | General benchmarking |
| `benchmark_fuzzed.sh` | 208 | `cargo xtask bench` (partial) | **Partial** | Low | Fuzz-guided benchmarks |
| `benchmark_pure_rust_vs_c.sh` | 96 | `cargo xtask compare` | **Replaced** | -- | Rust vs C comparison |
| `benchmark_rust_vs_c_simple.sh` | 79 | `cargo xtask compare` | **Replaced** | -- | Simple comparison |
| `compare_all_levels.sh` | 172 | `cargo xtask compare` | **Replaced** | -- | Multi-level comparison |
| `run_actual_benchmark.sh` | 104 | `cargo xtask bench` | **Replaced** | -- | Benchmark runner |
| `run_comparison_benchmarks.sh` | 305 | `cargo xtask compare` | **Replaced** | -- | Comparison benchmarks |
| `run_comparison.sh` | 45 | `cargo xtask compare` | **Replaced** | -- | Comparison runner |
| `run_comprehensive_benchmark.py` | 220 | `cargo xtask bench` | **Replaced** | -- | Comprehensive benchmarks |
| `run_parser_comparison.sh` | 11 | `cargo xtask ci-hygiene run-parser-comparison` | **Replaced** | -- | Parser comparison |
| `setup_benchmark.sh` | 244 | `cargo xtask bench` (partial) | **Partial** | Low | Benchmark environment setup |
| `simple_bench.sh` | 49 | `cargo xtask bench` | **Replaced** | -- | Simple benchmark |
| `quick_bench.sh` | 69 | `cargo xtask bench` | **Replaced** | -- | Quick benchmark |
| `optimized_benchmark.py` | 171 | `cargo xtask bench` | **Replaced** | -- | Optimized benchmarks |
| `generate_comparison.py` | 489 | `cargo xtask compare --report` | **Replaced** | -- | Comparison report generation |
| `generate_issue_summary.py` | 133 | -- | **Keep** | -- | One-time issue summary |
| `test_comparison.py` | 385 | `cargo xtask compare` | **Replaced** | -- | Comparison tests |
| `quick_test.py` | 57 | -- | **Keep** | -- | Quick ad-hoc test helper |
| `test_edge_cases.sh` | 12 | `cargo xtask test-edge-cases` | **Replaced** | -- | Edge case tests |
| `test_iterative_parser.sh` | 11 | `cargo xtask ci-hygiene test-iterative-parser` | **Replaced** | Low | Trivial test runner |
| `profile_stack_overflow.sh` | 51 | -- | **Keep** | -- | Debugging aid |
| `apply-workspace-simplification.sh` | 86 | -- | **Keep** | -- | One-time refactoring script |
| `deduplicate-crates.sh` | 93 | -- | **Keep** | -- | One-time deduplication |
| `generate-badges.sh` | 11 | `cargo xtask ci-hygiene generate-badges` | **Replaced** | Low | Trivial badge generation |

### scripts/gh/

| Script | Lines | Xtask Equivalent | Status | Priority | Notes |
|--------|------:|-------------------|--------|----------|-------|
| `ensure-labels.sh` | 63 | `cargo xtask gh-labels` | **Replaced** | -- | GitHub label management; `gh` CLI is the right tool |
| `issues-needing-triage.sh` | 26 | `cargo xtask gh-triage` | **Replaced** | -- | GitHub triage query |
| `backfill-prefixed-labels.sh` | 68 | `cargo xtask gh-backfill-prefixed-labels` | **Replaced** | -- | One-time label backfill |

### scripts/forensics/

| Script | Lines | Xtask Equivalent | Status | Priority | Notes |
|--------|------:|-------------------|--------|----------|-------|
| `pr-harvest.sh` | 408 | `cargo xtask forensics-harvest` | **Replaced** | Low | PR data harvesting |
| `temporal-analysis.sh` | 694 | `cargo xtask forensics-temporal` | **Replaced** | Low | Temporal analysis |
| `telemetry-runner.sh` | 1376 | `cargo xtask forensics-telemetry-full` / `cargo xtask forensics-telemetry-quick` | **Replaced** | Medium | Telemetry collection; largest script |
| `dossier-runner.sh` | 285 | `cargo xtask forensics-dossier` | **Replaced** | Low | Dossier generation |
| `render-dossier.sh` | 590 | `cargo xtask forensics-render` | **Replaced** | Low | Dossier rendering |
| `lib_gh.sh` | 178 | -- | **Not Started** | Low | Shared GitHub API helpers |

### .ci/scripts/

| Script | Lines | Xtask Equivalent | Status | Priority | Notes |
|--------|------:|-------------------|--------|----------|-------|
| `measure-ci-time.sh` | 114 | `cargo xtask ci-measure` | **Replaced** | Low | CI timing measurement |
| `measure-ci-baseline.sh` | 409 | `cargo xtask ci-baseline` | **Replaced** | Low | Baseline CI timing now handled by `cargo xtask ci-baseline` |
| `check-from-raw.sh` | 27 | `cargo xtask check-from-raw` | **Replaced** | -- | Small CI helper |

### benchmarks/scripts/

| Script | Lines | Xtask Equivalent | Status | Priority | Notes |
|--------|------:|-------------------|--------|----------|-------|
| `run-benchmarks.sh` | 194 | `cargo xtask bench-run` | **Replaced** | -- | Benchmark runner |
| `format-results.py` | 246 | `cargo xtask bench-format` | **Replaced** | Medium | Benchmark result formatting |
| `compare.sh` | 13 | `cargo xtask compare` | **Replaced** | -- | Comparison wrapper |
| `compare.py` | 276 | `cargo xtask compare --report` | **Replaced** | -- | Comparison analysis |
| `alert.py` | 479 | `cargo xtask bench-alert` | **Replaced** | Medium | Performance regression alerts |
| `extract-criterion.py` | 195 | `cargo xtask bench-extract` | **Replaced** | Low | Criterion output parser |
| `test_alert_system.sh` | 233 | `cargo xtask bench-alert-test` | **Replaced** | Low | Alert system tests |
| `test_regression.py` | 31 | -- | **Keep** | -- | Small regression test |

## Summary

| Category | Count | Total Lines |
|----------|------:|------------:|
| **Replaced** by xtask | 83 | ~14,150 |
| **Partially** replaced | 4 | ~689 |
| **Not Started** | 2 | ~1,128 |
| **Keep** as shell | 12 | ~1,367 |
| **Total scripts** | 101 | ~16,334 |

## Migration Criteria

### CONVERT to xtask when the script has:

- Complex logic (branching, loops, error recovery)
- CI gate role (failures block merges)
- Need for structured output (JSON receipts, reports)
- Cross-platform requirements
- Interaction with Cargo workspace metadata
- More than ~50 lines of non-trivial logic

### KEEP as shell when the script is:

- A trivial wrapper (under ~15 lines, just calls another tool)
- Platform-specific by design (e.g., `install.sh` for curl-pipe, `llvm.sh`)
- GitHub CLI (`gh`) heavy with no Rust benefit
- One-time/historical (validation scripts for past issues)
- A debugging aid not used in CI

## Recommended Migration Order

### Wave 1 -- Remaining Low-Priority Migrations

These scripts are the remaining not-started items.

1. **`lib_gh.sh`** (178 lines) -- Shared GitHub API helper library

## Cleanup Candidates

The following scripts are already fully replaced by xtask subcommands and can be deleted once the justfile recipes are updated to use `cargo xtask` instead:

| Script | Replaced By |
|--------|-------------|
| `execute-gate.sh` | `cargo xtask gates --gate <name>` |
| `run-gates.sh` | `cargo xtask gates` |
| `gate-local.sh` | `cargo xtask gates` / `cargo xtask ci` |
| `generate-receipt.sh` | `cargo xtask gates --receipt` |
| `generate-receipts.sh` | `cargo xtask gates --receipt` |
| `list-gates.py` | `cargo xtask gates --list` |
| `prepare-release.sh` | `cargo xtask release-turnkey` |
| `forbid-fatal-constructs.sh` | `cargo xtask forbid-fatal-constructs` |
| `build-timing-receipt.sh` | `cargo xtask build-timing-receipt` |
| `compare-build-timing.sh` | `cargo xtask compare-build-timing` |
| `inject-sha-assets.sh` | `cargo xtask inject-sha-assets` |
| `devex-targeted-checks.sh` | `cargo xtask targeted-checks` |
| `publish-release.sh` | `cargo xtask publish-release` |
| `publish-receipts.sh` | `cargo xtask publish-receipts` |
| `cargo-package-workspace-dry-run.sh` | `cargo xtask publish-crates --dry-run` |
| `generate-badges.sh` | `cargo xtask ci-hygiene generate-badges` |
| `install-githooks.sh` | `cargo xtask ci-hygiene install-githooks` |
| `populate-book.sh` | `cargo xtask populate-book` |
| `verify-publication-facts.sh` | `cargo xtask verify-publication-facts` |
| `ensure-labels.sh` | `cargo xtask gh-labels` |
| `issues-needing-triage.sh` | `cargo xtask gh-triage` |
| `backfill-prefixed-labels.sh` | `cargo xtask gh-backfill-prefixed-labels` |
| `prep-crates-io-launch.sh` | `cargo xtask prep-crates-io-launch` |
| `update-parser-matrix.py` | `cargo xtask parser-matrix` |
| `validate-workspace-exclusions.sh` | `cargo xtask validate-workspace-exclusions` |
| `validate_features.sh` | `cargo xtask features verify` |
| `ignored-test-count.sh` | `cargo xtask ci-hygiene ignored-test-count` |
| `preflight.sh` | `cargo xtask ci-hygiene preflight` |
| `e2e-gate.sh` | `cargo xtask ci-hygiene e2e-gate` |
| `test-capped.sh` | `cargo xtask ci-hygiene test-capped` |
| `quick-receipts.sh` | `cargo xtask ci-hygiene quick-receipts` |
| `verify_stacker.sh` | `cargo xtask ci-hygiene verify-stacker` |
| `test-e2e-capped.sh` | `cargo xtask ci-hygiene test-e2e-capped` |
| `test-lsp-cancellation.sh` | `cargo xtask ci-hygiene test-lsp-cancellation` |
| `test_iterative_parser.sh` | `cargo xtask ci-hygiene test-iterative-parser` |
| `run_parser_comparison.sh` | `cargo xtask ci-hygiene run-parser-comparison` |
| `cleanup-worktrees.sh` | `cargo xtask worktree-cleanup` |
| `benchmark_all.sh` | `cargo xtask bench` |
| `benchmark_pure_rust_vs_c.sh` | `cargo xtask compare` |
| `benchmark_rust_vs_c_simple.sh` | `cargo xtask compare` |
| `ci-cost-monitor.sh` | `cargo xtask ci-cost-monitor` |
| `.ci/scripts/measure-ci-baseline.sh` | `cargo xtask ci-baseline` |
| `compare_all_levels.sh` | `cargo xtask compare` |
| `run_actual_benchmark.sh` | `cargo xtask bench` |
| `run_comparison_benchmarks.sh` | `cargo xtask compare` |
| `run_comparison.sh` | `cargo xtask compare` |
| `run_comprehensive_benchmark.py` | `cargo xtask bench` |
| `simple_bench.sh` | `cargo xtask bench` |
| `quick_bench.sh` | `cargo xtask bench` |
| `optimized_benchmark.py` | `cargo xtask bench` |
| `generate_comparison.py` | `cargo xtask compare --report` |
| `test_comparison.py` | `cargo xtask compare` |
| `test_edge_cases.sh` | `cargo xtask test-edge-cases` |
| `benchmarks/scripts/run-benchmarks.sh` | `cargo xtask bench-run` |
| `benchmarks/scripts/compare.sh` | `cargo xtask compare` |
| `benchmarks/scripts/compare.py` | `cargo xtask compare --report` |
| `benchmarks/scripts/format-results.py` | `cargo xtask bench-format` |
| `benchmarks/scripts/alert.py` | `cargo xtask bench-alert` |
| `benchmarks/scripts/extract-criterion.py` | `cargo xtask bench-extract` |
| `benchmarks/scripts/test_alert_system.sh` | `cargo xtask bench-alert-test` |
| `scripts/security-hardening.sh` | `cargo xtask security-hardening` |
| `scripts/performance-hardening.sh` | `cargo xtask performance-hardening` |
| `scripts/e2e-validation.sh` | `cargo xtask e2e-validate` |
| `scripts/production-gates-validation.sh` | `cargo xtask production-gates-validation` |
| `scripts/forensics/pr-harvest.sh` | `cargo xtask forensics-harvest` |
| `scripts/forensics/temporal-analysis.sh` | `cargo xtask forensics-temporal` |
| `scripts/forensics/telemetry-runner.sh` | `cargo xtask forensics-telemetry-full` / `cargo xtask forensics-telemetry-quick` |
| `scripts/forensics/dossier-runner.sh` | `cargo xtask forensics-dossier` |
| `scripts/forensics/render-dossier.sh` | `cargo xtask forensics-render` |

**Before deleting**: Update the corresponding justfile recipes to call `cargo xtask` and verify that the xtask subcommand produces equivalent behavior (exit codes, output format, receipt schema).
