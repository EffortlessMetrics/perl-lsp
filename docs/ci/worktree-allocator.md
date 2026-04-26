# Worktree Allocator (local agent orchestration)

The worktree allocator introduces lease-managed local worktree creation so agent runs do not collide on writable branches.

## Commands

```bash
cargo xtask agent worktree acquire --pr <N> --base origin/master
cargo xtask agent worktree release --id <id>
cargo xtask agent worktree list
cargo xtask agent worktree gc --stale
```

## Safety rules

- A writable branch may only be checked out in one worktree at a time.
- Agent worktrees are managed under `.claude/worktrees/`.
- `gc` is dry-run by default. Use `--apply` to actually remove stale entries.
- Paths are printed before any removal.
- Worktrees with uncommitted changes are never removed unless `--force` is provided.
- Release is explicit: leases are removed only via `release` or `gc --apply`.

## Lease state

State is persisted at `.claude/worktrees/leases.json` and each acquire writes a receipt at
`target/receipts/worktree-lease-<worktree_id>.json`.

Each lease tracks:

- `worktree_id`
- `path`
- `task_id`
- `pr`
- `branch`
- `base_sha`
- `owner`
- `lease_expiry`
- `last_heartbeat`
