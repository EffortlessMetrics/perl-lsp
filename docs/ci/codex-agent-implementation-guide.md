# Codex Agent Implementation Guide (Control-Plane Modernization)

This guide defines how implementation agents contribute to the receipt-driven CI/control-plane model tracked by **#6853**.

## Codex agent guardrails

1. Inspect current `master` first so already-merged work is not reimplemented.
2. If the target is already implemented, deliver a minimal follow-up (or explicitly report a no-op).
3. Keep PRs narrowly scoped to one concern.
4. Do not edit unrelated high-churn global files.
5. Do not claim branch/ruleset enforcement was changed unless it was actually changed.

## Operating model for agents

- Agents do not own mutable canonical state.
- Agents emit receipts as evidence.
- Routing-critical operations must emit receipts.
- Reconciler/state builder derives canonical state.
- Labels are projected UI, not authority.

## Receipt locations

- Generated runtime receipts: `target/receipts/*.json`
- Committed schemas: `.ci/schemas/*.schema.{json,yaml}`
- Registry: `.ci/GATE_REGISTRY.toml`

When adding new receipt types, update schema + registry in the same scoped PR.

## Workflow rules agents must preserve

- Required-style workflows must always run and no-op internally when not applicable.
- Do not path-filter required-style workflows.
- For modernization work, ensure workflows are wired for:
  - `pull_request`
  - `merge_group`
  - `push` to `master`
- Use event-aware concurrency groups so events do not cancel unrelated truth-building runs.
- Use final aggregators to publish a single canonical pass/fail signal for the control-plane view.

## Staged rollout expectations

- **P0**: impossible states impossible.
- **P1**: receipts -> state -> labels.
- **P2**: Parser Ratchet scoped gate.
- **P3**: leases/worktree/queue health.
- **P4**: release evidence and scenario gates.

Agent PRs should state which stage they advance and what invariant/evidence boundary they add.

## Partial-closeout hygiene

Use close keywords intentionally:

- For scaffold/partial work: `Refs #6853` or `Part of #6853`.
- Use `Closes` / `Fixes` / `Resolves` only when acceptance criteria for the target issue are complete.

This avoids premature issue closure during staged rollout.
