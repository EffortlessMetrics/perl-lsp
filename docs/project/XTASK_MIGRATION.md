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
| `ci` | `ci.rs` | 117 | Lean CI suite (format + clippy + tests) |
| `check-only` | `ci.rs` | -- | Format and clippy checks only |
| `build` | `build.rs` | 69 | Build with configurable features/mode |
| `test` | `test.rs` | 148 | Run tests with suite/coverage options |
| `bench` | `bench.rs` | 355 | Run benchmarks |
| `compare` | `compare.rs` | 1174 | C vs Rust benchmark comparison |
| `doc` | `doc.rs` | 35 | Generate documentation |
| `check` | `check.rs` | 58 | Code quality checks (clippy, fmt) |
| `fmt` | `fmt.rs` | 43 | Format code |
| `clean` | `clean.rs` | 47 | Clean build artifacts |
| `dev` | `dev.rs` | 178 | Development server with watch |
| `parse-rust` | `parse_rust.rs` | 54 | Run pure Rust parser on a file |
| `release` | `release.rs` | 223 | Prepare a release |
| `bump-version` | `bump_version.rs` | 184 | Bump version numbers across project |
| `publish-crates` | `publish.rs` | 203 | Publish crates to crates.io |
| `publish-vscode` | `publish.rs` | -- | Publish VSCode extension |
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
| `corpus` | `corpus.rs` | 625 | Corpus tests (legacy feature) |
| `highlight` | `highlight.rs` | 272 | Highlight tests (parser-tasks feature) |
| `bindings` | `bindings.rs` | 49 | Generate bindings (parser-tasks feature) |

## Migration Status

### scripts/ (top-level)

| Script | Lines | Xtask Equivalent | Status | Priority | Notes |
|--------|------:|-------------------|--------|----------|-------|
| `dead-code-check.sh` | 457 | -- | **Not Started** | High | Called from `just dead-code*`; complex logic with baseline comparison |
| `execute-gate.sh` | 112 | `cargo xtask gates` | **Replaced** | -- | Xtask `gates` subsumes single-gate execution with receipts |
| `run-gates.sh` | 165 | `cargo xtask gates` | **Replaced** | -- | Xtask `gates` covers full gate runs |
| `gate-local.sh` | 135 | `cargo xtask gates` / `cargo xtask ci` | **Replaced** | -- | WSL-safe local gate; xtask handles parallelism natively |
| `generate-receipt.sh` | 133 | `cargo xtask gates --receipt` | **Replaced** | -- | Receipt generation built into xtask gates |
| `generate-receipts.sh` | 121 | `cargo xtask gates --receipt` | **Replaced** | -- | Batch receipt generation |
| `list-gates.py` | 21 | `cargo xtask gates --list` | **Replaced** | -- | Gate listing |
| `forbid-fatal-constructs.sh` | 12 | -- | **Not Started** | High | Policy gate; called from justfile; small but CI-critical |
| `check-version-sync.sh` | 12 | `cargo xtask bump-version` (partial) | **Partial** | Medium | Version sync check could be a `--check` flag on bump-version |
| `update-current-status.py` | 454 | -- | **Not Started** | High | Core truth-surface script; called from justfile `status-*` recipes |
| `debt-report.py` | 436 | -- | **Not Started** | High | Debt ledger reporter; called from `just debt-*` recipes |
| `debt-pr-summary.py` | 36 | -- | **Not Started** | Low | Small PR summary formatter; pipes from debt-report |
| `check_features_invariants.py` | 104 | `cargo xtask features verify` (partial) | **Partial** | Medium | Features invariant checking partially covered by xtask features |
| `ci-audit-workflows.py` | 123 | -- | **Not Started** | Medium | CI spend audit; called from justfile |
| `update-parser-matrix.py` | 255 | -- | **Not Started** | Low | Generates parser feature matrix from corpus audit report |
| `release-turnkey-pr.sh` | 431 | `cargo xtask release` (partial) | **Partial** | High | Full release orchestration; xtask release handles prep but not GH workflow triggering |
| `prepare-release.sh` | 61 | `cargo xtask release` | **Replaced** | -- | Basic release prep covered by xtask |
| `publish-release.sh` | 94 | `cargo xtask publish-crates` | **Replaced** | -- | Crate publishing |
| `publish-receipts.sh` | 68 | -- | **Not Started** | Low | Post-publish receipt archival |
| `install.sh` | 236 | -- | **Keep** | -- | Curl-pipe installer; must remain shell for `curl \| bash` UX |
| `install-githooks.sh` | 13 | -- | **Keep** | -- | Simple git hook installer; shell is the right tool |
| `lsp-smoke.sh` | 107 | `cargo xtask test-lsp` (partial) | **Partial** | Medium | LSP smoke test over JSON-RPC; xtask test-lsp covers demo scripts |
| `smoke-test-release.sh` | 74 | -- | **Not Started** | Medium | Post-release binary smoke test |
| `ci-cost-monitor.sh` | 409 | -- | **Not Started** | Low | GitHub Actions cost analysis; uses `gh api` heavily |
| `close-duplicate-prs.sh` | 63 | -- | **Keep** | -- | One-off GitHub housekeeping |
| `inject-sha-assets.sh` | 140 | -- | **Not Started** | Low | Release asset SHA injection |
| `update-homebrew.sh` | 132 | -- | **Not Started** | Low | Homebrew formula update; release-only |
| `populate-book.sh` | 135 | -- | **Not Started** | Low | mdBook content assembly |
| `render-docs.sh` | 130 | `cargo xtask doc` (partial) | **Partial** | Low | Full doc rendering pipeline; xtask doc handles cargo doc only |
| `build-timing-receipt.sh` | 197 | -- | **Not Started** | Low | Build timing data collection |
| `compare-build-timing.sh` | 214 | -- | **Not Started** | Low | Build timing comparison |
| `validate-workspace-exclusions.sh` | 97 | -- | **Not Started** | Low | Workspace exclusion validation |
| `validate_features.sh` | 71 | `cargo xtask features verify` | **Replaced** | -- | Feature validation |
| `validate_tests.sh` | 234 | -- | **Not Started** | Low | Test infrastructure validation |
| `validate-phase1.sh` | 85 | -- | **Keep** | -- | One-time phase validation; historical |
| `validate_issue_146.sh` | 201 | -- | **Keep** | -- | One-time issue validation; historical |
| `verify-test-infrastructure.sh` | 141 | -- | **Not Started** | Low | Test infra checks |
| `verify_stacker.sh` | 11 | -- | **Keep** | -- | Trivial one-liner |
| `devex-doctor.sh` | 109 | -- | **Not Started** | Medium | Developer environment diagnostics |
| `devex-targeted-checks.sh` | 124 | -- | **Not Started** | Medium | Targeted devex checks |
| `test-semver-integration.sh` | 107 | -- | **Not Started** | Low | SemVer integration tests |
| `test-lsp-cancellation.sh` | 12 | -- | **Not Started** | Low | LSP cancellation test |
| `cargo-package-workspace-dry-run.sh` | 47 | `cargo xtask publish-crates --dry-run` | **Replaced** | -- | Dry-run packaging |
| `prep-crates-io-launch.sh` | 79 | -- | **Not Started** | Low | Pre-publish checklist |
| `llvm.sh` | 233 | -- | **Keep** | -- | LLVM toolchain setup; platform-specific by nature |
| `security-hardening.sh` | 292 | -- | **Not Started** | Low | Production hardening; Phase 6 one-time |
| `performance-hardening.sh` | 334 | -- | **Not Started** | Low | Production hardening; Phase 6 one-time |
| `e2e-validation.sh` | 463 | -- | **Not Started** | Low | E2E validation; Phase 6 one-time |
| `e2e-gate.sh` | 11 | -- | **Keep** | -- | Trivial wrapper |
| `production-gates-validation.sh` | 333 | -- | **Not Started** | Low | Production gates; Phase 6 one-time |
| `preflight.sh` | 11 | -- | **Keep** | -- | Trivial wrapper |
| `test-capped.sh` | 11 | -- | **Keep** | -- | Trivial test wrapper |
| `test-e2e-capped.sh` | 11 | -- | **Keep** | -- | Trivial test wrapper |
| `quick-receipts.sh` | 12 | -- | **Keep** | -- | Trivial wrapper |
| `ignored-test-count.sh` | 12 | -- | **Keep** | -- | Simple grep-based counter |

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
| `run_parser_comparison.sh` | 11 | `cargo xtask compare` | **Replaced** | -- | Parser comparison |
| `setup_benchmark.sh` | 244 | `cargo xtask bench` (partial) | **Partial** | Low | Benchmark environment setup |
| `simple_bench.sh` | 49 | `cargo xtask bench` | **Replaced** | -- | Simple benchmark |
| `quick_bench.sh` | 69 | `cargo xtask bench` | **Replaced** | -- | Quick benchmark |
| `optimized_benchmark.py` | 171 | `cargo xtask bench` | **Replaced** | -- | Optimized benchmarks |
| `generate_comparison.py` | 489 | `cargo xtask compare --report` | **Replaced** | -- | Comparison report generation |
| `generate_issue_summary.py` | 133 | -- | **Keep** | -- | One-time issue summary |
| `test_comparison.py` | 385 | `cargo xtask compare` | **Replaced** | -- | Comparison tests |
| `quick_test.py` | 57 | -- | **Keep** | -- | Quick ad-hoc test helper |
| `test_edge_cases.sh` | 12 | `cargo xtask test-edge-cases` | **Replaced** | -- | Edge case tests |
| `test_iterative_parser.sh` | 11 | -- | **Keep** | -- | Trivial test runner |
| `profile_stack_overflow.sh` | 51 | -- | **Keep** | -- | Debugging aid |
| `apply-workspace-simplification.sh` | 86 | -- | **Keep** | -- | One-time refactoring script |
| `deduplicate-crates.sh` | 93 | -- | **Keep** | -- | One-time deduplication |
| `generate-badges.sh` | 11 | -- | **Keep** | -- | Trivial badge generation |

### scripts/gh/

| Script | Lines | Xtask Equivalent | Status | Priority | Notes |
|--------|------:|-------------------|--------|----------|-------|
| `ensure-labels.sh` | 63 | -- | **Keep** | -- | GitHub label management; `gh` CLI is the right tool |
| `issues-needing-triage.sh` | 26 | -- | **Keep** | -- | GitHub triage query |
| `backfill-prefixed-labels.sh` | 68 | -- | **Keep** | -- | One-time label backfill |

### scripts/forensics/

| Script | Lines | Xtask Equivalent | Status | Priority | Notes |
|--------|------:|-------------------|--------|----------|-------|
| `pr-harvest.sh` | 408 | -- | **Not Started** | Low | PR data harvesting |
| `temporal-analysis.sh` | 694 | -- | **Not Started** | Low | Temporal analysis |
| `telemetry-runner.sh` | 1376 | -- | **Not Started** | Medium | Telemetry collection; largest script |
| `dossier-runner.sh` | 285 | -- | **Not Started** | Low | Dossier generation |
| `render-dossier.sh` | 590 | -- | **Not Started** | Low | Dossier rendering |
| `lib_gh.sh` | 178 | -- | **Not Started** | Low | Shared GitHub API helpers |

### .ci/scripts/

| Script | Lines | Xtask Equivalent | Status | Priority | Notes |
|--------|------:|-------------------|--------|----------|-------|
| `measure-ci-time.sh` | 114 | `cargo xtask gates --receipt` (partial) | **Partial** | Low | CI timing measurement |
| `measure-ci-baseline.sh` | 409 | -- | **Not Started** | Low | Baseline CI timing |
| `check-from-raw.sh` | 27 | -- | **Keep** | -- | Small CI helper |

### benchmarks/scripts/

| Script | Lines | Xtask Equivalent | Status | Priority | Notes |
|--------|------:|-------------------|--------|----------|-------|
| `run-benchmarks.sh` | 194 | `cargo xtask bench` | **Replaced** | -- | Benchmark runner |
| `format-results.py` | 246 | -- | **Not Started** | Medium | Benchmark result formatting |
| `compare.sh` | 13 | `cargo xtask compare` | **Replaced** | -- | Comparison wrapper |
| `compare.py` | 276 | `cargo xtask compare --report` | **Replaced** | -- | Comparison analysis |
| `alert.py` | 479 | -- | **Not Started** | Medium | Performance regression alerts |
| `extract-criterion.py` | 195 | -- | **Not Started** | Low | Criterion output parser |
| `test_alert_system.sh` | 233 | -- | **Not Started** | Low | Alert system tests |
| `test_regression.py` | 31 | -- | **Keep** | -- | Small regression test |

## Summary

| Category | Count | Total Lines |
|----------|------:|------------:|
| **Replaced** by xtask | 29 | ~3,800 |
| **Partially** replaced | 8 | ~1,600 |
| **Not Started** | 31 | ~9,500 |
| **Keep** as shell | 24 | ~1,400 |
| **Total scripts** | 92 | ~16,300 |

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

### Wave 1 -- CI-Critical (High Priority)

These scripts are in the CI gate path and would benefit most from Rust error handling and cross-platform support.

1. **`dead-code-check.sh`** (457 lines) -- Complex baseline comparison logic, called from 5 justfile recipes
2. **`forbid-fatal-constructs.sh`** (12 lines) -- Small but CI-critical policy gate
3. **`update-current-status.py`** (454 lines) -- Core truth-surface generator, called from justfile
4. **`debt-report.py`** (436 lines) -- Debt ledger reporter, called from 5 justfile recipes

### Wave 2 -- Release Flow (High Priority)

5. **`release-turnkey-pr.sh`** (431 lines) -- Full release orchestration with GH workflow triggering
6. **`smoke-test-release.sh`** (74 lines) -- Post-release verification

### Wave 3 -- Developer Experience (Medium Priority)

7. **`devex-doctor.sh`** (109 lines) -- Environment diagnostics
8. **`devex-targeted-checks.sh`** (124 lines) -- Targeted checks
9. **`lsp-smoke.sh`** (107 lines) -- LSP smoke testing
10. **`ci-audit-workflows.py`** (123 lines) -- CI spend audit

### Wave 4 -- Benchmarks and Reporting (Medium Priority)

11. **`benchmarks/scripts/format-results.py`** (246 lines) -- Benchmark formatting
12. **`benchmarks/scripts/alert.py`** (479 lines) -- Regression alerting
13. **`check_features_invariants.py`** (104 lines) -- Feature invariant checks (extend `cargo xtask features verify`)

### Wave 5 -- Forensics and Telemetry (Low Priority)

14. **`scripts/forensics/telemetry-runner.sh`** (1376 lines) -- Largest script, telemetry collection
15. **`scripts/forensics/temporal-analysis.sh`** (694 lines)
16. **`scripts/forensics/render-dossier.sh`** (590 lines)

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
| `prepare-release.sh` | `cargo xtask release` |
| `publish-release.sh` | `cargo xtask publish-crates` |
| `cargo-package-workspace-dry-run.sh` | `cargo xtask publish-crates --dry-run` |
| `validate_features.sh` | `cargo xtask features verify` |
| `benchmark_all.sh` | `cargo xtask bench` |
| `benchmark_pure_rust_vs_c.sh` | `cargo xtask compare` |
| `benchmark_rust_vs_c_simple.sh` | `cargo xtask compare` |
| `compare_all_levels.sh` | `cargo xtask compare` |
| `run_actual_benchmark.sh` | `cargo xtask bench` |
| `run_comparison_benchmarks.sh` | `cargo xtask compare` |
| `run_comparison.sh` | `cargo xtask compare` |
| `run_comprehensive_benchmark.py` | `cargo xtask bench` |
| `run_parser_comparison.sh` | `cargo xtask compare` |
| `simple_bench.sh` | `cargo xtask bench` |
| `quick_bench.sh` | `cargo xtask bench` |
| `optimized_benchmark.py` | `cargo xtask bench` |
| `generate_comparison.py` | `cargo xtask compare --report` |
| `test_comparison.py` | `cargo xtask compare` |
| `test_edge_cases.sh` | `cargo xtask test-edge-cases` |
| `benchmarks/scripts/run-benchmarks.sh` | `cargo xtask bench` |
| `benchmarks/scripts/compare.sh` | `cargo xtask compare` |
| `benchmarks/scripts/compare.py` | `cargo xtask compare --report` |

**Before deleting**: Update the corresponding justfile recipes to call `cargo xtask` and verify that the xtask subcommand produces equivalent behavior (exit codes, output format, receipt schema).
