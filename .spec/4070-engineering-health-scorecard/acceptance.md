# Acceptance Criteria — Engineering Health Scorecard (#4070)

## Behavioral Assertions (Grid-Complete)

- [ ] **Per-crate test count extraction** | `count_tier_a_lib_tests()` in `update_status.rs` parses `cargo test --list` and groups counts by crate name prefix (before `::` separator) | `xtask/src/tasks/update_status.rs` around line 213 (existing function modified) | Unit test: parse synthetic cargo test output, verify grouping by crate
- [ ] **Per-crate test count table generation** | `generate_quality_status()` formats per-crate counts into markdown table with columns: crate | test count | trend | `xtask/src/tasks/update_status.rs:712` replace hard-coded metric generation | Integration test: `cargo xtask status-update`, verify `docs/project/status/quality.md` contains table
- [ ] **Mutation score table generation** | `generate_quality_status()` reads `mutants.out/mutants.json`, groups by crate via jq, formats as markdown table | `xtask/src/tasks/update_status.rs:712` new code path for mutation data (~50 lines) | `cargo test -p xtask -- quality_status` with mock mutants.json
- [ ] **Fallback on missing mutation data** | When `mutants.out/mutants.json` absent, `generate_quality_status()` uses aggregate 87% (no error, graceful degradation) | `xtask/src/tasks/update_status.rs:712` fallback logic | Test with nonexistent JSON path
- [ ] **Null improvement_metrics skipped** | `generate_quality_status()` silently skips null values in improvement_metrics BTreeMap (no error) | `xtask/src/tasks/update_status.rs:712` `if let Some(v) = ...` guard | Test with mixed null/non-null metrics
- [ ] **Cargo clippy clean** | `cargo clippy -p xtask` produces zero new warnings in metrics code | `xtask/src/tasks/update_status.rs` and metrics module follow Rust style | `cargo clippy -p xtask -- -D warnings` exit 0
- [ ] **Status page updated post-generation** | `docs/project/status/quality.md` file exists with header/footer markers and embedded per-crate tables | `xtask/src/tasks/update_status.rs:712` replace_block() calls | Verify file via `cat docs/project/status/quality.md | grep -c "crate\|test_count"`
- [ ] **Scorecard README link** | Root `README.md` status row links to `docs/project/status/index.md` (existing pattern, no change needed) | `README.md` already has this pattern | Verify link in README

## Structural Assertions (Non-Grid)

- [ ] `docs/project/status/quality.md` created with <!-- BEGIN: QUALITY_METRICS_TABLE --> / <!-- END: QUALITY_METRICS_TABLE --> markers
- [ ] `count_tier_a_lib_tests()` refactored to return `BTreeMap<String, usize>` (crate name → count) instead of aggregate total
- [ ] `generate_quality_status()` updated to call `count_tier_a_lib_tests()` and format results
- [ ] New function `parse_mutants_json(path: &Path) -> Result<BTreeMap<String, usize>>` added to parse mutants output
- [ ] No unwrap/expect/panic in new code (error propagation via Result<T>)

## Gates (Pre-Verify Checklist)

- `cargo build -p xtask` compiles without error
- `cargo test -p xtask -- quality_status` passes (if unit test exists)
- `cargo test -p xtask -- count_tier_a` passes (refactored function still works)
- `cargo clippy -p xtask -- -D warnings` no new warnings
- `cargo xtask status-update` succeeds and updates `docs/project/status/quality.md`
- `docs/project/status/quality.md` contains per-crate test count table
- `docs/project/status/quality.md` contains mutation score table (or graceful fallback with 87%)

## Context

> This is the MVP (minimum viable product) for the engineering health scorecard. It surfaces existing signal (per-crate test counts from cargo test --list, per-crate mutation scores from cargo mutants --json output) that is already being collected but not yet visible to developers.

> The scout report identified this as "READY" for per-crate test counts and mutation scores (lowest lift, highest value). Ignored test tracking and flaky test tracking depend on additional data wiring and are deferred to phase 2.

> The mutation score parsing was verified by accuracy-scout: `cargo mutants --json --output <dir>` produces a single `mutants.out/mutants.json` array with per-crate data natively embedded. No post-processing overhead.

> This spec focuses on surfacing data to `docs/project/status/quality.md` (a new file). The scorecard metrics infrastructure (#4105) is separate; this builder does not depend on it landing first (though both feed into the same dashboard eventually).
