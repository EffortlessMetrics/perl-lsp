---
description: Stash/patch dirty worktrees before cleanup
argument-hint: "[--prune-merged] [--dry-run]"
---

# Salvage Worktrees

Save uncommitted work from agent worktrees, then optionally prune merged ones. Context: **$ARGUMENTS**

## Steps

### 1. Inventory all worktrees
```bash
git worktree list
```

### 2. For each non-main worktree, check status
```bash
cd <worktree-path>
git status --porcelain
git log origin/master..HEAD --oneline
```

Classify:
- **Clean + merged** → safe to prune
- **Clean + unmerged** → active work, leave alone
- **Dirty + merged** → salvage then prune
- **Dirty + unmerged** → salvage then leave

### 3. Salvage dirty worktrees
```bash
mkdir -p .ops-perl-lsp/salvage/
cd <worktree-path>
# Save diff
git diff > /path/to/repo/.ops-perl-lsp/salvage/<branch>-$(date +%Y%m%d).patch
git diff --cached >> /path/to/repo/.ops-perl-lsp/salvage/<branch>-$(date +%Y%m%d).patch
# Save untracked file list
git ls-files --others --exclude-standard > /path/to/repo/.ops-perl-lsp/salvage/<branch>-$(date +%Y%m%d).untracked
```

### 4. Prune merged worktrees (if `--prune-merged`)
```bash
git worktree remove <worktree-path>
```

**NEVER delete**: `master`, `backup/*`, `release/*`, or branches with unique unreachable commits.

### 5. Clean merged local branches (if `--prune-merged`)
```bash
git branch --merged master | grep -v 'master\|backup/\|release/' | xargs git branch -d
git fetch --prune
```

### 6. Report
| Worktree | State | Action |
|----------|-------|--------|
| agent-xxx | dirty+merged | salvaged → pruned |
| agent-yyy | clean+merged | pruned |
| agent-zzz | dirty+unmerged | salvaged → kept |
