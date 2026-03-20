# Agent Catalog

## Architecture

```
Agent file = identity + objectives + todo list (WHAT to do)
Step skill = mechanical instructions per todo step (HOW to do it)
Flow command = spawn the right agent for a pipeline stage (ROUTING)
Crate CLAUDE.md = domain context carried by the codebase (CONTEXT)
GitHub Issue = task spec from scout to builder (HANDOFF)
```

## Core Pipeline

```
/flow-scout → /flow-build → /flow-review → /flow-merge → /flow-improve
   scout      plan-reviewer   reviewer       ops          improver
  (haiku)  →   (sonnet)   →  builder  →  (haiku+sonnet) → (haiku) → (sonnet)
                             (sonnet)
```

Haiku does the broad sweep cheaply. Sonnet refines the plan and builds.
Haiku checks standards. Sonnet checks correctness. Haiku merges.

## Agents (11)

### Pipeline Agents (7)

| Agent | Model | Steps | Role |
|-------|-------|-------|------|
| scout | haiku | 7 | Broad investigation → file initial plan |
| plan-reviewer | sonnet | 4 | Refine plan, stress-test, mark builder-ready |
| builder | sonnet | 5 | Implement from refined spec → draft PR |
| reviewer | haiku | 4 | Fast standards check (banned patterns, scope) |
| reviewer-deep | sonnet | 4 | Deep correctness check (logic, edge cases) |
| ops | haiku | 4 | Merge queue, CI, post-merge validation |
| improver | sonnet | 4 | Post-merge quality follow-ups |

### Specialized Scouts (3)

| Agent | Model | Domain |
|-------|-------|--------|
| scout-parser | sonnet | Error buckets, corpus, parser engine |
| scout-lsp | sonnet | features.toml, providers, LSP spec |
| scout-dap | sonnet | DAP protocol, bridge mode, security |

### Utility (1)

| Agent | Model | Role |
|-------|-------|------|
| research-web | haiku | Web search, doc lookup, fact verification |

## Step Skills (20)

**Scout steps:** scout-dedup, scout-locate, scout-reproduce, scout-root-cause, scout-design, scout-test-spec, scout-report

**Builder steps:** builder-read-spec, builder-write-test, builder-implement

**Reviewer steps:** reviewer-read-handoff, reviewer-check-diff, reviewer-decide

**Reviewer-deep steps:** reviewer-deep-read-spec, reviewer-deep-analyze, reviewer-deep-edges, reviewer-deep-decide

**Ops steps:** ops-check-queue, ops-merge-batch, ops-post-merge

## Flow Commands (5)

| Command | What it does |
|---------|-------------|
| /flow-scout | Pick scout variant, spawn, get issue |
| /flow-build | Validate spec, spawn builder in worktree |
| /flow-review | Two-tier: reviewer (haiku) → reviewer-deep (sonnet) |
| /flow-merge | Spawn ops for merge queue |
| /flow-improve | Spawn improver for quality follow-ups |

## Shared Operations (10)

verify, verify-master-green, pr-create, pr-ready, pr-respond,
coding-standards, health-check, status-drift, rebase-pr, worktree-pr

## Domain Operations (8)

parser-fix, parser-scout, corpus-ratchet, dep-check, dep-clean,
security-scout, dap-scout, changelog

## Design Principles

1. **Agent = personality + todo list.** Skills = step mechanics. Context stays clean.
2. **Scoped, short-lived agents** beat long-running team members. 20K context > 1M context.
3. **Safety from architecture** (worktree + review + CI) enables full autonomy.
4. **Every output is a knowledge artifact** — narrate thinking, leave breadcrumbs.
5. **Crate CLAUDE.md files** carry domain context. Agents don't need domain specialization.
6. **Issues carry task specs.** Scouts do 75% of the work; builders execute.
7. **"Not done, but here's what's next"** is a valid success state.
8. **Model tiering:** haiku for mechanical checks, sonnet for creative analysis.
