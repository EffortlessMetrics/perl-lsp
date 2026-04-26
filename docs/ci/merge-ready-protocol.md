# Merge-ready receipt protocol

`merge-ready` is only meaningful when it is bound to a verifiable receipt for an exact PR head/base pair under an exact CI gate graph.

## Receipt contract

The receipt is emitted to `target/receipts/merge-readiness.json` (or caller-provided path) and validated against `.ci/receipts/schemas/merge-readiness.schema.json`.

Mandatory fields:

- `check`: always `merge-readiness`
- `schema_version`
- `event`
- `pr`
- `head_sha`
- `base_sha`
- `gate_graph_version`
- `required_checks`
- `review_evidence`
- `blocker_labels_absent`
- `verdict`
- `expires_when`

Supported verifier statuses:

- `valid`
- `stale_head`
- `stale_base`
- `stale_gate_graph`
- `blocked`
- `missing`

## Gate-graph version

`gate_graph_version` is a deterministic SHA-256 over sorted path/content pairs from:

- `.ci/policies/required-checks.toml`
- `.ci/policies/**`
- `.ci/gates.d/**` (if present)
- `.github/workflows/*` files that mention required checks from policy

By construction it excludes timestamps, run IDs, and nondeterministic ordering.

## Commands

```bash
cargo xtask merge-ready emit --pr <N> --receipt target/receipts/merge-readiness.json
cargo xtask merge-ready verify --pr <N>
cargo xtask merge-ready reconcile --dry-run
cargo xtask merge-ready reconcile --apply
```

Reconciler rollout mode is **advisory/dry-run by default**. Apply mode is opt-in.
