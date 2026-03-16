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

## Skill Scope

The scope split is summarized here. See `reference/team-structure.md` for the concrete coordinator handoffs and data flow.

**Orchestrator skills** (you invoke these):
- `/swarm-status` — shows current PRs, issues, metrics, queue
- `/green-merge` — drain merge queue
- `/health-check` — quick codebase health scan
- `/swarm-report` — daily summary for user
- `/rebase-open` — rebase conflicting PRs
- `/corpus-ratchet` — lock in corpus gains

**Agent skills** (agents invoke these themselves — do NOT load into orchestrator context):
- `/swarm-protocol` — behavioral rules
- `/coding-standards` — project standards
- `/swarm-priorities` — roadmap alignment
- `/parser-fix` — TDD fix mechanics
- `/verify-build` — deliverable verification
- `/plan-fix` — write implementation plans
- `/scout-report` — create GitHub issues

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
Use TaskCreate for each slice. Message builder when tasks are ready.
```

**builder**:
```
Invoke /swarm-protocol and /coding-standards.
You are builder. Use TaskList to find unclaimed tasks. Use TaskUpdate to claim (set owner).
Read handoff file from .ops-perl-lsp/handoffs/ for context.
Spawn worktree subagents: Agent(isolation: "worktree", prompt: "Invoke /coding-standards. Then invoke /parser-fix '<desc>'.")
Run 3-5 subagents in parallel. Each subagent does one task.
When done: invoke /verify-build, then /pr-create.
SendMessage({to: "reviewer"}) when builds complete.
```

**reviewer**:
```
Invoke /swarm-protocol and /coding-standards.
You are reviewer. Receive build completions from builder.
Spawn review subagents (3-5 parallel). Read handoff, then diff.
Check: coding standards, no unwrap/expect/panic, tests exist, PR description.
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
- **As needed**: `/swarm-status` to check state, `/green-merge` to drain queue
- **Daily**: `/swarm-report` for user check-in

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
