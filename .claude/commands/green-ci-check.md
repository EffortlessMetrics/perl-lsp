---
description: Green CI agent step 1 — verify all CI checks pass on current HEAD SHA
user-invocable: false
---

# Green CI: Check

Verify CI is genuinely green on the current HEAD.

## Steps

1. Get current HEAD SHA:
   ```bash
   HEAD_SHA=$(gh pr view <number> --json headRefOid --jq .headRefOid)
   echo "HEAD: $HEAD_SHA"
   ```

2. Check all CI status checks:
   ```bash
   gh pr checks <number>
   ```

3. Verify freshness — checks must be on the current SHA:
   ```bash
   gh api repos/{owner}/{repo}/commits/$HEAD_SHA/check-runs --jq '.check_runs[] | "\(.name) | \(.status) | \(.conclusion) | \(.head_sha[0:8])"'
   ```

4. Check PR state:
   ```bash
   gh pr view <number> --json isDraft,mergeable,mergeStateStatus --jq '{draft: .isDraft, mergeable: .mergeable, mergeState: .mergeStateStatus}'
   ```

5a. Classify cancellations — for each check with `conclusion: cancelled`, extract
    `started_at` and `completed_at` from the check-runs API response:
    - If `started_at == completed_at` (zero-duration) → mark check **INFRA-NOISE**
      (GitHub concurrency-group kill; instantaneous, no work was done).
    - If `completed_at - started_at > 5s` → mark check **DEVELOPER-CANCEL**
      (manual cancel via GitHub UI or API; treat as RED).
    For each check with `conclusion: failure` → mark **RED** (ignore any cancel log
    content; failures are always RED).
    For each check with `conclusion: success` → no change (existing behavior).

5b. Determine verdict — using classified checks from step 5a:
   - All checks SUCCESS/NEUTRAL/INFRA-NOISE on current SHA + not draft + MERGEABLE → **GREEN**
   - Any check RED or DEVELOPER-CANCEL on current SHA → **RED** (list RED + DEVELOPER-CANCEL only; omit INFRA-NOISE from details)
   - Checks green but on old SHA → **STALE**
   - Draft or DIRTY or CONFLICTING → **BLOCKED**
