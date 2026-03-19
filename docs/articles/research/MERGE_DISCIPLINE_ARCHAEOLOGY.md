# Merge Discipline Archaeology

This note traces how merge governance in this repository evolved from staged flow packs and explicit merge rituals into the current swarm-era control plane. The throughline is consistent: the repo kept tightening the boundary between code generation, review, readiness, and merge authority until those responsibilities became first-class surfaces.

## 1. Early governance: the orchestration guide

The first clear merge-discipline artifact is [`.claude/ORCHESTRATION_GUIDE.md`](../../.claude/ORCHESTRATION_GUIDE.md), first tracked on `2025-08-28` in `3341bebdb` (`feat: update agent documentation and add orchestration guide`).

That guide already treats merge as a governed pipeline, not a single act. It defines distinct roles for:

- `pr-initial-reviewer`
- `test-runner-analyzer`
- `context-scout`
- `pr-cleanup-agent`
- `pr-finalize-agent`
- `pr-merger`
- `pr-doc-finalize`

The structure is important. Even before the later swarm surfaces, merge discipline was already split into review, cleanup, finalization, merge, and docs follow-through. That is a governance model, not just a checklist.

## 2. The Q3 flow-pack era: issue to draft, draft to PR, PR to merge

The canonical Q3 swarm packs in [`.claude/agents4/`](../../.claude/agents4/) represent the clearest pre-command merge system. The key artifacts are:

- [issue-to-draft.md](../../.claude/agents4/issue-to-draft.md)
- [draft-to-pr.md](../../.claude/agents4/draft-to-pr.md)
- [pr-to-merge.md](../../.claude/agents4/pr-to-merge.md)

These files encode a staged lifecycle:

- issue to draft
- draft to PR
- PR to merge

The interesting part is not the stage names, but the constraints. The flow files already formalize evidence handling, check runs, ledgering, and merge finalization. They also separate generative work from integrative validation and review. That is the precursor to the current swarm model.

`agents4` is also the first place where merge discipline looks like a reusable operating system rather than an ad hoc process pack:

- `review/` holds the quality and correctness checks
- `integration/` holds the merge-gate logic
- `generative/` holds the production of candidate changes

This is the historical root of the later "queue and gate" mentality.

## 3. March 2026: merge control becomes a command surface

The decisive shift happens in the March 15-19, 2026 burst of swarm infrastructure work. The repository starts surfacing merge governance as commands, skills, hooks, and durable state.

The important first-tracked commands and support files are:

- [`.claude/commands/swarm.md`](../../.claude/commands/swarm.md), first tracked `2026-03-15` in `9cc2d3b9a` (`feat(swarm): continuous swarm infrastructure with agent teams (#1553)`)
- [`.claude/commands/green-merge.md`](../../.claude/commands/green-merge.md), first tracked `2026-03-15` in `9cc2d3b9a`
- [`.claude/commands/status-drift.md`](../../.claude/commands/status-drift.md), first tracked `2026-03-15` in `9cc2d3b9a`
- [`.claude/commands/swarm-report.md`](../../.claude/commands/swarm-report.md), first tracked `2026-03-15` in `9cc2d3b9a`
- [`.claude/commands/swarm-status.md`](../../.claude/commands/swarm-status.md), first tracked `2026-03-15` in `9cc2d3b9a`
- [`.claude/hooks/task-completed.sh`](../../.claude/hooks/task-completed.sh), first tracked `2026-03-15` in `9cc2d3b9a`
- [`.claude/hooks/teammate-idle.sh`](../../.claude/hooks/teammate-idle.sh), first tracked `2026-03-15` in `9cc2d3b9a`

That commit cluster turns merge discipline into a live control plane:

- `green-merge` paces batches so CI is not canceled by aggressive merge churn
- `swarm-status` and `swarm-report` expose queue state and receipts
- `status-drift` ties merge activity back to corpus and project-state hygiene
- hooks provide event-driven coordination instead of manual prompting

This is the point where merge governance stops being just a flow pack and becomes an operational system.

## 4. Review readiness and draft-first discipline

On `2026-03-16`, the repo sharpens the boundary between review and readiness:

- [`.claude/commands/review-pr.md`](../../.claude/commands/review-pr.md), first tracked in `125089ac9` (`feat(review): add /review-pr skill, enforce one-PR-per-agent pattern (#1696)`)
- [`.claude/commands/pr-ready.md`](../../.claude/commands/pr-ready.md), first tracked in `13e6a3ea1` (`feat(skills): default to draft PRs, add /pr-ready skill for post-review readiness (#1694)`)

These surfaces codify two important rules:

- A review pass is not a merge pass.
- A draft PR becomes ready only after review evidence exists.

That is a concrete maturity step from the older orchestration model. The repo is no longer depending on a human to remember which phase a change is in. The phase is encoded in the control surface itself.

## 5. Merge governance becomes stateful and observable

The next stage is observability and durable state. On `2026-03-17`, the repo adds:

- [`.claude/swarm-state/README.md`](../../.claude/swarm-state/README.md), first tracked in `d9aab31bc` (`docs(swarm): track durable findings with schema (#1741)`)
- validation of empty findings ledgers in `37ddcf56d` (`fix(swarm): validate empty findings ledgers (#1743)`)

`swarm-state` matters because it turns merge governance into something that can be resumed, queried, and audited. That is a different discipline than a one-shot flow file. It is also how the repo starts preserving institutional memory around blocked work, findings, and queue state.

The associated status surfaces, especially [`.claude/commands/swarm-status.md`](../../.claude/commands/swarm-status.md) and [`.claude/commands/swarm-report.md`](../../.claude/commands/swarm-report.md), show the same shift:

- read current PR and issue state from GitHub
- read local agent and worktree state
- emit receipts and summaries
- keep the queue legible enough for another run to pick up cleanly

## 6. Triage becomes a dedicated cleanup lane

By `2026-03-18`, merge discipline has a dedicated cleanup and disposition layer:

- [`.claude/skills/triage-prs/SKILL.md`](../../.claude/skills/triage-prs/SKILL.md), first tracked in `b978a895d` (`feat(skill): add /triage-prs for post-batch-tool cleanup (#1961)`)

This is the modern answer to backlog entropy. Instead of letting duplicates, stale drafts, or overlapping PRs accumulate, the swarm gets a specific triage lane to cluster and dispose of them.

That is a governance signal: the repo treats non-mergeable PRs as a normal operational category, not a failure of process.

## 7. What the evolution means

The merge-discipline arc is not "old manual process to new automation." It is more specific:

- The orchestration guide established role separation.
- `agents4` turned that separation into a flow pack with explicit stage boundaries.
- March 2026 turned those stages into commands, hooks, skills, and persistent state.
- Triage, readiness, drift, and batch pacing became separate responsibilities rather than one overloaded merge ritual.

The result is a swarm-era merge governance model with distinct lanes for:

- generation
- review
- readiness
- merge pacing
- cleanup and triage
- state and drift tracking

That separation is what makes the current system scalable without making it opaque.

## 8. Primary sources used

- [`.claude/ORCHESTRATION_GUIDE.md`](../../.claude/ORCHESTRATION_GUIDE.md)
- [`.claude/agents4/issue-to-draft.md`](../../.claude/agents4/issue-to-draft.md)
- [`.claude/agents4/draft-to-pr.md`](../../.claude/agents4/draft-to-pr.md)
- [`.claude/agents4/pr-to-merge.md`](../../.claude/agents4/pr-to-merge.md)
- [`.claude/commands/green-merge.md`](../../.claude/commands/green-merge.md)
- [`.claude/commands/review-pr.md`](../../.claude/commands/review-pr.md)
- [`.claude/commands/pr-ready.md`](../../.claude/commands/pr-ready.md)
- [`.claude/commands/swarm-status.md`](../../.claude/commands/swarm-status.md)
- [`.claude/commands/swarm-report.md`](../../.claude/commands/swarm-report.md)
- [`.claude/commands/status-drift.md`](../../.claude/commands/status-drift.md)
- [`.claude/skills/triage-prs/SKILL.md`](../../.claude/skills/triage-prs/SKILL.md)
- [`.claude/swarm-state/README.md`](../../.claude/swarm-state/README.md)
