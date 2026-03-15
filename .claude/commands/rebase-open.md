---
description: Rebase all open PRs onto current master
argument-hint: "[--dry-run] [--filter <pattern>]"
---

# Rebase Open PRs

Rebase all open PR branches onto current master to resolve conflicts. Context: **$ARGUMENTS**

## Steps

### 1. Fetch latest
```bash
git fetch origin
```

### 2. List open PRs with conflicts
```bash
gh pr list --state open --json number,title,headRefName,mergeable --limit 50
```

### 3. For each conflicted PR
```bash
git checkout <branch>
git rebase origin/master
```

If rebase succeeds:
```bash
git push --force-with-lease
```

If rebase fails (complex conflicts):
```bash
git rebase --abort
```
Note as blocked — needs manual resolution or a swarm-fixer agent.

### 4. Report
| PR | Branch | Status |
|----|--------|--------|
| #N | fix/... | rebased + pushed |
| #N | feat/... | blocked (conflict in file.rs) |
