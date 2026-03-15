---
description: Load roadmap priorities and weight scout targets for strategic alignment
argument-hint: ""
---

# Swarm Priorities

Load the project's strategic direction so the swarm works on the RIGHT things, not just easy things.

## Priority Sources

Read these to understand what matters:

```bash
# Roadmap and strategic direction
cat NOW_NEXT_LATER.md 2>/dev/null || cat ROADMAP.md 2>/dev/null
cat docs/project/ROADMAP.md 2>/dev/null

# Current status (what's been accomplished)
cat docs/project/CURRENT_STATUS.md 2>/dev/null

# Open milestones
gh api repos/:owner/:repo/milestones --jq '.[] | "\(.title): \(.open_issues) open, \(.description)"' 2>/dev/null

# High-priority issues
gh issue list --state open --label "priority:high" --limit 20 2>/dev/null
gh issue list --state open --label "bug" --limit 20 2>/dev/null

# Features catalog
cat features.toml 2>/dev/null | head -50
```

## Priority Tiers

Scouts should weight slices in this order:

### P0: Blocking / Security
- Security vulnerabilities (cargo audit findings)
- Broken CI gates
- Regressions from recent merges
- Issues labeled `priority:high` or `bug`

### P1: Roadmap Alignment
- Work that advances the current NOW items in NOW_NEXT_LATER.md
- Parser corpus improvement (currently 51% — every point matters)
- LSP feature completion (features.toml gaps)
- Open milestones with deadlines

### P2: Test Infrastructure
- Mutation survivors in critical paths
- Flaky tests that block CI
- Coverage gaps in user-facing code
- Integration test gaps

### P3: Codebase Health
- DAP test coverage (known zero-test crates)
- Technical debt from debt-ledger.yaml
- Dead code and unused deps
- Documentation that's actively wrong

### P4: Polish
- Test naming improvements
- Error message quality
- Observability additions
- Docs for already-documented features

## How Scouts Use This

When creating slices, scouts should:
1. Read this skill to understand current priorities
2. Tag each SLICE with a priority tier: `priority: P0 | P1 | P2 | P3 | P4`
3. Builders should claim higher-priority tasks first
4. If the queue is mostly P3/P4, scouts should dig harder for P1/P2 work

## How the Lead Uses This

Periodically (every ~10 merges):
1. Invoke `/swarm-priorities`
2. Check: are most merged PRs P1/P2? Good. Mostly P3/P4? Steer scouts.
3. Message scouts with adjusted focus areas if priority drift is detected
