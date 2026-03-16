---
description: Clean up completed/abandoned agent worktrees mid-cycle
argument-hint: "[--dry-run]"
---

# Cleanup Worktrees

Intelligently clean up agent worktrees based on merge and PR state. Context: **$ARGUMENTS**

This solves worktree accumulation during swarm cycles with 20+ agents. Unlike the nuclear `scripts/cleanup-worktrees.sh` which removes ALL worktrees, this preserves active work.

## Decision Matrix

| State | Action |
|-------|--------|
| Branch merged to master | Remove worktree + delete branch |
| Open PR (any CI state) | Keep (might need fixups) |
| No PR + dirty worktree | Keep (uncommitted work) |
| No PR + unpushed commits | Keep (work in progress) |
| No PR + no unpushed + clean | Remove (abandoned) |

## Run

```bash
# Preview what would be cleaned (safe)
bash scripts/cleanup-completed-worktrees.sh --dry-run

# Execute cleanup
bash scripts/cleanup-completed-worktrees.sh
```

## When to Run

- Janitor runs this every 10 merged PRs during a swarm cycle
- Run manually with `--dry-run` first to preview
- Safe to run anytime — only removes confirmed-done or confirmed-abandoned worktrees
