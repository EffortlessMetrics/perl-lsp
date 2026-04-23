---
description: Save dirty worktrees before cleanup
argument-hint: "[--prune-merged] [--dry-run]"
---

# Salvage Worktrees

Save uncommitted work, optionally prune merged worktrees. Context: **$ARGUMENTS**

## Steps

### 1. Inventory
```bash
git worktree list
```

### 2. Classify each non-main worktree
```bash
cd <path> && git status --porcelain && git log origin/main..HEAD --oneline
```

- Clean + merged → prune
- Clean + unmerged → leave
- Dirty + merged → salvage then prune
- Dirty + unmerged → salvage then leave

### 3. Salvage dirty
```bash
mkdir -p .ops/salvage/
cd <worktree>
git diff > /repo/.ops/salvage/<branch>-$(date +%Y%m%d).patch
```

### 4. Prune (if `--prune-merged`)
```bash
git worktree remove <path>
git branch --merged main | grep -v 'main\|master\|backup/\|release/' | xargs git branch -d
git fetch --prune
```

**NEVER delete**: main, master, backup/*, release/*
