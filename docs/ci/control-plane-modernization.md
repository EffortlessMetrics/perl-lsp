# CI Control-Plane Modernization

This guide documents the receipt-driven CI/control-plane modernization tracked in **#6853** and broken into P0 issues **#6855–#6859**.

## Target architecture

The target model is:

1. Agents do **not** own state.
2. Agents emit **receipts**.
3. Receipts are **evidence**.
4. A reconciler derives **canonical state** from evidence.
5. Labels are a **projected UI** view of reconciled state.
6. CI enforces invariants (impossible states become impossible).
7. Routing is derived automatically from canonical state and receipt evidence.
8. `merge_group` becomes the final pre-merge truth gate.

## Staged rollout

### P0 — impossible states impossible

P0 establishes control-plane invariants and the baseline constraints:

- **#6855**: Methodology Gate
- **#6856**: Receipt schema registry
- **#6857**: Final aggregator
- **#6858**: `merge_group` triggers
- **#6859**: merge-ready SHA binding

Practical P0 result: state transitions that violate invariants are rejected by CI aggregation logic.

### P1 — receipts -> state -> labels

P1 formalizes the normal data flow:

- Receipts are emitted by agents and routing-critical workflows.
- Reconciler/state builder consumes receipt evidence.
- Labels are projected from reconciled state (never source-of-truth state).

### P2 — Parser Ratchet scoped gate

P2 introduces a scoped ratchet gate for parser quality/control-plane signal quality. It should be integrated as receipt-producing evidence and included in final aggregation logic.

### P3 — leases/worktree/queue health

P3 adds operational health evidence (lease validity, worktree hygiene, queue integrity) into the same receipt+reconciler model.

### P4 — release evidence and scenario gates

P4 extends the same architecture into release readiness and scenario-driven gates so release decisions are evidence-backed and aggregation-enforced.

## Current repository facts and constraints

- The repository uses **GitHub Rulesets**, not classic branch protection.
- Required status checks are **not yet configured at the GitHub Ruleset layer**.
- Therefore, references to “required checks” in this document are a **future/conventional target** until rulesets are updated.
- Runtime receipts belong under `target/receipts/`.
- Committed schemas and registry belong under `.ci/receipts/`.
- Partial implementation PRs use `Refs #<issue>` or `Part of #<issue>` (not `Closes #<issue>`).
- Avoid shared mutable state files; prefer sharded config and evidence-driven reconciliation.
- Required-style workflows must always run and no-op internally when out-of-scope; do not path-filter them.

## Label projection model

Labels are UI affordances only:

- Agents emit receipts.
- Reconciler derives canonical state.
- Label projector updates PR labels from canonical state.

Agents must not directly treat labels as state ownership.
