# Agent Catalog

## Architecture

```
Two interfaces, two agent types:

  Agent()     → Worker agents (12) — worktree-isolated, background, one task, exit
  TeamCreate  → Pipeline leads (3) — long-running, pipeline-stage coordinators, spawn workers

Agent file = identity + objectives + todo list (WHAT to do)
Step skill = mechanical instructions per todo step (HOW to do it)
Crate CLAUDE.md = domain context carried by the codebase (CONTEXT)
GitHub Issue = task spec from scout to builder (HANDOFF)

At scale:  User → Orchestrator → Pipeline leads (TeamCreate) → Workers (Agent())
At small:  User → Orchestrator → Workers (Agent()) directly
```

## Core Pipeline

```
scout → accuracy-scout → plan-reviewer → builder → reviewer → reviewer-deep → ops
(haiku)    (haiku)         (sonnet)      (sonnet)   (haiku)     (sonnet)     (haiku)

Variants: scout-parser, scout-lsp, scout-dap for domain-specific investigation
Continuation: spawn builder with /builder-read-pr instead of /builder-read-spec
Post-merge: wisdom (sonnet) synthesizes learnings
```

Haiku does the broad sweep cheaply. Accuracy-scout verifies mechanical facts cheaply.
Sonnet refines the plan and builds. Haiku checks standards. Sonnet checks correctness. Haiku merges.

## Pipeline Leads (TeamCreate — long-running coordinators)

| Agent | Model | Pipeline Stage | Workers it spawns |
|-------|-------|----------------|-------------------|
| lead-discovery | sonnet | Find work | scout, accuracy-scout, scout-parser, scout-lsp, scout-dap, plan-reviewer |
| lead-build | sonnet | Build from specs | builder |
| lead-review | sonnet | Review and merge | reviewer, reviewer-deep, ops, wisdom |

Each lead coordinates a pipeline stage, not a domain. They persist for the
session, manage a shared task list, and spawn workers via Agent(). Leads
never read code or investigate — they only work through subagents.
disallowedTools (Edit, Write) enforces orchestrator-only role.

## Worker Agents (Agent()) — 12

### Pipeline Agents (7)

| Agent | Model | Steps | Role |
|-------|-------|-------|------|
| scout | haiku | 8 | Broad investigation → file initial plan |
| accuracy-scout | haiku | 5 | Verify mechanical facts (file paths, functions, issue status) before plan-review |
| plan-reviewer | sonnet | 5 | Refine plan, stress-test, mark builder-ready |
| builder | sonnet | 6 | Implement from spec → draft PR. Also used for continuation via /builder-read-pr |
| reviewer | haiku | 5 | Fast standards check (banned patterns, scope) |
| reviewer-deep | sonnet | 5 | Deep correctness check (logic, edge cases) |
| ops | haiku | 5 | Merge queue, CI, post-merge validation |

### Specialized Scouts (3)

| Agent | Model | Domain |
|-------|-------|--------|
| scout-parser | haiku | Error buckets, corpus, parser engine |
| scout-lsp | haiku | features.toml, providers, LSP spec |
| scout-dap | sonnet | DAP protocol, bridge mode, security |

### Utility (2)

| Agent | Model | Role |
|-------|-------|------|
| research-web | sonnet | Web search, doc lookup, fact verification |
| wisdom | sonnet | Synthesize learnings from issue→PR→merge cycles |

## Step Skills (32)

**Scout steps:** scout-dedup, scout-locate, scout-reproduce, scout-root-cause, scout-design, scout-test-spec, scout-report

**Accuracy-scout steps:** accuracy-read-issue, accuracy-verify-files, accuracy-verify-claims, accuracy-verify-status, accuracy-comment

**Builder steps:** builder-read-spec, builder-read-pr, builder-write-test, builder-implement, builder-self-review

**Reviewer steps:** reviewer-read-handoff, reviewer-check-diff, reviewer-decide

**Reviewer-deep steps:** reviewer-deep-read-spec, reviewer-deep-analyze, reviewer-deep-edges, reviewer-deep-decide

**Ops steps:** ops-check-queue, ops-merge-batch, ops-post-merge, ops-cleanup

**Wisdom steps:** wisdom-read-trail, wisdom-synthesize, wisdom-document

**Shared steps:** agent-wrapup

## Shared Operations (10)

verify, verify-master-green, pr-create, pr-ready, pr-respond,
coding-standards, health-check, status-drift, rebase-pr, worktree-pr

## Domain Operations (8)

parser-fix, parser-scout, corpus-ratchet, dep-check, dep-clean,
security-scout, dap-scout, changelog

## Design Principles

1. **Two interfaces, two agent types.** Workers via Agent() (worktree-isolated, one task, exit). Pipeline leads via TeamCreate (long-running, manage workers).
2. **Workers are scoped and short-lived.** One issue, one PR, one task per agent. 20K context > 1M context.
3. **Every worker runs in its own worktree.** Full isolation = full freedom. Agents can't harm each other.
4. **Pipeline leads manage, workers execute.** Leads spawn workers, track progress, coordinate. They never write code or read code — disallowedTools enforces this.
5. **Scale by adding pipeline leads, not by the orchestrator tracking more workers.** Small sessions: direct Agent() calls. Large sessions: TeamCreate with pipeline leads.
6. **Issues carry task specs.** Scouts do 75% of the work; builders execute.
7. **Every output is a knowledge artifact** — narrate thinking, leave breadcrumbs.
8. **Model tiering:** haiku for mechanical checks, sonnet for creative analysis and coordination.
