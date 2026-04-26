# Control-Plane Modernization (Issue #6853)

This document describes the repository-internal rollout plan for the CI/control-plane modernization effort tracked by `#6853`.

## Architecture target

The target operating model is:

1. Agents do **not** own canonical state.
2. Agents emit receipts.
3. Receipts are evidence.
4. A reconciler derives canonical state.
5. Labels are projected UI.
6. CI enforces invariants.
7. Routes are derived automatically.
8. `merge_group` becomes final pre-merge truth.

## Ground rules

- Runtime receipts are generated under `target/receipts/`.
- Committed receipt schemas/registry live under `.ci/receipts/`.
- Avoid shared mutable state files; prefer sharded config and deterministic recompute.
- Required-style workflows must always run and internally no-op when not applicable.
- Do not use path filters for required-style workflows.
- The repository currently uses **GitHub Rulesets** (not classic branch protection).
- Treat “required check” wording as a future/conventional target until rulesets are updated to enforce those checks.

## Staged rollout

### P0 — Impossible states impossible

P0 hardens invariants so contradictory or unverifiable control-plane state cannot be accepted.

Tracked P0 items:

- `#6855` Methodology Gate
- `#6856` Receipt schema registry
- `#6857` Final aggregator
- `#6858` `merge_group` triggers
- `#6859` merge-ready SHA binding

Primary objective: make invalid state transitions unrepresentable and force evidence-backed outcomes.

### P1 — Receipts -> state -> labels

P1 codifies receipt-first processing:

- CI/runtime tasks emit receipts.
- A state builder/reconciler consumes receipts.
- Labels are projected from canonical state (never treated as authority).

Primary objective: labels become a view, not control-plane source-of-truth.

### P2 — Parser Ratchet scoped gate

P2 introduces a scoped Parser Ratchet gate as a receipt-producing invariant check.

Primary objective: tighten quality ratchets incrementally without introducing global fragility.

### P3 — Leases/worktree/queue health

P3 extends receipt coverage and reconciliation to coordination health:

- lease integrity
- worktree hygiene
- queue-level readiness

Primary objective: make operational health measurable and enforceable via evidence.

### P4 — Release evidence and scenario gates

P4 applies the same evidence model to release readiness and scenario-based gates.

Primary objective: release/merge decisions are backed by auditable receipt trails and final aggregations.

## Relationship to enforcement

Ruleset and check enforcement should be rolled out in lockstep with receipt confidence:

1. Generate and validate receipts.
2. Reconcile into canonical state.
3. Project labels/UI.
4. Elevate checks into enforcement once signal quality is stable.

Until ruleset-level required checks are fully configured, this doc should not be read as claiming completed enforcement changes.
