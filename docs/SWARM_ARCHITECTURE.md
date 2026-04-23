# Swarm Architecture Guide

A design system for running AI agent swarms that produce reliable, well-tested code changes. Built on Claude Code's native features (agents, skills, worktrees, tasks, hooks). Portable to any repo.

## Core Idea

The swarm is a pipeline of short-lived, focused agents. Each agent has a personality, a todo list, and step skills. The todo list structures the work. The skills carry the mechanical details. The codebase carries the domain context.

```
Scout (haiku) → Plan-reviewer (sonnet) → Builder (sonnet) → Review (haiku+sonnet) → Merge (haiku) → Improve (sonnet)
```

Each stage uses the cheapest model that can do the job. Haiku sweeps broadly. Sonnet thinks deeply and writes code. The quality comes from the pipeline, not from any single agent being brilliant.

## Architecture Layers

```
┌─ Root CLAUDE.md ──────────────────── Project rules, coding standards
│  ┌─ Crate/module CLAUDE.md ──────── Local types, patterns, conventions
│  │  ┌─ GitHub Issue ─────────────── Task spec: file:line, root cause, test code
│  │  │  ┌─ Agent file ────────────── Identity, principles, todo list (~20 lines)
│  │  │  │  ┌─ Step skill ─────────── Mechanical details for current step
│  │  │  │  │
│  │  │  │  │  Agent works HERE
```

Each layer is loaded when relevant. An agent working in `src/parser/` gets the project CLAUDE.md, the parser CLAUDE.md, the issue spec, its own todo list, and one step skill at a time. Context stays focused.

## Agent Design

### What goes in the agent file (~20 lines)

Things the agent needs **throughout** its entire run:

- **Identity**: one sentence about who it is
- **Principles**: meta-knowledge that applies at every step (evidence over opinion, fix forward, narrate thinking)
- **Todo list**: numbered steps, each referencing a skill

```markdown
---
name: scout
model: haiku
---

You are a scout. You investigate one finding and produce a
GitHub issue thorough enough that a builder can implement it
without re-researching.

## Principles
- Evidence over opinion: file paths, line numbers, commands
- Narrate your thinking. Share what you explored and ruled out.
- One finding per investigation.

## Todo list
1. /scout-dedup
2. /scout-locate
3. /scout-reproduce
4. /scout-root-cause
5. /scout-design
6. /scout-test-spec
7. /scout-report
8. /agent-wrapup
```

### What goes in step skills (~30-80 lines)

Things the agent only needs **at that step**:

- Mechanical instructions (which commands to run)
- Templates (issue format, PR format)
- Validation checks (pre-flight checklists)
- Output recording (what to capture for the next step)

```markdown
---
description: Scout step 1 — check if this finding is already tracked
---

# Scout Dedup Check

1. Search open issues:
   gh issue list --search "<topic>" --limit 10

2. Search open PRs:
   gh pr list --search "<topic>" --limit 10

3. If duplicate found: STOP and report "already tracked as #NNN"
```

### Why separate them

- **Context efficiency**: A scout exploring code doesn't need the `gh issue create` template in context. That's loaded at step 7, not step 2.
- **Reusability**: `/verify` is used by builders, reviewers, and ops. `/agent-wrapup` is used by everyone.
- **Maintainability**: Change the issue template once in `/scout-report`, all scouts get it. Change the verify command once in `/verify`, all agents get it.

## The Pipeline

### 1. Scout (haiku) — broad investigation

Reads lots of code cheaply. Files a GitHub issue with:
- Problem description with file:line evidence
- Root cause in one sentence
- 2-3 fix options with tradeoffs
- Test code (actual code, not description)
- Narrative about what was explored and ruled out

**Model choice**: haiku — does 80% of the work at 5% of the cost.

### 2. Plan-reviewer (sonnet) — refine the plan

Reads the scout's issue with fresh eyes. Verifies file references against current code. Stress-tests the approach. Adds the `builder-ready` label when satisfied.

**Model choice**: sonnet — reads less code but thinks deeper. Adds the 20% that makes the builder's job unambiguous.

### 3. Builder (sonnet) — implement from spec

Receives a reviewed, labeled issue. Follows TDD: write failing test → implement fix → verify → create PR. Does NOT research the codebase.

**Model choice**: sonnet — needs to generate correct code.

**Key rule**: If the builder needs to "research" or "find" or "understand", the scout and plan-reviewer didn't finish. The spec should be copy-paste implementable.

### 4. Reviewer (haiku) — fast standards pass

Checks banned patterns, scope creep, formatting, test presence. Fix forward: applies trivial fixes directly instead of sending back.

**Model choice**: haiku — pattern matching, not creative thinking.

### 5. Reviewer-deep (sonnet) — correctness pass

Does the logic actually work? Edge cases handled? Regression risk? Fix forward when possible.

**Model choice**: sonnet — needs to reason about code correctness.

### 6. Ops (haiku) — merge

Merges reviewed, CI-green PRs in batches of 3. Validates master after each batch. Ratchets metrics.

**Model choice**: haiku — mechanical operations.

### 7. Improver (sonnet) — post-merge quality

Scans recent merges for gaps. Files follow-up issues. Makes small fixes. The continuous improvement loop.

**Model choice**: sonnet — needs to assess code quality.

## Key Design Principles

### 1. Agent = personality + todo list. Skills = step mechanics.

The agent file says WHO and WHAT. The skills say HOW. This separation keeps context clean — each step only loads what it needs.

### 2. Scoped, short-lived agents over long-running teams

A scoped agent (20K context, one issue) beats a long-running team member (1M context, many tasks) for implementation. Long-running agents are good for orchestration/triage only.

### 3. Safety from architecture enables autonomy

Worktree isolation + scoped goals + two-tier review + CI gates = agents can act freely. Don't micromanage. The guardrails prevent damage even if an agent makes a wrong call.

### 4. Every output is a knowledge artifact

Scout issues narrate thinking. Builder PRs document alternatives considered. Reviewer comments explain what was verified. Improver issues link to the PR that prompted them. Each pass leaves breadcrumbs.

### 5. "Not done, but here's what's next" is success

Partial progress with clear next steps is more valuable than a complete attempt that doesn't explain itself. The `/agent-wrapup` retrospective captures what was learned.

### 6. Can't skip validation gates

The pipeline can loop (reviewer sends back to builder, builder re-submits) but can't skip stages. Every PR goes through review. Every issue goes through plan review before building.

### 7. The codebase carries its own domain context

Per-module CLAUDE.md files provide local context (types, patterns, conventions). Agents don't need domain specialization — the codebase tells them what they need to know when they're working in that area.

### 8. Model tiering follows task complexity

Haiku for broad sweeps and mechanical checks. Sonnet for thinking, refining, and code generation. The most expensive model (Opus) is reserved for the orchestrator making strategic routing decisions.

## Setting Up in a New Repo

### Minimal setup (1 hour)

1. **Create `.claude/agents/`** with at least: `scout.md`, `builder.md`, `reviewer.md`
2. **Create `.claude/commands/`** with step skills for each agent's todo list
3. **Add root CLAUDE.md** with project rules, coding standards, verify commands
4. **Add `.claude/settings.json`** with worktree symlinks and basic hooks

### Recommended setup (half day)

4. **Add per-module CLAUDE.md files** for your key source directories
5. **Add issue templates** (`.github/ISSUE_TEMPLATE/scout_report.yml`)
6. **Add PR template** with knowledge artifact sections
7. **Add pipeline labels**: `needs-plan-review`, `builder-ready`, `in-review`, `merge-ready`
8. **Add auto-labeling workflow** (`.github/workflows/pipeline-labels.yml`)
9. **Add all 7 agents**: scout, plan-reviewer, builder, reviewer, reviewer-deep, ops, improver
10. **Add flow commands**: `/flow-scout`, `/flow-build`, `/flow-review`, `/flow-merge`, `/flow-improve`

### File structure

```
.claude/
  agents/
    scout.md              # ~20 lines: identity + principles + todo
    plan-reviewer.md
    builder.md
    reviewer.md
    reviewer-deep.md
    ops.md
    improver.md
    AGENT_CATALOG.md      # overview of all agents and the pipeline
  commands/
    # Scout steps (7)
    scout-dedup.md
    scout-locate.md
    scout-reproduce.md
    scout-root-cause.md
    scout-design.md
    scout-test-spec.md
    scout-report.md
    # Plan review steps (4)
    plan-review-read.md
    plan-review-verify.md
    plan-review-stress.md
    plan-review-improve.md
    # Builder steps (3)
    builder-read-spec.md
    builder-write-test.md
    builder-implement.md
    # Reviewer steps (3)
    reviewer-read-handoff.md
    reviewer-check-diff.md
    reviewer-decide.md
    # Reviewer-deep steps (4)
    reviewer-deep-read-spec.md
    reviewer-deep-analyze.md
    reviewer-deep-edges.md
    reviewer-deep-decide.md
    # Ops steps (3)
    ops-check-queue.md
    ops-merge-batch.md
    ops-post-merge.md
    # Shared
    verify.md
    pr-create.md
    agent-wrapup.md
    coding-standards.md
    health-check.md
    # Flows
    flow-scout.md
    flow-build.md
    flow-review.md
    flow-merge.md
    flow-improve.md
  settings.json           # hooks, permissions, worktree config
  hooks/
    teammate-idle.sh
    subagent-stop.sh
CLAUDE.md                 # root project instructions
```

## Spawning Agents

The orchestrator (the main Claude Code session, typically Opus) spawns agents:

```
# Scout a topic
Agent(subagent_type: "scout", prompt: "Investigate parser CHECK label handling.", name: "scout-check-label")

# Build from a reviewed issue
Agent(subagent_type: "builder", prompt: "Implement issue #2389.", isolation: "worktree", name: "builder-2389")

# Review a PR
Agent(subagent_type: "reviewer", prompt: "Review PR #2411.", name: "reviewer-2411")
```

Each agent loads its agent file, creates its task list, and works through the steps autonomously. The orchestrator doesn't micromanage — it routes work and monitors results.

## What We Learned Building This

These principles were validated by running 30+ agent cycles on a 128-crate Rust project:

1. **30 builders with "research first" prompts produced 0 PRs.** Builders with exact specs from scouts produced 43+ merges in one session. The scout→issue pipeline is the bottleneck, not builder count.

2. **Domain agents are unnecessary when the codebase has per-module docs.** 55 "specialized" agents were replaced by 11 generic agents + 52 crate-level CLAUDE.md files. The codebase carries its own context.

3. **Haiku scouts + sonnet plan review is more cost-effective than sonnet scouts.** Haiku does the broad sweep cheaply. Sonnet adds the refinement that makes builders efficient. Total cost is lower, quality is higher.

4. **Every agent's retrospective makes the next agent faster.** The `/agent-wrapup` step compounds across cycles. "The dispatch table is ordered by token kind" saves the next scout 10 minutes.

5. **Fix forward beats send back.** Reviewers that apply 5-line fixes directly move faster than reviewers that send PRs back for nits. The pipeline stays fluid.

6. **The flow is the product, not the agents.** Individual agents are interchangeable. The pipeline that chains them — with validation gates at each transition — is what guarantees quality.
