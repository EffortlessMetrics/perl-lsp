---
description: Watch a CI run until completion (non-blocking)
argument-hint: "<pr-number|run-id>"
---

# CI Watch

Watch a CI run in the background. You'll be notified when it completes.

## Usage

Given: **$ARGUMENTS** (PR number or run ID)

## Steps

### If argument looks like a PR number (< 100000)

Resolve the latest run ID for the PR:

```bash
# Get the latest CI run for this PR
RUN_ID=$(gh api repos/:owner/:repo/actions/runs \
  --jq ".workflow_runs[] | select(.pull_requests[]?.number == $ARGUMENTS) | .id" \
  | head -1)
```

If that returns empty, fall back to:

```bash
RUN_ID=$(gh pr checks $ARGUMENTS --json name,link \
  --jq '.[0].link' 2>/dev/null \
  | grep -oP 'runs/\K\d+')
```

If both fail, report the error and stop.

### If argument looks like a run ID (>= 100000)

Use it directly:

```bash
RUN_ID=$ARGUMENTS
```

### Watch the run in background

```bash
gh run watch $RUN_ID --exit-status
```

Run this with `run_in_background: true`. You'll be notified when it completes.

### On completion

- **Exit 0** (CI passed): message merger or proceed with merge
- **Non-zero** (CI failed): inspect failures with:

```bash
gh run view $RUN_ID --log-failed | tail -20
```

Then message the fixer agent with the failure summary.

## Example

```
/ci-watch 1612         # Watch PR #1612's CI run
/ci-watch 23125748983  # Watch a specific run ID
```
