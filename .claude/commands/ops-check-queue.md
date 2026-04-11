---
description: Ops step 1 — find merge-ready PRs in the queue
user-invocable: false
---

# Ops Check Queue

Find PRs that are ready to merge.

## Steps

1. List all open PRs with their merge state:
   ```bash
   gh pr list --state open --limit 50 --json number,title,mergeable,mergeStateStatus,isDraft,reviewDecision --jq '.[] | "\(.number)\t\(.mergeable)/\(.mergeStateStatus)\tdraft:\(.isDraft)\treview:\(.reviewDecision)\t\(.title)"'
   ```

2. Filter for merge candidates:
   - MERGEABLE + CLEAN or UNSTABLE (CI may be running)
   - Not a draft (or promote with `gh pr ready` if appropriate)
   - reviewDecision: APPROVED or no review required

3. Check CI on each candidate:
   ```bash
   gh pr view <number> --json statusCheckRollup --jq '[.statusCheckRollup[] | select(.conclusion == "FAILURE") | (.context // .name)]'
   ```

4. Classify:
   - **MERGE NOW**: MERGEABLE + CI green + `just pre-merge-check <number>` passes
   - **WAIT**: CI still running
   - **BLOCKED**: CI failures — note which check failed
   - **NEEDS REBASE**: CONFLICTING

## Output

Record in your task:
```
Merge candidates: #NNN, #NNN, #NNN
Blocked: #NNN (reason), #NNN (reason)
Waiting: #NNN (CI running)
```
