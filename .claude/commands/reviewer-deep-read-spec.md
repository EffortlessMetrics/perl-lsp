---
description: Deep reviewer step 1 — read the original issue spec to understand intent
user-invocable: false
---

# Deep Reviewer Read Spec

Understand what the PR is SUPPOSED to do before judging if it does it.

## Steps

1. Get the linked issue from the PR:
   ```bash
   gh pr view <number> --json body --jq '.body' | grep -oE '#[0-9]+'
   ```

2. Read the issue's root cause and recommended fix:
   ```bash
   gh issue view <number> --json body --jq '.body'
   ```

3. Note the expected behavior change:
   - What was broken before?
   - What should work after?
   - What test proves it?

## Output

Record in your task:
```
Issue: #NNN
Root cause: <from issue>
Expected fix: <from issue's recommendation>
Test expectation: <what the test should verify>
```
