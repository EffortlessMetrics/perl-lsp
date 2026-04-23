---
description: Reviewer step 1 — read the PR handoff, check knowledge artifact quality
user-invocable: false
---

# Reviewer Read Handoff

Understand the PR before reading the diff. Also check that the PR
description is a useful knowledge artifact — not just for you, but
for whoever touches this code next.

## Steps

1. Claim the PR immediately to prevent double-assignment:
   ```bash
   gh pr edit <number> --add-label "in-review"
   ```
   Do this BEFORE reading the diff. The `in-review` label tells the orchestrator
   this PR is actively being reviewed and should not be dispatched to another reviewer.

   After claiming, write a version-bound receipt:
   ```
   /label-receipt-write pr <number> in-review reviewer
   ```

2. Read the PR description and linked issue:
   ```bash
   gh pr view <number> --json title,body,labels --jq '{title: .title, body: .body}'
   ```

3. If the PR links an issue, read the issue for the original spec:
   ```bash
   gh issue view <number> --json body --jq '.body'
   ```

4. Check for a verification receipt — did the builder run tests?
   Look for verification results in PR description or comments.

5. **Check knowledge artifact quality** — the PR description should have:
   - Summary: what changed and why (linked to issue)
   - Test: what test was added
   - What I considered but didn't do
   - What's next
   If these are missing or thin, note it — the builder should be
   writing useful context, not just "fixes #NNN."

6. Note what you expect to see in the diff:
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
PR description quality: GOOD / THIN (what's missing)
```
