# Task 0.1 Audit: Reconciled Current-State Findings

## Date: 2026-04-30

This file records the post-cleanup state after the Task 0 preflight findings were
reconciled. It supersedes the older crash-era notes that described an orphaned
receipt command, the old generated receipt output path, single `repro` fields,
and old route strings.

## Current Aligned State

- `cargo xtask ux-regression-receipt` is wired through exactly one command path.
- `.github/workflows/ux-regression-gate.yml` writes generated failure receipts to `target/receipts/ux-regression.json`.
- Scenario run receipts write to `target/receipts/editor-ux/`, overridable with `PERL_LSP_UX_RECEIPT_DIR`.
- `.ci/` holds schemas, policy, flake ledger, baselines, and committed metrics. It is not the destination for generated run receipts.
- `UxFailureClass`, `UxRoute`, `UxCiTier`, `UxScenarioResult`, `MetricState<T>`, and `UxComponent` live in `crates/perl-lsp-ux-tests/src/taxonomy.rs`.
- `UxScenarioResult` is limited to `pass`, `fail`, `quarantined`, and `skipped`; `insufficient_data` is represented by `MetricState::InsufficientData`.
- Failure receipts use `canonical_repro` and `friendly_repro`.
- Route mapping uses the shared taxonomy contract:
  - `provider_regression -> provider_fix`
  - `server_crash -> crash_fix`
  - `timeout -> timeout_triage`
  - `infra -> ci_investigation`
  - `matrix_drift -> fixture_update`
  - `baseline_drift -> baseline_update`
  - `test_race | new_test_bug -> test_fix`
  - `unknown -> triage`

## Schema Inventory

| Schema | Location | Status |
|--------|----------|--------|
| `ux-regression.schema.json` | `.ci/schemas/` | Aligned with taxonomy routes, dual repro fields, component, run_id, attempt, platform |
| `ux-scenario-run.schema.json` | `.ci/schemas/` | Added; requires per-case `test_name` |
| `editor-ux.schema.json` | `.ci/schemas/` | Existing scorecard artifact schema |
| `ux-flakes.schema.json` | `.ci/schemas/` | Pending Task 11.1 |

## Known Blockers

Scenario 14 module-resolution cases are tracked in `.ci/ux-flakes.json` with issue
`#7570`, `failure_class: provider_regression`, and `component: module_resolution`.
They should surface as quarantined or known-blocked with unhealthy stability, not
as passing rows.

## Next Guarded Scope

Keep the next implementation slice narrow:

1. Finish `UxRunRecorder` unit coverage.
2. Validate run receipts against `.ci/schemas/ux-scenario-run.schema.json`.
3. Instrument one simple single-test scenario.
4. Instrument one multi-test scenario with distinct `test_name` values.
5. Add a tiny fixture receipt set for last-run `lsp-stats` aggregation.

Do not add rolling windows or broad PR-lane instrumentation until the last-run
aggregation spine is proven end to end.
