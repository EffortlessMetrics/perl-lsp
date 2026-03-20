---
description: Ops step 2 — merge a batch of up to 3 PRs
user-invocable: false
---

# Ops Merge Batch

Merge up to 3 PRs from the candidates identified in step 1.

## Steps

1. Pick up to 3 PRs from the MERGE NOW list.
   Respect dependency order:
   - If PR B depends on PR A (same files), merge A first
   - Parser fixes before corpus ratchets
   - Infrastructure before features

2. **Fresh green check** — immediately before each merge, verify live state:
   ```bash
   gh pr view <number> --json isDraft,mergeable,headRefOid,reviewDecision,statusCheckRollup
   ```
   All of these must be true AT MERGE TIME (not remembered from earlier):
   - Not draft
   - Mergeable now
   - CI checks green on the current HEAD SHA
   - No blocking review comments

3. Merge each PR:
   ```bash
   gh pr merge <number> --merge
   ```

4. After each merge, verify it landed:
   ```bash
   gh pr view <number> --json state --jq .state
   ```

5. If a merge fails or pre-check fails:
   - CONFLICTING → skip, note "needs rebase"
   - CI red or pending → skip, note "CI not green on current HEAD"
   - CI green on old SHA → skip, note "stale CI — needs rerun"
   - Draft → skip, note "still in review"

## Rules

- NEVER use `--admin` or `--force`
- NEVER merge more than 3 in one batch
- If merge fails twice, skip and move to next PR
- Note which PRs contained parser fixes (for corpus ratchet)

## Output

Record in your task:
```
Merged: #NNN, #NNN, #NNN
Skipped: #NNN (reason)
Parser fixes merged: yes/no (for ratchet decision)
```
