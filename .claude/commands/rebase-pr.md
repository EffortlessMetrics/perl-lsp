---
description: Rebase a single PR branch onto current master
argument-hint: "<PR number or branch name>"
---

# Rebase PR

Rebase a single PR branch onto current master. Unlike `/rebase-open` which rebases ALL open PRs, this targets one specific PR. Context: **$ARGUMENTS**

## Steps

### 1. Identify the PR
```bash
gh pr view $ARGUMENTS --json number,title,headRefName,baseRefName,mergeable
```

Extract the branch name from `headRefName`.

### 2. Fetch latest
```bash
git fetch origin master
git fetch origin <branch>
```

### 3. Check out the branch
```bash
git checkout <branch>
```

### 4. Attempt rebase
```bash
git rebase origin/master
```

### 5. Handle conflicts

**If rebase succeeds** (no conflicts):
```bash
git push --force-with-lease
```
Report success.

**If conflicts are simple** (only in CLAUDE.md, `.claude/`, `docs/`, `Cargo.lock`, or other infrastructure files):
- Resolve by accepting the master version for infrastructure files
- For `Cargo.lock`: accept master's version (`git checkout --ours Cargo.lock` — during rebase, `--ours` is the upstream), then run `cargo generate-lockfile` to regenerate
- Re-run: `git add <resolved files> && git rebase --continue`
- Then: `git push --force-with-lease`

**If conflicts are complex** (in `src/`, `tests/`, or multiple crate files):
```bash
git rebase --abort
```
Report as blocked -- needs manual resolution or a dedicated fixer agent.

### 6. Return to previous branch
```bash
git checkout -
```

### 7. Report

```
### Rebase Result
- **PR**: #<number> (<title>)
- **Branch**: <headRefName>
- **Status**: SUCCESS / RESOLVED (infrastructure conflicts) / BLOCKED (complex conflicts)
- **Conflicts** (if any): <list of conflicting files>
- **Action taken**: rebased + force-pushed / aborted
```
