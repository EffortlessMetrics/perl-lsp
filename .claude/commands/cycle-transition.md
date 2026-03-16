---
description: Automate end-of-cycle / start-of-cycle transition (stash, cleanup, pull, verify)
argument-hint: "[--dry-run] [--skip-ci-check]"
---

# Cycle Transition

Automate the full cycle boundary: wind down the old cycle, clean up, and prepare a fresh master for the next cycle. Context: **$ARGUMENTS**

This replaces 10+ minutes of manual orchestration with a single defensive sequence.

## Steps

### 1. Stash local changes

If you are on a branch with uncommitted work, stash it before proceeding.

```bash
BRANCH=$(git branch --show-current)
if [[ -n "$(git status --porcelain)" ]]; then
    git stash push -m "cycle-transition-auto-stash-$(date +%Y%m%d-%H%M%S)"
    echo "STASHED: uncommitted changes on $BRANCH"
else
    echo "CLEAN: no uncommitted changes on $BRANCH"
fi
```

Record the starting branch for the report.

### 2. Remove agent worktrees blocking master checkout

Worktrees may hold locks on branches that prevent switching to master. Remove all agent worktrees first.

```bash
echo "=== Removing agent worktrees ==="
# Use cleanup-worktrees.sh which already handles this correctly
if [[ -f scripts/cleanup-worktrees.sh ]]; then
    bash scripts/cleanup-worktrees.sh
else
    # Fallback: manual cleanup
    git worktree list | grep '.claude/worktrees/' | awk '{print $1}' | while read -r wt; do
        echo "Removing: $wt"
        git worktree remove --force "$wt" 2>/dev/null || rm -rf "$wt"
    done
    git worktree prune
fi
echo "Worktree cleanup complete"
```

### 3. Switch to master

```bash
git checkout master 2>&1 || {
    echo "WARNING: checkout master failed, attempting force checkout"
    git checkout -f master
}
```

### 4. Pull latest master

Use the safe-pull script which handles untracked file conflicts and stale tracking refs:

```bash
echo "=== Pulling latest master ==="
if [[ -f scripts/safe-pull.sh ]]; then
    bash scripts/safe-pull.sh master
else
    # Fallback if safe-pull.sh not yet available
    git pull origin master 2>&1 || {
        echo "WARNING: pull failed, continuing anyway..."
    }
fi
```

### 5. Cleanup worktrees (final pass)

Run the cleanup script one more time now that we are on master:

```bash
echo "=== Final worktree cleanup ==="
git worktree prune
if [[ -f scripts/cleanup-worktrees.sh ]]; then
    bash scripts/cleanup-worktrees.sh
fi
```

### 6. Prune branches

Delete local branches that have been merged into master. Prune remote tracking refs.

```bash
echo "=== Pruning merged branches ==="
# Delete merged local branches (excluding protected patterns)
MERGED_BRANCHES=$(git branch --merged master | grep -v -E '^\*|master|main|backup/|release/' || true)
if [[ -n "$MERGED_BRANCHES" ]]; then
    echo "$MERGED_BRANCHES" | xargs git branch -d 2>&1 || true
    echo "Pruned merged local branches"
else
    echo "No merged branches to prune"
fi

# Prune stale remote tracking refs
git fetch --prune 2>&1 || echo "WARNING: fetch --prune failed"

# Count remaining branches
BRANCH_COUNT=$(git branch | wc -l | tr -d ' ')
echo "Remaining local branches: $BRANCH_COUNT"
```

### 7. Verify CI green (unless `--skip-ci-check`)

Check if master's latest CI run is passing.

```bash
echo "=== CI Status ==="
CI_STATUS=$(gh api repos/:owner/:repo/commits/master/status --jq '.state' 2>/dev/null || echo "unknown")
echo "Master CI status: $CI_STATUS"

if [[ "$CI_STATUS" == "failure" || "$CI_STATUS" == "error" ]]; then
    echo "WARNING: Master CI is NOT green. Check before starting next cycle."
    # Show failing checks
    gh api repos/:owner/:repo/commits/master/check-runs --jq '.check_runs[] | select(.conclusion != "success") | "\(.name): \(.conclusion)"' 2>/dev/null || true
elif [[ "$CI_STATUS" == "success" ]]; then
    echo "Master CI is green — ready for next cycle"
elif [[ "$CI_STATUS" == "pending" ]]; then
    echo "Master CI is still running — wait before starting next cycle"
else
    # Fall back to gh pr checks on master
    echo "Could not determine CI status via API, trying gh..."
    gh run list --branch master --limit 3 --json conclusion,status,name 2>/dev/null || echo "Could not fetch CI runs"
fi
```

### 8. Report

Compile and display the transition summary.

```bash
echo "=== Gathering report data ==="
WORKTREE_COUNT=$(git worktree list | wc -l)
BRANCH_COUNT=$(git branch | wc -l | tr -d ' ')
OPEN_PRS=$(gh pr list --state open --json number --jq 'length' 2>/dev/null || echo "?")
```

Format the report:

```
## Cycle Transition Complete

| Metric | Value |
|--------|-------|
| Current branch | master |
| Worktrees remaining | <count> (should be 1-2: main + review) |
| Local branches | <count> |
| Open PRs | <count> |
| CI status | <green/red/pending/unknown> |
| Stashed changes | <yes/no — from which branch> |

### Next Steps
- If CI is red: investigate before starting agents
- If stashed: `git stash pop` on the original branch if needed
- Start next cycle: `/swarm all` or spawn targeted agents
```
