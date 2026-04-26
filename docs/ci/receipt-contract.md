# Receipt Contract

This document defines the contract for evidence receipts used by the CI/control-plane model.

## Purpose

Receipts are durable evidence artifacts. They are emitted by routing-critical and gate-critical automation, then consumed by a reconciler/state builder to derive canonical control-plane state.

## Locations

### Runtime receipts

- Path: `target/receipts/*.json`
- Producer: CI/runtime jobs and gate steps
- Lifecycle: ephemeral build artifacts per run

### Committed schemas

- Path: `.ci/receipts/schemas/*.schema.json`
- Producer: repository-maintained contract definitions
- Lifecycle: versioned in git; reviewed like code

### Registry

- Path: `.ci/receipts/registry.toml`
- Purpose: authoritative mapping of receipt kinds to schema and validation metadata

## Required behaviors

1. All routing-critical gates emit receipts.
2. Receipts are validated against committed schemas.
3. Registry entries are the control point for receipt discoverability and validation wiring.
4. Reconciliation uses receipts as source evidence; labels are projection-only output.

## Non-goals

- Receipts are not a mutable shared state store.
- Labels are not a canonical state backend.

## Evolution rules

- Additive schema evolution is preferred.
- Breaking schema changes require coordinated registry updates and reconciler updates.
- Partial implementation PRs should reference parent issues with `Refs`/`Part of`, not closeout keywords.
