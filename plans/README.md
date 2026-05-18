# Plans

Plans are the sequencing layer for `perl-lsp` work. They translate accepted
proposals, specs, and ADRs into PR-sized work items with dependencies, proof
commands, rollback notes, and handoff state.

| Layer | Owns | Must not do |
|---|---|---|
| Plan | PR order, work items, dependencies, proof commands, rollback, handoff state | Product motivation, behavior contract, durable architecture decision, generated metric truth |

## How plans fit the stack

```text
Roadmap -> Proposal -> Spec -> ADR -> Plan -> Active goal -> PR -> Proof
```

Read plans after the active goal manifest points at a lane. A plan should tell an
agent what can land next, what it depends on, what proof commands define success,
and how to roll back the change. It should link back to the proposal for why, to
specs for acceptance, and to ADRs for durable constraints.

## Plan rules

- Keep one lane per `plans/<lane>/` directory.
- Keep the executable queue in `plans/<lane>/implementation-plan.md`.
- Keep product rationale in `docs/proposals/`, not in the plan.
- Keep behavior contracts in `docs/specs/`, not in the plan.
- Keep durable decisions in `docs/adr/`, not in the plan.
- Include proof commands and rollback notes for every work item.
- Use stable work-item anchors so `.perl-lsp/goals/active.toml` can point to the
  exact item being executed.

## Work item template

````md
## Work item: short-id

Status: ready | active | blocked | completed | superseded
Linked proposal:
Linked spec:
Linked ADR:
Blocks:
Blocked by:

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
cargo test ...
git diff --check
```

### Rollback

How to undo this PR safely.

### Notes

Optional.
````

## Current lanes

- [Real Perl editor trust](real-perl-editor-trust/) — active lane linked from
  `.perl-lsp/goals/active.toml`.

## Closeout

When a lane completes, add `plans/<lane>/closeout.md` with what shipped, proof,
receipts, deferred work, claim boundaries, and the next lane recommendation.
