---
description: Show progress of running agents — scan task output files and display a dashboard
argument-hint: "[--verbose]"
---

# Agent Dashboard

Scan for running agent output and display a progress summary. Context: **$ARGUMENTS**

## Steps

### 1. Scan active worktrees as agent proxies

Each agent worktree under `.claude/worktrees/` represents an active or recently-active agent.
Check their git state to infer progress:

```bash
echo "=== Active Agent Worktrees ==="
git worktree list | grep '.claude/worktrees/' | while read -r line; do
    WT_PATH=$(echo "$line" | awk '{print $1}')
    # Extract branch name — handle both "[branch]" and "(detached)" formats
    WT_REF=$(echo "$line" | sed -n 's/.*\[\(.*\)\]/\1/p')
    if [[ -z "$WT_REF" ]]; then
        WT_REF=$(echo "$line" | awk '{print $2}')  # fallback to commit hash
    fi

    # Get last commit time in this worktree
    LAST_COMMIT_TIME=$(git -C "$WT_PATH" log -1 --format='%cr' 2>/dev/null || echo "unknown")
    LAST_COMMIT_MSG=$(git -C "$WT_PATH" log -1 --format='%s' 2>/dev/null || echo "no commits")

    # Check for uncommitted work
    DIRTY=$(git -C "$WT_PATH" status --porcelain 2>/dev/null | wc -l | tr -d ' ')

    # Count commits ahead of master
    AHEAD=$(git -C "$WT_PATH" rev-list origin/master..HEAD --count 2>/dev/null || echo "?")

    echo "AGENT|$WT_REF|$AHEAD commits|$DIRTY dirty|$LAST_COMMIT_TIME|$LAST_COMMIT_MSG"
done
```

### 2. Check for associated PRs

```bash
echo "=== Agent PRs ==="
gh pr list --state open --json number,title,headRefName,statusCheckRollup --limit 50 2>/dev/null | \
    jq -r '.[] | "\(.number)|\(.headRefName)|\(.title)|\((.statusCheckRollup // [])[0].conclusion // "pending")"' 2>/dev/null || \
    echo "Could not fetch PR data"
```

### 3. Format dashboard

Combine all data into a readable table:

```
## Agent Dashboard — <timestamp>

### Active Worktrees
| Agent Branch | Commits Ahead | Dirty Files | Last Activity | Last Commit |
|-------------|--------------|-------------|---------------|-------------|
| feat/foo    | 3            | 0           | 5 min ago     | Add tests   |
| fix/bar     | 1            | 2           | 12 min ago    | Fix parser  |

### Associated PRs
| PR # | Branch | Title | CI Status |
|------|--------|-------|-----------|
| 1623 | feat/foo | Add foo feature | success |
| 1624 | fix/bar  | Fix bar issue   | pending |

### Summary
- Active agents: N
- With open PRs: N
- With dirty work: N
- Total commits ahead of master: N
```

### 4. Verbose mode (`--verbose`)

If `--verbose` is passed, also show:

```bash
# Full git log for each worktree
git worktree list | grep '.claude/worktrees/' | while read -r line; do
    WT_PATH=$(echo "$line" | awk '{print $1}')
    WT_BRANCH=$(echo "$line" | awk '{print $3}' | tr -d '[]')
    echo "--- $WT_BRANCH ---"
    git -C "$WT_PATH" log origin/master..HEAD --oneline 2>/dev/null || echo "  (no commits ahead)"
    git -C "$WT_PATH" status --short 2>/dev/null || true
    echo ""
done
```
