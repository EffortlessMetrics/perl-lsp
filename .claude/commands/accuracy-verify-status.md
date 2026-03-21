---
description: Accuracy-scout step 4 — check if issue already fixed via recent merges or commits
user-invocable: false
---

# Accuracy: Verify Status

Check whether the issue is already fixed, actively being worked on in another
PR, or is a duplicate of a known issue.

## Steps

1. **Check recent merged PRs for fixes:**

   ```bash
   # Last 50 merged PRs — look for keywords matching the issue
   gh pr list --state merged --limit 50 --json number,title,mergedAt \
     --jq '.[] | "#\(.number) \(.title) [merged: \(.mergedAt)]"' | \
     grep -i "<keyword1>\|<keyword2>" | head -10
   ```

   Use 2-3 keywords from the issue title. If a merged PR title mentions the
   same function, module, or behavior — issue may be fixed.

2. **Check open PRs that may already address the issue:**

   ```bash
   gh pr list --state open --limit 100 --json number,title,labels \
     --jq '.[] | "#\(.number) \(.title)"' | grep -i "<keyword>" | head -10
   ```

3. **Check git log for relevant commits:**

   ```bash
   git log --oneline -50 | grep -i "<keyword>" | head -10
   ```

4. **Check if the issue references a PR that's already closed:**

   If the issue body mentions "see PR #NNN" or "duplicate of #NNN":
   ```bash
   gh pr view <NNN> --json state,mergedAt --jq '{state, mergedAt}'
   gh issue view <NNN> --json state --jq '.state'
   ```

5. **Check if issue already has `accuracy-reviewed` label** (re-run guard):

   ```bash
   gh issue view <number> --json labels --jq '[.labels[].name]'
   ```

   If already labeled `accuracy-reviewed`, this issue was already processed.
   Report and stop.

## Decision Rules

| Signal | Recommendation |
|--------|---------------|
| Merged PR within 7 days covers exact function/file | LIKELY FIXED — recommend close |
| Open PR covers same area | DUPLICATE RISK — link in comment |
| Issue references a closed issue as "same as" | DUPLICATE — recommend close with link |
| No signal | PROCEED — no evidence already fixed |

## Output

```
Status check for issue #NNN:

  Recent merged PRs covering this area:
    #2528 "fix(parser): rename parse_hash_or_block" — merged 2026-03-15
  Open PRs covering this area:
    None found
  Git log matches:
    a1b2c3d fix: remove parse_method_call, use parse_method_invocation

Recommendation: LIKELY FIXED — PR #2528 merged and covers the exact function cited
```
