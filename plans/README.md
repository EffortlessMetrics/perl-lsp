# Plans

Plans are the sequencing layer for `perl-lsp` lanes. They translate accepted
proposals, specs, and ADRs into PR-sized work items with proof commands and
rollback notes.

| Layer | Owns | Must not do |
|---|---|---|
| Plan | PR sequence, work items, dependencies, proof commands, rollback, handoff state | Product motivation, durable architecture decisions, generated status truth |

## How Plans Fit the Stack

The repo source-of-truth stack is documented in
[`docs/reference/SPEC_SYSTEM.md`](../docs/reference/SPEC_SYSTEM.md):

```text
Roadmap -> Proposal -> Spec -> ADR -> Plan -> Active goal -> PR -> Proof
```

A plan should link back to the proposal that explains why, the specs that define
what must be true, and any ADRs that constrain implementation. It should also
link forward to `.perl-lsp/goals/active.toml` when a lane is active.

## Plan Layout

Lane plans live under:

```text
plans/<lane>/implementation-plan.md
```

Each lane directory may also include:

```text
plans/<lane>/README.md
plans/<lane>/closeout.md
```

Use `README.md` for lane-local orientation and `closeout.md` when a lane is
completed or superseded.

## Work Item Shape

Each work item should be small enough for one focused PR and include:

- status (`ready`, `active`, `blocked`, `completed`, or `superseded`);
- linked proposal, spec, ADR, and active goal item;
- blockers and dependencies;
- production delta;
- non-goals;
- acceptance criteria;
- proof commands;
- rollback notes;
- claim boundary.

## Template

````md
# Lane implementation plan

Status:
Owner:
Created:
Linked proposal:
Linked specs:
Linked ADRs:
Active goal:
Linked issues:
Linked PRs:
Support-tier impact:
Policy impact:

## Current state

Link to generated status docs and receipts instead of copying point-in-time
metrics.

## Work item: short-id

Status: ready | active | blocked | completed | superseded
Linked proposal:
Linked spec:
Linked ADR:
Active goal:
Blocks:
Blocked by:

### Goal

### Production delta

### Non-goals

### Acceptance

### Proof commands

```bash
git diff --check
```

### Rollback

### Claim boundary
````

## Plan Rules

- Do not put product strategy in plans; move it to a proposal.
- Do not record durable architecture choices in plans; move them to an ADR.
- Do not copy generated status tables; link to the generated status source.
- Do not mark work complete without proof commands or an explicit unavailable-proof note.
- Do not mix unrelated work items into one PR.
