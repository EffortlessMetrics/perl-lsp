---
description: Manage reusable worktree slots with query, allocate, release, and cleanup lifecycle commands
---

# Worktree Manager

Use this skill when you need to manage the repo's reusable worktree pool.
It is the canonical surface for slot lifecycle, not the low-level git cleanup
script.

## Ownership Model

- Policy surface: this skill and `/worktree-manager`
- Mutable runtime state: `.ops-perl-lsp/worktree-manager/state.json`
- Physical worktrees: sibling managed root outside the tracked repo,
  defaulting to `<repo-parent>/<repo-name>-worktrees/<slot>/`
- Low-level cleanup: `scripts/cleanup-completed-worktrees.sh`

The manager tracks slots, not just paths. A slot can be queried, allocated,
released for reuse, or cleaned up when stale.

## Core Rules

- Query before allocating if you want to reuse an existing slot.
- Allocate into a named slot, not into an ad-hoc path.
- Release marks a slot reusable; it does not delete the worktree.
- Cleanup prunes stale or retired slots after the state has been synced.
- Do not mutate the runtime state by hand unless the manager is broken.

When a hook or wrapper knows the agent identity, pass it as
`WORKTREE_MANAGER_OWNER` or `--owner` so the slot record carries a lead-readable
owner label through `query --json` and the table view. If no owner is provided,
the slot remains unowned rather than inheriting stale data from a prior task.
## Lifecycle

### Query

Show the current pool, including active, idle, missing, and stale slots.

```bash
python3 scripts/worktree-manager.py query
```

### Allocate

Reserve a slot for a new task. Prefer reusing an idle slot when one exists.

```bash
python3 scripts/worktree-manager.py allocate --slot issue-2157 --branch issue/2157 --owner builder-2157
```

### Release

Mark a completed slot as reusable once the worktree is clean and the work is
ready for handoff or cleanup.

```bash
python3 scripts/worktree-manager.py release --slot issue-2157 --owner builder-2157
```

Passing the same owner on release helps catch mismatched releases; once release
completes, the slot becomes idle/retired and its current-owner field is cleared.

### Cleanup

Remove stale idle slots and reconcile the state file with git worktree reality.

```bash
python3 scripts/worktree-manager.py cleanup
```

## When To Use The Lower-Level Cleanup

Use `scripts/cleanup-completed-worktrees.sh` when you only need a one-off prune.
Use this skill when you need slot reuse, explicit release, or lifecycle
tracking.
