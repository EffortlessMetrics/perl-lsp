# Ruleset Rollout Notes for Receipt-Driven CI

This document describes how to align repository rulesets and workflow behavior with the receipt-driven control plane.

## Current state

- Repository enforcement is managed with **GitHub Rulesets**.
- Classic branch protection assumptions should not be treated as active.
- Required status checks are not currently configured at the ruleset layer.

Until rulesets are updated, references to "required checks" are forward-looking/conventional.

## Workflow behavior standards

For required-style workflows in the modernization path:

- Do not use path filters.
- Always run on:
  - `pull_request`
  - `merge_group`
  - `push` to `master`
- No-op internally when a gate is non-applicable, rather than skipping trigger coverage.
- Use event-aware concurrency policies.
- Feed a final aggregator that computes completion/truth from all required evidence.

## Label and state interpretation

- Labels are projected UI only.
- Agents should emit receipts.
- State builder/reconciler should derive canonical state and then project labels.

## `merge_group` as final pre-merge truth

- `merge_group` must run the final pre-merge evaluation path.
- Final aggregator output under `merge_group` is the canonical pre-merge verdict.
- Merge-ready decisions should bind to the evaluated SHA evidence chain.

## Issue-closeout hygiene during rollout

- Use `Refs #6853` / `Part of #6853` for incremental rollout PRs.
- Use `Closes/Fixes/Resolves #6853` only when acceptance criteria are complete end-to-end.
