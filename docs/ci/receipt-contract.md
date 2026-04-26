# Receipt Contract for CI/Control-Plane Modernization

This document defines the receipt locations, schema contract, and governance rules for routing-critical CI evidence.

## Normative paths

- **Generated runtime receipts:** `target/receipts/*.json`
- **Committed schemas:** `.ci/receipts/schemas/*.schema.json`
- **Schema registry:** `.ci/receipts/registry.toml`

## Core contract

1. All routing-critical gates MUST emit receipts.
2. Receipts MUST be JSON documents written under `target/receipts/`.
3. Each receipt type MUST have a committed JSON schema under `.ci/receipts/schemas/`.
4. Schema identifiers and metadata MUST be listed in `.ci/receipts/registry.toml`.
5. Receipt validation MUST happen before reconciliation/final aggregation consumes evidence.

## Ownership and derivation

- Agents own receipt emission only.
- Canonical state ownership belongs to reconciler/state builder components.
- Labels are a projection/UI surface and are not canonical state.

## Evolution policy

- Additive schema changes are preferred.
- Breaking schema changes require coordinated registry updates and consumer rollout.
- Runtime receipt payloads should be deterministic and auditable.

## Anti-patterns to avoid

- Using mutable shared state files as an authority instead of receipts.
- Allowing routing-critical gates to pass without emitting evidence.
- Treating labels as a decision authority.
