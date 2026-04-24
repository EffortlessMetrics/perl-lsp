# Workflow Scorecard Contracts

This page defines machine-readable contracts for workflow-level scorecards.

The first contract is `editor_ux`: a thin workflow scorecard that sits above
the subsystem scorecards. It is intentionally narrow:

- top-line rows answer whether real editor workflows succeed, stay stable, and
  return useful results quickly;
- current component rows point back to the subsystem-owned behaviors the
  harness proves today: module-resolution success, multi-root workspace
  navigation, and cross-file definition success; and
- the fixture matrix ties each workflow metric to an executable scenario in the
  `perl-lsp-ux-tests` harness.

## Files

- [`.ci/schemas/editor-ux.schema.json`](../../../.ci/schemas/editor-ux.schema.json)
  defines the measured `editor_ux.json` output contract.
- [`crates/perl-lsp-ux-tests/fixtures/editor_ux_fixture_matrix.json`](../../../crates/perl-lsp-ux-tests/fixtures/editor_ux_fixture_matrix.json)
  maps workflow fixtures to scorecard rows and subsystem ownership.

## Top-line Metrics

- `workflow_pass_rate`
- `workflow_stability_rate`
- `p95_time_to_first_useful_result_ms`

## Confidence Signals (tracked workflow counts)

- `manual_editor_smoke` — PR-lane workflows that mirror the editor smoke path
- `first_five_minutes_harness` — all fixture-backed first-5-minutes workflows
- `issue_burndown_regression_guard` — workflows explicitly tagged as regression
  guards for UX issue burn-down

## Current Component Rows

- `cross_file_definition_success_rate`
- `module_resolution_workflow_success_rate`
- `multi_root_workspace_navigation_success_rate`

## Planned Next Rows

These stay intentionally out of the fixture-backed contract until the
underlying scenarios assert exact user-facing outcomes rather than transport or
shape-only success:

- hover correctness and declaration context
- completion usefulness and non-empty expectations
- exact goto-definition hit rate
- rename success
- settled diagnostics correctness after edit
- DAP happy-path success

## What This Does Not Claim

This scorecard is not parser breadth, capability count, mutation score, or a
generic CPU/memory report. Those remain supporting subsystem metrics. The
workflow layer exists to answer the narrower product question:

> when a user opens a realistic project and performs common editor actions, how
> often does perl-lsp behave correctly and quickly?

## Current Scope

The schema and fixture matrix land before a full measured emitter. That keeps
the contract honest: the workflow inventory is executable today, the current
component rows are backed by exact scenario assertions today, and the broader
UX scorecard can expand only when those stronger assertions exist.


## Published Artifact + Ratchet Policy

- `cargo xtask ux-scorecard --format json` emits a stable machine artifact at
  `.ci/metrics/editor_ux_scorecard.json` and refreshes
  `docs/project/status/editor_ux.json` from the same source payload.
- The UX scorecard uses **regression-only ratcheting** via
  `.ci/metrics/baselines/editor_ux.json` +
  `cargo xtask metrics ratchet-check editor_ux --current .ci/metrics/editor_ux_scorecard.json`.
- There are no fixed absolute pass/fail thresholds in this path: better numbers
  are always allowed, regressions relative to baseline are blocked.
