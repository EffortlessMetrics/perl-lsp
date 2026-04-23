---
description: Run agent preflight safety checks before making any edits
---

# Agent Preflight

Run the agent preflight checks to verify this worktree is safe to edit.

## Steps

Run the preflight script:

```bash
bash scripts/agent-preflight.sh
```

The script checks:

1. **Branch safety** — Not on `master` or `main`. Not in detached HEAD state. Exit 1 if failed.
2. **Worktree isolation** — Running inside a git worktree, not the main checkout. Exit 2 if failed.
3. **No merge conflicts** — No unresolved conflict markers in the working tree. Exit 3 if failed.
4. **cwd isolation** — Not running from the main repo root. Exit 4 if failed.
5. **CARGO_TARGET_DIR isolation** — Computes the recommended `CARGO_TARGET_DIR` (a per-branch path under `/tmp/`) and reports it. Prevents shared build artifact collisions between concurrent agents. **Note:** Because the script runs in a subshell, you must set the variable yourself before running cargo commands — see the builder environment setup section.
6. **No git stash entries** — Git stash is shared across all worktrees. Any entries risk cross-contamination between agents. Exit 6 if failed.

## Interpreting results

- **Exit 0**: All checks pass. Safe to begin work.
- **Exit 1 (branch issue)**: You are on a protected branch or detached HEAD.
  - Fix: Ensure the agent was spawned with `isolation: worktree` in the agent definition.
- **Exit 2 (worktree issue)**: Not in an isolated worktree.
  - Fix: Add `isolation: worktree` to the agent definition and respawn.
- **Exit 3 (conflict issue)**: Unresolved merge conflicts present.
  - Fix: Resolve conflicts manually, then re-run preflight.
- **Exit 4 (cwd issue)**: Running from the main repo root instead of the worktree.
  - Fix: cd to the worktree path.
- **Exit 6 (stash issue)**: Shared stash has entries from this or other worktrees.
  - Fix: Run `git stash clear` to drop all entries. Never use `git stash` -- use `git restore <file>` to discard changes or `git commit -m "wip"` to save work.

## On failure

Do not proceed with edits. Report the failure to the orchestrator with the exact error message from the script. This prevents agents from accidentally editing the wrong branch or polluting the main checkout.
