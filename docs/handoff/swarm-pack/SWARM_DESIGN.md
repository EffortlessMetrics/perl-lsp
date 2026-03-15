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
  ├── DISCOVERY
  │   ├── scout-1, scout-2              — find gaps, write handoff files
  │   └── (each spawns 5-8 Explore subagents in parallel)
  │
  ├── BUILD
  │   ├── builder-1, builder-2          — claim tasks, build in worktrees
  │   └── (each spawns 3-5 build subagents with isolation: "worktree")
  │
  ├── REVIEW
  │   ├── reviewer                      — review diffs, create PRs with labels, auto-merge
  │   └── pr-responder                  — address review comments, push fixes
  │
  ├── MERGE
  │   ├── merger                        — merge PRs, handle drift, signal validator
  │   └── validator                     — verify merges actually helped, catch regressions
  │
  ├── IMPROVE (~20% of capacity, always active)
  │   ├── improver-docs                 — ADRs, changelog, friction log, README, roadmap
  │   └── improver-tests                — mutants, flaky tests, coverage, deps, dead code
  │
  └── GOVERNANCE
      ├── strategist                    — priority alignment, roadmap tracking, agent health
      └── fixer                         — CI failures, regressions, known-pitfalls
```

**12 named teammates. 10 operational layers. 30-60 parallel workers at peak.**

### Why This Shape

**Thin teammates, thick subagents.** Each teammate carries a full context window. 30 teammates = 30 context windows accumulating noise. Instead, 12 coordinators manage lanes while spawning fresh subagents for each task. Fresh subagents are more context-efficient: focused prompt, no accumulated history, good agent definitions ensure consistent behavior, exit when done.

**Worktrees for all code changes.** Git worktrees give each subagent a physically separate working directory. The Agent tool's `isolation: "worktree"` handles creation and cleanup. No `git stash`/`checkout` conflicts between parallel workers.

**Overlap by files, not agent count.** The constraint isn't "too many agents" — it's "two agents editing the same file." Every scout SLICE includes a `files_touched` field. The orchestrator checks set intersections before assigning work. No overlap = no conflict = unlimited parallelism.

**Per-unit verification.** `cargo test --workspace` takes 3-5 min. `cargo test -p <crate>` takes 10-30 sec. For small, focused PRs, crate-level verification is 10x faster and sufficient. Escalate to workspace verification only for cross-cutting changes.

## Context Efficiency

The biggest performance issue in multi-agent systems is context waste: Agent B re-reads the same 10 files that Agent A already read. We solve this with **handoff files** and **skills-over-file-reads**.

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

Each handoff file (`.ops/handoffs/<branch>.md`) contains:
- **Problem** and context (from scout's investigation)
- **Code excerpts** (so builder doesn't re-read source files)
- **Test template** (pre-filled skeleton)
- **Fix strategy** (specific steps)
- **Known pitfalls** (relevant entries from failure knowledge base)
- **Builder briefing** (appended after build: what changed, key decisions, what to watch for)

**The efficiency hierarchy:**
1. Best: multiple context-chunked agents with effective handoffs
2. Good: one agent with skills and cache efficiency
3. Worst: multiple agents re-reading the same full context

### Skills Over File Reads

Protocol, standards, and priorities are **skills** (`/swarm-protocol`, `/coding-standards`, `/swarm-priorities`), not files. Agents invoke them directly into their context instead of spending a `Read` tool call. Subagent prompts are 7 lines pointing to handoff + skills, not 100 lines of inline instructions.

### Minimal Subagent Prompts

Builder coordinators compose prompts like:
```
"Read .ops/handoffs/<branch>.md for context.
 Read .claude/swarm-state/known-pitfalls.md for traps.
 Invoke /swarm-protocol and /coding-standards.
 Branch: X. Crate: Y. Verify: fmt && clippy && test.
 Append reviewer briefing to handoff. Write metrics.
 gh issue create --label swarm-discovered for out-of-scope finds."
```

7 lines. The handoff file has all the context. The skill invocations load the rules. No context wasted.

## Self-Governance

### Priority Weighting

The `/swarm-priorities` skill loads the roadmap (NOW/NEXT/LATER), open milestones, and high-priority issues. It defines P0-P4 tiers:

| Tier | What |
|------|------|
| P0 | Blocking: security vulnerabilities, broken CI, regressions |
| P1 | Roadmap: current NOW items, corpus improvement, feature completion |
| P2 | Test infrastructure: mutant survivors, flaky tests, coverage gaps |
| P3 | Codebase health: DAP tests, debt, dead code, unused deps |
| P4 | Polish: test naming, error messages, observability |

Scouts tag every SLICE with a priority tier. Builders claim higher-priority tasks first. The strategist monitors distribution and steers scouts when the swarm drifts toward easy P3/P4 work.

### Post-Merge Validation

The validator teammate runs after every merge:
- Parser fix → corpus sweep (did clean count increase?)
- Test addition → mutation re-test (is the target mutant killed?)
- LSP change → integration tests (all pass?)
- Any merge → clippy (no new warnings?)

Regressions trigger immediate `gh issue create --label priority:high` and a message to the fixer.

### Strategic Analysis

Every ~10 merges, the strategist produces a data-driven report:
- Priority distribution (are we doing P1 work or drifting to P4?)
- Roadmap progress (NOW items completed?)
- Agent effectiveness (who succeeds vs. fails?)
- Stale work (in-progress slices >24h)
- Recommendations (adjust scout focus, fix agent definitions)

## Self-Improvement

### Four Persistence Layers

| Layer | Lifetime | What |
|-------|----------|------|
| **Handoffs** | Until merge | Context transfer: scout→builder→reviewer |
| **Ops files** | Current cycle | known-pitfalls, completed-slices, discovered-issues, metrics, agent-patches |
| **GitHub** | Permanent | Issues (swarm-discovered, swarm-architectural), PRs (labeled), CI status |
| **Claude Code memories** | Cross-session | Critical lessons, session progress, architectural decisions |

### Learning Loops

1. **Fixers** → `known-pitfalls.md` → scouts/builders avoid repeating known traps
2. **All agents** → `discovered-issues.md` → scouts pick up pre-investigated leads
3. **All agents** → GitHub issues (`--label swarm-discovered`) → persistent, searchable backlog
4. **All agents** → `swarm-metrics.jsonl` → strategist spots performance patterns
5. **Failing agents** → `agent-patches/` → bootstrapper improves agent definitions
6. **Improver-docs** → reads handoff lessons → crystallizes into ADRs and friction logs
7. **Improver-devex** → reads handoff lessons → fixes the friction that slowed agents
8. **Merger** → analyzes metrics → reports which domains/agents need attention
9. **Lead** → Claude Code memories → carries critical knowledge to future sessions

### Agent Self-Improvement

When an agent hits friction caused by its own definition being wrong or incomplete, it writes a patch proposal to `.ops/agent-patches/<agent>.md`. The bootstrapper integrates validated patches during `--refresh`. The user reviews and merges. This means the agent definitions evolve based on actual field experience.

## GitHub-Native Tracking

### Labels
- **PR labels**: `swarm-core`, `swarm-improve-docs`, `swarm-improve-tests`, `swarm-improve-devex`, `swarm-improve-infra`
- **Issue labels**: `swarm-discovered` (found by agent during other work), `swarm-architectural` (needs user decision)

### Templates
- **Issue template**: `swarm_discovered.yml` — structured fields for agent, context, files, category
- **PR template**: summary, changes, verification, agent attribution

### Auto-Merge
Small PRs (improvements, side fixes) use `gh pr merge --auto --squash --delete-branch` to merge when checks pass without waiting for the merger.

### State Queries
```bash
gh pr list --state open --label "swarm-core"
gh issue list --label "swarm-discovered" --state open
gh run list --status failure --limit 10
```

## Background Improvement (~20%)

The swarm always dedicates ~20% of its branches to making the codebase better, not just fixing the primary task. This runs via two always-on improver teammates:

**improver-docs**: README, CHANGELOG, ADRs, friction log, roadmap updates, CLAUDE.md, command reference. Reads handoff "Key Decisions" and "Lesson Learned" sections to find ADR candidates. ADRs are the highest-value output — they document decisions that would otherwise be lost when agents exit.

**improver-tests**: Mutation survivors (highest priority), flaky test fixes, coverage gaps, test naming/BDD, integration tests. Also handles infrastructure: unused dep removal, dead code, security audits. Reads handoff lessons to find test gaps that were revealed during builds.

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
1. Load protocol, priorities, and current state
2. Sync repo, ensure GitHub labels exist
3. Check for pending work from previous sessions (in-progress slices, open PRs, stale worktrees)
4. Create 12-teammate team
5. Set up recurring loops: `/loop 10m /swarm-status`, `/loop 30m /green-merge`

### Continuous Operation
All lanes run concurrently. Scouts feed builders feed reviewers feed merger. Improvers run alongside. Validator verifies. Strategist steers. Fixer repairs. No batching.

### Graceful Shutdown (`/swarm-wind-down`)
~20 minutes: stop scouts → let builders finish → review and PR everything → merge green → clean up → write memories

### Emergency Stop (`/swarm-stop`)
~5 minutes: broadcast STOP → snapshot state → enable auto-merge on green PRs → write memory → halt team → leave worktrees for next session

### Session Resumption
Next `/swarm` picks up: in-progress slices from `completed-slices.md`, open PRs (some auto-merged), active worktrees, pending agent patches, discovered issues.

## Portable Pack

The `docs/handoff/swarm-pack/` directory contains everything needed to adopt this in another repo:

```bash
bash swarm-pack/setup.sh    # Install 25 agents, 15 skills, hooks, ops, GH labels
/bootstrap-agents            # Discover codebase → generate ~25-30 domain agents
/swarm all                   # Start continuous swarm
```

The pack installs portable agents (core swarm, improvers, quality, review, research, docs, infra, bootstrapper) and expects each repo to generate its own domain-specific agents via `/bootstrap-agents`.

### Agent Taxonomy (~50 total after bootstrap)

| Category | Pack (portable) | Repo-specific (generated) |
|----------|----------------|--------------------------|
| Core swarm | 6 (scout, builder, reviewer, fixer, merger, janitor) | — |
| Governance | 3 (validator, strategist, pr-responder) | — |
| Improvers | 4 (docs, tests, devex, infra) | — |
| Quality | 2 (mutant-killer, coverage-filler) | 3-5 (fuzz, flaky, test-quality) |
| Review | 3 (standards, security, scope) | 2-3 (performance, api) |
| Research | 2 (codebase, issues) | 1-2 (deps, PRs) |
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
6. **Skills over file reads.** `/swarm-protocol` not `Read .claude/SWARM_PROTOCOL.md`.
7. **Handoffs carry context.** Next agent reads previous agent's summary, not raw sources.
8. **Minimal subagent prompts.** 7 lines pointing to files/skills, not 100 lines inline.
9. **Per-unit verification.** Test the package you changed, not the workspace.

### Quality
10. **Validate merges.** Validator checks that work actually helped.
11. **Every agent is a scout.** Discoveries become GitHub issues for fresh agents.
12. **~20% goes to improvement.** Docs, tests, devex, infra — always on.
13. **Review comments get addressed.** PR responder monitors and fixes feedback.

### Governance
14. **Priority-weighted discovery.** Scouts check roadmap; strategist steers.
15. **Self-improving.** Metrics, agent patches, friction logs, ADRs.
16. **4 persistence layers.** Handoffs → ops files → GitHub → memories.
17. **GitHub-native.** Labels, issues, templates, auto-merge, `gh` CLI everywhere.

### Lifecycle
18. **Continuous, not batchy.** All lanes concurrent.
19. **Graceful shutdown.** `/swarm-wind-down` finishes work; `/swarm-stop` saves state.
20. **Session resumption.** Next `/swarm` picks up where the last one stopped.
