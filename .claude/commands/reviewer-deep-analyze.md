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

4. Check for **vacuous assertions** — assertions on hardcoded data that prove nothing about the code under test:
   - `assert!(vec.len() > 0)` where the Vec was constructed from a hardcoded non-empty list
   - `assert!(!s.is_empty())` on a string literal
   - `assert_eq!(result.len(), N)` when result was built directly from N hardcoded items
   - `assert!(result.is_some())` when result is `Some(hardcoded_value)`
   - The litmus test: would this assertion still pass if the feature code were commented out? If yes, the test is vacuous.

5. Check for regressions:
   - Does the change affect any other code paths?
   - Could existing callers break?
   - Are there related tests that might need updating?

6. **Fix forward:**
   - Logic slightly off? Fix it on the branch.
   - Test only asserts "no crash" instead of behavior? Strengthen the assertion.
   - Vacuous assertion? Rewrite to test actual behavior of the code under test.
   - Regression risk from an uncovered path? Add a test for it.

## Output

Record in your task:
```
Logic: CORRECT / FIXED <what you changed>
Tests: GOOD / IMPROVED <what you added>
Regression risk: LOW / MEDIUM / HIGH (details)
```
