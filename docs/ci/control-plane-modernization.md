# CI/Control-Plane Modernization Implementation Plan

This document describes the staged rollout for issue #6853 and related P0 work. It codifies a receipt-driven control plane where state is derived, auditable, and resilient to agent variance.

## Target architecture

The modernization model is built on the following invariants:

- Agents do **not** own canonical state.
- Agents emit **receipts**.
- Receipts are the evidence layer.
- A reconciler derives canonical state from receipts.
- Labels are projected UI, not authoritative storage.
- CI enforces invariants at merge boundaries.
- Routes are derived automatically from evidence/state.
- `merge_group` is the final pre-merge truth boundary.

## Staged rollout

## P0 — impossible states impossible

P0 establishes guardrails so contradictory outcomes cannot be represented as valid:

- #6855 Methodology Gate
- #6856 Receipt schema registry
- #6857 Final aggregator
- #6858 `merge_group` triggers
- #6859 Merge-ready SHA binding

Design intent in P0:

- Introduce invariant checks before behavior expansion.
- Ensure routing-critical decisions cannot bypass aggregation.
- Ensure merge-ready assertions are bound to concrete SHAs.

## P1 — receipts -> state -> labels

P1 introduces the core pipeline:

1. Producers emit receipts.
2. Reconciler/state builder computes canonical state.
3. Label projection updates UI labels from canonical state.

In this stage, labels are explicitly downstream and non-authoritative.

## P2 — Parser Ratchet scoped gate

P2 adds a scoped ratchet gate for parser quality/safety envelopes.

- Ratchet behavior must be explicit and reviewable.
- Scope boundaries must be clear to avoid accidental global enforcement.
- Receipt evidence should record ratchet decisions and outcomes.

## P3 — leases/worktree/queue health

P3 adds reliability controls around execution coordination:

- Lease lifecycle integrity.
- Worktree hygiene and ownership boundaries.
- Queue health and starvation/fairness checks.

Receipts should capture lifecycle transitions and terminal outcomes so the reconciler can detect drift and stuck states.

## P4 — release evidence and scenario gates

P4 formalizes release-grade evidence and scenario-level gates:

- Release assertions are evidence-backed.
- Scenario gates move beyond unit checks into end-to-end control-plane behavior.
- Final merge confidence is established through receipt-backed aggregation.

## Workflow and policy notes

- Required-style workflows must run on every relevant event and no-op internally when not applicable.
- Do **not** path-filter required-style workflows.
- Required status checks at GitHub Rulesets level are a future/conventional state until rulesets are updated.
