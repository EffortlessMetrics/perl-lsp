# Ruleset Rollout Plan for Receipt-Driven CI

This document explains how to align GitHub Rulesets with the control-plane modernization tracked in **#6853**.

## Current state

- The repository uses **GitHub Rulesets** (not classic branch protection).
- Required checks are **not yet configured at the Ruleset layer**.
- Treat “required check” language in this guide as the intended end-state until rulesets are updated.

## Workflow invariants to enforce

For control-plane reliability, required-style workflows must be structurally predictable:

1. No path filters for required-style workflows.
2. Include events:
   - `pull_request`
   - `merge_group`
   - `push` to `master`
3. Use event-aware concurrency (avoid cross-event cancellation of truth signals).
4. Use final aggregators to produce canonical pass/fail outputs.

Required-style workflows should always run and no-op internally when work is out-of-scope.

## Why `merge_group` matters

`merge_group` is the final pre-merge truth surface. Any control-plane signal used for merge confidence must execute and aggregate on `merge_group`, not only on `pull_request`.

## Staged rollout alignment

- **P0**: impossible states impossible via methodology gate, schema registry, final aggregator, `merge_group` triggers, and merge-ready SHA binding.
- **P1**: receipt evidence reconciled into canonical state, then projected to labels.
- **P2**: Parser Ratchet scoped gate in the same aggregator model.
- **P3**: leases/worktree/queue health evidence.
- **P4**: release evidence and scenario gates.

## Label projection and policy boundaries

- Labels are projected UI state, not policy authority.
- Agents emit receipts only; they do not own canonical state.
- Reconciler/state-builder is authoritative for derived status and label projection.

## Partial-closeout hygiene during rollout

Use issue closing keywords according to completion level:

- Partial/scaffold PRs: `Refs #...` or `Part of #...`
- Only fully complete acceptance criteria PRs: `Closes` / `Fixes` / `Resolves`

This prevents premature closure while the rollout is staged across P0–P4.
