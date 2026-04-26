# Queue state machine (dry-run)

This document describes the first dry-run canonical PR state builder for #6853.

## Commands

```bash
cargo xtask queue snapshot --out target/queue/snapshot.json
cargo xtask queue state --snapshot target/queue/snapshot.json --dry-run --receipt target/receipts/queue-state.json
```

## Inputs

The state builder consumes:

- PR facts from the snapshot (`draft`, `labels`, `head_sha`, `base_sha`, status rollup).
- Receipt JSON files from `target/receipts/` (or `--receipts-dir`), when present.

## Outputs

Per PR, dry-run output includes:

- `canonical_state`
- `blockers`
- `stale_receipts`
- `projected_next_routes`
- `projected_labels` (projection only; no apply mode)
- `contradictions`

## Canonical states

- DRAFT
- NEW
- NEEDS_STANDARDS_REVIEW
- NEEDS_DEEP_REVIEW
- NEEDS_DIFF_AUDIT
- NEEDS_MAINTAINER_REVIEW
- NEEDS_BUILDER_FIX
- NEEDS_DIFF_FIX
- NEEDS_CI_FIX
- NEEDS_CASCADE_UPDATE
- NEEDS_INFRA_FIX
- REVIEWED_WAITING_CI
- CI_GREEN
- MERGE_READY
- QUEUED
- MERGED
- SUPERSEDED
- BLOCKED_UNKNOWN

## Guardrails

- A PR with any `needs-*` blocker cannot be `CI_GREEN` or `MERGE_READY`.
- `merge-ready` requires a valid merge-readiness receipt aligned to current `head_sha` and `base_sha`.
- Review receipts with mismatched `head_sha` are marked stale.
- Red CI without classifier stays conservative (`BLOCKED_UNKNOWN` with blocker context).
- Draft PRs are always `DRAFT`.

## Non-goals (this phase)

- No label mutation.
- No comments.
- No merge action.
- No broad GitHub API behavior in tests.
- No label projector apply mode.
