# Agent Catalog

## Architecture

```
Two interfaces, two agent types:

  Agent()     → Worker agents (11) — worktree-isolated, background, one task, exit
  TeamCreate  → Sector leads (4) — long-running, pre-baked sector context, spawn workers

Agent file = identity + objectives + todo list (WHAT to do)
Step skill = mechanical instructions per todo step (HOW to do it)
Crate CLAUDE.md = domain context carried by the codebase (CONTEXT)
GitHub Issue = task spec from scout to builder (HANDOFF)

At scale:  User → Orchestrator → Sector leads (TeamCreate) → Workers (Agent())
At small:  User → Orchestrator → Workers (Agent()) directly
```

## Core Pipeline

```
scout → plan-reviewer → builder → reviewer → reviewer-deep → ops
(haiku)   (sonnet)      (sonnet)   (haiku)     (sonnet)     (haiku)

Variants: scout-parser, scout-lsp, scout-dap for domain-specific investigation
Continuation: spawn builder with /builder-read-pr instead of /builder-read-spec
Post-merge: wisdom (sonnet) synthesizes learnings
```

Haiku does the broad sweep cheaply. Sonnet refines the plan and builds.
Haiku checks standards. Sonnet checks correctness. Haiku merges.

## Sector Leads (TeamCreate — long-running coordinators)

| Agent | Model | Sector | Workers it spawns |
|-------|-------|--------|-------------------|
| lead-parser | sonnet | Parser/corpus | scout-parser, builder, plan-reviewer |
| lead-lsp | sonnet | LSP features | scout-lsp, builder, plan-reviewer |
| lead-quality | sonnet | Review/merge pipeline | reviewer, reviewer-deep, ops |
| lead-infra | sonnet | Tests, deps, docs, security, DX | scout, builder, research-web, wisdom |

Each lead has pre-baked sector context (crate paths, data sources, goals). They persist for the session, manage a shared task list, and spawn workers via Agent().

## Worker Agents (Agent()) — 11

### Pipeline Agents (6)

| Agent | Model | Steps | Role |
|-------|-------|-------|------|
| scout | haiku | 8 | Broad investigation → file initial plan |
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

## Step Skills (27)

**Scout steps:** scout-dedup, scout-locate, scout-reproduce, scout-root-cause, scout-design, scout-test-spec, scout-report

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

1. **Two interfaces, two agent types.** Workers via Agent() (worktree-isolated, one task, exit). Sector leads via TeamCreate (long-running, manage workers).
2. **Workers are scoped and short-lived.** One issue, one PR, one task per agent. 20K context > 1M context.
3. **Every worker runs in its own worktree.** Full isolation = full freedom. Agents can't harm each other.
4. **Sector leads manage, workers execute.** Leads spawn workers, track progress, coordinate. They never write code.
5. **Scale by adding sector leads, not by the orchestrator tracking more workers.** Small sessions: direct Agent() calls. Large sessions: TeamCreate with sector leads.
6. **Issues carry task specs.** Scouts do 75% of the work; builders execute.
7. **Every output is a knowledge artifact** — narrate thinking, leave breadcrumbs.
8. **Model tiering:** haiku for mechanical checks, sonnet for creative analysis and coordination.
