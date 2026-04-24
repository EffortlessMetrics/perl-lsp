---
description: Generate daily swarm summary for user check-in
argument-hint: "[--since 7d]"
---

# Swarm Report

Generate a summary for the user's check-in. Context: **$ARGUMENTS**

## Gather Data

```bash
SINCE="${1:-7d}"

echo "=== PRs Merged ==="
gh pr list --state merged --json number,title,mergedAt --limit 50

echo "=== PRs Open ==="
gh pr list --state open --json number,title,labels

echo "=== Issues Created ==="
gh issue list --label "swarm-discovered" --state open
gh issue list --label "swarm-architectural" --state open

echo "=== Agent Patches Pending ==="
ls -la .ops/agent-patches/*.md 2>/dev/null

echo "=== Metrics Dashboard ==="
cargo xtask swarm-summary .ops-perl-lsp --since "${SINCE}" --limit 20
cargo xtask swarm-summary .ops-perl-lsp --since "${SINCE}" --limit 20 --format json
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
- Entries in window: N
- Top agent types: <from metrics>
- Top session / worktree hotspots: <from metrics>
- Agent patches pending review: N

### Blockers
- <any blocked PRs or slices>

### Recommendations
- <patterns from metrics: which agents/domains need attention>
```
