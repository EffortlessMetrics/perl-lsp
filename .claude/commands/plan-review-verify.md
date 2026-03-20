---
description: Plan reviewer step 2 — verify the scout's claims against current code
user-invocable: false
---

# Plan Review: Verify

Check that the scout's file:line references and root cause analysis
are still accurate against current master.

## Steps

1. For each file:line the scout referenced, read it:
   ```
   Read the file at the referenced line number
   ```
   Does the code still look like what the scout described?
   Master may have moved since the scout investigated.

2. Verify the root cause:
   - Read the function the scout identified
   - Does the logic match the scout's explanation?
   - Could there be a different root cause?

3. Check for recent changes:
   ```bash
   git log --oneline -5 -- <file>
   ```
   Has someone already partially fixed this?

## Output

Record in your task:
```
File references: CURRENT / STALE (details)
Root cause: CONFIRMED / NEEDS UPDATE (details)
Recent changes: NONE / <list>
```
