# Codex Agent Implementation Guide for CI/Control-Plane Modernization

This guide captures how Codex-driven implementation work should be executed while #6853 is rolled out.

## Core operating model

- Agents emit receipts; they do not own canonical state.
- Labels are UI projection only.
- Reconciler/state builder is the future source of truth for label projection.
- CI gates enforce invariants and aggregation boundaries.

## Codex agent guardrails

- Inspect current `master`/latest branch tip first before implementing.
- If the target is already implemented, create a minimal follow-up or report no-op.
- Keep PRs tightly scoped to one concern.
- Do not edit unrelated high-churn global files.
- Do not claim branch/ruleset enforcement changed unless it was actually changed.

## Workflow rules for implementation PRs

- Required-style workflows must always run; they may no-op internally.
- Do not apply path filters to required-style workflows.
- Ensure workflow triggers include:
  - `pull_request`
  - `merge_group`
  - `push` on `master`
- Use event-aware concurrency keys to avoid accidental cancellation across event types.
- Use final aggregators to produce authoritative pass/fail signals.

## Label projection expectations

Current/future behavior should be documented and implemented with this split:

1. Agents emit receipts.
2. Reconciler/state builder derives canonical state.
3. Labels are projected from canonical state as UI.

Until full reconciler ownership lands, avoid introducing any new direct label-as-state coupling.

## Partial-closeout hygiene

For PRs that scaffold or partially implement a larger issue:

- Use `Refs #issue` or `Part of #issue`.
- Do **not** use `Closes/Fixes/Resolves #issue` unless acceptance criteria are complete.

For #6853 rollout PRs that are not final completion, use `Refs #6853`.
