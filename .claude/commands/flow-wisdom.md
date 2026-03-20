---
description: "Flow: synthesize learnings from a completed issue→PR→merge cycle"
argument-hint: "<issue-number>"
---

# Flow: Wisdom

After a change has been merged, spawn a wisdom agent to read the full
trail and extract learnings.

## Steps

1. Find the PR(s) linked to the issue:
   ```bash
   gh issue view $ARGUMENTS --json body --jq '.body' | grep -oE '#[0-9]+'
   ```

2. Spawn the wisdom agent:
   ```
   Agent(
     subagent_type: "wisdom",
     prompt: "Read the full trail for issue #$ARGUMENTS and its PR(s). Follow your todo list.",
     model: "sonnet",
     name: "wisdom-$ARGUMENTS"
   )
   ```

3. The wisdom agent reads the issue, plan review, PR, reviews, and merged code, then synthesizes patterns and documents findings.

## When to use

- After a batch of related issues are merged (e.g., "5 parser fixes in one cycle")
- After a particularly complex or multi-round change
- Periodically to assess pipeline health
- When the orchestrator wants to understand "what did we learn from this?"
