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

5. Determine verdict:
   - All checks SUCCESS/NEUTRAL on current SHA + not draft + MERGEABLE → **GREEN**
   - Any check FAILURE on current SHA → **RED** (list failures)
   - Checks green but on old SHA → **STALE** 
   - Draft or DIRTY or CONFLICTING → **BLOCKED**
