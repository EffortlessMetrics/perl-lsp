---
name: swarm-priorities
description: Load roadmap weighting for scouts and coordinators so slices are ranked by actual repo priorities instead of convenience.
user-invocable: false
---

# Swarm Priorities

Load the repo's current priority weighting before scouting or reprioritizing the
queue.

## Priority Sources

Read what is available and current:

```bash
cat NOW_NEXT_LATER.md 2>/dev/null || cat ROADMAP.md 2>/dev/null
cat docs/project/ROADMAP.md 2>/dev/null
cat docs/project/CURRENT_STATUS.md 2>/dev/null
gh api repos/:owner/:repo/milestones --jq '.[] | "\\(.title): \\(.open_issues) open, \\(.description)"' 2>/dev/null
gh issue list --state open --label "priority:high" --limit 20 2>/dev/null
gh issue list --state open --label "bug" --limit 20 2>/dev/null
cat features.toml 2>/dev/null | head -50
```

## Priority Tiers

- `P0`: security, broken CI, merge regressions, urgent bugs
- `P1`: roadmap-aligned parser, LSP, milestone, or capability work
- `P2`: test infrastructure, flaky tests, mutation survivors, coverage gaps
- `P3`: codebase health, DAP quality, debt, dead code, stale docs
- `P4`: polish and low-leverage cleanup

## How To Use It

- scouts should tag slices with a priority tier
- builders should prefer higher-priority queued work first
- leads should check for drift when merges skew toward low-leverage cleanup

If the queue is dominated by `P3` or `P4`, scout harder for `P1` or `P2`
work instead of just consuming the easy backlog.
