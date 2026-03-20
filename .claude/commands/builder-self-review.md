---
description: Builder step 5 — re-read your own diff before publishing
---

# Builder Self-Review

Before creating the PR, re-read your own diff with fresh eyes.
This catches dumb mistakes before a reviewer has to.

## Steps

1. Read your diff:
   ```bash
   git diff HEAD~1
   ```

2. Check each changed file:
   - Does this change match what the spec asked for?
   - Did I accidentally include debug code, extra files, or unrelated changes?
   - Are my test names descriptive?
   - Is the diff minimal — no unnecessary whitespace, reformatting, or refactoring?

3. Check your test:
   - Does it test behavior, not implementation details?
   - Would it fail before the fix and pass after?
   - Are edge cases from the plan-reviewer's comments covered?

4. Quick sanity:
   - Any `unwrap()`, `expect()`, `panic!()`, `todo!()`, `dbg!()` that slipped in?
   - Any `.clone()` on Copy types?
   - Any commented-out code?

5. If you find issues: fix them now, re-run `/verify`, then continue.

## Output

Record in your task:
```
Self-review: CLEAN / FIXED <what>
Diff size: <lines added/removed>
Files changed: <count>
```
