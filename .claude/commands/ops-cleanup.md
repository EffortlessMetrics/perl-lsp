---
description: Ops step 5 — post-merge hygiene and drift check
user-invocable: false
---

# Ops Cleanup

After a merge batch, tidy up. This prevents drift from accumulating
across cycles.

## Steps

1. **Clean stale worktrees:**
   ```bash
   just clean-worktrees 2>/dev/null || git worktree prune
   ```

2. **Check for CURRENT_STATUS drift:**
   ```bash
   python3 scripts/update-current-status.py 2>/dev/null
   git diff --quiet docs/project/CURRENT_STATUS.md || echo "STATUS DRIFTED"
   ```
   If drifted, commit and create a quick PR.

3. **Check corpus baseline freshness:**
   If parser fixes were merged, the baseline may need ratcheting:
   ```bash
   just cpan-corpus-sweep 2>/dev/null | tail -5
   ```
   Compare with `.ci/cpan-corpus-baseline.json`. If stale, ratchet.

4. **Check for broken master:**
   ```bash
   gh run list --branch master --limit 1 --json conclusion --jq '.[0].conclusion'
   ```
   If not "success", flag for investigation.

5. **Prune merged branches:**
   ```bash
   git fetch --prune origin
   ```

## Output

Record in your task:
```
Worktrees cleaned: <count removed>
Status drift: NONE / FIXED
Corpus baseline: CURRENT / RATCHETED
Master CI: GREEN / RED
```
