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
4. **CARGO_TARGET_DIR isolation** — Computes the recommended `CARGO_TARGET_DIR` (a per-branch path under `/tmp/`) and reports it. Prevents shared build artifact collisions between concurrent agents. **Note:** Because the script runs in a subshell, you must set the variable yourself before running cargo commands — see the builder environment setup section.

## Interpreting results

- **Exit 0**: All checks pass. Safe to begin work.
- **Exit 1 (branch issue)**: You are on a protected branch or detached HEAD.
  - Fix: Ensure the agent was spawned with `isolation: worktree` in the agent definition.
- **Exit 2 (worktree issue)**: Not in an isolated worktree.
  - Fix: Add `isolation: worktree` to the agent definition and respawn.
- **Exit 3 (conflict issue)**: Unresolved merge conflicts present.
  - Fix: Resolve conflicts manually, then re-run preflight.

## On failure

Do not proceed with edits. Report the failure to the orchestrator with the exact error message from the script. This prevents agents from accidentally editing the wrong branch or polluting the main checkout.
