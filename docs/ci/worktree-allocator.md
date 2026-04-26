# Worktree allocator (agent orchestration)

The worktree allocator provides lease-based local worktree management for agent orchestration.

## Commands

```bash
cargo xtask agent worktree acquire --pr <N> --base origin/master
cargo xtask agent worktree release --id <id>
cargo xtask agent worktree list
cargo xtask agent worktree gc --stale
```

## Lease state

Leases are persisted at `.claude/worktree-allocator/leases.json`.

Each lease tracks:

- `worktree_id`
- `path`
- `agent_task_id`
- `pr`
- `branch`
- `base_sha`
- `owner`
- `lease_expiry`
- `last_heartbeat`

## Invariants

- A writable branch cannot be checked out in two worktrees.
- Nested worktree paths are rejected.
- Stale leases (TTL expired) are eligible for garbage collection.
- Release is explicit (`release --id ...`).
- Acquire emits a receipt that includes `worktree_id` at `target/receipts/worktree-lease.json`.

## Safety behavior

- `gc --stale` is dry-run by default.
- Destructive cleanup requires `--apply`.
- Removal refuses dirty worktrees unless `--force` is provided.
- Paths are printed before any removal.

## Scope note

This allocator is additive and does not globally change existing agent behavior.
