# Receipt Contract (CI Control-Plane)

This document defines the receipt contract for the control-plane modernization tracked by **#6853**.

## Contract summary

- Receipts are machine-readable evidence emitted by agents and routing-critical gates.
- Reconciler/state-builder logic consumes receipts to derive canonical state.
- Labels are projected from canonical state and are UI only.

## Storage and ownership

### Generated runtime receipts

- Path: `target/receipts/*.json`
- Characteristics:
  - ephemeral build/runtime outputs
  - not source-controlled as canonical policy artifacts

### Committed schemas

- Path: `.ci/receipts/schemas/*.schema.json`
- Characteristics:
  - versioned contract for receipt payload validation
  - code-reviewed alongside producer/consumer changes

### Committed registry

- Path: `.ci/receipts/registry.toml`
- Characteristics:
  - catalog of known receipt kinds and schema mappings
  - single source for schema discovery by validators/reconcilers

## Required behavior

1. All routing-critical gates must emit receipts.
2. Every receipt kind must map to a committed schema in the registry.
3. Schema changes and producer changes should land together.
4. Reconciler must treat receipts as evidence, not inferred label state.

## Design constraints

- Avoid shared mutable state files in control-plane logic.
- Prefer sharded config/evidence artifacts that aggregate deterministically.
- Required-style workflows run every time and no-op internally as needed; they are not path-filtered.

## Rollout linkage

- **P0** establishes invariants and baseline machinery (methodology gate, schema registry, aggregator, `merge_group`, merge-ready SHA binding).
- **P1+** scales the same receipt contract into richer state/label projection and domain-specific gates.

## Partial implementation PR semantics

Use issue keywords carefully during staged rollout:

- Partial/scaffold: `Refs #...` or `Part of #...`
- Complete acceptance criteria: `Closes #...`, `Fixes #...`, or `Resolves #...`
