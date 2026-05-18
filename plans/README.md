# Plans

Plans are the sequencing layer for `perl-lsp` work. They translate accepted
proposals, specs, and ADRs into reviewable PR-sized work items with proof
commands and rollback notes.

| Layer | Owns | Must not do |
|---|---|---|
| Plan | PR order, work items, dependencies, acceptance, proof commands, rollback, handoff | Product rationale, durable architecture decisions, generated status truth |

## Current Lanes

| Lane | Plan | Active goal |
|---|---|---|
| Real Perl editor trust | [implementation plan](real-perl-editor-trust/implementation-plan.md) | [`.perl-lsp/goals/active.toml`](../.perl-lsp/goals/active.toml) |

## Plan Contract

A plan should include:

- source-of-truth links to the proposal, specs, ADRs, and active goal;
- a factual current-state baseline that links to status docs instead of copying generated tables;
- one section per work item using a stable anchor;
- status for each work item (`ready`, `active`, `blocked`, `completed`, `superseded`, or `deferred`);
- production delta and non-goals;
- acceptance criteria;
- proof commands;
- rollback guidance.

Plans answer **what PR lands next**. If plan text starts explaining why the lane
exists, move that material to `docs/proposals/`. If it records a durable
architecture constraint, move that material to `docs/adr/`.

## Work Item Template

````md
## Work item: short-id

Status: ready
Linked proposal: docs/proposals/PLSP-PROP-####-lane.md
Linked spec: docs/specs/PLSP-SPEC-####-contract.md
Linked ADR: docs/adr/PLSP-ADR-####-decision.md
Blocks: n/a
Blocked by: n/a

### Goal

One paragraph.

### Production delta

What files, commands, APIs, workflows, or behavior change?

### Non-goals

What is explicitly out of scope?

### Acceptance

What must be true for the PR to merge?

### Proof commands

```bash
git diff --check
```

### Rollback

How to undo this PR safely.

### Notes

Optional.
````

## Closeout

When a lane ends, add `plans/<lane>/closeout.md` with shipped work, proof,
receipts, generated status updates, support-tier or policy changes, deferred
items, claim boundaries, and the next recommended lane.
