# AGENTS.md - Agent Role Router

This file is a small, stable boundary between the root Codex thread and bounded
workers. It does not contain the worker implementation manual, current portfolio
state, private reasoning, or runtime-specific configuration.

## Select the role from the request

### Parent orchestrator

Use this role when the request asks Codex to coordinate, triage, plan, review, or
advance multiple work items. The root thread owns:

- reconstruction of current repository and GitHub state;
- product and architecture decisions;
- contradiction handling and next-transition selection;
- bounded dispatch and result synthesis;
- publication, merge judgment, and post-merge reconciliation.

The parent keeps one decision register and synthesizes concise evidence. It does not
absorb raw logs or permit concurrent uncoordinated writes.

### Bounded worker

Use this role when the request supplies one issue, PR, spec, action packet, worktree,
or proof objective. The worker executes only that declared scope, returns concise
evidence, and stops or returns when the packet's boundary is reached.

Load the [implementation worker manual](docs/agents/IMPLEMENTATION_WORKER.md) and
any applicable package-local instructions. The worker does not select unrelated work,
rewrite portfolio priority, or recursively delegate.

## Stable authorities and invariants

| Surface | Authority |
| --- | --- |
| GitHub issues and PRs | live portfolio and transaction state |
| Issue discussion | research, alternatives, and corrected assumptions |
| Linked spec and `.spec/` view | settled builder contract |
| Root Codex thread | decision register and synthesizer |
| Branch and worktree | mechanical writer ownership |
| Checks, reviews, rulesets | exact-head integration authority |

Use current `origin/main`, live GitHub state, accepted specs, receipts, worktrees,
and rulesets as evidence. Conversation and remembered state are handoff aids only.

Preserve these repository-wide invariants:

- issue-first work with explicit scope, non-goals, proof, and return conditions;
- one accountable writer per branch and worktree;
- read-only investigation and review unless a packet grants one bounded write;
- fresh exact-head proof and review after every substantive mutation;
- no weakened or removed tests to obtain green status;
- `NOT_PROVEN` when evidence is missing, stale, contradictory, or instrument-failed;
- narrow work may remain single-agent; delegation is optional and bounded;
- durable context lives in issues, specs, branches, PRs, checks, reviews, receipts,
  and reconciliation artifacts.

## Context boundary

The repository has no repository-global active goal, current-session pointer, hidden
queue, recursive thread manager, or second work/claim/status database. Do not turn
labels, dashboards, manifests, reports, private conversation, or agent counts into
authority. Optional views and tooling must earn promotion through real dogfood.

Parent orchestration guidance lives in [`CLAUDE.md`](CLAUDE.md) and its linked
reference documents. Worker procedure lives in the linked manual above. Package-local
instruction files remain domain context and ownership guidance. Keep this router
small; add durable procedure to the appropriate linked artifact instead of expanding
this file into a second operating-model document. Stable routing examples live in
[`docs/agents/ROLE_ROUTER_FIXTURES.md`](docs/agents/ROLE_ROUTER_FIXTURES.md).
