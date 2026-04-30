# Requirements Document

## Introduction

The Measured UX Readiness System transforms perl-lsp's existing UX testing contract into a fully wired measurement, classification, trending, and release-gating pipeline on master. The north star: when a user opens a real Perl project and performs normal editor actions, does perl-lsp respond correctly, quickly, and stably? This system provides the infrastructure to answer that question with measured evidence at every commit, PR, nightly, and release boundary.

The system spans eight functional areas: structured failure receipts, per-scenario run receipts, first-useful-result timing, a measured editor_ux scorecard, protocol-visible golden assertions, a flake ledger with quarantine, real-workspace baselines against open-source Perl projects, and a release readiness dashboard with thresholds.

### Implementation Ordering Constraint

Requirements MUST be implemented in the following linear spine. Each step depends on the artifacts produced by the preceding step:

1. **Receipt consolidation** (R0, R1) — Unify enum taxonomy, classify existing failures
2. **Scenario run receipts** (R2, R2b) — Emit per-scenario receipts including panic-safe wrapper
3. **Timing** (R3, R3b) — Operation-scoped first-useful-result instrumentation
4. **Scorecard aggregation** (R4, R4b) — Aggregate receipts into measured scorecard with insufficient-data handling
5. **Status rendering** (R8) — Release dashboard from measured artifacts
6. **Protocol goldens** (R5, R5b) — Assert user-visible LSP fields, correctness-gated latency
7. **Flake/quarantine** (R6, R6b) — Flake ledger, quarantine accountability

### Unified Receipt Taxonomy

Both Failure_Receipts and UxScenarioRunReceipts MUST share a single set of enums to prevent divergence:

- **UxFailureClass**: `provider_regression`, `server_crash`, `timeout`, `test_race`, `infra`, `matrix_drift`, `baseline_drift`, `new_test_bug`, `unknown`
- **UxRoute**: `ci_investigation`, `fixture_update`, `test_fix`, `provider_fix`, `triage`, `baseline_update`, `crash_fix`, `timeout_triage`
- **UxCiTier**: `pr`, `nightly`, `release`
- **UxScenarioResult**: `pass`, `fail`, `quarantined`, `skipped`

These enums MUST be defined once in a shared module and re-exported by both the failure receipt emitter and the run receipt recorder. Parallel enum definitions that could diverge are prohibited.

Insufficient data is a **metric state** (`MetricState::InsufficientData`), not a scenario execution result.

## Glossary

- **UX_Scenario**: A single executable test file (`ux_scenario_*.rs`) in `crates/perl-lsp-ux-tests/tests/` that exercises one user-visible editor workflow against a real LSP server process.
- **UxScenarioRunReceipt**: A machine-readable JSON document emitted after each UX_Scenario execution, containing result, timing, assertion counts, and failure classification. Shares the unified enum taxonomy with Failure_Receipts.
- **Failure_Receipt**: A structured JSON document emitted when a UX_Scenario fails, containing failure classification, panic location, repro commands, and routing information. Shares the unified enum taxonomy with UxScenarioRunReceipts.
- **UxFailureClass**: An enumerated classification of why a UX_Scenario failed (e.g., `provider_regression`, `server_crash`, `timeout`, `test_race`, `infra`, `matrix_drift`, `baseline_drift`, `new_test_bug`, `unknown`). Defined once in a shared module.
- **UxRoute**: A semantic routing hint indicating which team or process should investigate a failure (e.g., `ci_investigation`, `fixture_update`, `test_fix`, `provider_fix`, `triage`, `baseline_update`, `crash_fix`, `timeout_triage`). Defined once in a shared module.
- **First_Useful_Result**: The latency from LSP request initiation (not scenario start) to the first response that delivers product value to the user (e.g., first non-empty completion list, first expected goto-definition location, first valid publishDiagnostics notification).
- **Editor_UX_Scorecard**: The aggregated JSON artifact (`.ci/metrics/editor_ux.json`) containing measured workflow pass rate, stability rate, p95 first-useful-result latency, component metrics, and per-workflow rows.
- **Workflow_Pass_Rate**: The fraction of canonical editor workflows that complete with the expected result in a given CI run.
- **Workflow_Stability_Rate**: Pass consistency over N receipts, excluding skipped scenarios and counting quarantined scenarios as unstable.
- **CI_Tier**: The execution context for a UX_Scenario run: `pr` (fast, on every pull request), `nightly` (full matrix, scheduled), or `release` (comprehensive, before publishing). Defined in the UxCiTier enum.
- **Flake_Ledger**: A structured registry (`.ci/ux-flakes.json`) of UX_Scenarios that exhibit non-deterministic failure, with ownership, issue tracking, and resolution history. Schema at `.ci/schemas/ux-flakes.schema.json`.
- **Quarantine**: A state in which a known-flaky UX_Scenario continues to execute but does not block PR merge gates; quarantined scenarios still count against Workflow_Stability_Rate. Each quarantine entry requires owner, issue, failure_class, and age.
- **Real_Workspace_Baseline**: A measured UX readiness profile obtained by running UX_Scenarios against a real open-source Perl project checkout (e.g., Mojolicious, DBIx::Class, Catalyst).
- **Protocol_Visible_Golden**: An assertion in a UX_Scenario that validates the specific LSP response fields a user actually experiences (e.g., completion label, hover content, diagnostic code/range/message).
- **UxRunRecorder**: A Rust API used within UX_Scenario test code to record assertions, mark operation-scoped first-useful-result timestamps, and emit UxScenarioRunReceipts.
- **UxRegressionReceiptEmitter**: The component (currently `xtask ux-regression-receipt`) that parses test output logs and emits structured Failure_Receipts with classification, routing, and dual repro commands.
- **Release_Dashboard**: The status surface (rendered from measured artifacts) showing current UX pass/stability/latency, active quarantines, real-workspace readiness, and release thresholds.
- **Ratchet**: A CI enforcement mechanism that prevents measured metrics from regressing below established floor baselines.
- **Fixture_Matrix**: The JSON manifest (`editor_ux_fixture_matrix.json`) mapping each UX_Scenario to its workflow ID, CI_Tier, subsystem owner, and confidence signals.
- **run_ux_scenario**: A panic-safe wrapper function using `catch_unwind` that ensures a UxScenarioRunReceipt is emitted on pass, fail, AND panic.

## Requirements

### Requirement 0: Existing Failure Visibility

**User Story:** As a CI operator, I want existing UX failures classified by the new receipt system before changing UX expectations, so that the current failure landscape is visible and categorized before any new requirements alter what counts as a pass or fail.

#### Acceptance Criteria

1. WHEN the receipt consolidation step is complete, THE UxRegressionReceiptEmitter SHALL classify all currently-known UX failures using the unified UxFailureClass enum.
2. THE UxRegressionReceiptEmitter SHALL produce a Failure_Receipt for each existing UX failure in the test suite, using the unified enum taxonomy shared with UxScenarioRunReceipts.
3. WHEN a previously-unclassified failure exists in CI logs, THE UxRegressionReceiptEmitter SHALL assign it a UxFailureClass and UxRoute using the same log-content analysis rules as new failures.
4. THE UxRegressionReceiptEmitter SHALL emit receipts with dual repro fields: `canonical_repro` containing the full `cargo test -p perl-lsp-ux-tests <test_name> -- --test-threads=1 --nocapture` command, and `friendly_repro` containing the `just ux-tests` shorthand.
5. IF an existing failure cannot be classified by log-content analysis, THEN THE UxRegressionReceiptEmitter SHALL set the UxFailureClass to `unknown` and the UxRoute to `triage`.

### Requirement 1: UX Failure Receipt Emission

**User Story:** As a CI operator, I want structured failure receipts emitted when UX scenarios fail, so that I can quickly triage, classify, and route failures without reading raw test logs.

#### Acceptance Criteria

1. WHEN a UX_Scenario test fails in CI, THE UxRegressionReceiptEmitter SHALL produce a JSON document conforming to `.ci/schemas/ux-regression.schema.json` containing: workflow, scenario_file, first_failing_test, panic_location, dual repro commands, UxFailureClass, and UxRoute.
2. THE UxRegressionReceiptEmitter SHALL classify each failure into exactly one UxFailureClass using log content analysis (panic patterns, assertion messages, timeout indicators, infrastructure errors).
3. WHEN the UxFailureClass is `provider_regression`, THE UxRegressionReceiptEmitter SHALL set the UxRoute field to `provider_fix`.
4. WHEN the UxFailureClass is `server_crash`, THE UxRegressionReceiptEmitter SHALL set the UxRoute field to `crash_fix`.
5. WHEN the UxFailureClass is `timeout`, THE UxRegressionReceiptEmitter SHALL set the UxRoute field to `timeout_triage`.
6. WHEN the UxFailureClass is `infra`, THE UxRegressionReceiptEmitter SHALL set the UxRoute field to `ci_investigation`.
7. WHEN the UxFailureClass is `matrix_drift`, THE UxRegressionReceiptEmitter SHALL set the UxRoute field to `fixture_update`.
8. WHEN the UxFailureClass is `baseline_drift`, THE UxRegressionReceiptEmitter SHALL set the UxRoute field to `baseline_update`.
9. WHEN the UxFailureClass is `test_race` or `new_test_bug`, THE UxRegressionReceiptEmitter SHALL set the UxRoute field to `test_fix`.
8. IF the UxFailureClass cannot be determined, THEN THE UxRegressionReceiptEmitter SHALL set the UxFailureClass to `unknown` and the UxRoute to `triage`.
9. THE UxRegressionReceiptEmitter SHALL include dual repro fields: `canonical_repro` containing `cargo test -p perl-lsp-ux-tests <test_name> -- --test-threads=1 --nocapture`, and `friendly_repro` containing the `just ux-tests` shorthand.
10. THE UxRegressionReceiptEmitter SHALL use UxFailureClass and UxRoute enums from the shared unified taxonomy module, not locally-defined duplicates.

### Requirement 2: Per-Scenario Run Receipt Recording

**User Story:** As a metrics pipeline consumer, I want every UX scenario execution to emit a machine-readable run receipt, so that downstream aggregation can compute pass rates, stability rates, and latency percentiles from actual CI data.

#### Acceptance Criteria

1. WHEN a UX_Scenario completes execution (pass or fail), THE UxRunRecorder SHALL emit a UxScenarioRunReceipt JSON document to `target/receipts/editor-ux/` (overridable via `PERL_LSP_UX_RECEIPT_DIR`) containing: workflow_id, scenario_file, test_name, component, ci_tier, result, duration_ms, time_to_first_useful_result_ms, assertions, failure_class, route, canonical_repro, friendly_repro, and run_identity (sha, branch, run_id, attempt, platform).
2. THE UxScenarioRunReceipt result field SHALL use one of the UxScenarioResult enum values: `pass`, `fail`, `quarantined`, or `skipped`.
3. WHEN a UX_Scenario is in quarantine state and fails, THE UxRunRecorder SHALL set the result field to `quarantined` instead of `fail`.
4. THE UxRunRecorder SHALL record duration_ms as wall-clock elapsed time from scenario start to scenario completion.
5. THE UxRunRecorder SHALL record assertion counts using one of two approaches: (A) for new scenarios, an explicit `recorder.check("description", condition)` API that tracks passed and failed counts with descriptions; (B) for existing scenarios not yet instrumented, `assertions: { passed: null, failed: null, basis: "not_yet_instrumented" }`. The UxRunRecorder SHALL NOT claim assertion counts derived from raw `assert!` panics.
6. WHEN a UX_Scenario fails, THE UxRunRecorder SHALL populate the failure_class field using the same UxFailureClass enum from the unified taxonomy as the UxRegressionReceiptEmitter.
7. WHEN a UX_Scenario passes, THE UxRunRecorder SHALL set failure_class to `null`.

### Requirement 2b: Panic-Safe Receipt Emission

**User Story:** As a CI operator, I want a failing or panicking scenario to still emit a run receipt, so that no scenario execution is invisible to the metrics pipeline regardless of how it terminates.

#### Acceptance Criteria

1. THE UxRunRecorder SHALL provide a `run_ux_scenario` wrapper function that uses `catch_unwind` to intercept panics during scenario execution.
2. WHEN a UX_Scenario completes successfully, THE `run_ux_scenario` wrapper SHALL emit a UxScenarioRunReceipt with result `pass`.
3. WHEN a UX_Scenario fails with an assertion error (non-panic), THE `run_ux_scenario` wrapper SHALL emit a UxScenarioRunReceipt with result `fail` and the appropriate UxFailureClass.
4. WHEN a UX_Scenario panics, THE `run_ux_scenario` wrapper SHALL catch the panic via `catch_unwind` and emit a UxScenarioRunReceipt with result `fail`, UxFailureClass `server_crash` or `new_test_bug` as appropriate, and the panic location if available.
5. THE `run_ux_scenario` wrapper SHALL ensure the UxScenarioRunReceipt is written to disk before re-raising the panic (if applicable) so that downstream aggregation always has receipt data.

### Requirement 3: First Useful Result Timing

**User Story:** As a performance engineer, I want latency measurements that reflect product value delivery rather than total CI duration, so that I can track and optimize the time users actually wait for useful editor responses.

#### Acceptance Criteria

1. THE UxRunRecorder SHALL provide a `mark_first_useful_result()` API that scenario test code calls when the first product-valuable response is received.
2. THE UxRunRecorder SHALL record time_to_first_useful_result_ms as the elapsed time from the LSP request initiation to the `mark_first_useful_result()` call.
3. WHEN a completion scenario receives the first non-empty completion response containing an expected item, THE UX_Scenario SHALL call `mark_first_useful_result()`.
4. WHEN a goto-definition scenario receives the first response containing an expected location or an expected clean null, THE UX_Scenario SHALL call `mark_first_useful_result()`.
5. WHEN a diagnostics scenario receives the first valid `textDocument/publishDiagnostics` notification matching expected criteria, THE UX_Scenario SHALL call `mark_first_useful_result()`.
6. WHEN a diagnostics-after-edit scenario receives the first settled diagnostics update or clear after an edit, THE UX_Scenario SHALL call `mark_first_useful_result()`.
7. IF `mark_first_useful_result()` is not called during a scenario, THEN THE UxRunRecorder SHALL set time_to_first_useful_result_ms to `null` in the UxScenarioRunReceipt.

### Requirement 3b: Operation-Scoped Timing

**User Story:** As a performance engineer, I want first-useful-result timing to start at request initiation rather than scenario start, so that latency measurements reflect the actual user-perceived wait time for a specific LSP operation.

#### Acceptance Criteria

1. THE UxRunRecorder SHALL provide a `mark_request_start(operation: &str)` API that scenario test code calls immediately before initiating an LSP request (e.g., `recorder.mark_request_start("completion")`).
2. THE UxRunRecorder SHALL provide a `mark_first_useful_result(operation: &str)` API that scenario test code calls when the first product-valuable response for that operation is received (e.g., `recorder.mark_first_useful_result("completion")`).
3. THE UxRunRecorder SHALL compute time_to_first_useful_result_ms for each operation as the elapsed time from the corresponding `mark_request_start` call to the `mark_first_useful_result` call.
4. WHEN `mark_first_useful_result(operation)` is called without a preceding `mark_request_start(operation)`, THE UxRunRecorder SHALL record `timing_status: "missing_request_start"` and set the operation timing to null. The UxRunRecorder SHALL NOT fall back to measuring from scenario construction time.
5. THE UxScenarioRunReceipt SHALL include per-operation timing entries when multiple operations are timed within a single scenario.

### Requirement 4: Measured Editor UX Scorecard

**User Story:** As a release manager, I want `cargo xtask metrics lsp-stats --json` to aggregate scenario receipts into a measured scorecard with pass rate, stability rate, and p95 latency, so that release readiness is determined by measured evidence rather than manual assessment.

#### Acceptance Criteria

1. WHEN `cargo xtask metrics lsp-stats --json` is invoked, THE Scorecard_Aggregator SHALL read all UxScenarioRunReceipt files from `target/receipts/editor-ux/` (or `--receipt-dir` override) and aggregate them into `.ci/metrics/editor_ux.json`.
2. THE Scorecard_Aggregator SHALL compute workflow_pass_rate as the fraction of non-quarantined, non-skipped scenarios with result `pass`.
3. THE Scorecard_Aggregator SHALL compute workflow_stability_rate as pass consistency over the last N receipts for each scenario, excluding `skipped` results and counting `quarantined` results as unstable.
4. THE Scorecard_Aggregator SHALL compute p95_time_to_first_useful_result_ms as the 95th percentile of all non-null time_to_first_useful_result_ms values across scenario receipts.
5. THE Scorecard_Aggregator SHALL produce per-workflow rows in the output, each containing: workflow id, scenario file, subsystem owner, pass_rate, stability_rate, and p95_time_to_first_useful_result_ms.
6. THE Scorecard_Aggregator SHALL produce component-level metrics for cross_file_definition_success_rate, module_resolution_workflow_success_rate, and multi_root_workspace_navigation_success_rate.
7. THE Editor_UX_Scorecard output SHALL conform to the schema defined in `.ci/schemas/editor-ux.schema.json`.
8. THE Scorecard_Aggregator SHALL include provenance metadata indicating the fixture matrix path, harness crate, and CI tiers included in the aggregation.
9. THE Scorecard_Aggregator SHALL support richer stability signals (diagnostic_flicker_count, empty_result_regression_count, stale_symbol_count) as scenarios are instrumented to report them.

### Requirement 4b: Insufficient-Data Handling

**User Story:** As a release manager, I want missing or incomplete receipt data to produce an explicit `insufficient_data` status rather than zero or pass, so that the scorecard never misrepresents untested scenarios as healthy.

#### Acceptance Criteria

1. WHEN a workflow has no UxScenarioRunReceipts in the receipts directory, THE Scorecard_Aggregator SHALL report that workflow's metric values as `MetricState::InsufficientData` rather than zero or pass.
2. WHEN a workflow has fewer receipts than the minimum threshold for stability computation, THE Scorecard_Aggregator SHALL report workflow_stability_rate as `MetricState::InsufficientData` with a reason note.
3. THE Scorecard_Aggregator SHALL distinguish between `InsufficientData` (no receipts available) and `skipped` (scenario was intentionally not run at this CI_Tier).
4. THE Editor_UX_Scorecard top-line workflow_pass_rate computation SHALL exclude `InsufficientData` workflows from both numerator and denominator.

### Requirement 5: Protocol-Visible Golden Assertions

**User Story:** As a developer, I want UX scenarios to assert the specific LSP response fields that users actually experience, so that regressions in user-visible behavior are caught before release.

#### Acceptance Criteria

1. WHEN a completion UX_Scenario executes, THE UX_Scenario SHALL assert protocol-visible fields: completion item label, sortText, and rank position within the completion list.
2. WHEN a hover UX_Scenario executes, THE UX_Scenario SHALL assert the hover content string returned by the server.
3. WHEN a goto-definition UX_Scenario executes, THE UX_Scenario SHALL assert the target URI and range returned by the server.
4. WHEN a diagnostics UX_Scenario executes, THE UX_Scenario SHALL assert the diagnostic code, range, and message classification returned by the server.
5. WHEN a workspace-symbol UX_Scenario executes, THE UX_Scenario SHALL assert workspace symbol metadata (name, kind, container) returned by the server.
6. WHEN a rename UX_Scenario executes, THE UX_Scenario SHALL assert the workspace edit shape (affected URIs and edit ranges) returned by the server.
7. THE UX_Scenario assertions SHALL compare against golden expected values defined in the scenario test code or companion fixture files.

### Requirement 5b: Correctness Before Latency

**User Story:** As a performance engineer, I want latency metrics to count only correct or expected-clean responses, so that fast-but-wrong results do not inflate latency numbers.

#### Acceptance Criteria

1. THE Scorecard_Aggregator SHALL include a scenario's time_to_first_useful_result_ms in latency percentile computation only when the scenario result is `pass`.
2. WHEN a scenario result is `fail`, `quarantined`, or `skipped`, THE Scorecard_Aggregator SHALL exclude that scenario's timing data from p95_time_to_first_useful_result_ms computation.
3. WHEN a scenario produces a correct empty result (expected-clean null for goto-definition, expected-empty diagnostics after fix), THE Scorecard_Aggregator SHALL include that scenario's timing in latency computation.
4. THE per-workflow rows in the scorecard SHALL indicate whether the timing value was included in or excluded from the top-line latency aggregation.

### Requirement 6: Flake Ledger and Quarantine System

**User Story:** As a CI maintainer, I want known-flaky scenarios tracked in a UX-scoped ledger with quarantine support, so that flakes do not block PR merges while remaining visible, counted against stability, and requiring ownership.

#### Acceptance Criteria

1. THE Flake_Ledger SHALL maintain a structured registry in `.ci/ux-flakes.json` (UX-scoped, separate from the repo-wide `.ci/flaky-tests.json`) where each entry contains: test name, crate, subsystem, state (`active` or `resolved`), failure_mode description, issue number, owner, and first_seen date. The schema SHALL be defined at `.ci/schemas/ux-flakes.schema.json`.
2. WHILE a UX_Scenario is in `active` quarantine state, THE CI_Gate SHALL allow PR merges to proceed even when the quarantined scenario fails.
3. WHILE a UX_Scenario is in `active` quarantine state, THE CI_Gate SHALL continue executing the quarantined scenario in nightly and release CI_Tiers.
4. WHILE a UX_Scenario is in `active` quarantine state, THE Scorecard_Aggregator SHALL count quarantined failures against Workflow_Stability_Rate.
5. THE Flake_Ledger SHALL require each active entry to have an associated GitHub issue number and an owner field identifying the responsible party.
6. WHEN a quarantined UX_Scenario is resolved, THE Flake_Ledger entry SHALL be updated to `resolved` state with the resolving PR number and resolution date.
7. THE Flake_Ledger summary section SHALL maintain counts of total, active, and resolved entries grouped by subsystem.

### Requirement 6b: Quarantine Accountability

**User Story:** As a CI maintainer, I want every quarantine entry to carry structured accountability metadata, so that quarantined scenarios have clear ownership and do not silently age without resolution.

#### Acceptance Criteria

1. THE Flake_Ledger SHALL require every `active` quarantine entry to include: `owner` (GitHub handle), `issue` (GitHub issue number), `failure_class` (UxFailureClass enum value), and `age` (computed from first_seen date).
2. THE Flake_Ledger schema at `.ci/schemas/ux-flakes.schema.json` SHALL enforce `owner`, `issue`, and `failure_class` as required fields for entries with state `active`.
3. WHEN a quarantine entry's age exceeds 14 days, THE Release_Dashboard SHALL highlight the entry as stale and requiring escalation.
4. THE Scorecard_Aggregator SHALL include quarantine age in per-workflow rows for quarantined scenarios.

### Requirement 7: Real-Workspace Baselines

**User Story:** As a release manager, I want UX readiness measured against real open-source Perl project checkouts, so that release confidence reflects actual user project complexity rather than synthetic fixtures only.

#### Acceptance Criteria

1. THE Real_Workspace_Harness SHALL support cloning and running UX_Scenarios against real open-source Perl project checkouts within CI.
2. THE Real_Workspace_Harness SHALL produce UxScenarioRunReceipts in the same format as synthetic UX_Scenario runs, enabling aggregation by the Scorecard_Aggregator.
3. THE Real_Workspace_Harness SHALL include a baseline for the Mojolicious project as the first real-workspace target.
4. THE Real_Workspace_Harness SHALL include baselines for DBIx::Class and Catalyst as additional real-workspace targets.
5. THE Real_Workspace_Harness SHALL include at least one Windows checkout baseline to validate cross-platform UX readiness.
6. WHEN a real-workspace baseline run completes, THE Real_Workspace_Harness SHALL emit receipts that the Scorecard_Aggregator can ingest alongside synthetic scenario receipts.
7. IF a real-workspace checkout fails to clone or initialize, THEN THE Real_Workspace_Harness SHALL emit a receipt with result `skipped` and a descriptive failure_class of `infra`.

### Requirement 8: Release Dashboard and Thresholds

**User Story:** As a release manager, I want a single status surface showing UX readiness with defined thresholds, so that release decisions are based on measured evidence with clear go/no-go criteria.

#### Acceptance Criteria

1. THE Release_Dashboard SHALL display current measured values for: Workflow_Pass_Rate, Workflow_Stability_Rate, and p95_time_to_first_useful_result_ms.
2. THE Release_Dashboard SHALL display the count and details of active quarantined scenarios from the Flake_Ledger, including owner, issue, failure_class, and age for each entry.
3. THE Release_Dashboard SHALL display real-workspace baseline readiness status for each configured real project (Mojolicious, DBIx::Class, Catalyst, Windows checkout).
4. THE Release_Dashboard SHALL display AI completion end-to-end validation status showing at least one validated editor/provider pair.
5. THE Release_Dashboard SHALL display an editor client matrix distinguishing validated, manual-test-only, and docs-only support tiers.
6. THE Release_Dashboard SHALL define and display release thresholds: minimum Workflow_Pass_Rate, minimum Workflow_Stability_Rate, and maximum p95_time_to_first_useful_result_ms required for release.
7. WHEN all release thresholds are met and all required real-workspace baselines pass, THE Release_Dashboard SHALL indicate a `ready` release status.
8. WHEN any release threshold is not met or a required real-workspace baseline fails, THE Release_Dashboard SHALL indicate a `not-ready` release status with specific failing criteria listed.

### Requirement 9: AI Completion End-to-End Validation

**User Story:** As a developer using AI-assisted completion, I want end-to-end validation that at least one editor/provider pair produces useful AI completions through perl-lsp, so that AI completion readiness is part of the release gate.

#### Acceptance Criteria

1. THE AI_Completion_Harness SHALL validate that inline completion requests through perl-lsp produce non-empty responses from at least one configured AI provider.
2. THE AI_Completion_Harness SHALL emit a UxScenarioRunReceipt documenting the validated editor/provider pair, response latency, and result.
3. IF no AI provider is configured or reachable, THEN THE AI_Completion_Harness SHALL emit a receipt with result `skipped` and failure_class `infra`.
4. THE Release_Dashboard SHALL consume AI completion receipts and display the validation status for each tested editor/provider pair.

### Requirement 10: Editor Client Matrix

**User Story:** As a release manager, I want a structured record of which editor clients have been validated at which support tier, so that release notes and the dashboard accurately reflect editor compatibility.

#### Acceptance Criteria

1. THE Editor_Client_Matrix SHALL classify each supported editor client into one of three tiers: `validated` (automated E2E tests pass), `manual` (manual smoke test performed), or `docs-only` (documentation exists but no test evidence).
2. THE Editor_Client_Matrix SHALL be maintained as a structured artifact that the Release_Dashboard can consume.
3. WHEN an editor client has automated E2E test receipts, THE Editor_Client_Matrix SHALL classify that client as `validated`.
4. WHEN an editor client has only manual test evidence, THE Editor_Client_Matrix SHALL classify that client as `manual`.
5. THE Release_Dashboard SHALL display the Editor_Client_Matrix with per-client tier classification and last-validated date.

### Requirement 11: Scorecard Ratchet Enforcement

**User Story:** As a CI maintainer, I want measured UX metrics enforced by a ratchet that prevents regressions below established baselines, so that UX quality monotonically improves over time.

#### Acceptance Criteria

1. WHEN `cargo xtask ux-scorecard --ratchet-check` is invoked, THE Ratchet_Enforcer SHALL compare current measured metrics against floor baselines in `.ci/metrics/baselines/editor_ux.json`.
2. IF any correctness metric (hover, completion, definition, symbol, diagnostics, rename, cross-file) falls below its floor baseline minus the configured tolerance, THEN THE Ratchet_Enforcer SHALL fail the CI gate with a violation report listing each regressed metric, baseline value, current value, and regression percentage.
3. IF any latency metric (p50 or p95 for any request class) exceeds its floor baseline plus the configured tolerance, THEN THE Ratchet_Enforcer SHALL fail the CI gate with a violation report.
4. THE Ratchet_Enforcer SHALL use the tolerance_pct value from the baseline file to determine acceptable variance.
5. WHEN all metrics meet or exceed their baselines within tolerance, THE Ratchet_Enforcer SHALL pass the CI gate silently.

### Requirement 12: Run Receipt Schema and Validation

**User Story:** As a tooling developer, I want a formal JSON schema for run receipts, so that receipt producers and consumers can validate compatibility and detect schema drift.

#### Acceptance Criteria

1. THE UxScenarioRunReceipt_Schema SHALL be defined in `.ci/schemas/ux-scenario-run.schema.json` specifying all required and optional fields with their types and constraints.
2. THE UxScenarioRunReceipt_Schema SHALL require: kind (`ux_scenario_run`), schema_version, measured_at, run_identity, workflow_id, scenario_file, test_name, ci_tier (UxCiTier enum: `pr`, `nightly`, `release`), result (UxScenarioResult enum: `pass`, `fail`, `quarantined`, `skipped`), duration_ms, assertions, canonical_repro, and friendly_repro.
3. THE UxScenarioRunReceipt_Schema SHALL define assertions as an object with fields: passed (nullable integer), failed (nullable integer), and basis (enum: `instrumented`, `not_yet_instrumented`).
4. THE UxScenarioRunReceipt_Schema SHALL define time_to_first_useful_result_ms as an optional numeric field (null when not measured).
5. THE UxScenarioRunReceipt_Schema SHALL define failure_class as an optional field using the UxFailureClass enumeration shared with the Failure_Receipt schema.
6. WHEN a UxScenarioRunReceipt is written, THE UxRunRecorder SHALL validate the receipt against the schema before writing to disk.

### Requirement 13: UxRunRecorder API

**User Story:** As a UX scenario author, I want a simple Rust API for recording assertions and timing within scenario test code, so that receipt emission is consistent and requires minimal boilerplate.

#### Acceptance Criteria

1. THE UxRunRecorder SHALL provide a `new(workflow_id, scenario_file, test_name, ci_tier, component)` constructor that initializes timing and assertion counters. The `test_name` field is required to distinguish individual test cases within a multi-test scenario file.
2. THE UxRunRecorder SHALL provide a `check(description: &str, condition: bool) -> Result<(), UxCheckFailure>` method for new scenarios that records a named assertion check, incrementing the appropriate passed or failed counter, and returning an error on failure for `?` chaining.
3. THE UxRunRecorder SHALL provide a `mark_request_start(operation: &str)` method that records the start time for a specific LSP operation.
4. THE UxRunRecorder SHALL provide a `mark_first_useful_result(operation: &str)` method that records the elapsed time from the corresponding `mark_request_start` call as time_to_first_useful_result_ms for that operation.
5. WHEN `mark_first_useful_result(operation)` is called more than once for the same operation, THE UxRunRecorder SHALL retain only the first recorded timestamp for that operation.
6. THE UxRunRecorder SHALL provide `finish_pass()`, `finish_fail(failure_class: UxFailureClass)`, and `finish_skipped(failure_class: UxFailureClass)` methods that finalize the receipt with duration and result.
7. THE UxRunRecorder SHALL provide a `write_to_default_dir()` method that writes the finalized UxScenarioRunReceipt JSON to `target/receipts/editor-ux/{workflow_id}-{scenario_stem}-{test_name}-{sha_short}.json`, overridable via `PERL_LSP_UX_RECEIPT_DIR`, with optional run_id and attempt filename segments when present.
8. IF the output directory does not exist, THEN THE UxRunRecorder `write_to_default_dir()` method SHALL create it before writing.
9. WHEN a scenario uses existing `assert!` macros and is not yet instrumented with `check()`, THE UxRunRecorder SHALL emit assertions as `{ passed: null, failed: null, basis: "not_yet_instrumented" }`.

### Requirement 14: Multi-Tier CI Execution

**User Story:** As a CI architect, I want UX scenarios executed at different tiers (pr, nightly, release) with appropriate fixture matrices, so that PR feedback is fast while nightly and release runs are comprehensive.

#### Acceptance Criteria

1. THE Fixture_Matrix SHALL tag each workflow entry with a ci_tier field indicating the minimum tier at which the scenario runs: `pr`, `nightly`, or `release` (using the UxCiTier enum).
2. WHEN CI runs at the `pr` tier, THE CI_Runner SHALL execute only scenarios tagged with ci_tier `pr`.
3. WHEN CI runs at the `nightly` tier, THE CI_Runner SHALL execute scenarios tagged with ci_tier `pr` or `nightly`.
4. WHEN CI runs at the `release` tier, THE CI_Runner SHALL execute all scenarios regardless of ci_tier tag.
5. THE UxScenarioRunReceipt emitted by each scenario SHALL include the ci_tier at which the scenario was executed.
