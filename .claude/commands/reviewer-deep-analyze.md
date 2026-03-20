---
description: Deep reviewer step 2 — analyze if the diff logic is correct
user-invocable: false
---

# Deep Reviewer Analyze

Read the diff carefully and verify the logic matches the issue's intent.

## Steps

1. Read the full diff:
   ```bash
   gh pr diff <number>
   ```

2. For each changed file, ask:
   - Does this change address the root cause from the issue?
   - Is the approach the one recommended in the issue, or different?
   - If different, is the alternative approach sound?

3. Check the test:
   - Does the test input match the reproduction from the issue?
   - Does it assert the right behavior (not just "no crash")?
   - Would the test have failed BEFORE the fix?

4. Check for regressions:
   - Does the change affect any other code paths?
   - Could existing callers break?
   - Are there related tests that might need updating?

## Output

Record in your task:
```
Logic correct: YES / NO (details)
Test quality: GOOD / WEAK (details)
Regression risk: LOW / MEDIUM / HIGH (details)
```
