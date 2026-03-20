---
description: Plan reviewer step 3 — stress-test the proposed approach
user-invocable: false
---

# Plan Review: Stress Test

Think adversarially about the scout's recommended approach.

## Steps

1. **What could go wrong with this fix?**
   - Could it break other code paths that use the same function?
   - Does it handle all variants of the construct, or just the sampled ones?
   - Could it cause regressions in existing tests?

2. **Is there a simpler approach?**
   - Read the surrounding code — is there an existing pattern for similar fixes?
   - Could a one-line change work instead of a multi-line refactor?
   - Are there other recent PRs that solved similar problems?

3. **Edge cases the scout might have missed:**
   - Nested versions of the construct
   - The construct inside strings/regex/heredocs
   - Unusual whitespace, comments, or line breaks
   - Empty or minimal versions

4. **Test completeness:**
   - Does the proposed test actually test the right thing?
   - Would it fail before the fix and pass after?
   - Are there edge case tests that should be added?

## Output

Record in your task:
```
Risk assessment: LOW / MEDIUM / HIGH
Simpler alternative: NONE / <description>
Missed edge cases: NONE / <list>
Test improvements: NONE / <suggestions>
```
