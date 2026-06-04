# Agent Catalog

## Architecture

```
Two interfaces, two agent types:

  Agent()     → Worker agents (20) — worktree-isolated, background, one task, exit
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
scout → accuracy-scout → research-verifier → oppositional-planner → advocatus-diaboli → architecture-reviewer → maintainer-issue → plan-reviewer → spec-planner → red-tdd → builder → green-tdd → reviewer → maintainer-pr → pr-responder → refactor-planner → green-refactor → reviewer-deep → green-ci → diff-auditor → ops
(haiku)    (haiku)          (haiku)              (haiku)                (haiku)              (haiku)                (haiku)          (sonnet)        (haiku)       (haiku)   (sonnet)    (haiku)     (haiku)     (haiku)         (haiku)            (haiku)          (sonnet)       (sonnet)      (haiku)     (haiku)     (haiku)

Variants: scout-parser, scout-lsp, scout-dap for domain-specific investigation
Continuation: spawn builder with /builder-read-pr instead of /builder-read-spec
Post-merge: wisdom (sonnet) synthesizes learnings
```

Nine cheap haiku passes lock down facts, surface challenges, check architecture, verify project alignment, plan implementation, and write red tests before sonnet builds.
Sonnet plan-reviewer sees: verified spec + objections + existence verdict → makes the call.
Haiku spec-planner creates branch with .spec/ files. Haiku red-tdd writes failing tests.
Sonnet builder receives a branch where "done" is already defined — just make the tests green.
Haiku green-tdd adds edge case tests. Haiku reviewer checks standards. Haiku maintainer-pr
checks project fit. Haiku pr-responder addresses bot comments and CI failures.
Haiku refactor-planner analyzes the diff for simplification opportunities.
Sonnet green-refactor executes the refactor plan — the R in red-green-refactor.
Sonnet deep-reviewer checks correctness. Haiku green-ci verifies CI freshness
and fixes mechanical failures. Haiku diff-auditor checks the cumulative diff
is coherent across all agent commits. Haiku ops merges.

## Pipeline Leads (TeamCreate — long-running coordinators)

| Agent | Model | Pipeline Stage | Workers it spawns |
|-------|-------|----------------|-------------------|
| lead-discovery | sonnet | Find work | scout, accuracy-scout, scout-parser, scout-lsp, scout-dap, scout-find-* (6 discovery scouts), plan-reviewer |
| lead-build | sonnet | Build from specs | builder |
| lead-review | sonnet | Review and merge | reviewer, reviewer-deep, ops, wisdom |

Each lead coordinates a pipeline stage, not a domain. They persist for the
session, manage a shared task list, and spawn workers via Agent(). Leads
never read code or investigate — they only work through subagents.
disallowedTools (Edit, Write) enforces orchestrator-only role.

## Worker Agents (Agent()) — 32

### Pipeline Agents (21)

| Agent | Model | Steps | Role |
|-------|-------|-------|------|
| scout | haiku | 8 | Broad investigation → file initial plan |
| accuracy-scout | haiku | 5 | Verify mechanical facts (file paths, functions, issue status) before plan-review |
| research-verifier | haiku | 5 | Verify external claims (Perl docs, LSP spec, crate APIs) via web + grep |
| oppositional-planner | haiku | 4 | Challenge approach, surface alternatives, flag risks |
| advocatus-diaboli | haiku | 4 | Challenge existence — should this be built at all? BUILD/DEFER/CLOSE |
| architecture-reviewer | haiku | 4 | Verify design fits microcrate layering, dependency direction, type placement |
| maintainer-issue | haiku | 4 | Check issue aligns with perl-lsp goals, roadmap, user base |
| plan-reviewer | sonnet | 5 | Refine plan, stress-test, mark builder-ready |
| spec-planner | haiku | 6 | Read spec, create branch, commit .spec/ files with checklist/acceptance/context |
| red-tdd | haiku | 5 | Write failing tests on impl branch, commit, hand off to builder |
| builder | sonnet | 6 | Implement from spec → draft PR. Also used for continuation via /builder-read-pr |
| green-tdd | haiku | 5 | Add edge case, boundary, and regression tests after builder implements |
| reviewer | haiku | 5 | Fast standards check (banned patterns, scope) — push fixes directly |
| maintainer-pr | haiku | 4 | Check PR implementation fits project direction and quality bar |
| pr-responder | haiku | 3 | Address bot comments, CI failures, resolve conversations before deep review |
| refactor-planner | haiku | 4 | Analyze diff for simplification, reuse, dead code — post plan for green-refactor |
| green-refactor | sonnet | 5 | Execute refactor plan: simplify while keeping tests green — the R in red-green-refactor |
| reviewer-deep | sonnet | 5 | Deep correctness check (logic, edge cases) |
| green-ci | haiku | 3 | Verify all CI checks pass on current HEAD SHA, fix mechanical failures |
| diff-auditor | haiku | 3 | Final coherence check — cumulative diff matches spec, no artifacts or regressions |
| ops | haiku | 5 | Merge queue, CI, post-merge validation |

### Specialized Scouts (3)

| Agent | Model | Domain |
|-------|-------|--------|
| scout-parser | haiku | Error buckets, corpus, parser engine |
| scout-lsp | haiku | features.toml, providers, LSP spec |
| scout-dap | sonnet | DAP protocol, bridge mode, security |

These file **one builder-ready issue** with a full spec. They differ from
the discovery scouts below, which file lightweight candidate packets.

### Discovery Scouts (6)

The **Issue Discovery / Bug Scout Desk** — the swarm's radar, upstream of
plan review. Read-only sweeps that file evidence-backed *candidate packets*
(label `candidate-issue`), never builder-ready specs. Doctrine:
[`docs/reference/ISSUE_DISCOVERY_DOCTRINE.md`](../../docs/reference/ISSUE_DISCOVERY_DOCTRINE.md).
Kick off with `/issue-discovery`.

| Agent | Model | Surface |
|-------|-------|---------|
| scout-find-dap-gaps | haiku | DAP stack/scopes/variables/lifecycle/transport |
| scout-find-lsp-gaps | haiku | document state, URI isolation, completion, hover, code-action, semantic tokens |
| scout-find-parser-gaps | haiku | parser/AST/NodeKind/recovery/fixtures |
| scout-find-ci-ops-gaps | haiku | workflow routing, path filters, labels, cleanup, runner capacity |
| scout-find-robustness-gaps | haiku | panic/DoS/unsafe-indexing/byte-slicing in server paths |
| scout-find-docs-receipt-drift | haiku | status-doc vs receipt drift and basis conflicts |

### Utility (2)

| Agent | Model | Role |
|-------|-------|------|
| research-web | sonnet | Ad-hoc web research — single question, spawned by other agents |
| wisdom | sonnet | Synthesize learnings from issue→PR→merge cycles |

## Step Skills (62)

**Scout steps:** scout-dedup, scout-locate, scout-reproduce, scout-root-cause, scout-design, scout-test-spec, scout-verify, scout-report

**Research-verifier steps:** research-read-issue, research-verify-perl, research-verify-spec, research-verify-api, research-comment

**Accuracy-scout steps:** accuracy-read-issue, accuracy-verify-files, accuracy-verify-claims, accuracy-verify-status, accuracy-comment

**Architecture-reviewer steps:** architecture-read, architecture-check, architecture-comment

**Maintainer-issue steps:** maintainer-issue-read, maintainer-issue-check, maintainer-issue-comment

**Maintainer-PR steps:** maintainer-pr-read, maintainer-pr-check, maintainer-pr-comment

**Oppositional-planner steps:** oppositional-read, oppositional-challenge, oppositional-comment

**Advocatus-diaboli steps:** diaboli-read, diaboli-challenge, diaboli-comment

**Spec-planner steps:** spec-planner-read, spec-planner-verify, spec-planner-plan, spec-planner-branch, spec-planner-comment

**Red-TDD steps:** red-tdd-read, red-tdd-write, red-tdd-verify, red-tdd-commit

**Builder steps:** builder-read-spec, builder-read-pr, builder-write-test, builder-implement, builder-self-review

**Green-TDD steps:** green-tdd-read, green-tdd-write, green-tdd-verify, green-tdd-commit

**Reviewer steps:** reviewer-read-handoff, reviewer-check-diff, reviewer-decide

**PR-responder steps:** pr-respond (existing shared skill)

**Refactor-planner steps:** refactor-planner-read, refactor-planner-analyze, refactor-planner-comment

**Green-refactor steps:** green-refactor-read, green-refactor-simplify, green-refactor-verify, green-refactor-comment

**Green-CI steps:** green-ci-check, green-ci-comment

**Diff-auditor steps:** diff-audit-check, diff-audit-comment

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
