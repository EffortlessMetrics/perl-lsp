---
description: Ops step 4 — post-merge corpus ratchet and status update
user-invocable: false
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

4. Note user-visible changes for changelog:
   For each merged PR, check if it's user-facing (feat, fix affecting behavior).
   If so, note it for the next CHANGELOG update:
   ```
   - feat: <description> (#NNN)
   - fix: <description> (#NNN)
   ```

5. Post a merge summary comment on the most recently merged PR in this batch:
   ```bash
   gh pr comment <NUMBER> --body "$(cat <<'MERGE_EOF'
   ## Merge Summary

   **Merged:** <list of PRs merged in this batch>
   **Master status:** <CI passing | blocked>
   **Corpus ratcheted:** <yes (new count) | no | N/A>
   **User-visible changes:** <list or none>
   MERGE_EOF
   )"
   ```

## Output

Record in your task:
```
Corpus ratcheted: yes/no (new count)
Status updated: yes/no
Master CI: green/red
Changelog candidates: <list of user-visible PRs>
```
