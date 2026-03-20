---
description: Ops step 2 — merge a batch of up to 3 PRs
---

# Ops Merge Batch

Merge up to 3 PRs from the candidates identified in step 1.

## Steps

1. Pick up to 3 PRs from the MERGE NOW list.
   Respect dependency order:
   - If PR B depends on PR A (same files), merge A first
   - Parser fixes before corpus ratchets
   - Infrastructure before features

2. Merge each PR:
   ```bash
   gh pr merge <number> --merge
   ```

3. After each merge, wait a moment for GitHub to process:
   ```bash
   # Verify it landed
   gh pr view <number> --json state --jq .state
   ```

4. If a merge fails:
   - CONFLICTING → skip, note "needs rebase"
   - CI red → skip, note "CI failure: <check name>"
   - Draft → `gh pr ready <number>` first, then retry

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
