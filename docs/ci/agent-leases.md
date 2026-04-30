# Agent leases and idempotent receipts

This document defines the `cargo xtask agent ...` primitives used by disconnected
orchestration. It does **not** dispatch agents or apply reconciler mutations.

## Commands

```bash
cargo xtask agent lease acquire --task <task.json> --out target/agent/lease.json
cargo xtask agent lease verify --lease target/agent/lease.json --current <snapshot.json>
cargo xtask agent receipt validate --receipt <receipt.json>
```

## Task schema

Task JSON must follow `.ci/receipts/schemas/agent-task.schema.json`.

Required fields:

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

## Behavioral rules encoded in these primitives

- **stale head**: receipt/lease verification fails when `head_sha` differs from current.
- **expired lease**: lease verification and receipt validation reject mutations.
- **idempotency key**: receipt carries `idempotency_key`; reconciler can upsert by `task_id`.
- **newer wins**: receipt carries `received_at`; reconciler can supersede older receipts.
- **allowed mutations only**: receipt validation rejects any mutation not in `allowed_mutations`.

## Scope

These commands intentionally stop at typed validation and invariants so that a
separate reconciler can decide whether to apply or ignore late/stale receipts.
