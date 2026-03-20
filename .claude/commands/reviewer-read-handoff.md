---
description: Reviewer step 1 — read the PR handoff and understand the change
---

# Reviewer Read Handoff

Understand the PR before reading the diff.

## Steps

1. Read the PR description and linked issue:
   ```bash
   gh pr view <number> --json title,body,labels --jq '{title: .title, body: .body}'
   ```

2. If the PR links an issue, read the issue for the original spec:
   ```bash
   gh issue view <number> --json body --jq '.body'
   ```

3. Check for a verification receipt — did the builder run tests?
   Look for verification results in PR description or comments.

4. Note what you expect to see in the diff:
   - Which files should be changed?
   - What test should be added?
   - What behavior should change?

## Output

Record in your task:
```
PR: #<number>
Issue: #<number> or none
Expected changes: <files and behavior>
Builder verified: yes/no
```
