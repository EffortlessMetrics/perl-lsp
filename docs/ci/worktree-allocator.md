# Worktree allocator (local agent orchestration)

The worktree allocator introduces an explicit lease model for locally-created agent worktrees.
It prevents writeable-branch collisions and gives operators a safe way to list, release, and
garbage-collect stale worktrees.

## Commands

```bash
cargo xtask agent worktree acquire --pr <N> --base <ref>
cargo xtask agent worktree release --id <worktree_id>
cargo xtask agent worktree list
cargo xtask agent worktree gc --stale
```

## Lease state

Allocator state is written to `.claude/worktrees/lease-state.json` with:

- `worktree_id`
- `path`
- `task_id`
- `pr`
- `branch`
- `base_sha`
- `owner`
- `lease_expiry`
- `last_heartbeat`

Acquire also writes a receipt at `target/receipts/worktree-lease.json`.

## Safety rules

- No writable branch can be leased twice.
- No nested agent worktree paths (for `.claude/worktrees` descendants below one level).
- Release is explicit (`release --id ...`).
- GC is dry-run by default.
- GC prints exact candidate paths before removal.
- GC will not remove worktrees with uncommitted changes unless `--force` is provided.

## Destructive cleanup

Use `--apply` to execute removals:

```bash
cargo xtask agent worktree gc --stale --apply
```

Use `--force` only when you intend to drop dirty worktrees:

```bash
cargo xtask agent worktree gc --stale --apply --force
```
