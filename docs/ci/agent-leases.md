# Agent leases and idempotent receipts

This document defines the local primitives used by disconnected orchestration to avoid stale or late agent updates corrupting canonical state.

## Commands

```bash
cargo xtask agent lease acquire --task <task.json> --out target/agent/lease.json
cargo xtask agent lease verify --lease target/agent/lease.json --current <snapshot.json>
cargo xtask agent receipt validate --receipt <receipt.json>
```

## Task shape

Task JSON conforms to `.ci/receipts/schemas/agent-task.schema.json` and includes:

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

## Receipt rules

Receipt JSON conforms to `.ci/receipts/schemas/agent-receipt.schema.json` and is validated with these rules:

- stale head receipts are ignored for state reconciliation
- expired leases reject mutation
- same `task_id` is idempotent for comment/update writes
- newer receipt sequence supersedes older receipts
- mutations not listed in `allowed_mutations` are rejected
