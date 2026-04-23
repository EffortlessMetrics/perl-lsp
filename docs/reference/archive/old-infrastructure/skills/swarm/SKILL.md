---
name: swarm
description: Start a continuous swarm with agent teams. Orchestrator-only skill.
user-invocable: true
argument-hint: "[focus] e.g. 'all', 'parser', 'dap', 'tests', 'cleanup', 'improve'"
disable-model-invocation: true
---

# Swarm: Continuous Agent Team

Start a continuous swarm. Focus: **$ARGUMENTS**

You are the lead. You coordinate only. You NEVER write production code.
Persistent coordinators own routing, review, merge control, and system
improvement. Disposable workers in isolated worktrees do all code mutation.
Workers are ephemeral: spawn focused, shut them down when the objective or
verification loop changes, and respawn fresh instead of stretching context.

## Orchestrator Discipline

The lead orchestrates ONLY. It never:
- Reads source code (agents investigate)
- Creates issues directly (agents create issues with file:line references)
- Fixes code (agents in worktrees fix code)
- Reviews diffs (agents checkout and build)

The lead ONLY:
- Decides WHAT to investigate (areas, questions, goals)
- Routes agents to work (SendMessage to idle agents, spawn new ones)
- Processes agent results (route findings to next agent)
- Captures learnings (memory files)
- Manages the merge queue priority

Why: Orchestrator context is expensive and strategic. Agent context is cheap
and disposable. Every byte of orchestrator context spent on implementation
details is a byte not spent on routing. Agents always find something the
orchestrator would miss because they READ THE CODE.

## Slash Entry Point Scope

`/swarm` is the main control-plane entrypoint. The core worker procedures
listed below now also ship from `.claude/skills/`:
`/swarm-protocol`, `/coding-standards`, `/swarm-priorities`, `/plan-fix`,
`/parser-fix`, and `/verify-build`. Broader operator procedures currently live
under `.claude/commands/`. Agents invoke both skills and commands the same way
unless frontmatter intentionally changes who can call them or how they run.

The scope split is summarized here. See `reference/team-structure.md` for the concrete coordinator handoffs and data flow.

**Orchestrator slash entrypoints** (you invoke these):
- `/swarm-status` — shows current PRs, issues, metrics, queue
- `/green-merge` — drain merge queue
- `/health-check` — quick codebase health scan
- `/swarm-report` — daily summary for user
- `/rebase-open` — rebase conflicting PRs
- `/corpus-ratchet` — lock in corpus gains

**Worker slash entrypoints** (workers invoke these themselves — do NOT load into orchestrator context):
- `/swarm-protocol` — behavioral rules
- `/coding-standards` — project standards
- `/swarm-priorities` — roadmap alignment
- `/parser-fix` — TDD fix mechanics
- `/verify-build` — deliverable verification
- `/plan-fix` — write implementation plans
- `/scout-report` — create GitHub issues

## Execution Boundaries

Treat each layer as a different boundary:

1. **Worktree = write boundary**: every PR-shaped code change happens in its own worktree.
2. **Worker = context boundary**: spawn a fresh worker when objective, file surface, tool profile, permissions, verification loop, or branch changes materially.
3. **Skill = durable procedure boundary**: stable instructions live in skills and other reusable slash entrypoints, not in repeated inline prose.
4. **Hook = deterministic control boundary**: anything that must always happen belongs in hooks, not in agent memory.

If a coding task crosses into a different crate, file surface, or verification loop, do not stretch the current worker. Write or update the handoff and spawn a fresh worker in a fresh worktree.
Subagents do not inherit parent skills automatically. Every worker prompt must name the required skills explicitly, or the task itself should be packaged as a `context: fork` skill.
Each coordinator and worker should use the local todo or task tool. Every item
should name the skill or command to invoke for that step so the procedure stays
attached to the work, not to ambient memory.

## Phase 1: Bootstrap

### Check state
```
Invoke /swarm-status
```

### Sync repo
```bash
git fetch origin && git checkout master && git pull
```

### Ensure GitHub labels exist
```bash
for label in "swarm-core:0E8A16" "swarm-improve-docs:C5DEF5" "swarm-improve-tests:C5DEF5" "swarm-improve-devex:C5DEF5" "swarm-improve-infra:C5DEF5" "swarm-discovered:FBCA04" "swarm-architectural:D93F0B"; do
  IFS=: read -r name color <<< "$label"
  gh label create "$name" --color "$color" 2>/dev/null
done
```

### Clean up stale worktrees
```bash
for wt in .claude/worktrees/agent-*; do
  git worktree remove --force "$wt" 2>/dev/null || rm -rf "$wt"
done
git worktree prune
```

### Verify master CI is green
```bash
gh run list --branch master --limit 5 --json status,conclusion,headBranch
```

### Check for pending work
- Agent patches: `ls .ops-perl-lsp/agent-patches/*.md 2>/dev/null`
- Discovered issues: `gh issue list --label swarm-discovered --state open`

## Phase 2: Create Team (5 coordinators)

Use `TeamCreate` then spawn 5 teammates. Each teammate's spawn prompt includes:
1. Their role and domain
2. `Invoke /swarm-protocol and /coding-standards.`
3. Domain-specific instructions
4. Task tool reminders
5. Metrics mandate

The coordinator contract lives in `.claude/agents/README.md`. The swarm roster,
including specialist workers, is summarized in
`.claude/agents/AGENT_CATALOG.md`. The persistent coordinator names are `scout`,
`builder`, `reviewer`, `ops`, and `improver`. The catalog records who usually
spawns each tracked worker, where it hands work next, and which slash
entrypoints it should invoke first. The agent files carry the role framing and
todo shape, while the mechanical substep instructions stay in skills so agents
can load them when they become relevant mid-run.

See `templates/teammate-prompt-template.md` for the standard prompt format.
See `reference/team-structure.md` for full team layout and spawn prompts.

### Team Structure (5 coordinators)

| Name | Role | Subagent Strategy |
|------|------|-------------------|
| `scout` | Discovery coordinator | Spawns 5-8 Explore subagents/round |
| `builder` | Build coordinator | Spawns 3-5 worktree subagents/round |
| `reviewer` | Review + PR creation | Spawns 3-5 review subagents/round |
| `ops` | Merge + validate + fix CI | Sequential merges, spawns fix subagents |
| `improver` | Docs + tests + devex | Spawns 2-4 worktree subagents |

### Teammate spawn prompts

**scout**:
```
Invoke /swarm-protocol and /coding-standards.
You are scout. Domain: all discovery — parser error buckets, DAP test gaps, open issues, dead code.
Read .ci/parser-corpus-baseline.json for error buckets.
Read .claude/swarm-state/discovered-issues.md and completed-slices.md for dedup.
Invoke /swarm-priorities to understand what matters.
Spawn 5-8 Explore subagents per round (1 per error bucket for parser work).
For each finding: invoke /plan-fix to write handoff, then /scout-report to create issue.
Agents create their own issues — scout output quality >>> orchestrator guesses.
If a discovery would produce a different crate surface or verification loop, split it into a new task instead of bundling it into an existing slice.
Use TaskCreate for each slice. Message builder when tasks are ready.
```

**builder**:
```
Invoke /swarm-protocol and /coding-standards.
You are builder. Use TaskList to find unclaimed tasks. Use TaskUpdate to claim (set owner).
Read handoff file from .ops-perl-lsp/handoffs/ for context.
Anchor each worktree worker to a concrete issue or scout finding before writing code.
Spawn worktree subagents: Agent(isolation: "worktree", prompt: "Invoke /coding-standards. Then invoke /parser-fix '<desc>'.")
Run 3-5 subagents in parallel. Each subagent does one task.
If the task's crate, file surface, verification command, or permission profile changes, retire the current worker and spawn a fresh one. One worktree worker should produce one PR-shaped unit of change.
Do NOT rebase speculatively — only at merge time via ops.
Run python3 scripts/update-current-status.py if tests were added.
When done: invoke /verify-build, then /pr-create.
SendMessage({to: "reviewer"}) when builds complete.
```

**reviewer**:
```
Invoke /swarm-protocol and /coding-standards.
You are reviewer. Receive build completions from builder.
Spawn review subagents (3-5 parallel). Read handoff, then diff.
Check: coding standards, no unwrap/expect/panic, tests exist, PR description.
Keep reviewer workers one-PR-at-a-time. If feedback requires materially different implementation scope, send it back to builder for a fresh worktree worker instead of reusing the reviewer context for code mutation.
Approve: SendMessage({to: "ops"}) for merge-ready PRs.
Reject: SendMessage({to: "builder"}) with specific feedback.
Also handle PR review comments: gh pr list --state open --json reviews.
```

**ops**:
```
Invoke /swarm-protocol.
You are ops. Merge + validate + fix CI + corpus ratchet.
ONLY merge when CI Gate shows SUCCESS. Never merge red.
Merge in batches of 3 (rapid merges cancel each other's CI).
After merges: invoke /status-drift to fix computed metrics.
After parser merges: invoke /corpus-ratchet to lock in gains.
If CI fails: spawn fix subagent in worktree.
Do not reuse one fixer across unrelated failures. Each failure mode gets a fresh worker with the logs and the exact verification loop for that incident.
When queue is low: SendMessage({to: "scout"}) for more work.
```

**improver**:
```
Invoke /swarm-protocol and /coding-standards.
You are improver. Always running alongside core work (~20% capacity).
Domains: docs, tests, devex, infra.
Check: mutation results, flaky tests, coverage gaps, stale docs.
Spawn 2-4 subagents (isolation: "worktree") for improvements.
Create PRs with --label swarm-improve-docs or swarm-improve-tests.
```

## Phase 3: Recurring Loops

The lead's periodic duties:
- **Every ~10 merges**: Check priority drift; send scout priority steering if needed
- **Queue low**: Message scout to find more work
- **Idle agents**: Repurpose via SendMessage instead of spawning new ones
- **As needed**: `/swarm-status` to check state, `/green-merge` to drain queue
- **Daily**: `/swarm-report` for user check-in
- **Late cycle**: Reserve 10 agent slots for routing — do not fill all capacity

## Phase 4: Continuous Operation

```
DISCOVERY → BUILD → REVIEW → MERGE → IMPROVE
```

```
scout ──────→ TaskCreate ─────→ builder claims via TaskList
builder ────→ SendMessage ────→ reviewer
reviewer ───→ gh pr create ───→ ops (merge queue)
ops ────────→ gh pr merge ────→ ops (verify post-merge)
ops ────────→ SendMessage ────→ scout (queue low)
improver ───→ worktree subs ──→ improvement PRs (always ~20%)
```

## Spawn Rules

- New worktree: separate PR, separate rebase surface, or separate verification loop.
- New worker: different objective, crate, file surface, permissions, or hypothesis.
- New agent for investigation: always — even "quick" tasks. The orchestrator never reads code.
- New skill: instructions are stable enough to reuse across runs.
- New hook: behavior must be guaranteed rather than requested.
- No new worker: sequential branch-local work with the same goal, files, and verification loop.
- Orchestrator creates issues: never. Agents create issues with code analysis and file:line references.
