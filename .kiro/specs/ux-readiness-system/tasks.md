# Implementation Plan: Measured UX Readiness System

## Overview

This plan implements the Measured UX Readiness System following the linear spine ordering from the requirements. Each task is a reviewable, complete slice that builds on the previous step.

**Language:** Rust (matching the existing codebase and design document)

**Acceptance pattern for every task:**
```bash
cargo check -p <crate> --all-targets
cargo test -p <crate> <relevant_test_filter>
```
Clippy is scoped to code-quality-sensitive tasks, not every schema/doc change.

**Checkpoint failure protocol:** If targeted acceptance commands fail, stop. Report the exact failing command, include the first error, classify as implementation issue or pre-existing master issue. Do not continue into the next task until resolved or explicitly waived.

**Nonzero test protocol:** A test command is not accepted as scenario verification if it reports `running 0 tests` for the target scenario. Use the integration-test binary target (`cargo test -p perl-lsp-ux-tests --test <file_stem>`) or exact test function names.

## Tasks

- [x] 0. Preflight: current-state alignment and active PR reconciliation
  - [x] 0.1 Inspect current master and in-flight UX receipt PRs
    - Check `xtask/src/tasks/ux_regression_receipt.rs` current state on master
    - Check `.ci/schemas/ux-regression.schema.json` current state
    - Check `.github/workflows/ux-regression-gate.yml` and `.github/workflows/ci.yml` for receipt upload steps
    - Check `target/receipts/` conventions in existing workflows
    - Inspect active PRs: #7540 (receipt hardening), #7561 (CI upload), #7569 (command registration), #7571 (scenario_14 ignores / #7570 tracking)
    - Decide per PR: merge/extend keeper, close duplicate, or rebuild only if current work is wrong
    - There must be exactly one `ux-regression-receipt` command path after this step
  - [x] 0.2 Confirm `cargo xtask ux-regression-receipt` is registered and functional
    - `cargo xtask ux-regression-receipt --help` must succeed
    - CI must write receipts to `target/receipts/`
  - [x] 0.3 Track ignored scenario_14 cases as known-blocked
    - Add scenario_14 ignored test cases to `.ci/ux-flakes.json` (or temporary known-ux-blockers file) with tracking issue #7570
    - Mark state as `active` with `failure_class: provider_regression`, `component: module_resolution`
    - Ensure aggregator/status shows them as quarantined or known-blocked, with unhealthy stability, not pass
  - [x] 0.4 Add current failure classifier fixtures for scenario_14 and scenario_19
    - Add test `classifier_extracts_scenario_14_external_module_failure` using representative log content
    - Add test `classifier_extracts_scenario_14_system_inc_opt_in_failure` using representative log content
    - Add test `classifier_extracts_scenario_19_diagnostics_race_failure` using representative log content
    - Expected: `failure_class = provider_regression`, `component = module_resolution`, `route = provider_fix` for scenario_14; `failure_class = test_race`, `route = test_fix` for scenario_19

- [x] 1. Shared taxonomy module
  - [x] 1.1 Create `crates/perl-lsp-ux-tests/src/taxonomy.rs` with shared enums
    - Define `UxFailureClass`, `UxRoute`, `UxCiTier`, `UxScenarioResult` (pass/fail/quarantined/skipped only — no `insufficient_data`), `MetricState<T>` (Measured/InsufficientData), and `UxComponent` enums
    - All enums: `#[non_exhaustive]`, `Serialize`/`Deserialize`, `#[serde(rename_all = "snake_case")]`
    - Implement `route_for_failure_class()` with precise 1:1 mapping: `provider_regression→provider_fix`, `server_crash→crash_fix`, `timeout→timeout_triage`, `infra→ci_investigation`, `matrix_drift→fixture_update`, `baseline_drift→baseline_update`, `test_race|new_test_bug→test_fix`, `unknown→triage`
    - Add `serde = { workspace = true }` to `perl-lsp-ux-tests` Cargo.toml; prefer `std::time::SystemTime` for timestamps over adding `chrono` unless already present
    - Re-export taxonomy from `lib.rs`
    - If `cargo check -p xtask --all-targets` drags heavy or circular dependencies, split taxonomy into a tiny `perl-ux-metrics` crate instead
  - [x] 1.2 Write table tests for taxonomy enums and route mapping
    - Exhaustive table test: every `UxFailureClass` variant maps to the correct `UxRoute` per the specification table
    - Serde round-trip test: serialize then deserialize each enum variant produces the original value
    - Do not use proptest for finite enum mapping — table tests are clearer and more reviewable
  - [x] 1.3 Migrate `xtask/src/tasks/ux_regression_receipt.rs` to shared taxonomy
    - Remove local `FailureClass` enum and `route_for_class()` function
    - Import `UxFailureClass`, `UxRoute`, `UxComponent`, `route_for_failure_class` from `perl_lsp_ux_tests::taxonomy`
    - Update `infer_failure_class()` to return `UxFailureClass`
    - Add `component` field to `UxRegressionReceipt`
    - Replace single `repro` with `canonical_repro` and `friendly_repro` (dual repro)
    - Change `route` field type from `String` to `UxRoute`
    - Add nullable `run_id`, `attempt`, `platform` fields
    - Update all existing tests; do not weaken existing UX scenario expectations
  - [x] 1.4 Write dual repro completeness test
    - For representative test names, verify receipt contains both non-empty `canonical_repro` (cargo test command) and non-empty `friendly_repro` (just ux-tests shorthand)

- [x] 2. Checkpoint — Verify taxonomy consolidation
  - `cargo check -p perl-lsp-ux-tests --all-targets` passes
  - `cargo check -p xtask --all-targets` passes
  - `cargo test -p xtask ux_regression_receipt` passes (existing + new classifier fixture tests)
  - `cargo test -p perl-lsp-ux-tests taxonomy` passes

- [x] 2.1 Correct UX route semantics test names, assertions, and schema
  - Rename stale tests:
    - `classify_baseline_drift_routes_to_baseline_update`
    - `classify_server_crash_routes_to_crash_fix`
  - Confirm assertions match corrected route mapping:
    - `baseline_drift -> baseline_update`
    - `server_crash -> crash_fix`
  - Confirm `.ci/schemas/ux-regression.schema.json` allows `baseline_update` and `crash_fix`
  - Confirm no emitted UX regression receipt still uses `provider_fix` for `server_crash`
  - Confirm no emitted UX regression receipt still uses `fixture_update` for `baseline_drift`
  - Acceptance: `cargo test -p xtask ux_regression_receipt`

- [x] 3. UxRunRecorder and per-scenario run receipts
  - [x] 3.1 Create `crates/perl-lsp-ux-tests/src/recorder.rs` with UxRunRecorder API
    - `UxRunRecorder::new(workflow_id, scenario_file, test_name, ci_tier, component)` — test_name is required for per-case receipt identity
    - `check(description: &str, condition: bool) -> Result<(), UxCheckFailure>` — records named assertion, returns error on failure so callers can use `?`
    - `mark_request_start(operation: &str)` — records operation start time
    - `mark_first_useful_result(operation: &str)` — records elapsed from corresponding `mark_request_start`; only first call per operation is kept. If no `mark_request_start` preceded it, records `timing_status: "missing_request_start"` and sets timing to null (no fallback to scenario start)
    - `finish_pass()`, `finish_fail(class: UxFailureClass)`, and `finish_skipped(class: UxFailureClass)` returning `UxScenarioRunReceipt`
    - `write_receipt()` writes to `target/receipts/editor-ux/{workflow_id}-{scenario_stem}-{test_name}-{sha_short}.json`, overridable via `PERL_LSP_UX_RECEIPT_DIR`; creates directory if missing; adds `run_id`/`attempt` to filename when available to avoid collisions
    - Define `UxScenarioRunReceipt` with fields: `kind`, `schema_version`, `measured_at`, `run_identity` (sha, branch, run_id, attempt, platform — optional when unavailable), `workflow_id`, `scenario_file`, `test_name`, `component`, `ci_tier`, `result`, `duration_ms`, `time_to_first_useful_result_ms`, `operation_timings`, `assertions`, `failure_class`, `route`, `canonical_repro`, `friendly_repro`
    - `AssertionCounts`: `passed` (nullable u32), `failed` (nullable u32), `basis` (instrumented/not_yet_instrumented), `failed_check_names` (Vec of failed check descriptions)
    - For uninstrumented scenarios: `{ passed: null, failed: null, basis: "not_yet_instrumented" }`
    - Re-export recorder types from `lib.rs`
  - [x] 3.2 Implement `run_ux_scenario` panic-safe wrapper
    - Closure signature: `FnOnce(&mut UxRunRecorder) -> Result<()>`
    - `Ok(())` → `finish_pass()`, write receipt
    - `Err(e)` with `UxScenarioSkip` → `finish_skipped(class)`, write receipt; other `Err(e)` → `finish_fail(classify_error(&e))`, write receipt (non-panic assertion failure via `check()`)
    - Panic → `catch_unwind`, `finish_fail(UxFailureClass::Unknown)`, write receipt, `resume_unwind`. Do not over-classify panics — a missing completion item expressed as `assert!` panic is not necessarily `NewTestBug` without classifier evidence
    - Receipt is always written to disk before re-raising panic
  - [x] 3.3 Write recorder unit tests
    - Deterministic tests for: pass, check failure, panic, skip, quarantine paths
    - Assertion counting: sequence of `check()` calls → `passed` and `failed` counts match, `basis` is `instrumented`, `failed_check_names` contains failed descriptions
    - Operation timing: `mark_request_start` + `mark_first_useful_result` → non-negative timing; `mark_first_useful_result` without `mark_request_start` → null timing with `missing_request_start` status; second call to `mark_first_useful_result` for same operation is ignored
    - Panic-safe: `run_ux_scenario` with panicking body → receipt file exists on disk after wrapper returns
    - Use deterministic tests, not proptest, for these — the input space is small and specific

- [x] 4. Run receipt JSON schema and matrix instrumentation
  - [x] 4.1 Create `.ci/schemas/ux-scenario-run.schema.json`
    - Required: `kind`, `schema_version`, `measured_at`, `run_identity`, `workflow_id`, `scenario_file`, `test_name`, `ci_tier`, `result` (pass/fail/quarantined/skipped), `duration_ms`, `assertions`, `canonical_repro`, `friendly_repro`
    - Optional inside `run_identity`: sha, branch, run_id, attempt, platform; optional top-level fields: `component`, `time_to_first_useful_result_ms`, `operation_timings`, `failure_class`, `route`
    - Assertions: `passed` (nullable int), `failed` (nullable int), `basis` (enum), `failed_check_names` (array of strings)
  - [x] 4.2 Extend `editor_ux_fixture_matrix.json` with instrumentation metadata
    - Add `instrumentation` object per workflow: `{ "run_receipt": bool, "first_useful_result": bool, "protocol_goldens": bool }`
    - Add `component` field per workflow
    - Update `editor_ux_fixture_matrix.rs` integrity test to validate new fields
  - [x] 4.3 Write unit tests validating example receipts against the schema
    - Validate: full-success receipt, partial-failure receipt, uninstrumented receipt, skipped receipt

- [x] 5. Instrument first scenarios
  - [x] 5.1 Instrument one single-test scenario with `run_ux_scenario`
    - Pick a simple passing scenario (e.g., `ux_scenario_01_simple_file`)
    - Wrap with `run_ux_scenario`, add `check()` assertions, mark `first_useful_result`
    - Verify receipt is emitted to `target/receipts/editor-ux/`
    - Update fixture matrix: `run_receipt: true`, `first_useful_result: true`
    - Acceptance command: `cargo test -p perl-lsp-ux-tests --test ux_scenario_01_simple_file -- --test-threads=1 --nocapture`
    - Acceptance output must report nonzero tests, e.g. `running 3 tests`; `running 0 tests` is invalid verification
    - Before verification, clear `target/receipts/editor-ux/`; after verification, confirm JSON receipts exist under that directory
    - Validate at least one emitted receipt has `kind`, `workflow_id`, `scenario_file`, `test_name`, `component`, `ci_tier`, `result`, `duration_ms`, `run_identity`, `canonical_repro`, and `friendly_repro`
  - [x] 5.2 Instrument one multi-test scenario with per-case receipts
    - Pick a scenario with multiple test functions (e.g., a hover/goto scenario)
    - Each test function gets its own `run_ux_scenario` call with distinct `test_name`
    - Verify separate receipt files per test case
    - Confirm aggregator can distinguish pass/fail per case within the same scenario file

- [x] 6. Checkpoint — Verify recorder, receipts, and first instrumented scenarios
  - `cargo check -p perl-lsp-ux-tests --all-targets` passes
  - `cargo test -p perl-lsp-ux-tests recorder` passes
  - `cargo test -p perl-lsp-ux-tests receipt` passes
  - `cargo test -p xtask ux_regression_receipt` passes
  - Receipt files exist in `target/receipts/editor-ux/` after test run

- [x] 7. Last-run scorecard aggregation from receipts
  - [x] 7.1 Extend `xtask/src/tasks/metrics/lsp_stats.rs` with receipt-based aggregation
    - Add `aggregate_from_receipts(receipts_dir, fixture_matrix, flake_ledger)` function
    - Read `UxScenarioRunReceipt` JSON files from `target/receipts/editor-ux/` (or `--receipt-dir` override)
    - Last-run aggregation only (no rolling windows yet)
    - `workflow_pass_rate`: `count(pass) / count(pass + fail)`, excluding quarantined/skipped
    - `workflow_stability_rate`: pass consistency over available receipts per workflow, quarantined counts as unstable; workflows with fewer receipts than minimum threshold → `MetricState::InsufficientData`
    - `p95_time_to_first_useful_result_ms`: 95th percentile from passing scenarios only with non-null timing (correctness-gated); timeouts are failures, not slow successes
    - Per-workflow rows: workflow_id, scenario_file, test_name, subsystem_owner, pass_rate, stability_rate, p95 timing
    - Component metrics: `cross_file_definition_success_rate`, `module_resolution_workflow_success_rate`, `multi_root_workspace_navigation_success_rate`
    - Output conforms to `.ci/schemas/editor-ux.schema.json`
    - Provenance metadata: fixture_matrix path, harness crate, CI tiers
  - [x] 7.2 Implement `MetricState::InsufficientData` handling
    - Zero-receipt workflows → `MetricState::InsufficientData` (not zero, not pass)
    - Below-threshold stability → `MetricState::InsufficientData` with reason
    - Distinguish `InsufficientData` from `skipped`
    - Top-line `workflow_pass_rate` excludes `InsufficientData` workflows from numerator and denominator
  - [x] 7.3 Write aggregation unit tests
    - Pass rate: known receipt set → expected rate
    - Stability: quarantined counts as unstable
    - Latency: only passing scenarios with non-null timing contribute to p95
    - Insufficient data: zero-receipt workflow → `InsufficientData`, not zero
    - Edge cases: empty receipt directory, single receipt, all-quarantined receipts

- [x] 8. Checkpoint — Verify scorecard aggregation
  - `cargo check -p xtask --all-targets` passes
  - `cargo test -p xtask lsp_stats` passes
  - `cargo xtask metrics lsp-stats --json --receipt-dir target/receipts/editor-ux` produces valid output

- [x] 9. Release dashboard rendering
  - [x] 9.1 Extend `xtask/src/tasks/update_status/editor_ux.rs` to render from measured artifacts
    - Consume measured `editor_ux.json` scorecard
    - Render top-line metrics: pass rate, stability rate, p95 latency
    - Render `InsufficientData` rows explicitly (not blank, not zero)
    - Render active quarantine table with owner, issue, failure_class, age, stale flag (>14 days)
    - Placeholder rows for real-workspace, AI completion, editor matrix (show `not_yet_measured`)
    - Release threshold go/no-go summary
  - [x] 9.2 Write dashboard rendering tests
    - Stale quarantine detection: active entry with `first_seen` > 14 days ago → flagged stale
    - `InsufficientData` workflows render as explicit insufficient-data rows
    - Placeholder rows render correctly

- [x] 10. Protocol-visible golden assertions
  - [x] 10.1 Add protocol-visible golden assertions to 3 representative scenarios
    - Completion: assert `label`, `sortText`, rank position
    - Goto-definition: assert target URI and range (or expected-clean null)
    - Diagnostics: assert diagnostic code, range, message classification
    - Compare against golden expected values in scenario code or companion fixtures
    - Do not convert every scenario — make three representative scenarios exact and measurable
    - Do not weaken existing UX scenario expectations
  - [x] 10.2 Write golden assertion edge case tests
    - Expected-clean null for goto-definition
    - Expected-empty diagnostics after fix
    - Correct empty result contributes to latency (correctness-gated)

- [x] 11. Flake ledger and quarantine system
  - [x] 11.1 Create `.ci/schemas/ux-flakes.schema.json` and formalize `.ci/ux-flakes.json`
    - Schema enforces: `test`, `crate`, `subsystem`, `state` (active/resolved), `failure_mode`, `failure_class`, `component`, `issue` (required for active), `owner` (required for active), `first_seen`, `scope` (pr/nightly/release), `quarantine_effect` (non_blocking_pr/release_blocking/advisory), `expires_after_days`, `resolved_in`, `resolved_at`, `notes`
    - Summary: `total`, `active`, `resolved`, `by_subsystem`
    - Seed with scenario_14 entries from Task 0.3
  - [x] 11.2 Wire quarantine into scorecard aggregation and CI gate
    - Aggregator reads `.ci/ux-flakes.json` to identify quarantined scenarios
    - Quarantined failures → `result: quarantined`
    - PR gate: quarantined scenarios non-blocking
    - Nightly/release: quarantined scenarios still execute
    - Quarantined failures count against `workflow_stability_rate`
    - Include quarantine age in per-workflow rows
  - [x] 11.3 Write flake ledger tests
    - Summary consistency: `total` = entry count, `active` = active count, `resolved` = resolved count, `by_subsystem` counts match
    - Schema validation: active entry without owner/issue fails validation
    - Quarantine effect: `non_blocking_pr` does not block PR, `release_blocking` blocks release

- [x] 12. Checkpoint — Verify flake ledger and dashboard
  - `cargo check -p xtask --all-targets` passes
  - `cargo test -p xtask update_status` passes
  - `cargo test -p xtask lsp_stats` passes (quarantine-aware aggregation)

- [x] 13. Scorecard ratchet enforcement
  - [x] 13.1 Wire receipt-based metrics into `cargo xtask ux-scorecard --ratchet-check`
    - Compare receipt-based scorecard metrics against `.ci/metrics/baselines/editor_ux.json`
    - Reuse existing `ratchet::check_floor_metrics()` infrastructure
    - Correctness: fail if below floor minus tolerance
    - Latency: fail if above floor plus tolerance
    - Violation report: metric, baseline, current, regression %
    - Pass silently when all metrics within tolerance
  - [x] 13.2 Write ratchet enforcement tests
    - All pass within tolerance
    - Correctness regression triggers violation
    - Latency regression triggers violation
    - Null baseline/current skipped

- [x] 14. Multi-tier CI execution
  - [x] 14.1 Implement CI tier filtering for scenario execution
    - `pr`: only `pr`-tagged scenarios
    - `nightly`: `pr` + `nightly`
    - `release`: all scenarios
    - Each receipt includes `ci_tier`

- [x] 15. Checkpoint — Verify ratchet and CI tier support
  - `cargo check -p xtask --all-targets` and `cargo check -p perl-lsp-ux-tests --all-targets` pass
  - `cargo test -p xtask ux_scorecard` and `cargo test -p perl-lsp-ux-tests` pass

- [x] 16. Real-workspace baselines
  - [x] 16.1 Implement real-workspace harness
    - Clone and run UX scenarios against real project checkouts in CI
    - Emit `UxScenarioRunReceipt` files in standard format
    - Mojolicious as first target, then DBIx::Class and Catalyst
    - At least one Windows checkout baseline
    - Clone/init failure → `result: skipped`, `failure_class: infra`
  - [x] 16.2 Write real-workspace harness tests
    - Successful run emits receipt
    - Clone failure emits skipped receipt

- [x] 17. AI completion end-to-end validation
  - [x] 17.1 Implement AI completion harness
    - Validate inline completion through perl-lsp from at least one AI provider
    - Emit `UxScenarioRunReceipt` with editor/provider pair, latency, result
    - No provider → `result: skipped`, `failure_class: infra`
    - Dashboard consumes AI completion receipts

- [x] 18. Editor client matrix
  - [x] 18.1 Implement editor client matrix artifact
    - Classify editors: `validated` / `manual` / `docs-only`
    - Structured artifact consumable by dashboard
    - Dashboard displays per-client tier and last-validated date

- [x] 19. Release thresholds
  - [x] 19.1 Define and enforce release thresholds
    - Minimum `workflow_pass_rate`, minimum `workflow_stability_rate`, maximum `p95_time_to_first_useful_result_ms`
    - All thresholds met + baselines pass → `ready`
    - Any threshold not met → `not-ready` with specific failing criteria

- [x] 20. Final checkpoint — Full system verification
  - `cargo check -p perl-lsp-ux-tests --all-targets` passes
  - `cargo check -p xtask --all-targets` passes
  - `cargo test -p perl-lsp-ux-tests` passes
  - `cargo test -p xtask` passes
  - `cargo clippy -p perl-lsp-ux-tests --all-targets -- -D warnings` passes
  - `cargo clippy -p xtask --all-targets -- -D warnings` passes

## Notes

- Tasks marked with `*` are optional property-test tasks (none remain — replaced with deterministic tests inline)
- Receipt unit is test case (test_name), not scenario file — a multi-test scenario file produces multiple receipts
- `insufficient_data` is `MetricState`, not `UxScenarioResult`
- Generated receipts write to `target/receipts/` (not `.ci/`); `.ci/` holds schemas, policy, flake ledger, baselines
- Scorecard aggregation stays in xtask, not the UX test crate
- No timing fallback to scenario start — missing `mark_request_start` produces null timing
- Do not weaken existing UX scenario expectations to make infrastructure green
- Before Task 1, reconcile PRs #7540, #7561, #7569, #7571 — do not create a third parallel receipt path
- Keep the next implementation milestone small: one passing receipt, one failing/panic receipt, one skipped receipt, and one measured `editor_ux.json` row before broad PR-lane instrumentation
- Do not add rolling windows until last-run aggregation, receipt identity, and quarantine handling are proven
- Broad `cargo test -p xtask` failures must be classified before action: UX-local failures block this lane; unrelated parser/token/DAP/update-status failures are separate status drift unless caused by UX changes
- Focused current-lane commands are: `cargo check -p perl-lsp-ux-tests --all-targets`, `cargo test -p perl-lsp-ux-tests receipt`, `cargo test -p perl-lsp-ux-tests --test editor_ux_fixture_matrix`, `cargo test -p perl-lsp-ux-tests --test ux_scenario_01_simple_file -- --test-threads=1 --nocapture`, `cargo check -p xtask --all-targets`, and `cargo test -p xtask ux_regression_receipt`
