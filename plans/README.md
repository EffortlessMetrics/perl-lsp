# Implementation Plans

`plans/<lane>/` is where perl-lsp keeps implementation plans for long-lived
lanes. Plans translate proposals, specs, ADRs, generated status receipts, and
active goal manifests into reviewable PR sequences.

Plans are not specs. A plan owns the implementation path: PR order, proof
commands, rollback notes, closeout/handoff state, and links to the current truth
sources. Product motivation belongs in proposals, behavior contracts belong in
specs, durable decisions belong in ADRs, and generated metric truth belongs in
status docs.

## Source-of-Truth Flow

```text
Roadmap → Proposal → Specs → ADRs → Plan → Active goal → PRs → Receipts
```

| Artifact | Plan responsibility |
| --- | --- |
| Roadmap | Link the release direction or milestone that makes the lane relevant. |
| Proposal | Link the user problem, value, alternatives, and claim boundary. |
| Specs | Link behavior contracts and acceptance criteria the plan implements. |
| ADRs | Link durable decisions that constrain sequencing or architecture. |
| Status / receipts | Link generated status and proof files; do not copy their tables. |
| Active goal | Link the machine-readable current state when the lane is active. |
| Closeout | Record what landed, what remains, proof commands, rollback notes, and handoff. |

## Lane Directory Shape

A lane directory should normally contain:

```text
plans/<lane>/
  README.md
  implementation-plan.md
  closeout.md              # optional, once the lane closes or hands off
```

Use short, stable lane slugs such as `real-perl-editor-trust` or
`semantic-receiver-facts`. The lane README should explain the plan boundary and
link the governing proposal/spec/ADR/status artifacts. The implementation plan
should be a PR-sized work queue, not a broad design document.

## Work Item Shape

Each work item should make the next reviewable change obvious:

```md
## Work item: id

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

## Boundaries

- Do not use plans to redefine user value; update or add a proposal instead.
- Do not use plans to define behavior contracts; update or add a spec instead.
- Do not use plans to record durable architecture policy; update or add an ADR
  instead.
- Do not hand-edit generated status sections or copy generated metric tables;
  link status docs and receipts instead.
- Do not mix unrelated lanes in one plan PR.

## Current Lanes

- [Real Perl Editor Trust](real-perl-editor-trust/) translates
  `PLSP-PROP-0001`, `PLSP-SPEC-0001` through `PLSP-SPEC-0004`,
  `PLSP-ADR-0001`, `PLSP-ADR-0002`, status receipts, and the active goal
  manifest into PR-sized work.

See [docs/reference/SPEC_SYSTEM.md](../docs/reference/SPEC_SYSTEM.md) for the
full proposal/spec/ADR/plan/goal authoring contract.
