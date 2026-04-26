# Worktree allocator (local agent orchestration)

The worktree allocator introduces an explicit lease model for local agent worktree creation and cleanup.

## Commands

- `cargo xtask agent worktree acquire --pr <N> --base origin/master`
- `cargo xtask agent worktree release --id <id>`
- `cargo xtask agent worktree list`
- `cargo xtask agent worktree gc --stale`

## State model

Each lease tracks:

- `worktree_id`
- `path`
- `agent_task_id`
- `pr`
- `branch`
- `base_sha`
- `owner`
- `lease_expiry_epoch_secs`
- `last_heartbeat_epoch_secs`

State is persisted under `.claude/worktrees/allocator-state.json`.

## Safety rules

- The allocator will not lease the same writable branch twice.
- Nested agent worktree paths are rejected.
- `release` is explicit and lease-driven.
- `gc` is dry-run by default; pass `--apply` to remove stale worktrees.
- GC and release both print exact paths before deletion.
- Worktrees with uncommitted changes are never deleted unless `--force` is set.

## Receipts

On successful acquisition, a receipt is written to:

- `target/receipts/worktree-leases/<worktree_id>.json`

The receipt schema is defined at:

- `.ci/receipts/schemas/worktree-lease.schema.json`
