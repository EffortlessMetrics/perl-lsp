---
description: Gracefully wind down the swarm — finish in-progress work, merge what's ready, clean up
argument-hint: "[--reason <why>]"
---

# Swarm Wind Down (Graceful)

The user has some time. Wind down cleanly over ~15-30 minutes. Context: **$ARGUMENTS**

## Phase 1: Stop New Work (immediate)

1. **Message all scouts**: `SendMessage({to: "scout-1"}, "WIND DOWN: stop launching new subagents. Do not create new tasks. Finish writing any handoff files in progress.")`
2. **Message all scouts**: same to scout-2
3. **Message strategist**: `SendMessage({to: "strategist"}, "WIND DOWN: produce a final strategy report and write Claude Code memories for session progress.")`

## Phase 2: Let Builders Finish (5-10 min)

1. **Message builders**: `SendMessage({to: "builder-1"}, "WIND DOWN: finish your current subagent builds. Do not claim new tasks. When current builds complete, send results to reviewer as normal.")`
2. Same to builder-2
3. Wait for in-progress builds to complete (check task list for in-progress items)
4. **Message improvers**: `SendMessage({to: "improver-docs"}, "WIND DOWN: finish current subagents, create PRs for completed work, then stop.")`
5. Same to improver-tests

## Phase 3: Review and PR Everything (5-10 min)

1. **Message reviewer**: `SendMessage({to: "reviewer"}, "WIND DOWN: review all pending builds, create PRs for everything merge-ready. Enable auto-merge on all improvement PRs.")`
2. **Message pr-responder**: `SendMessage({to: "pr-responder"}, "WIND DOWN: address any outstanding review comments on open PRs, then stop.")`
3. Wait for all PRs to be created

## Phase 4: Merge What's Green (5 min)

1. **Message merger**: `SendMessage({to: "merger"}, "WIND DOWN: merge all green PRs. Enable auto-merge on anything with pending checks. Run /status-drift --commit after merges. Write a Claude Code memory summarizing this session's results.")`
2. **Message validator**: `SendMessage({to: "validator"}, "WIND DOWN: validate any recent merges, then stop.")`

## Phase 5: Clean Up and Report

1. Invoke `/salvage-worktrees --prune-merged` to clean up
2. Invoke `/swarm-report` to generate session summary
3. Write Claude Code memories:
   - Session results: PRs merged, issues created, corpus/coverage changes
   - Roadmap progress: what NOW items advanced
   - Agent performance: what worked well, what didn't
   - Unfinished work: what's in-progress, what's queued

4. **Final status check**:
```bash
echo "=== Remaining open PRs ==="
gh pr list --state open --json number,title,labels
echo "=== Remaining in-progress slices ==="
grep "in-progress" .claude/swarm-state/completed-slices.md
echo "=== Agent patches pending review ==="
ls .ops/agent-patches/*.md 2>/dev/null
echo "=== Discovered issues ==="
gh issue list --label swarm-discovered --state open
```

5. Shut down all teammates
6. Clean up the team

## What Gets Preserved for Next Session

- `.claude/swarm-state/known-pitfalls.md` — failure knowledge
- `.claude/swarm-state/completed-slices.md` — with in-progress items for resumption
- `.ops/swarm-metrics.jsonl` — performance history
- `.ops/agent-patches/` — pending improvements
- GitHub issues labeled `swarm-discovered` — persistent backlog
- Open PRs with auto-merge enabled — will merge when checks pass
- Claude Code memories — session progress for next orchestrator
