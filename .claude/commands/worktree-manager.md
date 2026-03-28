---
description: Manage reusable worktree slots — query, allocate, release, cleanup
argument-hint: "<query|allocate|release|cleanup> [options]"
user-invocable: true
---

# Worktree Manager

Lifecycle manager for reusable worktree slots.

## Commands

### `query`

Show the current pool and reuse candidates.

```bash
python3 scripts/worktree-manager.py query
```

### `allocate`

Claim or reuse a slot for a new task.

```bash
python3 scripts/worktree-manager.py allocate --slot issue-2157 --branch issue/2157
```

### `release`

Mark a slot reusable after the worktree is clean.

```bash
python3 scripts/worktree-manager.py release --slot issue-2157
```

### `cleanup`

Prune stale slots and reconcile state with git worktree state.

```bash
python3 scripts/worktree-manager.py cleanup
```

## Notes

- The manager stores runtime state in `.ops-perl-lsp/worktree-manager/`.
- Managed worktrees live outside the tracked repo by default, in a sibling
  `<repo-name>-worktrees/` directory.
- Use named slots so reuse stays predictable across sessions.
- `cleanup-completed-worktrees.sh` remains the lower-level prune helper.
