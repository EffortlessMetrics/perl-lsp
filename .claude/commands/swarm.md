---
description: Start a continuous swarm — route work through the pipeline
argument-hint: "[focus] e.g. 'all', 'parser', 'lsp', 'dap', 'tests'"
---

# Swarm

Start continuous work on **$ARGUMENTS**. You are the orchestrator.
You route work through the pipeline. You never write production code.

## Pipeline

```
/flow-scout → /flow-build → /flow-review → /flow-merge
                  ↓ didn't finish?
             /flow-continue
                  ↓ merged?
             /flow-wisdom
```

## Phase 1: Bootstrap

```bash
git fetch origin && git pull origin master
gh pr list --state open --limit 200 --json number | jq length
gh issue list --state open --limit 200 --json number | jq length
gh run list --branch master --limit 1
just clean-worktrees 2>/dev/null || git worktree prune
```

**Stop if master CI is red.** Fix it first.

## Phase 2: Assess

Check what needs work:
- `gh issue list --label "builder-ready" --state open` — ready to build
- `gh issue list --label "needs-plan-review" --state open` — need plan review
- `gh pr list --label "merge-ready"` — ready to merge
- `gh pr list --label "in-review"` — being reviewed
- `gh issue list --label "swarm-discovered" --state open` — scout findings

## Phase 3: Route Work

Use flow commands to move work through the pipeline. Spawn short-lived,
scoped agents — not long-running teams.

### Scouting (find work)
```
/flow-scout <topic>        — spawn a haiku scout, get an issue back
```

### Plan review (refine specs)
For issues labeled `needs-plan-review`:
```
Agent(subagent_type: "plan-reviewer", prompt: "Review issue #NNN.", name: "plan-review-NNN")
```

### Building (implement)
For issues labeled `builder-ready`:
```
/flow-build <issue-number>  — spawn a sonnet builder in worktree
```

### Continuing (finish incomplete PRs)
For draft PRs with "what's next" notes:
```
/flow-continue <pr-number>  — spawn builder to continue from existing PR
```

### Reviewing (validate)
For open PRs:
```
/flow-review <pr-number>    — two-tier: haiku standards + sonnet correctness
```

### Merging
```
/flow-merge                 — spawn ops to merge approved PRs
```

### Learning (post-merge)
After a batch merges:
```
/flow-wisdom <issue-number> — synthesize learnings from a completed cycle
```

## Orchestrator Principles

- **Spawn scoped, short-lived agents.** One issue, one PR, one task per agent.
  Don't create long-running teams. Each agent follows its todo list and exits.
- **Route by label.** `needs-plan-review` → plan-reviewer. `builder-ready` → builder.
  `in-review` → already being reviewed. `merge-ready` → ops.
- **Don't micromanage.** Agents have autonomy within their scope. The pipeline
  and guardrails ensure quality. You just route work.
- **Parallel lanes.** Run scouts, builders, reviewers, and ops simultaneously
  on different issues/PRs. They don't conflict because of worktree isolation.
- **Can't skip validation.** Every PR goes through review. Every issue goes
  through plan review before building. The pipeline can loop but not skip.

## Focus Variants

| Focus | Scout targets | Builder capacity |
|-------|--------------|-----------------|
| `all` | Everything: parser, LSP, DAP, tests, docs | Full |
| `parser` | Parser error buckets, corpus | Full |
| `lsp` | LSP features, providers, spec compliance | Full |
| `dap` | DAP protocol, test gaps | 1-2 builders |
| `tests` | Test coverage gaps | Full |

## Monitoring

```bash
# Quick status
gh pr list --state open --limit 20 --json number,title,labels
gh issue list --label builder-ready --state open --limit 10
gh run list --branch master --limit 3

# Health check
/health-check
```
