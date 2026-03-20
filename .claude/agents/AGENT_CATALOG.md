# Agent Catalog

## Architecture

```
Agent file = identity + objectives + todo list (WHAT to do)
Step skill = mechanical instructions per todo step (HOW to do it)
Crate CLAUDE.md = domain context carried by the codebase (CONTEXT)
GitHub Issue = task spec from scout to builder (HANDOFF)
Orchestrator = reads catalog + agent files to route work (ROUTING)
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

## Agents (11)

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

1. **Agent = personality + todo list.** Skills = step mechanics. Context stays clean.
2. **Scoped, short-lived agents** beat long-running team members. 20K context > 1M context.
3. **Every agent runs in its own worktree.** Full isolation = full freedom. Agents can't harm each other.
4. **Every output is a knowledge artifact** — narrate thinking, leave breadcrumbs.
5. **Crate CLAUDE.md files** carry domain context. Agents don't need domain specialization.
6. **Issues carry task specs.** Scouts do 75% of the work; builders execute.
7. **"Not done, but here's what's next"** is a valid success state.
8. **Model tiering:** haiku for mechanical checks, sonnet for creative analysis.
