# Repo source-of-truth system

This repo uses a linked source-of-truth stack so humans and agents can tell
which artifact owns each kind of truth.

## Stack

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

## Artifact roles

| Artifact | Owns | Does not own |
|---|---|---|
| Roadmap | Release direction, milestone framing, lane inventory | PR queue, proof receipts, generated metrics |
| Proposal | Why the lane exists, users, affected surfaces, alternatives, risks | Behavior contract, PR sequence, current metric state |
| Spec | Required behavior, acceptance examples, proof requirements, claim boundaries | Product rationale, active queue, durable architecture decision |
| ADR | Durable architecture or operating decision, context, consequences | Task list, current metric state, implementation queue |
| Plan | PR sequence, work items, dependencies, proof commands, rollback | Product strategy, generated status truth |
| Active goal | Current machine-readable objective, active work items, proof commands, status pointers | Long prose, generated metrics, durable decisions |
| Support tiers | Public support claims, proof pointers, limitations, promotion requirements | Feature design, roadmap strategy |
| Policy ledgers | Exceptions, owners, coverage, review dates, expiry | Broad architecture, undocumented debt |

## Rules

1. One kind of truth per artifact.
2. One semantic artifact or one implementation work item per PR unless the plan says otherwise.
3. Proposals explain why; specs define behavior; ADRs record durable decisions.
4. Plans define sequencing and proof; active goals tell agents what to do now.
5. Generated status is updated by tools, not by hand.
6. Public claims require support-tier proof or an equivalent proof pointer.
7. Policy exceptions require an owner, reason, coverage, and review date.
8. Claim boundaries must be explicit when proof is partial or advisory.

## Required headers

Use `n/a` when a header does not apply. Existing legacy artifacts may predate
this system, but new source-of-truth artifacts should include the relevant
headers for their layer.

### Proposal headers

```text
Status:
Owner:
Created:
Target milestone:
Linked specs:
Linked ADRs:
Linked plan:
Support/status impact:
Policy impact:
```

### Spec headers

```text
Status:
Owner:
Created:
Linked proposal:
Linked ADRs:
Linked plan:
Linked issues:
Linked PRs:
Support-tier impact:
Policy impact:
```

### ADR headers

```text
Status:
Date:
Owner:
Linked proposal:
Linked specs:
Linked plan:
```

### Plan headers

```text
Status:
Owner:
Linked proposal:
Linked specs:
Linked ADRs:
Active goal:
```

## Agent workflow

Agents must follow this boot order before changing files:

1. Read `AGENTS.md` or `CLAUDE.md`.
2. Read this file.
3. Read `.perl-lsp/goals/active.toml`.
4. Read the linked implementation plan.
5. Read the linked proposal only for lane rationale.
6. Read the linked spec for acceptance and proof.
7. Read linked ADRs for constraints.
8. Inspect the current git state and recent commits.
9. Pick exactly one ready work item.
10. Implement only that work item.
11. Run the proof commands listed by the plan or active goal.
12. Update receipts, status, or policy only when the selected work item requires it.

If no ready work item is identifiable, stop and report instead of inventing one.

## Stop conditions

Stop and report when any of these are true:

- the active goal is missing, stale, or contradictory;
- a linked proposal, spec, ADR, or plan is missing;
- the requested work conflicts with an accepted ADR;
- the work item lacks proof commands;
- proof commands cannot run and no substitute evidence is defined;
- generated status is dirty and the plan does not direct updating it;
- unrelated staged changes are present;
- a public claim lacks a support-tier row or equivalent proof pointer;
- a policy exception lacks owner, reason, coverage, or review date.

## Active goal lifecycle

The active goal lives at `.perl-lsp/goals/active.toml`.

```toml
status = "active"
```

Use `status = "paused"` with a reason when there is no selected implementation
lane. Archive replaced manifests under `.perl-lsp/goals/archive/` with a dated
filename, then create the new active manifest. Do not leave multiple active
manifests.

## Closeout format

When a lane ends, add `plans/<lane>/closeout.md` with:

```md
# Lane closeout: <lane>

Status: completed
Date:
Owner:
Linked proposal:
Linked specs:
Linked ADRs:
Linked plan:
Active goal archive:

## What shipped

## Proof

## Receipts

## What did not ship

## Deferred work

## Claim boundary

## Next lane recommendation
```

Closeout prevents the next human or agent from rediscovering completed work.

## Common failure modes

### Spec becomes a task list

Move PR order to `plans/<lane>/implementation-plan.md`; keep the spec focused
on behavior, acceptance examples, and proof.

### Plan becomes product rationale

Move lane motivation to `docs/proposals/`; keep the plan focused on work items,
dependencies, proof commands, and rollback.

### Active goal becomes prose

Keep the manifest as TOML with stable IDs and links. Put narrative in proposals,
specs, plans, or handoffs.

### Generated status is hand-edited

Run the named generator or checker. If no generator exists, record the gap in
the plan instead of editing generated sections by hand.

### Support claims drift

Require support-tier impact on source-of-truth artifacts and keep public claims
linked to proof commands, known limitations, and next promotion proof.

### Policy exceptions become silent debt

Every exception belongs in `policy/*.toml` with owner, reason, `covered_by`, and
`review_after`; temporary exceptions should also include expiry.

### Mega PR

Split proposal, spec, ADR, plan, active-goal, runtime, policy, and support-tier
changes unless the selected plan item explicitly says to combine them.

## What good looks like

A contributor or agent should be able to answer these questions without chat
history:

```text
What are we doing?
Why?
What must be true?
What decision constrains it?
What PR lands next?
What command proves it?
What may we claim?
What must we not claim?
```

If the repo answers those questions through stable IDs, explicit links, small
work items, proof commands, claim boundaries, and stop conditions, the source-of-
truth system is working.
