---
description: Manage reusable worktree slots — query, allocate, release, cleanup
argument-hint: "<query|allocate|release|cleanup> [options]"
user-invocable: true
---

# Worktree Manager

Lifecycle manager for reusable worktree slots.

## Commands

### `query`

Show the current pool, reuse candidates, and recorded owner for each slot.

```bash
python3 scripts/worktree-manager.py query
```

### `allocate`

Claim or reuse a slot for a new task.

```bash
python3 scripts/worktree-manager.py allocate --slot issue-2157 --branch issue/2157 --owner builder-2157
```

When a hook allocates a slot, pass the agent name through `WORKTREE_MANAGER_OWNER`
or `--owner` so the lead can see which agent owns the slot in `query`/JSON output.
If no owner is provided, the slot stays unowned instead of inheriting stale metadata
from a previous run.

### `release`

Mark a slot reusable after the worktree is clean.

```bash
python3 scripts/worktree-manager.py release --slot issue-2157 --owner builder-2157
```

Use the same owner value on release when the hook already knows which agent is
closing the slot. The manager will reject a mismatched release unless `--force`
is set, and it clears the owner field once the slot becomes reusable.

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
