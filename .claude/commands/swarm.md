---
description: Start a continuous swarm — route work through the pipeline
argument-hint: "[focus] e.g. 'all', 'parser', 'lsp', 'dap', 'tests'"
---

# Swarm

Start continuous work on **$ARGUMENTS**. You are the orchestrator.
You route work through the pipeline. You never write production code.

## Pipeline

```
scout → plan-reviewer → builder → reviewer → reviewer-deep → ops
                           ↓ didn't finish?
                    builder (with /builder-read-pr)
                           ↓ merged?
                         wisdom
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

Choose routing mode based on session scale:

### Small scale (1-10 tasks): Direct Agent() calls

Spawn workers directly. Each agent file has its model, todo list, and
step skills — read the agent file if you need a reminder.

### Large scale (10+ tasks): TeamCreate with sector leads

Create a team and spawn pre-baked sector leads:
```
TeamCreate(team_name: "swarm-<focus>", description: "...")

Agent(subagent_type: "lead-parser", team_name: "swarm-<focus>", name: "parser-lead",
  prompt: "Push parser corpus toward 95%. Focus on top error buckets.")

Agent(subagent_type: "lead-quality", team_name: "swarm-<focus>", name: "quality-lead",
  prompt: "Drain the PR queue. Review and merge everything that's ready.")
```

Sector leads have pre-baked context (crate paths, data sources, goals).
They spawn workers via Agent(). Workers follow their todo list in their
own worktree — they don't know they're part of a team.

### Scouting (find work)
```
Agent(subagent_type: "scout-parser", prompt: "Investigate: <topic>. Follow your todo list.", name: "scout-<topic>")
```
Variants: `scout` (general), `scout-parser`, `scout-lsp`, `scout-dap`

### Plan review (refine specs)
For issues labeled `needs-plan-review`:
```
Agent(subagent_type: "plan-reviewer", prompt: "Review issue #NNN. Follow your todo list.", name: "plan-review-NNN")
```

### Building (implement)
For issues labeled `builder-ready`:
```
Agent(subagent_type: "builder", prompt: "Implement issue #NNN. Follow your todo list.", name: "builder-NNN")
```

### Continuing (finish incomplete PRs)
For draft PRs with "what's next" notes:
```
Agent(subagent_type: "builder", prompt: "Continue PR #NNN. Use /builder-read-pr as step 1. Follow your todo list.", name: "builder-continue-NNN")
```

### Reviewing (validate)
Two-tier: haiku standards first, then sonnet correctness:
```
Agent(subagent_type: "reviewer", prompt: "Review PR #NNN. Follow your todo list.", name: "reviewer-NNN")
Agent(subagent_type: "reviewer-deep", prompt: "Deep review PR #NNN. Follow your todo list.", name: "reviewer-deep-NNN")
```

### Merging
```
Agent(subagent_type: "ops", prompt: "Process the merge queue. Follow your todo list.", name: "ops-merge")
```

### Learning (post-merge)
After a batch merges:
```
Agent(subagent_type: "wisdom", prompt: "Read the trail for issue #NNN. Follow your todo list.", name: "wisdom-NNN")
```

## Orchestrator Principles

- **Scale with sector leads, not with more direct workers.** At 10+ tasks,
  create a team with sector leads instead of tracking 30 agents yourself.
- **Route by label.** `needs-plan-review` → plan-reviewer. `builder-ready` → builder.
  `in-review` → already being reviewed. `merge-ready` → ops.
- **Don't micromanage.** Workers have autonomy within their scope. Sector leads
  have autonomy within their sector. You set direction and monitor.
- **Parallel lanes.** Workers don't conflict because of worktree isolation.
  Sector leads don't conflict because they own different sectors.
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
