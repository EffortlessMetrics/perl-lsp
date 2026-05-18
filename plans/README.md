# Implementation Plans

`plans/<lane>/` is where implementation plans live. Plans translate proposals,
specs, ADRs, and current receipts into a reviewable PR sequence that agents can
execute without relying on chat history.

Plans are not specs. Specs define behavior contracts, acceptance, proof, and
claim boundaries. Plans define how to land those contracts: PR order, work-item
scope, proof commands, rollback notes, and handoff state.

## Source-of-Truth Position

Long-lived work should move through the repository in this order:

```text
Roadmap → Proposal → Specs → ADRs → Plan → Active goal → PRs → Receipts
```

The plan layer is responsible for connecting durable source-of-truth artifacts to
short-lived implementation work. It should link to status docs and receipts, not
copy generated metric tables or support-tier state.

| Layer | Owns | Must not do |
| --- | --- | --- |
| Plan | PR order, work-item decomposition, proof commands, rollback notes, issue/PR handoff state | Product motivation, durable architecture decisions, generated metric content |

## Lane Directory Shape

Use one directory per lane:

```text
plans/<lane>/
  README.md
  implementation-plan.md
  closeout.md          # optional, when the lane is completed or handed off
```

A lane `README.md` should state the lane scope and link to the proposal, specs,
ADRs, status docs, and active goal manifest. `implementation-plan.md` should hold
PR-sized work items with proof commands and rollback notes.

## Work Item Shape

Each implementation-plan work item should include:

```md
## Work item: short-id

Status:
Linked proposal:
Linked spec:
Linked ADR:
Blocks:
Blocked by:

Goal
Production delta
Non-goals
Acceptance
Proof commands
Rollback
```

## Current Lanes

- [Real Perl Editor Trust](real-perl-editor-trust/) demonstrates the current
  source-of-truth stack with linked proposal, specs, ADRs, plan, status receipts,
  and active goal manifest.

## Related Source-of-Truth Docs

- [docs/README.md](../docs/README.md) — documentation front door and stack map
- [docs/reference/SPEC_SYSTEM.md](../docs/reference/SPEC_SYSTEM.md) — reusable
  proposal/spec/ADR/plan/goal operating guide
- [docs/proposals/README.md](../docs/proposals/README.md) — proposal ownership
- [docs/specs/README.md](../docs/specs/README.md) — spec ownership
- [docs/adr/README.md](../docs/adr/README.md) — ADR ownership
- [.perl-lsp/goals/README.md](../.perl-lsp/goals/README.md) — active goal
  manifest contract
