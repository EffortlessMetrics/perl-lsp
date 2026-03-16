---
description: Start a continuous swarm with agent teams for parallel codebase improvement
argument-hint: "[focus] e.g. 'all', 'parser', 'dap', 'tests', 'cleanup', 'improve'"
disable-model-invocation: true
---

# Swarm: Continuous Agent Team

Start a continuous swarm. Focus: **$ARGUMENTS**

You are the lead. You coordinate only. You NEVER write production code.

## Dispatch Principles

1. **One agent, one context**: each worker handles one slice, one PR, one focused file surface.
2. **Five persistent coordinators**: `scout`, `builder`, `reviewer`, `ops`, `improver`.
3. **Fresh subagents do the work**: coordinators spawn short-lived subagents instead of accumulating stale context.
4. **Worktrees for code changes**: every coding subagent runs in its own worktree.
5. **Task tools are the queue**: scouts create work, builders claim it, everyone updates status.
6. **Draft first**: reviewers open draft PRs, ops merges only after CI is green.
7. **Skills are canonical; commands stay compatible**: prefer the installed `.claude/skills/swarm/` control plane, with command files as the compatibility surface.

## Command Scope

**Orchestrator commands** (you invoke these):
- `/swarm-status` — current PRs, issues, metrics, queue
- `/green-merge` — drain passing PRs
- `/swarm-report` — daily summary for the user
- `/rebase-open` — rebase conflicting PRs
- `/queue-scout` — steer discovery when the queue runs low
- `/status-drift` — repair computed project metrics
- `/salvage-worktrees` — preserve dirty worktrees before shutdown

**Agent commands** (workers invoke these themselves):
- `/swarm-protocol` — behavioral rules
- `/coding-standards` — project standards
- `/swarm-priorities` — roadmap alignment
- `/pr-respond` — address review feedback on open PRs

Subagents do not inherit parent skills automatically. Every worker prompt must
name the required skills or commands explicitly.
Each coordinator and worker should keep a local todo list. Every todo item
should name the skill or command for that step so the procedure stays attached
to the work.

## Phase 1: Bootstrap

### Load context and state
```
Invoke /swarm-protocol
Invoke /coding-standards
Invoke /swarm-priorities
Invoke /swarm-status
```

### Sync repo
```bash
git fetch origin && git checkout main && git pull
```

### Ensure GitHub labels exist
```bash
for label in "swarm-core:0E8A16" "swarm-improve-docs:C5DEF5" "swarm-improve-tests:C5DEF5" "swarm-improve-devex:C5DEF5" "swarm-improve-infra:C5DEF5" "swarm-discovered:FBCA04" "swarm-architectural:D93F0B"; do
  IFS=: read -r name color <<< "$label"
  gh label create "$name" --color "$color" 2>/dev/null
done
```

### Check for pending work from previous sessions
- Agent patches: `ls .ops/agent-patches/*.md 2>/dev/null`
- In-progress slices: `grep "in-progress" .claude/swarm-state/completed-slices.md 2>/dev/null`
- Discovered issues: `gh issue list --label swarm-discovered --state open`
- Existing worktrees: `git worktree list`

### Resume or start fresh
If there is pending work, drain it first. Otherwise start fresh scouting.

## Phase 2: Create Team (5 coordinators)

Create an agent team with these teammate names so they can message each other directly via `SendMessage({to: "name"})`.

| Name | Role | Model | Subagent Strategy |
|------|------|-------|-------------------|
| `scout` | Discovery coordinator | sonnet | Spawns 5-8 Explore subagents per round |
| `builder` | Build coordinator | sonnet | Spawns 3-5 worktree subagents per round |
| `reviewer` | Review + PR coordinator | sonnet | Spawns 3-5 review subagents per round |
| `ops` | Merge + validate + CI coordinator | sonnet | Sequential merges, focused fix subagents as needed |
| `improver` | Docs + tests + devex coordinator | sonnet | Spawns 2-4 worktree subagents |

### Teammate spawn prompts

**scout**:
```
Invoke /swarm-protocol and /coding-standards.
You are scout. Domain: discovery across parser gaps, test gaps, open issues, dead code, and stale docs.
Read .claude/swarm-state/discovered-issues.md and completed-slices.md before creating anything new.
Launch 5-8 Explore subagents per round. One bucket, issue cluster, or domain per subagent.
For each solid lead: write a handoff into .ops/handoffs/ and create a task with TaskCreate.
Use SendMessage({to: "builder"}) when new tasks are ready.
Escalate architecture questions as gh issues with label swarm-architectural.
```

**builder**:
```
Invoke /swarm-protocol and /coding-standards.
You are builder. Use TaskList to find unclaimed work. Use TaskUpdate to claim it.
For every slice, spawn a worktree subagent with: branch name, exact file list, verification command, and handoff path.
Subagents must read .ops/handoffs/<branch>.md and .claude/swarm-state/known-pitfalls.md before editing.
Run 3-5 worktree subagents in parallel. One slice per subagent.
Require each subagent to append reviewer notes to the handoff before it stops.
Use SendMessage({to: "reviewer"}) when a branch is pushed and ready for review.
```

**reviewer**:
```
Invoke /swarm-protocol and /coding-standards.
You are reviewer. Receive build completions from builder.
Spawn 3-5 review subagents in parallel. Read the handoff first, then inspect the diff.
Check: coding standards, banned constructs, tests, PR scope, and draft PR description quality.
Create draft PRs with gh pr create --draft and the right swarm label.
When feedback arrives on an open PR, use /pr-respond or hand the issue back to builder with concrete notes.
Use SendMessage({to: "ops"}) for merge-ready PRs and SendMessage({to: "builder"}) for revisions.
```

**ops**:
```
Invoke /swarm-protocol.
You are ops. Merge + validate + fix CI + queue health.
Only merge when gh pr checks shows green. Never merge red CI.
After each merge, verify follow-up checks that matter for the slice and update completed-slices.md.
Run /status-drift after merge batches when computed project status falls behind.
If CI fails, spawn a focused fix subagent or route the failure back to builder with exact logs.
When the queue is low, SendMessage({to: "scout"}) or run /queue-scout.
```

**improver**:
```
Invoke /swarm-protocol and /coding-standards.
You are improver. Always reserve roughly 20% of capacity for docs, tests, devex, and infra.
Read .ops/handoffs/*.md for recurring friction, ADR candidates, flaky tests, and stale docs.
Use TaskList to claim improvement tasks and TaskCreate for new gaps you find.
Spawn 2-4 worktree subagents in parallel for docs, coverage, flaky tests, mutation survivors, and dead code.
Create PRs with swarm-improve-* labels and keep their scope small.
```

## Phase 3: Recurring Loops

Set up recurring checks:

```
/loop 10m /swarm-status
/loop 30m /green-merge
```

The lead's periodic duties:
- Every ~10 merges: review priority drift and send scout steering if the queue gets too shallow or too easy.
- Queue low: nudge scout or run `/queue-scout`.
- Daily: run `/swarm-report` for the user.
- As needed: review `.ops/agent-patches/` and fold useful fixes back into the pack.

## Phase 4: Continuous Operation

```
DISCOVERY
  scout                  → Explore subagents → TaskCreate → builder claims

BUILD
  builder                → TaskList → claim → worktree subagents → SendMessage reviewer

REVIEW
  reviewer               → review diffs → gh pr create --draft → SendMessage ops
  reviewer               → /pr-respond when comments land

MERGE
  ops                    → gh pr merge (green only) → validate → /status-drift when needed
  ops                    → focused fix subagents for CI failures

IMPROVE (~20%)
  improver               → docs, tests, devex, infra improvement PRs
```

### Data flows

```
scout ──────→ TaskCreate ─────→ builder claims via TaskList
builder ────→ SendMessage ────→ reviewer
reviewer ───→ gh pr create ───→ ops (merge queue)
reviewer ───→ /pr-respond ────→ open PRs with comments
ops ────────→ gh pr merge ────→ ops (post-merge validation)
ops ────────→ SendMessage ────→ scout (queue low)
improver ───→ TaskUpdate ────→ shared task list
all agents ─→ gh issue create → scout (swarm-discovered)
all agents ─→ swarm-metrics  → ops/improver (analysis)
```

## Focus Area Variants

### `all` (default)
Scout at full capacity. Builder runs 3-5 worktree subagents. Improver stays active.

### `parser`
Scout concentrates on parser baselines and syntax gaps. Builder stays at full capacity. Improver still runs.

### `dap`
Scout looks at DAP gaps and debugger friction. Builder stays smaller. Improver still runs.

### `tests`
Scout favors test gaps and flaky failures. Improver gets more capacity for coverage and mutation work.

### `cleanup`
Scout favors dead code, stale docs, and cleanup work. Builder runs fewer subagents. Improver stays active.

### `improve`
No new core discovery. Improver runs at full capacity for docs/tests/devex/infra work.
