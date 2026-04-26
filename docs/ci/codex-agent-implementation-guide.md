# Codex Agent Implementation Guide for CI/Control-Plane Modernization

This guide describes how implementation agents should execute scoped work for the receipt-driven CI/control-plane model.

## Operating model

Agents are executors, not state authorities.

- Do not mutate shared control-plane state as a source of truth.
- Emit receipts for work performed.
- Treat receipt generation as required evidence for routing-critical gates.
- Expect a reconciler/state builder to derive canonical state and later project labels.

## Codex agent guardrails

Before and during implementation:

1. **Inspect current `master` first.**
   - Confirm whether targeted functionality already exists before writing new changes.
2. **If already implemented, do minimal follow-up or no-op.**
   - Prefer gap-fix PRs or report no-op with evidence over duplicate implementation.
3. **Keep PRs scoped.**
   - One concern per PR; avoid opportunistic cleanup.
4. **Avoid unrelated high-churn global files.**
   - Do not edit broad/global files outside scope unless strictly required.
5. **Do not claim enforcement changes that were not made.**
   - In particular, do not claim ruleset/branch enforcement updates unless they are actually changed.

## Receipt-first execution pattern

For routing-critical jobs:

1. Run job logic.
2. Emit runtime receipt to `target/receipts/*.json`.
3. Validate receipt against committed schema.
4. Let the final aggregator consume evidence.
5. Let reconciler derive canonical state and downstream projections.

## Workflow expectations for agent-authored changes

- Required-style workflows must not use path filters.
- Required-style workflows should run on:
  - `pull_request`
  - `merge_group`
  - `push` on `master`
- Workflows may no-op internally based on context, but they should still execute.
- Concurrency should be event-aware.
- Aggregation should happen in explicit final aggregator jobs.

## Partial-closeout language

Use issue verbs precisely:

- Use `Refs #6853` or `Part of #6853` for incremental/scaffold PRs.
- Reserve `Closes/Fixes/Resolves #6853` for full acceptance-criteria completion.

## Current P0 focus map

- #6855 Methodology Gate
- #6856 Receipt schema registry
- #6857 Final aggregator
- #6858 `merge_group` triggers
- #6859 Merge-ready SHA binding

These P0 items collectively implement the baseline where impossible states are rejected and final pre-merge truth comes from `merge_group` evidence.
