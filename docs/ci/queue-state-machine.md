# Queue State Machine (Dry-Run)

Refs #6853.

This document describes the first **dry-run** canonical PR state builder.

## Commands

```bash
cargo xtask queue snapshot --out target/queue/snapshot.json
cargo xtask queue state --snapshot target/queue/snapshot.json --dry-run --receipt target/receipts/queue-state.json
```

## Inputs

- PR facts from GitHub event payload when available.
- Snapshot fixture JSON in tests.
- Labels, `head_sha`, `base_sha`, and status rollup.
- Receipts (from snapshot-provided paths).

## Outputs

The dry-run receipt emits:

- `canonical_state`
- `blockers`
- `stale_receipts`
- `projected_next_routes`
- `projected_labels` (projection only; no apply)
- `contradictions`

## Dry-run guarantees

- No label mutations.
- No PR comments.
- No merge operations.
- No label projector apply mode in this phase.

