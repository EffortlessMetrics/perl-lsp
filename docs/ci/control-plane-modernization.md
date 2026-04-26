# CI/Control-Plane Modernization (Issue #6853)

This guide defines the repository-internal rollout for the receipt-driven control plane.

## Target architecture

The modernization target is:

- Agents do **not** own canonical state.
- Agents emit **receipts**.
- Receipts are the evidence substrate.
- A **reconciler/state builder** derives canonical state from receipts.
- Labels are projected UI, not authority.
- CI enforces invariants.
- Routes are derived automatically from canonical state.
- `merge_group` is the final pre-merge truth.

## Repo facts and constraints (current)

- P0 issue set exists and remains active:
  - #6855 Methodology Gate
  - #6856 Receipt schema registry
  - #6857 Final aggregator
  - #6858 `merge_group` triggers
  - #6859 Merge-ready SHA binding
- The repository currently relies on **GitHub Rulesets**, not classic branch protection.
- Required checks are **not yet configured at the ruleset layer**; references to "required checks" in this guide are target/conventional behavior pending ruleset updates.
- Runtime receipts live under `target/receipts/`.
- Committed schemas and registry live under `.ci/receipts/`.

## Staged rollout

### P0 — Make impossible states impossible

Primary objective: prevent contradictory or unverifiable control-plane states.

- Introduce invariant checks that reject impossible transitions.
- Ensure gates fail closed when evidence is missing or malformed.
- Establish the Methodology Gate baseline (#6855).

### P1 — Receipts → state → labels

Primary objective: establish unidirectional derivation.

- Routing-critical jobs emit machine-readable receipts.
- Reconciler derives canonical state from receipts.
- Label projection becomes a downstream rendering of canonical state.
- Introduce schema/registry governance (#6856) and final aggregation (#6857).

### P2 — Parser Ratchet scoped gate

Primary objective: isolate parser-ratchet policy in a scoped, auditable gate.

- Keep ratchet policy encoded as evidence-backed gate logic.
- Avoid broad global coupling; only the scoped gate owns ratchet evaluation.
- Emit receipts so downstream state derivation remains uniform.

### P3 — Leases/worktree/queue health

Primary objective: operational correctness of orchestration substrate.

- Add health evidence for lease safety, worktree hygiene, and queue consistency.
- Model health status through receipts and reconciliation, not mutable shared state files.
- Keep sharded config boundaries to avoid cross-concern drift.

### P4 — Release evidence and scenario gates

Primary objective: release confidence from explicit evidence.

- Add release-oriented evidence receipts.
- Add scenario gates that validate release-critical paths.
- Maintain `merge_group` as final pre-merge truth source.

## Workflow rules (implementation policy)

- Required-style workflows must **always run** and no-op internally when not applicable.
- Do **not** path-filter required-style workflows.
- Add/keep event coverage on:
  - `pull_request`
  - `merge_group`
  - `push` to `master`
- Use event-aware concurrency to avoid stale/cross-event cancellation bugs.
- Use final aggregators so decision status is based on complete evidence.

## Partial-closeout hygiene

- Use `Refs #issue` or `Part of #issue` for scaffolding/partial implementation PRs.
- Use `Closes/Fixes/Resolves #issue` only when acceptance criteria are complete.
- For this modernization track, partial infra/doc/mechanics PRs should default to `Refs`/`Part of`.

## Guardrails

- Do not introduce shared mutable state files for control-plane truth.
- Prefer sharded config and receipt-derived state.
- Do not claim ruleset-required-check enforcement is active until rulesets are actually updated.
