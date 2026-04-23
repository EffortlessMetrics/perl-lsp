---
description: Launch scouts across multiple focus areas to find improvement slices
argument-hint: "[focus] e.g. 'all', 'bugs', 'tests', 'dead-code'"
---

# Queue Scout

Launch `scout` agents to find improvement slices. Focus: **$ARGUMENTS**

## `all` (default)

Launch 10-15 scouts across:
- Error/bug sources (3-4 scouts)
- Test gaps (2-3 scouts)
- Open issues (2-3 scouts)
- Dead code / unused deps (1-2 scouts)
- Ignored/skipped tests (1-2 scouts)

## Dispatch Pattern

```
Agent(
  subagent_type: "scout",
  prompt: "Focus area: <target>. Find ONE actionable improvement.",
  model: "sonnet",
  run_in_background: true,
  name: "scout-<focus>-<N>"
)
```

## After Scouts Complete

1. Collect SLICE outputs
2. Check `files_touched` for overlaps
3. Keep higher-impact slice when overlapping
4. Create tasks for non-overlapping slices
5. Update `.claude/swarm-state/swarm-queue.json`
