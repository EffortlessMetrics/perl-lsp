# Implementation Plans

Implementation plans are the queue layer of the `perl-lsp` source-of-truth
stack. They sequence focused PRs after a proposal explains why, specs define
what must be true, and ADRs record any durable decisions.

| Layer | Owns | Must not do |
|---|---|---|
| Plan | PR order, work items, dependencies, proof commands, rollback, status handoff | Product rationale, behavior contract, generated status truth |

## How Plans Fit

```text
Roadmap
  -> Proposal
    -> Spec
      -> ADR
        -> Implementation plan
          -> Active goal
            -> PR
              -> Proof
```

Plans should link back to their proposal, specs, and ADRs. The active goal at
`.perl-lsp/goals/active.toml` should point to the current plan and selected work
items.

## Directory Shape

Use one directory per lane:

```text
plans/
  <lane>/
    README.md
    implementation-plan.md
    closeout.md
```

`closeout.md` is added when the lane completes.

## Work Item Requirements

Each work item should include:

- stable ID that can be linked from `.perl-lsp/goals/active.toml`;
- status: `ready`, `active`, `blocked`, `completed`, or `superseded`;
- linked proposal, spec, and ADR when applicable;
- goal and production delta;
- non-goals and claim boundary;
- acceptance criteria;
- proof commands;
- rollback notes.

## Template

````md
# Lane implementation plan

Status: active
Owner:
Linked proposal:
Linked specs:
Linked ADRs:
Active goal:

## Current state

Link to status docs and receipts. Do not copy generated tables.

## Work item: short-id

Status: ready | active | blocked | completed | superseded
Linked proposal:
Linked spec:
Linked ADR:
Blocks:
Blocked by:

### Goal

### Production delta

### Non-goals

### Acceptance

### Proof commands

```bash
cargo test ...
git diff --check
```

### Rollback

### Notes
````

## Plan Hygiene

- Keep product motivation in proposals.
- Keep behavior contracts in specs.
- Keep durable decisions in ADRs.
- Keep generated metric state in generated status docs.
- Keep the active machine-readable objective in `.perl-lsp/goals/active.toml`.
- Do not turn a plan into a broad backlog; split unrelated lanes.
