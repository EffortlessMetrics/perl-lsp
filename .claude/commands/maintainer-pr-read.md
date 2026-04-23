---
description: Maintainer vision (PR) step 1 — read the PR diff, issue spec, and .spec/ files
user-invocable: false
---

# Maintainer PR: Read

Understand what was built and whether it matches the project's direction.

## Steps

1. Read the PR:
   ```bash
   gh pr view <number> --json title,body,labels,files
   gh pr diff <number>
   ```

2. Read the linked issue:
   ```bash
   ISSUE=$(gh pr view <number> --json closingIssuesReferences --jq '.closingIssuesReferences[0].number // empty')
   gh issue view "$ISSUE" --json title,body,labels,comments
   ```

3. Read .spec/ files if they exist on the branch:
   ```bash
   gh pr checkout <number>
   ls .spec/*/
   cat .spec/*/acceptance.md 2>/dev/null
   cat .spec/*/context.md 2>/dev/null
   ```

4. Check the diff scope — which crates changed?
   ```bash
   gh pr diff <number> --stat
   ```
