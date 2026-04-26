# Agent leases and idempotent receipts

This document defines xtask primitives for disconnected agent execution where stale
or late workers must not mutate canonical state.

## Commands

```bash
cargo xtask agent lease acquire --task <task.json> --out target/agent/lease.json
cargo xtask agent lease verify --lease target/agent/lease.json --current <snapshot.json>
cargo xtask agent receipt validate --receipt <receipt.json>
```

## Task contract

`agent-task.schema.json` requires:

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

## Enforcement rules

- **Stale head:** receipt and lease state transitions are ignored when `head_sha` does
  not match current snapshot head.
- **Expired lease:** any post-expiry mutation is rejected.
- **Idempotency:** `idempotency_key` must equal `task_id`; same task updates comment
  state idempotently.
- **Supersedence:** newer receipts may supersede older receipts via
  `supersedes_receipt_id`.
- **Mutation allowlist:** each applied mutation must appear in `allowed_mutations` and
  must not appear in `forbidden_mutations`.

## Fixtures

Reference fixtures live in `xtask/tests/fixtures/agent-leases/` and cover:

- valid lease + snapshot verification
- expired lease rejection
- stale head rejection
- forbidden mutation receipt rejection
