# Swarm Agent Architecture for Claude Code

A design for running continuous, self-improving, highly-parallel development swarms using Claude Code's agent teams, subagents, worktree isolation, skills, hooks, GitHub-native tracking, and cross-session memory.

## Problem Statement

Repos with hundreds of packages accumulate improvement opportunities faster than a single developer or single agent session can address them: parser bugs, test gaps, dead code, security advisories, documentation drift, performance issues, flaky tests, unused dependencies. These improvements are mostly independent — they can be done in parallel without coordination if file ownership doesn't overlap.

Claude Code's agent teams documentation recommends 3-5 teammates. That's fine for a feature or a review. But for continuous codebase improvement at scale, you need:

1. **Many more workers** — 30-60 parallel streams, not 3-5
2. **Continuous operation** — not batch-then-stop, but an always-running pipeline
3. **Self-governance** — priority alignment, post-merge validation, strategic steering
4. **Self-improvement** — the system learns from its own failures and successes
5. **Context efficiency** — agents shouldn't re-read what previous agents already condensed

## Core Architecture

**Thin coordinator teammates + thick subagent fanout + worktree isolation.**

```
Lead (orchestrator) — coordinates only, never writes code
  │
  ├── scout      — DISCOVERY: find gaps, write handoffs and issues
  │               (spawns 5-8 Explore subagents per round)
  │
  ├── builder    — BUILD: claim tasks, implement in worktrees
  │               (spawns 3-5 worktree subagents per round)
  │
  ├── reviewer   — REVIEW: review diffs, create PRs, address comments
  │               (spawns 3-5 review subagents per round)
  │
  ├── ops        — MERGE + VALIDATE + FIX: merge green PRs, validate,
  │               fix CI failures (sequential merges, spawns fix subs)
  │
  └── improver   — IMPROVE (~20% of capacity, always active)
                  (spawns 2-4 worktree subagents for docs/tests/devex)
```

**5 coordinator teammates. 30-60 parallel workers at peak.**

### Why This Shape

**Thin teammates, thick subagents.** Each teammate carries a full context window. 30 teammates = 30 context windows accumulating noise. Instead, 5 coordinators manage lanes while spawning fresh subagents for each task. Fresh subagents are more context-efficient: focused prompt, no accumulated history, good agent definitions ensure consistent behavior, exit when done.

**Worktrees for all code changes.** Git worktrees give each subagent a physically separate working directory. The Agent tool's `isolation: "worktree"` handles creation and cleanup. No `git stash`/`checkout` conflicts between parallel workers.

**Overlap by files, not agent count.** The constraint isn't "too many agents" — it's "two agents editing the same file." Every scout SLICE includes a `files_touched` field. The orchestrator checks set intersections before assigning work. No overlap = no conflict = unlimited parallelism.

**Per-unit verification.** `cargo test --workspace` takes 3-5 min. `cargo test -p <crate>` takes 10-30 sec. For small, focused PRs, crate-level verification is 10x faster and sufficient. Escalate to workspace verification only for cross-cutting changes.

## Boundary Doctrine

The swarm is optimized around cleaner boundaries, not around keeping more
workers alive.

### Worktree Boundary

The worktree is the write-isolation boundary. Every PR-shaped code change gets
its own worktree by default.

### Worker Boundary

The worker is the context boundary. Spawn a fresh worker whenever any of these
change materially:
- objective or hypothesis
- dominant crate or file surface
- tool or permission profile
- verification command
- branch or PR target

If those change, write or update the handoff and replace the worker. Do not
stretch the same implementation context across multiple PR-shaped changes.

### Knowledge Boundary

Pre-encode only durable knowledge:
- repo rules in `CLAUDE.md`
- reusable procedure in skills
- deterministic enforcement in hooks
- output shape in templates

Keep volatile task state in handoffs, worktrees, PRs, issues, and queue files.

### Team Boundary

Persistent teammates are a control plane, not an implementation pool. Keep the
always-on layer small (`scout`, `builder`, `reviewer`, `ops`, `improver`) and
push most code mutation into disposable specialists.

## Skills Architecture

Skills are structured as directories under `.claude/skills/<name>/`, each containing:

- `SKILL.md` — the skill definition with frontmatter and content
- `templates/` — reusable templates for the skill's outputs
- `examples/` — worked examples (optional)
- `reference/` — supporting reference material (optional)

### Skill Frontmatter

Skills use YAML frontmatter to declare their scope and constraints:

```yaml
---
name: parser-fix
description: TDD parser fix — failing test, minimal fix, verify, PR.
user-invocable: true
context: fork          # Runs in isolated context (doesn't pollute caller)
agent: general-purpose # What kind of agent can run this
allowed-tools: Read, Edit, Write, Grep, Glob, Bash(cargo *), Bash(git *)
---
```

Key frontmatter fields:
- `context: fork` — skill runs in its own isolated context
- `allowed-tools` — restricts what tools the skill can use
- `user-invocable: false` — hidden from user's skill list (internal use only)
- `disable-model-invocation: true` — pure instruction, no model call

### Slash Entry Point Scoping

Slash entrypoints have scopes. Loading a worker procedure into orchestrator
context wastes context and causes the orchestrator to do worker work directly.
`/swarm` is the main control-plane entrypoint. The core worker entrypoints
below also ship from `.claude/skills/` today:
`/swarm-protocol`, `/coding-standards`, `/swarm-priorities`, `/plan-fix`,
`/parser-fix`, and `/verify-build`. Other slash entrypoints currently live
under `.claude/commands/`. Agents invoke both the same way unless frontmatter
intentionally changes who can call them or how they run.

**Orchestrator slash entrypoints** (lead invokes these):
- `/swarm-status` — shows current PRs, issues, metrics, queue
- `/green-merge` — drain merge queue
- `/health-check` — quick codebase scan
- `/swarm-report` — daily summary for user
- `/rebase-open` — rebase conflicting PRs
- `/corpus-ratchet` — lock in corpus gains

**Worker slash entrypoints** (workers invoke these — do NOT load into orchestrator context):
- `/swarm-protocol` — behavioral rules
- `/coding-standards` — project standards
- `/swarm-priorities` — roadmap alignment
- `/parser-fix` — TDD fix mechanics
- `/verify-build` — deliverable verification
- `/plan-fix` — write implementation plans
- `/scout-report` — create GitHub issues

**Dual-use slash entrypoints** (either context):
- `/scout`, `/queue-scout`, `/audit`

## Hooks Architecture

Hooks enforce rules that prompts can't. Agents forget instructions; hooks don't.

### Hook Types

Hooks are registered in `.claude/settings.json` under `hooks.<EventType>`:

```json
{
  "hooks": {
    "PostToolUse": [...],
    "TeammateIdle": [...],
    "TaskCompleted": [...],
    "SubagentStart": [...],
    "SubagentStop": [...],
    "PreToolUse": [...],
    "SessionStart": [...]
  }
}
```

Each hook entry has a `type` and `command`. Hooks receive JSON context via stdin:

```bash
INPUT=$(cat)   # Read JSON from stdin
CMD=$(echo "$INPUT" | jq -r '.tool_input.command // empty')
```

### Active Hooks

| Event | Matcher | What It Does |
|-------|---------|-------------|
| `PostToolUse` | `Edit\|Write\|NotebookEdit` | Auto-runs `cargo fmt` + `cargo check` on edited `.rs` files |
| `TeammateIdle` | — | Tracks idle transitions, checks for unclaimed work |
| `TaskCompleted` | — | Verifies deliverables exist (branch, PR, fmt clean) before marking done |
| `SubagentStart` | `builder\|reviewer\|fixer\|validator\|bootstrapper\|pr-responder\|ops\|improver` | Auto-injects coding standards reminder |
| `SubagentStop` | `builder\|reviewer\|fixer\|validator\|bootstrapper\|pr-responder\|ops\|improver` | Records worker teardown and handoff boundaries in metrics |
| `PreToolUse` | `Bash` | Reads command from stdin JSON; blocks dangerous commands |
| `SessionStart` | `compact` | Injects context refresh after conversation compaction |

`WorktreeCreate` and `WorktreeRemove` are intentionally **not** registered in
the shared settings by default. In Claude Code those hooks replace the default
git worktree behavior, so they should only be used when the hook itself is
responsible for creating or removing the working copy.

### Hook Design Principles

1. **Gates over suggestions** — exit code 2 BLOCKS the action; prompts get ignored
2. **Fast execution** — hooks run on every matching event; keep under 5 seconds
3. **Informative errors** — when a hook blocks, tell the agent exactly what to fix
4. **Idempotent** — hooks may run multiple times; don't double-count metrics

## Context Efficiency

The biggest performance issue in multi-agent systems is context waste: Agent B re-reads the same 10 files that Agent A already read. We solve this with **handoff files** and **skills-over-file-reads**.

### Context Layering Model

Context flows from general to specific across 6 layers:

1. **CLAUDE.md** — project-wide rules, crate map, commands
2. **Hooks** — enforcement (PostToolUse, TaskCompleted, TeammateIdle)
3. **Agent definition** — role, domain, orchestration loop
4. **Skills** — mechanics for each step (parser-fix, verify-build, etc.)
5. **Handoffs** — per-task context (root cause, fix code, test templates)
6. **Source code** — the actual codebase

Each layer is more specific than the last. Don't duplicate information across layers.

### What To Pre-Encode

Pre-encode:
- coding standards
- review checklists
- task templates
- queue conventions
- merge and completion rules

Do not pre-encode:
- ephemeral task detail
- branch-local findings
- per-PR reviewer notes
- a worker's temporary reasoning state

Subagents do not inherit the caller's loaded skills automatically. If a worker
needs repo rules or procedure, name the required skills explicitly in the spawn
prompt or package the task itself as a `context: fork` skill.

Workers should also use the local todo or task tool for the active slice. Each
item should name the skill or command for that step so the procedure stays
attached to the work item instead of floating in coordinator memory.

### Handoff Protocol

```
Scout reads 10 source files
  │ writes handoff with code excerpts + test template
  ▼
Builder reads 1 handoff file (not 10 source files)
  │ appends reviewer briefing
  ▼
Reviewer reads briefing + focused diff (not cold diff)
  │ creates PR
  ▼
Improvers read "Lesson Learned" sections → ADRs, friction log
```

Each handoff file (`.ops-perl-lsp/handoffs/<branch>.md`) contains:
- **Problem** and context (from scout's investigation)
- **Code excerpts** (so builder doesn't re-read source files)
- **Test template** (pre-filled skeleton)
- **Fix strategy** (specific steps)
- **Known pitfalls** (relevant entries from failure knowledge base)
- **Builder briefing** (appended after build: what changed, key decisions, what to watch for)

### Slash Entry Points Over File Reads

Protocol, standards, and priorities are reusable slash entrypoints
(`/swarm-protocol`, `/coding-standards`, `/swarm-priorities`), not
ad-hoc file reads. Agents invoke them directly into their context instead of
spending a `Read` tool call. In the live repo today, `/swarm` and the core
worker procedures named here ship from `.claude/skills/`, while broader
operator flows currently live under `.claude/commands/`. Subagent prompts are
7 lines pointing to handoff + slash entrypoints, not 100 lines of inline
instructions.

### Minimal Subagent Prompts

Builder coordinators compose prompts like:
```
"Invoke /coding-standards. Then invoke /parser-fix '<description from handoff>'.
 Crate: <crate>. Files: <file list from handoff>.
 Verify: cargo fmt && cargo clippy -p <crate> --tests && cargo test -p <crate>.
 Commit and push."
```

7 lines. The handoff file has all the context. The slash entrypoints load the
rules. No context wasted.

### Context Shift = New Worker

When the work stops being "same branch, same files, same verification loop,"
spawn again. This is the default:
- different crate → new worker
- different PR target → new worker
- different verification gate → new worker
- different tool or permission profile → new worker

Worker reuse is the exception. Handoffs are the continuity mechanism.

## Self-Governance

### Priority Weighting

The `/swarm-priorities` slash entrypoint loads the roadmap (NOW/NEXT/LATER),
open milestones, and high-priority issues. It defines P0-P4 tiers:

| Tier | What |
|------|------|
| P0 | Blocking: security vulnerabilities, broken CI, regressions |
| P1 | Roadmap: current NOW items, corpus improvement, feature completion |
| P2 | Test infrastructure: mutant survivors, flaky tests, coverage gaps |
| P3 | Codebase health: DAP tests, debt, dead code, unused deps |
| P4 | Polish: test naming, error messages, observability |

Scouts tag every SLICE with a priority tier. Builders claim higher-priority tasks first. The lead watches the distribution and can spin up a strategist-style analysis subagent when the swarm drifts toward easy P3/P4 work.

### Post-Merge Validation

The ops teammate runs validation after every merge:
- Parser fix → corpus sweep (did clean count increase?)
- Test addition → mutation re-test (is the target mutant killed?)
- LSP change → integration tests (all pass?)
- Any merge → clippy (no new warnings?)

Regressions trigger immediate `gh issue create --label priority:high`.

### Strategic Analysis

Every ~10 merges, the lead triggers a data-driven check:
- Priority distribution (are we doing P1 work or drifting to P4?)
- Roadmap progress (NOW items completed?)
- Agent effectiveness (who succeeds vs. fails?)
- Stale work (in-progress slices >24h)
- Recommendations (adjust scout focus, fix agent definitions)

## Self-Improvement

### Five Persistence Layers

| Layer | Lifetime | What |
|-------|----------|------|
| **Handoffs** | Until merge | Context transfer: scout→builder→reviewer |
| **Tracked swarm state** | Cross-session, committed | findings, known-pitfalls, completed-slices, discovered-issues, queue |
| **Ops runtime** | Current session | metrics, agent-patches |
| **GitHub** | Permanent | Issues (swarm-discovered, swarm-architectural), PRs (labeled), CI status |
| **Claude Code memories** | Cross-session | Critical lessons, session progress, architectural decisions |

### Learning Loops

1. **Fixers** → `known-pitfalls.md` → scouts/builders avoid repeating known traps
2. **All agents** → `discovered-issues.md` → scouts pick up pre-investigated leads
3. **Coordinators + improver** → `findings.json` → durable control-plane conclusions survive beyond the session
4. **All agents** → GitHub issues (`--label swarm-discovered`) → persistent, searchable backlog
5. **All agents** → `swarm-metrics.jsonl` → ops + improver spot performance patterns
6. **Failing agents** → `agent-patches/` → bootstrapper improves agent definitions
7. **Improver** → reads handoff lessons → crystallizes into ADRs and friction logs
8. **Ops** → analyzes metrics → reports which domains/agents need attention
9. **Lead** → Claude Code memories → carries critical knowledge to future sessions

### Agent Self-Improvement

When an agent hits friction caused by its own definition being wrong or incomplete, it writes a patch proposal to `.ops-perl-lsp/agent-patches/<agent>.md`. The bootstrapper integrates validated patches during `--refresh`. The user reviews and merges. This means the agent definitions evolve based on actual field experience.

## GitHub-Native Tracking

### Labels
- **PR labels**: `swarm-core`, `swarm-improve-docs`, `swarm-improve-tests`, `swarm-improve-devex`, `swarm-improve-infra`
- **Issue labels**: `swarm-discovered` (found by agent during other work), `swarm-architectural` (needs user decision)

### Templates
- **Issue template**: `swarm_discovered.yml` — structured fields for agent, context, files, category
- **PR template**: summary, changes, verification, agent attribution

### Auto-Merge
Small PRs (improvements, side fixes) use `gh pr merge --auto --squash --delete-branch` to merge when checks pass without waiting for the main ops lane.

### State Queries
```bash
gh pr list --state open --label "swarm-core"
gh issue list --label "swarm-discovered" --state open
gh run list --status failure --limit 10
```

## Background Improvement (~20%)

The swarm always dedicates ~20% of its branches to making the codebase better, not just fixing the primary task. This runs via the always-on improver coordinator:

**improver**: README, CHANGELOG, ADRs, friction log, roadmap updates, CLAUDE.md, command reference, mutation survivors, flaky test fixes, coverage gaps, test naming/BDD, integration tests, unused dep removal, dead code, security audits. Reads handoff "Key Decisions" and "Lesson Learned" sections to find ADR candidates and test gaps.

This ensures the codebase gets healthier with every swarm cycle, not just bigger.

## Discovery Protocol

Every agent is a passive scout. When any agent (builder, reviewer, fixer, improver) notices something wrong outside their current scope:

| Severity | Action |
|----------|--------|
| Trivial | Fix in the same PR (formatting, typo in your file) |
| Small-medium | `gh issue create --label swarm-discovered` with enough context for a fresh agent |
| Large/architectural | `gh issue create --label swarm-architectural` — user weighs in |

The key: include enough context in the issue that the NEXT agent doesn't have to re-investigate. Paste code excerpts, error messages, file paths.

## Lifecycle

### Startup (`/swarm all`)
1. Load protocol, priorities, and current state (`/swarm-status`)
2. Sync repo, ensure GitHub labels exist
3. Check for pending work from previous sessions (in-progress slices, open PRs, stale worktrees)
4. Create 5-coordinator team
5. Lead monitors: messages scout when queue is low, asks ops to validate recent merges, and triggers strategist-style analysis when priority drift appears

### Continuous Operation
All lanes run concurrently. Scouts feed builders feed reviewers feed ops. Improver runs alongside. No batching.

### Graceful Shutdown (`/swarm-wind-down`)
~20 minutes: stop scouts → let builders finish → review and PR everything → merge green → clean up → write memories

### Emergency Stop (`/swarm-stop`)
~5 minutes: broadcast STOP → snapshot state → enable auto-merge on green PRs → write memory → halt team → leave worktrees for next session

### Session Resumption
Next `/swarm` picks up: in-progress slices from `completed-slices.md`, open PRs (some auto-merged), active worktrees, pending agent patches, discovered issues, and the tracked findings ledger.

## Portable Pack

The `docs/handoff/swarm-pack/` directory is a derived export of the live swarm
control plane for adoption in another repo:

```bash
bash swarm-pack/setup.sh    # Install agents, slash commands, hooks, ops, GH labels
/bootstrap-agents            # Discover codebase → generate ~25-30 domain agents
/swarm all                   # Start continuous swarm
```

The pack installs reusable specialist agent definitions plus slash commands and
hooks. In the current source tree, the reusable specialist roster now lives
under `.claude/agents/` and is summarized in
`.claude/agents/AGENT_CATALOG.md`; `/bootstrap-agents` refreshes and extends
that roster when the codebase changes. The live swarm still runs as 5
named coordinators; optional specialists are spawned on demand.
The catalog records who usually spawns each tracked specialist, where
it hands work next, and which slash entrypoints it should invoke first, so the
agent list and the flow mapping stay coupled.
Compatibility donor agents now live under `.claude/agents-compat/` rather than
inside the archived roster directory.

### Agent Taxonomy (~50 total after bootstrap)

| Category | Pack (portable) | Repo-specific (generated) |
|----------|----------------|--------------------------|
| Core swarm | 5 (scout, builder, reviewer, ops, improver) | — |
| Optional specialist subagents | 6 (bootstrapper, pr-responder, janitor, validator, strategist, fixer) | — |
| Quality | 2 (mutant-killer, coverage-filler) | 3-5 (fuzz, flaky, test-quality) |
| Review | 3 (standards, security, scope) | 2-3 (performance, api) |
| Research | 3 (web, docs, verify) | 1-2 (deps, PRs) |
| Documentation | 2 (adr-writer, friction-logger) | 2 (changelog, api-docs) |
| Infrastructure | 2 (dep-cleaner, dead-code) | 2-3 (ci-gate, security-audit, baseline-ratchet) |
| Domain scouts | — | 3-6 (parser, LSP, DAP, etc.) |
| Domain builders | — | 5-10 (parser-fix, lsp-provider, dap-test, etc.) |
| Bootstrapper | 1 | — |

## Design Principles

### Execution
1. **Coordinators don't code.** Teammates manage lanes. Subagents do work.
2. **Fresh beats stale.** New subagent > reused context. Agent definitions are reusable.
3. **Parallel beats sequential.** All independent subagents in one message.
4. **Worktrees for all code changes.** `isolation: "worktree"` prevents conflicts.
5. **Overlap by files, not count.** Unlimited agents if files don't overlap.

### Efficiency
6. **Skills over file reads.** `/swarm-protocol` not `Read .claude/skills/swarm/...`.
7. **Handoffs carry context.** Next agent reads previous agent's summary, not raw sources.
8. **Minimal subagent prompts.** 7 lines pointing to files/skills, not 100 lines inline.
9. **Per-unit verification.** Test the package you changed, not the workspace.

### Quality
10. **Validate merges.** Ops verifies that work actually helped.
11. **Every agent is a scout.** Discoveries become GitHub issues for fresh agents.
12. **~20% goes to improvement.** Docs, tests, devex, infra — always on.
13. **Review comments get addressed.** PR responder monitors and fixes feedback.

### Governance
14. **Priority-weighted discovery.** Scouts check roadmap; the lead or a strategist-style subagent steers when the swarm drifts.
15. **Self-improving.** Metrics, agent patches, friction logs, ADRs.
16. **4 persistence layers.** Handoffs → ops files → GitHub → memories.
17. **GitHub-native.** Labels, issues, templates, auto-merge, `gh` CLI everywhere.

### Lifecycle
18. **Continuous, not batchy.** All lanes concurrent.
19. **Graceful shutdown.** `/swarm-wind-down` finishes work; `/swarm-stop` saves state.
20. **Session resumption.** Next `/swarm` picks up where the last one stopped.
