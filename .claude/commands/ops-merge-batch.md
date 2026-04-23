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

3. **Policy gate (defense-in-depth)** — run the scripted pre-merge guard:
   ```bash
   just pre-merge-check <number>
   # or: bash scripts/pre-merge-check.sh <number>
   ```
   This codifies the policy:
   - non-docs PRs need `deep-reviewed`
   - docs-only PRs may merge with `merge-ready` alone
   - draft/title/label mistakes still fail loud
   If it fails, **skip this PR** with the script's reason.

4. **Build a good commit message** for each PR:
   ```bash
   # Get the PR title and body
   gh pr view <number> --json title,body
   ```
   The squash commit message should be: `<PR title> (#<number>)` as the first line,
   followed by a blank line and a 1-3 sentence summary of WHAT changed and WHY.
   Future readers should understand the change without opening the PR.

5. Merge each PR with squash:
   ```bash
   gh pr merge <number> --squash --subject "<title> (#<number>)" --body "<summary>"
   ```

6. After each merge, verify it landed and clean up labels:
   ```bash
   gh pr view <number> --json state --jq .state
   # Remove merge-ready from the now-merged PR
   gh pr edit <number> --remove-label "merge-ready"
   # Remove deep-reviewed from the now-merged PR
   gh pr edit <number> --remove-label "deep-reviewed"
   # Remove in-build from the linked issue (if any)
   CLOSING_ISSUE=$(gh pr view <number> --json closingIssuesReferences --jq '.closingIssuesReferences[0].number // empty')
   if [ -n "$CLOSING_ISSUE" ]; then
     gh issue edit "$CLOSING_ISSUE" --remove-label "in-build"
   fi
   ```
   Label cleanup prevents stale `merge-ready`, `deep-reviewed`, and `in-build` labels from
   misleading future orchestrator queries.

7. If a merge fails or pre-check fails:
   - CONFLICTING → skip, note "needs rebase"
   - CI red or pending → skip, note "CI not green on current HEAD"
   - CI green on old SHA → skip, note "stale CI — needs rerun"
   - Draft → skip, note "still in review"
   - Missing `deep-reviewed` on a non-docs PR → skip, note "missing deep review signal"

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
