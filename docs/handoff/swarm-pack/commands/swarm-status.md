---
description: Show current swarm state — open PRs, issues, metrics summary, queue depth
argument-hint: "[--full]"
---

# Swarm Status

Aggregate current swarm state. Context: **$ARGUMENTS**

## Quick View

```bash
echo "=== Open PRs ==="
gh pr list --state open --json number,title,labels --limit 30

echo "=== Discovered Issues ==="
gh issue list --label "swarm-discovered" --state open --limit 20

echo "=== Architectural Decisions Needed ==="
gh issue list --label "swarm-architectural" --state open

echo "=== Recent Merges (last 24h) ==="
gh pr list --state merged --limit 20 --json number,title,mergedAt

echo "=== Queue Depth ==="
grep -c "in-progress" .claude/swarm-state/completed-slices.md 2>/dev/null || echo "0 in-progress"
wc -l < .claude/swarm-state/discovered-issues.md 2>/dev/null || echo "0 discoveries"
jq -r '(.findings // []) | length | "\(.) tracked findings"' .claude/swarm-state/findings.json 2>/dev/null || echo "0 tracked findings"
ls .ops/handoffs/*.md 2>/dev/null | wc -l || echo "0 active handoffs"

echo "=== Agent Patches Pending Review ==="
ls .ops/agent-patches/*.md 2>/dev/null | wc -l || echo "0 patches"
```

## Full View (`--full`)

Also includes:
```bash
echo "=== Metrics (last 50 entries) ==="
tail -50 .ops/swarm-metrics.jsonl 2>/dev/null

echo "=== Known Pitfalls ==="
cat .claude/swarm-state/known-pitfalls.md 2>/dev/null

echo "=== Tracked Findings ==="
jq -r '.findings[]? | "\(.id) | \(.status) | \(.summary)"' .claude/swarm-state/findings.json 2>/dev/null

echo "=== Worktrees ==="
git worktree list
```
