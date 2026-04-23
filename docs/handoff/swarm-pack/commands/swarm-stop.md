---
description: Emergency stop the swarm — save state and halt within 5 minutes
argument-hint: "[--reason <why>]"
---

# Swarm Stop (Emergency)

User needs everything stopped in ~5 minutes. No new work, save state, halt. Context: **$ARGUMENTS**

## Minute 1: Halt All Work

Broadcast to ALL teammates simultaneously:
```
SendMessage({to: "scout"}, "STOP: halt immediately, do not launch new subagents or create new tasks")
SendMessage({to: "builder"}, "STOP: do not claim new tasks; let running worktree subagents finish naturally")
SendMessage({to: "reviewer"}, "STOP: do not launch new review subagents; snapshot anything already in progress")
SendMessage({to: "ops"}, "STOP: do not merge; save queue state and recent CI context")
SendMessage({to: "improver"}, "STOP: halt new improvement work; capture any unfinished notes for next session")
```

## Minute 2-3: Save State

1. **Snapshot in-progress work**:
```bash
echo "=== In-progress tasks ==="
# TaskList to see what's claimed but not done

echo "=== Open PRs ==="
gh pr list --state open --json number,title,headRefName

echo "=== Active worktrees ==="
git worktree list

echo "=== Pending handoffs ==="
ls .ops/handoffs/*.md 2>/dev/null
```

2. **Enable auto-merge on all open green PRs** (they'll merge after we stop):
```bash
for pr in $(gh pr list --state open --json number --jq '.[].number'); do
  gh pr merge "$pr" --auto --squash --delete-branch 2>/dev/null && echo "Auto-merge enabled: #$pr"
done
```

3. **Write a session memory**:
Write a Claude Code memory with:
- What was in progress when stopped
- Which PRs are open and their status
- Any blockers or issues discovered
- Reason for emergency stop

## Minute 4: Clean Up Team

1. Shut down all teammates (they should already be idle from the STOP messages)
2. Clean up the team
3. Do NOT prune worktrees — they may have unsaved work. Leave them for `/salvage-worktrees` next session.

## Minute 5: Final Report

```
SWARM STOPPED
reason: $ARGUMENTS
prs_open: <N> (auto-merge enabled on green ones)
tasks_in_progress: <N> (will resume next session)
worktrees_active: <N> (preserved, not pruned)
memory_written: yes
time_to_stop: <actual minutes>
```

## Next Session Resumption

When `/swarm` starts next time, Phase 1 checks for pending work:
- In-progress slices in completed-slices.md
- Open PRs (some may have auto-merged)
- Active worktrees with unsaved work
- Agent patches pending review

The swarm resumes where it left off.
