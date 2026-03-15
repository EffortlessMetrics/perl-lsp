---
description: Generate daily swarm summary for user check-in
argument-hint: "[--since '24 hours ago']"
---

# Swarm Report

Generate a summary for the user's check-in. Context: **$ARGUMENTS**

## Gather Data

```bash
SINCE="${1:-24 hours ago}"

echo "=== PRs Merged ==="
gh pr list --state merged --json number,title,mergedAt --limit 50 | \
  jq --arg since "$(date -d "$SINCE" -Iseconds 2>/dev/null || date -v-1d -Iseconds)" \
  '[.[] | select(.mergedAt > $since)]'

echo "=== PRs Open ==="
gh pr list --state open --json number,title,labels

echo "=== Issues Created ==="
gh issue list --label "swarm-discovered" --state open
gh issue list --label "swarm-architectural" --state open

echo "=== Agent Patches Pending ==="
ls -la .ops/agent-patches/*.md 2>/dev/null

echo "=== Metrics Summary ==="
tail -100 .ops/swarm-metrics.jsonl 2>/dev/null | \
  jq -s 'group_by(.outcome) | map({outcome: .[0].outcome, count: length})'
```

## Report Format

Summarize as:

```markdown
## Swarm Report — <date>

### Shipped
- N PRs merged: <titles>

### In Progress
- N PRs open: <titles>

### Discovered
- N issues created: <titles>
- N items in discovery log

### Health
- Green rate: N% (from metrics)
- Agent patches pending review: N
- Known pitfalls: N active

### Blockers
- <any blocked PRs or slices>

### Recommendations
- <patterns from metrics: which agents/domains need attention>
```
