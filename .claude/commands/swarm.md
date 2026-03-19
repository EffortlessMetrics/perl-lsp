---
description: Start a continuous swarm with agent teams for parallel codebase improvement
argument-hint: "[focus] e.g. 'all', 'parser', 'dap', 'tests', 'cleanup', 'improve'"
disable-model-invocation: true
---

# Swarm: Continuous Agent Team

Start a continuous swarm. Focus: **$ARGUMENTS**

You are the lead. You coordinate only. You NEVER write production code.
Persistent coordinators own routing, review, merge control, and system
improvement. Disposable workers in isolated worktrees do all code mutation.

## Dispatch Principles

1. **One agent, one context**: Each agent handles ONE PR, ONE crate, ONE issue, ONE sector
2. **Scouts**: One sector per scout. Fresh agent for different context group.
3. **Builders**: One crate per builder. Worktree isolated.
4. **Reviewers**: One PR per reviewer. Fresh context for clean review.
5. **Ops**: One PR at a time for merges. Verify CI between each.
6. **Draft first**: Builders create draft PRs. Reviewer marks ready after checking.
7. **Slash entrypoints over inline**: Agents invoke reusable slash procedures
   (/verify-build, /pr-create, /plan-fix, /scout-report) instead of repeating
   long inline workflows.

## Slash Entry Point Scope

`/swarm` is the main control-plane entrypoint. The core worker procedures
listed below now also ship from `.claude/skills/`:
`/swarm-protocol`, `/coding-standards`, `/swarm-priorities`, `/plan-fix`,
`/parser-fix`, and `/verify-build`. Broader operator procedures currently live
under `.claude/commands/`. Agents invoke both skills and commands the same way
unless frontmatter intentionally changes who can call them or how they run.

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
- `/parser-fix` — task-specific implementation flow
- `/verify-build` — branch/test/PR verification
- `/plan-fix` — scout handoff generation
- `/scout-report` — GitHub issue creation for discovered work
- `/pr-create` — draft PR creation
- `/pr-ready` — mark reviewed PRs ready

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
Invoke /swarm-status       — shows current PRs, issues, metrics, queue
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

### Clean up stale worktrees (REQUIRED before team creation)

Stale agent worktrees from previous sessions pollute IDE diagnostics and cause false alarms. Remove them first:

```bash
git worktree list
git worktree prune
ls .claude/worktrees/agent-* 2>/dev/null | head -20
for wt in .claude/worktrees/agent-*; do
  git worktree remove --force "$wt" 2>/dev/null || rm -rf "$wt"
done
git worktree prune
```

### Verify master CI is green before starting

**Do not start new work if master CI is red.** Fix it first.

```bash
gh run list --branch master --limit 5 --json status,conclusion,headBranch
# If any run shows conclusion != "success": fix master CI before proceeding.
```

### Check CI queue depth before launching builders

```bash
gh run list --json status --jq '[.[] | select(.status == "in_progress")] | length'
```

**If > 5 runs in progress**: wait before launching builders. CI runners are finite. Adding more work while the queue is saturated delays all PRs and obscures failures.

### Check for pending work from previous sessions
- Agent patches: `ls .ops-perl-lsp/agent-patches/*.md 2>/dev/null`
- In-progress slices: `grep "in-progress" .claude/swarm-state/completed-slices.md 2>/dev/null`
- Discovered issues: `gh issue list --label swarm-discovered --state open`

### Resume or start fresh
If there's pending work, prioritize it. Otherwise start fresh scouting.

## Phase 2: Create Team (5 coordinators)

Create an agent team with these 5 teammates. Use `TeamCreate` with specific names so teammates can message each other directly via `SendMessage({to: "name"})`.

Each teammate fans out to subagents for actual parallelism. Net capacity is 20-40 parallel workers with only 5 coordination slots.

The coordinator contract lives in `.claude/agents/README.md`. The archived
roster, including specialist workers, lives under
`.claude/agents/archive/agent-roster.json` and is summarized in
`.claude/agents/AGENT_CATALOG.md`. The persistent coordinator names are `scout`,
`builder`, `reviewer`, `ops`, and `improver`. The catalog records who usually
spawns each tracked worker, where it hands work next, and which slash
entrypoints it should invoke first.

### Team structure

| Name | Role | Model | Subagent Strategy |
|------|------|-------|-------------------|
| `scout` | Discovery coordinator | sonnet | 5-8 Explore subagents/round |
| `builder` | Build coordinator | sonnet | 3-5 worktree subagents/round |
| `reviewer` | Review + PR creation | sonnet | 3-5 review subagents/round |
| `ops` | Merge + validate + fix CI | sonnet | Sequential merges, fix subagents as needed |
| `improver` | Docs + tests + devex | sonnet | 2-4 worktree subagents |

### Teammate spawn prompts

Each teammate gets a focused prompt that tells them to:
1. Invoke `/swarm-protocol` (loads behavioral rules)
2. Invoke `/coding-standards` (loads project standards)
3. Invoke `/swarm-priorities` (loads roadmap alignment)
4. Their specific domain instructions (below)

**scout**:
```
Invoke /swarm-protocol and /coding-standards.
You are scout. Domain: all discovery — parser error buckets, DAP test gaps, open issues, dead code.
Read .ci/parser-corpus-baseline.json for error buckets.
Read .claude/swarm-state/discovered-issues.md and completed-slices.md for dedup.
Invoke /swarm-priorities to understand what matters.
Spawn 5-8 Explore subagents per round (1 per error bucket for parser work).
For each finding: invoke /plan-fix to write handoff, then /scout-report to create issue.
If a discovery would produce a different crate surface or verification loop, split it into a new task instead of bundling it into an existing slice.
Use TaskCreate for each slice. Use TaskList to check what already exists.
SendMessage({to: "builder"}) when tasks are ready.
After each round, append to .ops-perl-lsp/swarm-metrics.jsonl.
```

**builder**:
```
Invoke /swarm-protocol and /coding-standards.
You are builder. Claim tasks, spawn build subagents with isolation: "worktree".

Before spawning any subagent, confirm it has ALL of the following:
  - Named worktree (e.g., agent-fix-parser-heredoc)
  - Branch name
  - Claimed file surface (exact list of files to touch — no open-ended scope)
  - Verification command (cargo fmt && cargo clippy -p <Y> --tests && cargo test -p <Y>)
  - PR size confirmation: if the change touches >10 files, split into multiple subagents with non-overlapping file surfaces

Track all spawned subagent IDs. Before shutting down, list them in your shutdown message so the lead knows what is still running.

Check open PR count before creating new PRs. If more than 5 are open, message the lead for guidance instead of adding to the queue:
  gh pr list --state open --json number --jq length

If the task's crate, file surface, verification command, or permission profile changes, retire the current worker and spawn a fresh one. One worktree worker should produce one PR-shaped unit of change.

Subagent prompt pattern (required fields):
  "Worktree: <worktree-name>. Branch: <X>. Crate: <Y>.
   Files: <exact list — max 10>.
   Goal: <one sentence>.
   Verify: cargo fmt && cargo clippy -p <Y> --tests && cargo test -p <Y>.
   Run ALL commands from your worktree path. Do NOT cd to the main repo.
   Read .ops-perl-lsp/handoffs/<branch>.md for context.
   Read .claude/swarm-state/known-pitfalls.md for traps.
   Invoke /swarm-protocol, /coding-standards, and the task-specific skill (usually /parser-fix).
   Append reviewer briefing to handoff. Write metrics. gh issue create --label swarm-discovered for out-of-scope finds."

Use TaskList to find unclaimed tasks. Use TaskUpdate to claim (owner: "builder") and complete.
Run 3-5 subagents in parallel. Each subagent does one task.
SendMessage({to: "reviewer"}) when builds complete.
After each build, append to .ops-perl-lsp/swarm-metrics.jsonl.
```

**reviewer**:
```
Invoke /swarm-protocol and /coding-standards.
You are reviewer. Receive build completions from builder.
Spawn review subagents (3-5 parallel). Read handoff briefings first, then focused diff.
Check: coding standards, no unwrap/expect/panic, tests exist, PR description.
Invoke /pr-create to open draft PRs with the right labels.
Use /pr-ready only after feedback is addressed and checks are green.
Keep reviewer workers one-PR-at-a-time. If feedback requires materially different implementation scope, send it back to builder for a fresh worktree worker instead of reusing the reviewer context for code mutation.
Use TaskUpdate to mark review tasks completed.
Approve: SendMessage({to: "ops"}) for merge-ready PRs.
Reject: SendMessage({to: "builder"}) with specific feedback.
Also handle PR review comments: gh pr list --state open --json reviews.
After each review, append to .ops-perl-lsp/swarm-metrics.jsonl.
```

**ops**:
```
Invoke /swarm-protocol.
You are ops. Merge + validate + fix CI + corpus ratchet.
ONLY merge PRs where CI Gate shows SUCCESS. Never merge red CI.

Before every merge, run:
  gh pr checks <N>          — must show all checks passing
  gh run list --limit 5     — confirm master CI is also green

If CI Gate is not SUCCESS: do NOT merge. Spawn fix subagent (isolation: "worktree").
If master CI is red: stop all merges. Fix master CI first.
Do not reuse one fixer across unrelated failures. Each failure mode gets a fresh worker with the logs and the exact verification loop for that incident.

Merge: gh pr merge <N> --squash --delete-branch (only when CI Gate is SUCCESS)
Use TaskUpdate to track merge progress.
Merge in batches of 3 (rapid merges cancel each other's CI runs).
After ~5 merges: invoke /status-drift to fix computed metrics.
After parser merges: invoke /corpus-ratchet to lock in gains.
Every ~10 merges: analyze .ops-perl-lsp/swarm-metrics.jsonl and report trends.
When queue is low: SendMessage({to: "scout"}) for more work.
After each batch, append to .ops-perl-lsp/swarm-metrics.jsonl.
```

**improver**:
```
Invoke /swarm-protocol and /coding-standards.
You are improver. Always running alongside core work (~20% capacity).
Domains: docs, tests, devex, infra.
Read .ops-perl-lsp/handoffs/*.md for "Key Decisions" and "Lesson Learned" — ADR and friction-log candidates.
Check: mutation results, flaky tests (.ci/debt-ledger.yaml), coverage gaps, stale docs.
Use TaskList to find improvement tasks. Use TaskCreate for gaps you discover.
Spawn 2-4 subagents (isolation: "worktree") for: ADRs, changelog, mutants, flaky fixes, coverage.
Create PRs with --label swarm-improve-docs, swarm-improve-tests, or swarm-improve-infra.
Write Claude Code memories when architectural decisions crystallize.
After each PR, append to .ops-perl-lsp/swarm-metrics.jsonl.
```

## Phase 3: Recurring Loops

Set up recurring checks:

```
/loop 10m /swarm-status    — state check every 10 min (auto)
/loop 30m /green-merge     — drain merge queue every 30 min (auto)
```

The lead's periodic duties:
- **Every ~10 merges**: Check priority drift; send scout priority steering if needed
- **Queue low**: Message scout to find more work
- **Daily**: `/swarm-report` for user check-in
- **As needed**: Review `.ops-perl-lsp/agent-patches/`, apply improvements
- **As needed**: Write Claude Code memories for cross-session knowledge

## Phase 4: Continuous Operation

```
DISCOVERY
  scout                  → Explore subagents → TaskCreate → builder claims

BUILD
  builder                → TaskList → claim → worktree subagents → SendMessage reviewer

REVIEW
  reviewer               → review diffs → /pr-create → SendMessage ops

MERGE
  ops                    → gh pr merge (CI green only) → verify → /corpus-ratchet if improved
  ops                    → spawn fix subagents for CI failures

IMPROVE (~20%)
  improver               → ADRs, changelog, mutants, flaky tests, coverage, deps
```

### Data flows

```
scout ──────→ TaskCreate ─────→ builder claims via TaskList
builder ────→ SendMessage ────→ reviewer
reviewer ───→ /pr-create ────→ ops (merge queue)
ops ────────→ gh pr merge ────→ ops (verify post-merge)
ops ────────→ SendMessage ────→ scout (queue low)
ops ────────→ /corpus-ratchet → lock in gains
improver ───→ handoffs/ ──────→ ADRs, friction log, docs
all agents ─→ gh issue create → scout (swarm-discovered)
all agents ─→ swarm-metrics  → ops (analysis)
all agents ─→ TaskUpdate ────→ shared task list
```

### Spawn Rules

- New worktree: separate PR, separate rebase surface, or separate verification loop.
- New worker: different objective, crate, file surface, permissions, or hypothesis.
- New skill: instructions are stable enough to reuse across runs.
- New hook: behavior must be guaranteed rather than requested.
- No new worker: sequential branch-local work with the same goal, files, and verification loop.

### Auto-merge
```bash
gh pr merge <N> --auto --squash --delete-branch
```
Use for improvement PRs and small core PRs. Ops handles the rest.

### CI monitoring
```bash
gh run list --limit 5 --json status,conclusion,headBranch
gh pr checks <N>
```

### Issue-driven work
Scout checks `gh issue list --label swarm-discovered` — these are pre-investigated leads with full context from other agents.

## Focus Area Variants

### `all` (default)
Scout at full capacity. Builder with 3-5 subagents. Improver active.

### `parser`
Scout focuses on parser error buckets. Builder at full capacity. Improver still active.

### `dap`
Scout focuses on DAP test gaps. Builder at 1-2 subagents. Improver still active.

### `tests`
Scout focuses on test gaps. Builder at full capacity. Improver gets extra capacity.

### `cleanup`
Scout focuses on dead code and cleanup. Builder at 1-2 subagents. Improver active.

### `improve`
No scout or builder discovery. Improver at full capacity for docs/tests/devex/infra.
