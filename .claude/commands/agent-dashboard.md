---
description: Show progress of running agents — render a compact dashboard from worktrees and PR state
argument-hint: "[--verbose] [--no-prs]"
---

# Agent Dashboard

Render the reusable dashboard script for active agent worktrees. Context: **$ARGUMENTS**

## Primary command

```bash
bash scripts/agent-dashboard.sh $ARGUMENTS
```

## What it shows

- Summary counters for active worktrees, dirty worktrees, and total commits ahead
- A compact activity table with commit-density bars for each agent branch
- Open PR state with CI health when `gh` and `jq` are available
- Optional verbose drill-down with per-worktree commit logs and git status

## Examples

```bash
bash scripts/agent-dashboard.sh
bash scripts/agent-dashboard.sh --verbose
bash scripts/agent-dashboard.sh --no-prs
```
