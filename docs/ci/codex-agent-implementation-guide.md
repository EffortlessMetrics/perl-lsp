# Codex Agent Implementation Guide for Control-Plane Modernization

This guide defines how Codex-driven implementation work should be performed for the receipt-driven control-plane architecture in `#6853` and linked issues.

## Core operating model

- Agents produce evidence (receipts), not canonical control-plane state.
- Reconciler/state-builder logic derives canonical state from receipts.
- Labels are projected UI and can lag or be recomputed.
- CI gates enforce invariant checks using event-complete execution (`pull_request`, `merge_group`, `push` to `master`).

## Workflow contract

For required-style workflows:

- Include triggers for:
  - `pull_request`
  - `merge_group`
  - `push` on `master`
- Use event-aware concurrency so runs cancel safely by event/ref semantics.
- Use final aggregators to compute authoritative pass/fail from upstream jobs.
- Do **not** path-filter required-style workflows.
- If a workflow is not relevant for a change, it should still start and no-op internally.

## Receipt-first implementation expectations

- Runtime-generated receipts: `target/receipts/*.json`
- Committed schemas: `.ci/receipts/schemas/*.schema.json`
- Committed registry: `.ci/receipts/registry.toml`
- All routing-critical gates must emit receipts.

## Partial-closeout hygiene

When a PR is a scaffold or partial implementation:

- Use `Refs #issue` or `Part of #issue`.
- Do **not** use `Closes #issue`, `Fixes #issue`, or `Resolves #issue` unless acceptance criteria are fully complete.

## Codex agent guardrails

1. Inspect current `master` (or latest local equivalent) before implementing to avoid duplicating already-merged work.
2. If target behavior already exists, create only a minimal follow-up (docs/test hardening) or report a no-op.
3. Keep PRs tightly scoped to one concern.
4. Do not edit unrelated high-churn global files.
5. Do not claim branch/ruleset enforcement has been changed unless it was actually changed.
