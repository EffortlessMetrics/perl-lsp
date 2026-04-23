---
description: Plan reviewer step 1 — read the issue and understand the scout's analysis
user-invocable: false
---

# Plan Review: Read

Read the issue that a scout filed and understand the proposed change.

## Steps

1. Read the issue:
   ```bash
   gh issue view <number> --json title,body,labels --jq '{title: .title, body: .body}'
   ```

2. Extract the key elements:
   - **Root cause**: What does the scout say is wrong?
   - **Recommended fix**: What change does the scout propose?
   - **File:line**: What locations are referenced?
   - **Test spec**: What test does the scout suggest?
   - **Verify command**: How to confirm the fix works?

3. Note any gaps — fields that are vague, missing, or feel uncertain. **You'll fill these in step 4**, not punt them back.

## Output

Record in your task:
```
Issue: #NNN
Root cause claim: <from issue>
Proposed fix: <from issue>
File references: <list>
Gaps noticed: <list or NONE>
```
