---
description: Start a continuous swarm — route work through the pipeline
argument-hint: "[focus] e.g. 'all', 'parser', 'lsp', 'dap', 'tests'"
---

# Swarm

Start continuous work on **$ARGUMENTS**. You are the orchestrator.
You route work through the pipeline. You never write production code.

## Pipeline

```
scout → research-verifier → plan-reviewer → builder → reviewer → reviewer-deep → ops
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

Check what needs work using label-driven state queries:

```bash
# Pipeline stage queries (label-driven state machine)
gh issue list --label "builder-ready" --state open      # ready to build (not yet claimed)
gh issue list --label "in-build" --state open           # builder assigned (check for stalls)
gh issue list --label "needs-plan-review" --state open  # need plan review
gh issue list --label "research-reviewed" --state open  # verified, ready for plan-review
gh issue list --label "swarm-discovered" --state open   # scout findings awaiting research-verify
gh issue list --label "structural-blocker" --state open # blocked work (needs architectural decision)
gh pr list --label "merge-ready"                        # ready to merge
gh pr list --label "in-review"                          # being reviewed (check for stalls)
```

**Routing rules:**
- `swarm-discovered` → research-verifier
- `research-reviewed` OR `needs-plan-review` → plan-reviewer
- `builder-ready` (without `in-build`) → builder
- `in-build` but PR not found → check for stalled builders, possibly re-route
- `in-review` → already being reviewed (do not double-assign)
- `merge-ready` → ops
- `structural-blocker` → escalate to orchestrator or human decision

## Phase 3: Route Work

Choose routing mode based on session scale:

### Small scale (1-10 tasks): Direct Agent() calls

Spawn workers directly. Each agent file has its model, todo list, and
step skills — read the agent file if you need a reminder.

### Large scale (10+ tasks): TeamCreate with pipeline leads

Create a team and spawn pipeline-stage leads:
```
TeamCreate(team_name: "swarm-<focus>", description: "...")

Agent(subagent_type: "lead-discovery", team_name: "swarm-<focus>", name: "discovery-lead",
  prompt: "Find work: scout parser error buckets, LSP feature gaps, test coverage.")

Agent(subagent_type: "lead-build", team_name: "swarm-<focus>", name: "build-lead",
  prompt: "Build everything in the builder-ready queue.")

Agent(subagent_type: "lead-review", team_name: "swarm-<focus>", name: "review-lead",
  prompt: "Drain the PR queue. Review and merge everything that's ready.")
```

Pipeline leads coordinate by stage (discover, build, review), not by domain.
They spawn workers via Agent(). Workers follow their todo list in their
own worktree — they don't know they're part of a team.

For domain-heavy sessions (e.g. parser-only or LSP-only), you can still
spawn domain-specific scout variants (scout-parser, scout-lsp, scout-dap)
directly instead of using leads.

### Scouting (find work)
```
Agent(subagent_type: "scout-parser", prompt: "Investigate: <topic>. Follow your todo list.", name: "scout-<topic>")
```
Variants: `scout` (general), `scout-parser`, `scout-lsp`, `scout-dap`

### Research verification (fact-check scout claims)
For issues labeled `swarm-discovered` that have external claims to verify:
```
Agent(subagent_type: "research-verifier", prompt: "Verify facts in issue #NNN. Follow your todo list.", name: "research-verify-NNN")
```
Adds `research-reviewed` label when done. Then route to plan-reviewer.

### Plan review (refine specs)
For issues labeled `needs-plan-review` or `research-reviewed`:
```
Agent(subagent_type: "plan-reviewer", prompt: "Review issue #NNN. Follow your todo list.", name: "plan-review-NNN")
```
Keep the `plan-review-NNN` naming shape stable. The stop hook uses that
canonical agent name as a fallback issue binding when `ISSUE_NUMBER` or
`issue_number` are not provided explicitly.

### Building (implement)
For issues labeled `builder-ready`:
```
Agent(subagent_type: "builder", prompt: "Implement issue #NNN. Follow your todo list.", name: "builder-NNN")
```
Builder will claim the issue with `in-build` and remove `builder-ready` on pickup.

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

## Label-Driven State Machine

Labels are the authoritative state of every issue and PR. Agents write labels; the orchestrator reads them.

| Label | Written by | Queried by | Means |
|-------|-----------|-----------|-------|
| `needs-plan-review` | scout (/scout-report) | orchestrator | Awaiting plan-reviewer |
| `plan-reviewed` | plan-reviewer (/plan-review-improve) | orchestrator | Spec verified |
| `builder-ready` | plan-reviewer (/plan-review-improve) | orchestrator, builder | Ready for builder pickup |
| `in-build` | builder (/builder-read-spec) | orchestrator | Builder claimed this issue |
| `in-review` | reviewer (/reviewer-read-handoff) | orchestrator | PR actively in review — set at review start |
| `merge-ready` | reviewer (/pr-ready) | ops agent | Ready for merge pickup |
| `structural-blocker` | any agent | orchestrator | Blocks parallel work |
| `needs-deep-review` | reviewer (/reviewer-decide) | orchestrator | Standards review done, awaiting deep correctness review |
| `deep-reviewed` | reviewer-deep (/reviewer-deep-decide) | pr-ready, ops | Deep correctness review complete — required for non-docs PRs |
| `follow-up-recommended` | wisdom or reviewer | orchestrator | Related follow-up issue needed |
| `already-fixed` | plan-reviewer or scout | orchestrator | Close without build |

**Key principle:** Labels gate entry, not skip execution. Multiple passes of the same agent are normal — a reviewer may see `in-review` and still choose to do another pass if quality warrants it.

**Label freshness:** Labels are version-bound via receipt comments (see `.ops-perl-lsp/receipts/README.md`). When an artifact changes after a label is set, the label becomes stale. Use `/label-receipt-validate` to check freshness before routing. Labels without receipts (set before the receipt system) should be treated as potentially stale.

**Note on `needs-accuracy-scout` and `accuracy-reviewed`:** These labels are reserved for the accuracy-scout agent (issue #2628) and are intentionally not listed here until that agent ships.

## Orchestrator Principles

- **Scale with pipeline leads, not with more direct workers.** At 10+ tasks,
  create a team with pipeline leads instead of tracking 30 agents yourself.
- **Route by label.** `swarm-discovered` → research-verifier. `research-reviewed` → plan-reviewer.
  `needs-plan-review` → plan-reviewer. `builder-ready` → builder.
  `in-review` → already being reviewed. `merge-ready` → ops.
  `structural-blocker` → escalate, do not route other work that depends on this.
- **Check `in-build` before spawning builders.** If an issue already has `in-build`, skip it.
- **Don't micromanage.** Workers have autonomy within their scope. Pipeline leads
  have autonomy within their pipeline stage. You set direction and monitor.
- **Parallel lanes.** Workers don't conflict because of worktree isolation.
  Pipeline leads don't conflict because they own different pipeline stages.
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
gh issue list --label in-build --state open --limit 10
gh run list --branch master --limit 3

# Advanced queries
# Issues with builder-ready but missing plan-reviewed (skipped plan-review)
gh issue list --label "builder-ready" --state open --json number,title,labels \
  | jq '.[] | select(.labels | map(.name) | contains(["plan-reviewed"]) | not) | "\(.number) \(.title)"'

# Health check
/health-check
```
