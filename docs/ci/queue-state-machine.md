# Queue State Machine (Dry-Run)

This document defines the dry-run canonical PR state builder introduced for #6853.

## Commands

```bash
cargo xtask queue snapshot --out target/queue/snapshot.json
cargo xtask queue state --snapshot target/queue/snapshot.json --dry-run --receipt target/receipts/queue-state.json
```

## Inputs

The state builder consumes:

- PR facts (draft flag, labels, head/base sha)
- status/check rollup
- receipts from `target/receipts` or fixture paths

## Outputs

`queue-state` receipt emits, per PR:

- `canonical_state`
- `blockers`
- `stale_receipts`
- `projected_next_routes`
- `projected_labels` (projection only; not applied)
- `contradictions`

## Dry-run guarantees

- No label mutations
- No comments
- No merges
- No label projector apply mode

## Rule highlights

- Any `needs-*` blocker prevents `CI_GREEN`/`MERGE_READY`.
- `merge-ready` label requires a valid `merge-readiness` receipt.
- Review receipts become stale when their `head_sha` differs from current `head_sha`.
- Red CI without classifier becomes `BLOCKED_UNKNOWN` (or policy-controlled `NEEDS_CI_FIX`).
- Draft PRs remain in `DRAFT`.
