# Agent leases and idempotent receipts

This document defines disconnected orchestration primitives used by `cargo xtask agent`.

## Task shape

`agent-task.schema.json` defines a typed task contract:

- `task_id`
- `snapshot_id`
- `lane`
- `pr`
- `head_sha`
- `base_sha`
- `canonical_state`
- `allowed_mutations`
- `forbidden_mutations`
- `required_output_schema`
- `expires_at`

## Commands

```bash
cargo xtask agent lease acquire --task <task.json> --out target/agent/lease.json
cargo xtask agent lease verify --lease target/agent/lease.json --current <snapshot.json>
cargo xtask agent receipt validate --receipt <receipt.json>
```

## Reconciliation rules

- stale head (`head_sha` mismatch) means receipt output is ignored for state.
- expired lease (`expires_at` in the past) rejects mutations.
- same `task_id` should be comment/update idempotent.
- newer receipts supersede older receipts via `supersedes` linkage and `received_at` ordering.
- receipt `mutation` not in `allowed_mutations` is rejected.

## Scope note

This primitive layer intentionally does **not** dispatch agents, mutate labels, allocate worktrees, or invoke GitHub APIs.
