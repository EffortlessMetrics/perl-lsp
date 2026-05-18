# Implementation Plans

`plans/<lane>/` is where `perl-lsp` implementation plans live. Plans translate
accepted proposals, specs, ADRs, generated receipts, and current status pointers
into reviewable PR sequences.

Plans are not specs. They do not own product motivation, durable architecture
decisions, generated metric state, or public support claims.

| Layer | Owns | Must not do |
| --- | --- | --- |
| Plan | PR order, work-item decomposition, proof commands, rollback notes, and handoff state | Product claims, durable decisions, behavior contracts, or generated status content |

## Source-of-Truth Chain

Long-lived work should remain connected through this stack:

```text
Roadmap → Proposal → Specs → ADRs → Plan → Active goal → PRs → Receipts
```

Use:

- [docs/project/ROADMAP.md](../docs/project/ROADMAP.md) for release direction
  and active milestone framing.
- [docs/proposals/](../docs/proposals/) for the user problem, alternatives,
  success criteria, and claim boundary.
- [docs/specs/](../docs/specs/) for behavior contracts, acceptance, proof, and
  claim limits.
- [docs/adr/](../docs/adr/) for durable architecture or operating decisions.
- `plans/<lane>/implementation-plan.md` for PR-sized sequencing, rollback, and
  handoff state.
- [.perl-lsp/goals/](../.perl-lsp/goals/) for the machine-readable current
  agent state.
- [docs/project/status/](../docs/project/status/) for generated current truth
  and public claim proof.

For the reusable authoring contract, see
[docs/reference/SPEC_SYSTEM.md](../docs/reference/SPEC_SYSTEM.md). For the lane
pattern in use today, see
[real-perl-editor-trust/README.md](real-perl-editor-trust/README.md).

## Plan Shape

Each lane should include a short `README.md` plus an `implementation-plan.md`.
When a lane has closeout or handoff state that no longer belongs in the active
plan, add `closeout.md` or link to a forensic note instead of editing generated
status docs.

Work items should include:

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

## Status Discipline

Plans may summarize intent, but current facts must come from the generated or
human-owned truth surfaces they link:

- [docs/project/CURRENT_STATUS.md](../docs/project/CURRENT_STATUS.md)
- [docs/project/ROADMAP.md](../docs/project/ROADMAP.md)
- [docs/project/status/](../docs/project/status/)
- policy ledgers under [policy/](../policy/) and enforcement receipts under
  [.ci/](../.ci/)

Do not copy generated tables, support-tier matrices, or point-in-time metrics
into plan files.
