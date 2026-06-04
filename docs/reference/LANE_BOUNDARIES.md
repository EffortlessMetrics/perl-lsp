# Lane boundaries

This repo coordinates multiple implementation agents working in **lanes**.
A lane is a scoped, named stream of work — its issues, its PRs, its
priority, and the agent that owns it. Lanes do not overlap, and lane
ownership does not roll up to "the swarm" as a whole. An agent acting on
behalf of one lane has **no authority over PRs that belong to another
lane**, regardless of how the orchestrator routes its own queue.

This document is the canonical lane model. It complements `AGENTS.md`
(which scopes a single agent's actions) and `CLAUDE.md` (which scopes the
orchestrator's pipeline). The "Lane scope" section of `AGENTS.md`
references this file.

## Identifying a lane

A lane is identified primarily by the `lane: <N>` label on its PRs and
tracking issues. Secondary signals:

- **Title or branch prefix** that names the lane (e.g.
  `claude/neovim-0.15.1-*` for the 0.15.1 Neovim latency lane).
- A **lane-defining doc** under `docs/development/*_ROLLOUT.md` or
  `docs/reference/*_RAIL.md` — these describe the lane's scope and exit
  criteria.
- A **maintainer comment on the PR** explicitly asserting the lane (e.g.
  "this is lane 6", "lane 3 owns this", "explicitly resumed for lane 6").

When two signals disagree, the **maintainer comment wins**, and an
explicit maintainer override always overrides any agent's automated
routing decision.

## Lane numbers

The lane numbering is a working coordination system. It is not a static
list — lanes open and close as work is queued. The current active lanes
are visible by filtering open PRs and issues on the `lane: <N>` label:

```bash
rtk gh pr list --search "label:\"lane: 6\""
rtk gh issue list --label "lane: 3"
```

A non-exhaustive map of recent lanes:

| Lane label | Theme |
|------------|-------|
| `lane: 3` | parser/LSP accuracy control-plane cleanup |
| `lane: 6` | 0.15.1 Neovim latency hardening (runtime tuning, syntax-only diagnostics, scheduler cancellation, e2e receipts) |

This table is **descriptive, not prescriptive** — it documents the lane
labels in active use so agents can recognise them, not enumerate every
lane that has ever existed. Treat unfamiliar `lane: N` labels as
authoritative even if not in the table.

## The non-overlap rule

A lane owns:

- Its open PRs (anything labeled `lane: <N>`).
- Its branches (typically `<agent>/<theme>-<slug>`).
- Its tracking issues.
- The merge order and rebase coordination among those PRs.

A lane **does not** own:

- PRs in other lanes, regardless of how stale, draft, or out-of-scope
  they look from the current lane's perspective.
- Branches owned by other lanes.
- Merge decisions for other lanes' PRs.

The non-overlap rule is one-line:

> **Your lane's burn-down does not include closing other lanes' PRs.**

If a PR's lane label is `lane: 6` and you are an agent assigned to
`lane: 3`, that PR is **out of your scope**. The fact that your lane is
in cleanup mode and prefers a quiet queue is not a justification for
closing other lanes' work — that decision belongs to those lanes and to
the maintainer.

## What an agent IS authorised to do in another lane's PR

- **Read** the diff, comments, CI status, and review activity.
- **Comment** with focused, on-point feedback (correctness bugs you
  noticed; pointers to overlapping work in your lane that the author
  should know about).
- **Reply** to an `@<your-agent>` mention or a maintainer's direct
  question.

## What an agent is NOT authorised to do in another lane's PR

- **Close** the PR.
- **Force-push** to its branch.
- **Resolve** or **request changes** as if the PR were inside your lane.
- **Add `needs-*` routing labels** that imply your pipeline owns
  routing.
- **Open a competing PR** for the same work.

These are blocking violations. Each one is grounds for the maintainer to
revert your action and re-scope you back inside your lane.

## When a lane is genuinely paused

A maintainer may explicitly **pause** a lane — for example, "freeze
runtime work until cleanup burn-down lands". In that case:

- The pause is announced by the **maintainer**, on the lane's tracking
  issue or as a top-level comment on the lane's open PRs.
- The pause is enforced by **not opening new work in that lane**.
- The pause is **not** enforced by closing the lane's existing in-flight
  PRs from outside the lane.

If you believe another lane is creating risk for yours (e.g. a runtime
PR is about to land that will conflict with your cleanup work), the
correct action is:

1. **Comment** on the conflicting PR with the specific, observable risk.
2. **Tag the maintainer** for arbitration.
3. **Wait for the maintainer** to confirm the pause / lane re-routing.

Do not act unilaterally. The maintainer arbitrates lane priority.

## Override semantics

A direct maintainer instruction overrides any standing lane heuristic.
Common override forms:

- "Explicitly resumed for lane <N>" — the named lane is active, ignore
  the previous pause posture.
- "This is not your lane. Stop closing." — you have crossed a boundary;
  back out.
- "Direct maintainer permission" — the action is authorised even if it
  contradicts your default pipeline behaviour.

When you receive an override, **back out of the action you were about to
take** and treat the override as the next instruction in your queue.

## Cross-lane communication

Use PR comments. Tag specifically:

- `@<other-lane's agent>` when you have feedback the other lane should
  see.
- `@<maintainer>` when you need arbitration.

Avoid:

- "The swarm" framing for unilateral cross-lane action. The swarm is
  a coordination layer, not an entity that owns other lanes' decisions.
- "Cleanup" or "burn-down" framing as a justification for closing
  another lane's PR. Cleanup means *finishing your own lane*, not
  pruning other lanes.

## See also

- `AGENTS.md` — the "Lane scope" section references this doc.
- `CLAUDE.md` — orchestrator routing model; the lane label is one of the
  state labels the orchestrator reads.
- `docs/reference/ISSUE_DISCOVERY_DOCTRINE.md` — the Issue Discovery / Bug
  Scout Desk lane: the swarm's radar, upstream of plan review.
- `docs/articles/CONTINUOUS_REVIEW_PATTERNS.md` — the
  orchestration pattern overview.
