# Swarm Quick Reference

## Lifecycle

```
/swarm all              Start the swarm (5 coordinators, continuous)
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

## Coordinator Teammates (5)

| Name | Role | Spawns |
|------|------|--------|
| scout | Discovery — find gaps, write handoffs | 5-8 Explore subagents/round |
| builder | Build — claim tasks, implement | 3-5 worktree subagents/round |
| reviewer | Review — review diffs, create PRs | 3-5 review subagents/round |
| ops | Merge + validate + fix CI | Sequential merges, fix subagents |
| improver | Background improvement | 2-4 worktree subagents |

## Data Flow

```
scout ──────→ TaskCreate ─────→ builder claims via TaskList
builder ────→ SendMessage ────→ reviewer
reviewer ───→ gh pr create ───→ ops (merge queue)
ops ────────→ gh pr merge ────→ ops (verify post-merge)
ops ────────→ SendMessage ────→ scout (queue low)
ops ────────→ /status-drift ──→ repair computed drift when needed
improver ───→ worktree subs ──→ improvement PRs
all agents ─→ gh issue create → scout (swarm-discovered)
all agents ─→ swarm-metrics  → ops (analysis)
all agents ─→ TaskUpdate ────→ shared task list
```

## State Files

### Tracked (`.claude/swarm-state/` — committed, persists across sessions)

| File | Purpose | Writers | Readers |
|------|---------|---------|---------|
| `known-pitfalls.md` | Failure knowledge | fixer | scout, builder |
| `completed-slices.md` | Dedup log | scout, ops | scout, improvers |
| `discovered-issues.md` | Agent-flagged leads | all agents | scout |
| `findings.json` | Durable control-plane findings | scout, improver, reviewer, ops | all lanes |
| `findings.schema.json` | Contract for findings ledger | repo maintainers | all lanes |
| `swarm-queue.json` | Overlap tracking | scout, lead | scout, lead |

### Ephemeral (`.ops/` — gitignored, per-session runtime)

| File | Purpose | Writers | Readers |
|------|---------|---------|---------|
| `handoffs/<branch>.md` | Context transfer | scout, builder, fixer | builder, reviewer, improvers |
| `swarm-metrics.jsonl` | Performance data | all agents | ops, lead |
| `agent-patches/` | Self-improvement | fixer, any agent | bootstrapper |
| `salvage/` | Emergency worktree dumps | janitor | user |

## Commands — Scope Reference

### Orchestrator-only (lead invokes these)
```
/swarm-status      /green-merge        /swarm-report
/rebase-open       /queue-scout        /status-drift
/salvage-worktrees /swarm-stop         /swarm-wind-down
```

### Agent-only (agents invoke — do NOT load into orchestrator context)
```
/swarm-protocol   /coding-standards   /swarm-priorities
/pr-respond
```

### Shared setup commands
```
/bootstrap-agents
```

## Hooks (auto-fire, no agent memory required)

| Event | What It Does |
|-------|-------------|
| `PostToolUse` (Edit/Write) | Auto-format + check edited source files |
| `TaskCompleted` | Block ghost completions — verify deliverables exist |
| `TeammateIdle` | Detect idle agents with unclaimed work |
| `SubagentStart` (builder/reviewer/fixer/etc.) | Auto-inject coding standards |
| `SubagentStop` (builder/reviewer/fixer/etc.) | Record worker teardown and handoff boundaries |
| `PreToolUse` (Bash) | Block dangerous commands |
| `SessionStart` (compact) | Inject context refresh after compaction |

All hooks read JSON from stdin. Register in `.claude/settings.json`.

## Research Agents (spawn from any agent)

```
Agent(prompt: "Research: <question>", run_in_background: true, name: "research-web")
Agent(prompt: "Look up docs: <API>", run_in_background: true, name: "research-docs")
Agent(prompt: "Verify: <claim>", run_in_background: true, name: "research-verify")
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
