# Receipt Contract (Control-Plane Modernization)

This contract defines where receipts/schemas live and what must emit evidence.

## Required locations

- Generated runtime receipts: `target/receipts/*.json`
- Committed receipt schemas: `.ci/receipts/schemas/*.schema.json`
- Committed schema registry: `.ci/receipts/registry.toml`

## Contract rules

1. All routing-critical gates emit receipts.
2. Receipts are the evidence source; they are not optional metadata.
3. Committed schemas and registry define the contract surface for receipt validation.
4. Runtime receipts are ephemeral artifacts and should not be treated as committed state.
5. Avoid shared mutable global state files; prefer sharded config and evidence composition.

## Why this contract exists

The contract creates a stable interface between:

- Producers (agents/workflows/gates),
- Validators (schema + registry), and
- Reconciler/aggregator consumers.

This separation keeps state derivation deterministic and auditable.
