---
description: Drain green PRs sequentially (merge all passing PRs)
argument-hint: "[--dry-run] [--limit N]"
---

# Green Merge

Merge all open PRs that have passing checks. Context: **$ARGUMENTS**

## Steps

### 1. Inventory open PRs
```bash
gh pr list --state open --json number,title,headRefName,mergeable,statusCheckRollup --limit 50
```

### 2. Classify each PR
- **Green**: mergeable, no failing checks → merge
- **Conflicted**: merge conflicts → skip (use `/rebase-open` first)
- **Failing**: CI failures → skip (needs `/parser-fix` or `fixer`)
- **Draft**: skip unless `--include-drafts`

### 3. Merge green PRs sequentially
For each green PR, in dependency order:
```bash
gh pr merge <number> --squash --delete-branch
```
Wait for each merge to complete before the next. Sequential merging prevents race conditions.

### 4. Handle post-merge drift
After all merges complete:
```bash
# Regenerate status
python3 scripts/update-current-status.py
git diff docs/project/CURRENT_STATUS.md
# If changed, commit
```

### 5. Report
| PR | Title | Status |
|----|-------|--------|
| #N | ... | merged / skipped (reason) |
