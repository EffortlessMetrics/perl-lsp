---
description: Drain green PRs sequentially (merge all passing PRs)
argument-hint: "[--dry-run] [--limit N] [--batch-size N]"
disable-model-invocation: true
---

# Green Merge

Merge all open PRs that have passing checks. Context: **$ARGUMENTS**

## Arguments

- `--dry-run`: List what would be merged without merging
- `--limit N`: Max PRs to consider (default 50)
- `--batch-size N`: How many PRs to merge before waiting for CI (default 3)

## Steps

### 1. Parse arguments

Extract `--batch-size N` from $ARGUMENTS (default: 3). This controls how many PRs
are merged before pausing to verify master CI is green. This prevents merge → cancel →
merge → cancel cascades where rapid merges cause each CI run to cancel the previous one.

### 2. Inventory open PRs
```bash
gh pr list --state open --json number,title,headRefName,mergeable,statusCheckRollup --limit 50
```

### 3. Classify each PR
- **Green**: mergeable, no failing checks → merge
- **Conflicted**: merge conflicts → skip (use `/rebase-open` first)
- **Failing**: CI failures → skip (needs `/parser-fix` or swarm-fixer)
- **Draft**: skip unless `--include-drafts`

### 4. Merge green PRs in batches

For each green PR, in dependency order, merge sequentially. After every `batch-size`
merges (default 3), **stop and wait for master CI** before continuing.

#### 4a. Merge one PR
```bash
gh pr merge <number> --squash --delete-branch
```

If the merge **fails due to a Cargo.lock conflict**, attempt auto-resolution:
1. Check out the PR branch locally
2. Rebase onto the latest master, then regenerate the lockfile:
   ```bash
   gh pr checkout <number>
   git fetch origin master
   git rebase origin/master
   # If rebase conflicts on Cargo.lock, accept master's version and regenerate.
   # During rebase, --ours refers to the upstream (master) side.
   git checkout --ours Cargo.lock
   cargo generate-lockfile
   git add Cargo.lock
   git rebase --continue
   git push --force-with-lease
   ```
3. Retry the merge: `gh pr merge <number> --squash --delete-branch`
4. If it still fails, skip the PR and report the failure

#### 4b. Batch pacing — wait for master CI after every batch

After merging `batch-size` PRs (default 3), **pause and verify master is green**
before continuing. This prevents CI cancellation cascades.

**Check master CI status:**
```bash
gh run list --branch master --limit 1 --json status,conclusion
```

**Wait loop** (up to 10 retries, 30s apart):
1. If `status` is `in_progress` or `queued` → wait 30 seconds, re-check
2. If `conclusion` is `success` → master is green, continue with next batch
3. If `conclusion` is `failure` → **STOP ALL MERGING** and report:
   - Which commit/run failed
   - Which PRs were merged in this batch
   - Suggest: inspect the failing run with `gh run view <id> --log-failed`

**Pseudocode:**
```
merged_count = 0
for pr in green_prs:
    merge(pr)
    merged_count += 1

    # Check after every batch AND after the final merge
    is_last = (pr == last green_pr)
    if merged_count % batch_size == 0 or is_last:
        # Wait for master CI to go green
        for retry in 1..10:
            run = gh run list --branch master --limit 1
            if run.status == "completed":
                if run.conclusion == "success":
                    break  # CI green, continue merging
                else:
                    STOP and report failure
            sleep 30s
        else:
            STOP and report: "CI did not complete after 5 minutes"
```

### 5. Handle post-merge drift
After all merges complete:
```bash
# Regenerate status
python3 scripts/update-current-status.py
git diff docs/project/CURRENT_STATUS.md
# If changed, commit
```

### 6. Report
| PR | Title | Batch | Status |
|----|-------|-------|--------|
| #N | ... | 1 | merged / skipped (reason) / failed (Cargo.lock conflict) |

Summary line:
- Total merged: N
- Total skipped: N (with reasons)
- Batches completed: N
- Master CI status: green / failed at batch N
