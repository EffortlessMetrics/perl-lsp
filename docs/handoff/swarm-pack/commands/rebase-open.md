---
description: Rebase all open PRs onto current main branch
argument-hint: "[--dry-run]"
---

# Rebase Open PRs

Rebase all open PR branches onto current main. Context: **$ARGUMENTS**

## Steps

### 1. Fetch
```bash
git fetch origin
```

### 2. List conflicted PRs
```bash
gh pr list --state open --json number,title,headRefName,mergeable --limit 50
```

### 3. For each conflicted PR
```bash
git checkout <branch>
git rebase origin/main
# Success:
git push --force-with-lease
# Failure:
git rebase --abort
# Note as blocked
```

### 4. Report
| PR | Branch | Status |
|----|--------|--------|
| #N | fix/... | rebased |
| #N | feat/... | blocked (conflict in file.rs) |
