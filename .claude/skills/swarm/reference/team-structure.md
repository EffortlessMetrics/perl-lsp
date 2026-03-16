# Swarm Team Structure

## Roles

| Name | Role | Agent Type | Model | Subagent Strategy |
|------|------|-----------|-------|-------------------|
| `scout` | Discovery coordinator | swarm-scout | sonnet | Spawns 5-8 Explore subagents/round |
| `builder` | Build coordinator | swarm-builder | sonnet | Spawns 3-5 worktree subagents/round |
| `reviewer` | Review + PR creation | swarm-reviewer | sonnet | Spawns 3-5 review subagents/round |
| `ops` | Merge + validate + fix CI | swarm-ops | sonnet | Sequential merges, spawns fix subagents |
| `improver` | Docs + tests + devex | swarm-improver | sonnet | Spawns 2-4 worktree subagents |

## Execution Doctrine

- Coordinators are persistent; implementation workers are disposable.
- Every PR-shaped code change gets its own worktree.
- Every materially different context gets a fresh worker.
- Stable procedure belongs in skills and templates; volatile task state belongs in handoffs, worktrees, and PRs.

### Context Shift Triggers

Spawn a new worker when any of these change:
- objective or hypothesis
- dominant crate or file surface
- tool or permission profile
- verification command
- branch or PR target

## Data Flow

```
scout ──────→ TaskCreate ─────→ builder claims via TaskList
builder ────→ SendMessage ────→ reviewer
reviewer ───→ gh pr create ───→ ops (merge queue)
ops ────────→ gh pr merge ────→ ops (verify post-merge)
ops ────────→ SendMessage ────→ scout (queue low)
ops ────────→ /corpus-ratchet → lock in gains
improver ───→ worktree subs ──→ improvement PRs
all agents ─→ gh issue create → scout (swarm-discovered)
all agents ─→ swarm-metrics  → ops (analysis)
all agents ─→ TaskUpdate ────→ shared task list
```

## Capacity Allocation

- **Core work**: ~80% (scout, builder, reviewer, ops)
- **Background improvement**: ~20% (improver)

## Communication Patterns

- Scout → builder: via TaskCreate + SendMessage
- Builder → reviewer: via SendMessage with branch/handoff path
- Reviewer → ops: via SendMessage with PR number
- Ops → scout: via SendMessage when queue is low
- All → ops: via SendMessage for CI failures

## Spawn Rules

- One scout subagent per discovery bucket or issue cluster
- One builder worker per PR-shaped change
- One reviewer worker per PR
- One fixer worker per failure mode
- Retire a worker instead of stretching it across a new crate, branch, or verification loop
