---
description: Ops step 4 — post-merge corpus ratchet and status update
---

# Ops Post-Merge

After a merge batch, lock in gains and update metrics.

## Steps

1. If parser fixes were merged, run corpus ratchet:
   ```bash
   just cpan-corpus-ratchet
   ```
   If new modules were added to manifest, commit and PR:
   ```bash
   git checkout -b chore/corpus-ratchet-$(date +%Y%m%d)
   git add .ci/cpan-corpus-manifest.txt
   git commit -m "chore(corpus): ratchet baseline after parser fix merge"
   git push -u origin HEAD
   gh pr create --title "chore(corpus): ratchet baseline" --body "Auto-ratchet after parser merges."
   ```

2. If tests were added, update status:
   ```bash
   python3 scripts/update-current-status.py
   ```
   If changed, commit and PR.

3. Check for systemic CI issues:
   ```bash
   gh run list --branch master --limit 3 --json status,conclusion --jq '.[] | "\(.status) \(.conclusion)"'
   ```

## Output

Record in your task:
```
Corpus ratcheted: yes/no (new count)
Status updated: yes/no
Master CI: green/red
```
