---
description: Ensemble-curator step 1 — detect whether a PR is part of a cluster from the same external-agent generation run
---

# Ensemble Detection

Before triaging a single PR in isolation, check whether it's part of a cluster produced by a single external-agent prompt / task. Clusters need coordinated triage, not one-at-a-time processing.

## Signals

A PR is likely part of a cluster when ANY of these hold:

### 1. Shared Codex task ID in body

```bash
gh pr view <N> --json body -q .body | grep -oE 'task_e_[a-z0-9]{8,}' | head -1
```

If a task ID appears, search for other PRs with the same ID:

```bash
gh pr list --state open --limit 100 --search "task_e_<id>" --json number,title
```

### 2. Creation-time burst

Other PRs created within 15 minutes of this one by the same author:

```bash
MY_TIME=$(gh pr view <N> --json createdAt -q .createdAt)
gh pr list --state open --limit 100 --json number,createdAt,author \
  --author <this-author> --jq '.[] | select(.createdAt > "'"$(date -u -d "$MY_TIME - 15 minutes" -Iseconds)"'" and .createdAt < "'"$(date -u -d "$MY_TIME + 15 minutes" -Iseconds)"'") | .number'
```

### 3. Title stem match

Titles differing only by stem word (`add`/`improve`/`expand`/`support`):

```bash
TITLE=$(gh pr view <N> --json title -q .title | sed 's/add/X/; s/improve/X/; s/expand/X/; s/support/X/')
gh pr list --state open --search "$TITLE in:title" --limit 20 --json number,title
```

### 4. Branch name pattern

External-agent branches follow patterns:

- `codex/improve-<topic>-<suffix>` (Codex)
- `codex/improve-<topic>` (variants)
- `jules/<topic>`
- `hermes/<topic>`
- `droid/<topic>`

Siblings share the `codex/improve-<topic>` prefix with different suffixes.

## When you find a cluster

1. Note the cluster size (N PRs)
2. Note shared scope (topic)
3. Identify the task_id if present
4. Route to `/cluster-triage` for file-path-based winner selection — don't process any PR in the cluster in isolation

## When you don't find a cluster

Treat as solo PR. Continue to `/hallucination-check` + rest of the todo list.

## What this skill outputs

Either:
- `cluster: N=<count>, task=<id-or-null>, PRs=<list>, topic=<topic>` — pass to `/cluster-triage`
- `solo: <PR#>` — pass to `/hallucination-check`

## Why cluster-first matters

Processing a 4-shot cluster one PR at a time:
- Burns 4× triage cost
- May approve the first, then close the rest as dupes without extracting their novelties
- Can produce contradictory verdicts if one agent-triage run closes A, another closes B

Processing as a cluster:
- Single triage pass
- Cross-pollinate edges from losers → winner before closing
- Consistent verdicts

Cost: 1 extra query per PR to detect cluster. Worth it.
