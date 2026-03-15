# Swarm Quick Reference

## Lifecycle

```
/swarm all              Start the swarm (12 teammates, continuous)
/swarm parser           Focus on parser work
/swarm improve          Full capacity to codebase health
/swarm-wind-down        Graceful shutdown (~20 min)
/swarm-stop             Emergency halt (~5 min)
```

## Observability

```
/swarm-status           PRs, issues, metrics, queue depth
/swarm-report           Daily summary for check-in
/swarm-priorities       Roadmap alignment and P0-P4 tiers
```

## Operations

```
/green-merge            Merge all passing PRs
/rebase-open            Rebase conflicted PRs onto main
/status-drift           Fix computed metric drift
/salvage-worktrees      Save dirty worktrees before cleanup
/queue-scout            Launch scouts across focus areas
/pr-respond <N>         Address review comments on PR #N
```

## Setup

```
/bootstrap-agents       Discover codebase → generate domain agents
/coding-standards       Load project coding standards
/swarm-protocol         Load swarm behavioral rules
```

## Teammates (12)

| Name | Role | Talks to |
|------|------|----------|
| scout-1 | Parser + corpus gaps | builder-1, builder-2 |
| scout-2 | DAP + issues + cleanup | builder-1, builder-2 |
| builder-1 | Build in worktrees | reviewer |
| builder-2 | Build in worktrees | reviewer |
| reviewer | Review + create PRs | merger, fixer, pr-responder |
| pr-responder | Address review comments | merger |
| merger | Merge + drift | validator, scouts, fixer |
| validator | Post-merge verification | fixer, improver-tests |
| improver-docs | ADRs, changelog, friction | (creates PRs directly) |
| improver-tests | Mutants, flaky, coverage | (creates PRs directly) |
| strategist | Priority alignment | scouts (steering) |
| fixer | CI failures | merger |

## State Files

### Tracked (`.claude/swarm-state/` — committed, persists across sessions)

| File | Purpose | Writers | Readers |
|------|---------|---------|---------|
| `known-pitfalls.md` | Failure knowledge | fixer | scout, builder |
| `completed-slices.md` | Dedup log | scout, merger | scout, improvers |
| `discovered-issues.md` | Agent-flagged leads | all agents | scout |
| `swarm-queue.json` | Overlap tracking | scout, lead | scout, lead |

### Ephemeral (`.ops/` — gitignored, per-session runtime)

| File | Purpose | Writers | Readers |
|------|---------|---------|---------|
| `handoffs/<branch>.md` | Context transfer | scout, builder, fixer | builder, reviewer, improvers |
| `swarm-metrics.jsonl` | Performance data | all agents | strategist, merger |
| `agent-patches/` | Self-improvement | fixer, any agent | bootstrapper |
| `salvage/` | Emergency worktree dumps | janitor | user |

## Research Agents (spawn from any agent)

```
Agent(prompt: "Research: <question>", run_in_background: true, name: "research-<topic>")
Agent(prompt: "Look up docs: <API>", run_in_background: true, name: "docs-<topic>")
Agent(prompt: "Verify: <claim>", run_in_background: true, name: "verify-<topic>")
```

## GitHub Labels

| Label | Meaning |
|-------|---------|
| `swarm-core` | Primary task implementation |
| `swarm-improve-docs` | Documentation improvement |
| `swarm-improve-tests` | Test quality improvement |
| `swarm-improve-devex` | Developer experience improvement |
| `swarm-improve-infra` | Infrastructure improvement |
| `swarm-discovered` | Issue found by agent during other work |
| `swarm-architectural` | Needs architectural decision from user |

## Priority Tiers

| Tier | What | Scout action |
|------|------|-------------|
| P0 | Security, broken CI, regressions | Always first |
| P1 | Roadmap NOW items, corpus, features | Primary focus |
| P2 | Test infrastructure, mutants, flaky | Secondary focus |
| P3 | Health: DAP tests, debt, dead code | Background |
| P4 | Polish: naming, errors, observability | When queue is light |

## Data Flow

```
scouts → TaskCreate → builders claim → build in worktrees
builders → SendMessage → reviewer → gh pr create → merger
merger → gh pr merge → validator (verify) → lock in gains
merger → SendMessage → scouts (queue low)
validator → gh issue create → fixer (regression)
strategist → SendMessage → scouts (priority steering)
fixer → known-pitfalls → scouts, builders (avoid traps)
all agents → gh issue create → scouts (swarm-discovered)
improvers → read handoffs → ADRs, friction log, docs
lead → memories → future sessions
```
