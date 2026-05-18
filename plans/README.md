# Implementation Plans

`plans/<lane>/` is where implementation plans live for long-lived `perl-lsp`
work lanes. Plans translate proposals, specs, ADRs, status receipts, and policy
constraints into a reviewable PR sequence.

Plans are not specs. A plan explains how to land the work safely; it does not
own the user problem, the durable behavior contract, or the architectural
decision.

| Layer | Owns | Must not do |
| --- | --- | --- |
| Plan | PR order, work-item decomposition, proof commands, rollback notes, issue/PR handoff state | Product motivation, durable behavior contracts, architecture decisions, generated metric content |

## Source-of-Truth Chain

For long-lived work, plans sit in this chain:

```text
Roadmap → Proposal → Specs → ADRs → Plan → Active goal → PRs → Receipts
```

Use [docs/reference/SPEC_SYSTEM.md](../docs/reference/SPEC_SYSTEM.md) for the
full operating guide. Use [real-perl-editor-trust/](real-perl-editor-trust/) as
the current lane example.

## What Belongs in `plans/<lane>/`

A lane plan directory should usually contain:

- `README.md` that names the lane scope, linked source-of-truth artifacts, and
  status pointers.
- `implementation-plan.md` that decomposes the lane into PR-sized work items.
- Optional `closeout.md` when a lane needs a handoff record after completion.

Implementation plans should include proof commands and rollback notes for each
work item. They should link to generated status docs and support-tier receipts
instead of copying generated tables or point-in-time metrics.

## Relationship to Active Goals

Plans describe the intended sequence. The active goal manifest at
[../.perl-lsp/goals/active.toml](../.perl-lsp/goals/active.toml) records the
current machine-readable lane state for agents. When a lane changes, update or
archive the active goal manifest in `.perl-lsp/goals/archive/` as part of the
lane-management PR; do not bury the active state only in prose.
