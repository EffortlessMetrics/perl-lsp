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

5. **Fix forward:**
   - Logic slightly off? Fix it on the branch.
   - Test only asserts "no crash" instead of behavior? Strengthen the assertion.
   - Regression risk from an uncovered path? Add a test for it.

## Output

Record in your task:
```
Logic: CORRECT / FIXED <what you changed>
Tests: GOOD / IMPROVED <what you added>
Regression risk: LOW / MEDIUM / HIGH (details)
```
