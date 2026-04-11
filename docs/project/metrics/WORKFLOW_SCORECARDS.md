# Workflow Scorecard Contracts

This page defines machine-readable contracts for workflow-level scorecards.

The first contract is `editor_ux`: a thin workflow scorecard that sits above
the subsystem scorecards. It is intentionally narrow:

- top-line rows answer whether real editor workflows succeed, stay stable, and
  return useful results quickly;
- component rows point back to subsystem-owned behavior such as hover,
  completion, goto-definition, diagnostics, module resolution, workspace
  freshness, and DAP happy paths; and
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

## Component Rows

- `hover_correctness_rate`
- `hover_declaration_context_accuracy`
- `completion_top5_usefulness_rate`
- `completion_empty_when_should_not_be_empty_rate`
- `goto_definition_exact_hit_rate`
- `cross_workspace_definition_success_rate`
- `rename_success_rate`
- `settled_diagnostics_correctness_rate`
- `module_resolution_workflow_success_rate`
- `multi_root_workspace_navigation_success_rate`
- `dap_happy_path_success_rate`

## What This Does Not Claim

This scorecard is not parser breadth, capability count, mutation score, or a
generic CPU/memory report. Those remain supporting subsystem metrics. The
workflow layer exists to answer the narrower product question:

> when a user opens a realistic project and performs common editor actions, how
> often does perl-lsp behave correctly and quickly?

## Current Scope

The schema and fixture matrix land before a full measured emitter. That keeps
the contract honest: the workflow inventory is executable today, while the
published metrics can arrive once the harness emits pass/stability/latency
receipts per workflow.
