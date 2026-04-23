---
description: Ops step 1 — find merge-ready PRs in the queue
user-invocable: false
---

# Ops Check Queue

Find PRs that are ready to merge.

## Steps

### Step 0 — Sweep stale in-build claims

Before checking the merge queue, clear noise from stale builder claims.

Issues with `in-build` but no linked open PR for more than 7 days are routing dead weight.
Each one adds latency to every orchestrator dispatch decision.

```bash
gh issue list --label "in-build" --state open --json number,title,updatedAt --jq '.[] | select((now - (.updatedAt | fromdateiso8601)) > (7*86400)) | "#\(.number) \(.title)"'
```

For each result, check if a linked open PR exists:

```bash
gh pr list --search "closes #<number>" --state open --json number,title
```

Classify and act:
- **Has open PR**: skip — builder is active.
- **No open PR, > 7 days stale**: remove `in-build` label and add a comment: `in-build label removed — no linked PR after 7 days; issue returned to queue`.

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
