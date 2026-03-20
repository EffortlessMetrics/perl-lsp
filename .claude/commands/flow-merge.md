---
description: "Flow: merge approved PRs in safe batches"
---

# Flow: Merge

Process the merge queue — merge approved PRs in batches of 3.

## Steps

1. Spawn the ops agent:
   ```
   Agent(
     subagent_type: "ops",
     prompt: "Process the merge queue. Follow your todo list.",
     model: "haiku",
     name: "ops-merge"
   )
   ```

2. Ops follows its 4-step todo:
   check-queue → merge-batch → verify-master-green → post-merge

3. After merge:
   - If parser fixes merged → corpus ratcheted
   - If tests added → CURRENT_STATUS updated
   - Master CI verified green

## What a successful flow produces

- Merged PRs (batches of 3)
- Master CI green
- Corpus ratcheted (if applicable)
- Status updated (if applicable)
