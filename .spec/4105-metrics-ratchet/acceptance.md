# Acceptance Criteria — Scorecard Metrics Ratchet Infrastructure (#4105)

## Behavioral Assertions (Grid-Complete)

- [ ] **Load baseline from committed file** | `load_baseline(repo_root, "parser")` reads and parses `.ci/metrics/baselines/parser.json` without error | `xtask/src/tasks/metrics/ratchet.rs:load_baseline()` function | `cargo test -p xtask -- metrics::load_baseline`
- [ ] **Detect floor regression: lower-is-better metric** | `check_floor_metrics()` detects when `system_crash_count` increases from baseline 0 to current 1 and returns a violation | `xtask/src/tasks/metrics/ratchet.rs:check_floor_metrics()` logic for `_count` suffix | `cargo test -p xtask -- metrics::check_floor_metrics` with mock data
- [ ] **Detect floor regression: higher-is-better metric** | `check_floor_metrics()` detects when `system_clean_rate` drops from baseline 0.971 to current 0.950 and returns a violation | `xtask/src/tasks/metrics/ratchet.rs:check_floor_metrics()` logic for rate metrics | `cargo test -p xtask -- metrics::check_floor_metrics` with mock data
- [ ] **Tolerance band prevents false positives** | `check_floor_metrics()` with `tolerance_pct: 0.005` does NOT report violation when `system_clean_rate` drops by 0.3% (0.968 vs 0.971 baseline) | `xtask/src/tasks/metrics/ratchet.rs:check_floor_metrics()` tolerance check logic | `cargo test -p xtask -- metrics::tolerance_band`
- [ ] **Null metrics are skipped** | `check_floor_metrics()` silently skips any metric with null value in current or baseline (no error, no violation) | `xtask/src/tasks/metrics/ratchet.rs:check_floor_metrics()` `Some(bv)` guard | `cargo test -p xtask -- metrics::null_handling`
- [ ] **Ratchet check exits 0 on master** | `cargo xtask metrics ratchet-check parser` exits with code 0 when run against current master (no regressions) | `xtask/src/main.rs` MetricsRatchetCheck handler + `check_floor_metrics()` | verify command on master
- [ ] **Ratchet check exits nonzero on violation** | `cargo xtask metrics ratchet-check parser --current <file>` exits with code 1 when current-metrics JSON has regression (e.g., `system_clean_rate: 0.90`) | `xtask/src/main.rs` MetricsRatchetCheck handler exit logic | manual test with synthetic current-metrics
- [ ] **Fallback to existing baselines** | When `target/receipts/metrics/parser.json` absent, `MetricsRatchetCheck` reads from `.ci/parser-corpus-baseline.json` via existing `read_sweep_report()` | `xtask/src/main.rs` MetricsRatchetCheck handler fallback logic + `xtask/src/tasks/update_status.rs:read_sweep_report()` existing function | verify path on master
- [ ] **Record run to stable-wins state** | `cargo xtask metrics ratchet-check parser --record` writes `target/metrics/stable_wins/parser.json` with N recent runs recorded | `xtask/src/tasks/metrics/stable_wins.rs:record_run()` + handler logic | verify file created and contains correct structure
- [ ] **Promote-baseline identifies stable improvements** | After 3 consecutive `--record` runs with improved values, `cargo xtask metrics promote-baseline parser --delta-pct 0.01` prints eligible metrics (those stable across runs and improved >=1%) | `xtask/src/tasks/metrics/stable_wins.rs:stable_improvements()` + handler logic | `cargo test -p xtask -- metrics::stable_improvements`
- [ ] **Scorecard labels created in repo** | All 7 `scorecard/*` labels present in repo (checked via `gh label list --json name`) | `git label create` commands run by builder (label creation is not code, is operational step) | `gh label list --json name | jq '.[] | select(.name | startswith("scorecard"))'` returns 7 rows
- [ ] **CI gate integration: ratchet-check runs in ci-gate** | `just ci-gate` executes `ci-metrics-ratchet` recipe which calls `cargo xtask metrics ratchet-check parser` and `cargo xtask metrics ratchet-check engineering_health` | `justfile` lines 724+ includes `just ci-metrics-ratchet &&` before final `exit 0` | `just ci-gate` complete run succeeds
- [ ] **PR-fast passes with no-op baseline check** | `just pr-fast` executes `pr-fast-metrics-ratchet` which calls `cargo xtask metrics ratchet-check parser --current .ci/metrics/baselines/parser.json` (baseline vs self always passes) | `justfile` contains `pr-fast-metrics-ratchet` recipe | `just pr-fast` complete run succeeds
- [ ] **Clippy passes with no warnings** | `cargo clippy -p xtask` produces no new warnings in metrics code | `xtask/src/tasks/metrics/ratchet.rs` and `xtask/src/tasks/metrics/stable_wins.rs` follow Rust style | `cargo clippy -p xtask -- -D warnings` completes with exit 0
- [ ] **Gate policy configured** | `.ci/gate-policy.yaml` has entry for `ci-metrics-ratchet` under workflow_integration.job_mapping.ci-gate | `.ci/gate-policy.yaml` new lines added | file content verification

## Structural Assertions (Non-Grid)

- [ ] `.ci/metrics/baselines/parser.json` created with current master metrics (schema_version: 1, floor_metrics with 7 fields, improvement_metrics with 3 fields as null)
- [ ] `.ci/metrics/baselines/engineering_health.json` created (mostly null improvement_metrics, strict_clean_subset_pass_rate: 1.0)
- [ ] `xtask/src/tasks/metrics/mod.rs` created (pub mod ratchet; pub mod stable_wins;)
- [ ] `xtask/src/tasks/mod.rs` updated: `pub mod metrics;` added
- [ ] `xtask/src/main.rs` updated: Commands enum has MetricsRatchetCheck and MetricsPromoteBaseline variants with full handler implementations
- [ ] `justfile` updated: ci-metrics-ratchet recipe added, pr-fast-metrics-ratchet recipe added, ci-gate target updated to include ratchet
- [ ] No use of unwrap/expect/panic in new metrics code (error propagation via `?` and Result<T>)

## Gates (Pre-Verify Checklist)

- `ls .ci/metrics/baselines/` shows parser.json and engineering_health.json
- `cargo xtask metrics ratchet-check parser` exits 0 on master
- `cargo test -p xtask -- metrics::` passes all unit tests (load, check, tolerance, null handling, stable improvements)
- `cargo clippy -p xtask -- -D warnings` no new warnings
- `gh label list --json name | jq '.[] | select(.name | startswith("scorecard"))' | wc -l` returns 7
- `just ci-gate` complete run succeeds (or shows ratchet pass if parser corp not swept yet)
- `just pr-fast` complete run succeeds

## Context

> This is the infrastructure layer that enables scorecard floor metrics. The floor-metric pattern already exists for parser (`enforce_ratchet()` at `xtask/src/tasks/parser_corpus_sweep.rs:764`) and CPAN (`cpan_corpus::ratchet()` at `xtask/src/tasks/cpan_corpus.rs:694`). This spec generalizes the pattern into reusable `SubsystemBaseline` struct and `check_floor_metrics()` function.

> The stable-wins tracking is NEW infrastructure for multi-run stability (Layer 3). It enables re-baselining improvements only when they've been stable for N consecutive runs (currently N=3). This prevents noisy metrics from inflating baselines.

> Scorecard labels are operational (not code): 7 new labels for parser/diagnostics/workspace/module-resolution/editor-intelligence/dap/engineering-health scorecards. These labels link issues to the scorecards that measure them.

> Coordination point: #4063 builder will emit `target/receipts/metrics/parser.json` from `parser-stats --json` command; this builder's fallback logic (reading `.ci/parser-corpus-baseline.json`) bridges until #4063 lands.
