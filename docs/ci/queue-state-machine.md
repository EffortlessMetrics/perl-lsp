# Queue state machine (dry-run)

This document describes the first **dry-run** canonical queue-state builder for #6853.

## Commands

```bash
cargo xtask queue snapshot --out target/queue/snapshot.json
cargo xtask queue state --snapshot target/queue/snapshot.json --dry-run --receipt target/receipts/queue-state.json
```

## Inputs

The builder reads:

- PR facts from a snapshot (from GitHub via `gh` when available)
- labels, `head_sha`, `base_sha`, and status/check rollup
- receipts under `target/receipts` (or test fixtures)

## Outputs

Each PR emits:

- `canonical_state`
- `blockers`
- `stale_receipts`
- `projected_next_routes`
- `projected_labels` (projection only, no apply)
- `contradictions`

## Guardrails

- Any `needs-*` blocker prevents `CI_GREEN` / `MERGE_READY`.
- `merge-ready` without a valid merge-readiness receipt is not `MERGE_READY`.
- Review receipts are stale when `head_sha` differs.
- Red status without classifier maps to `BLOCKED_UNKNOWN` (or `NEEDS_CI_FIX` if classified).
- Draft PRs are always `DRAFT`.

## Scope

This phase is intentionally read-only:

- no label mutations
- no comment creation
- no merge actions
- no label projector apply mode
