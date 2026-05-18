# Repo source-of-truth system

This repo uses a linked source-of-truth stack so contributors and agents can
find the right kind of truth without scraping chat history or treating stale
planning notes as current state.

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
| Roadmap | Release direction, milestone framing, lane names | PR queue, generated metrics, proof receipts |
| Proposal | Why, users, alternatives, risks, lane success criteria | Behavior contract, task list, generated status |
| Spec | Required behavior, acceptance, evidence, claim boundaries | Product rationale, PR sequencing, active queue |
| ADR | Durable architecture or operating decision | Task list, implementation queue, current metric state |
| Plan | PR order, work items, proof commands, rollback | Product rationale, durable decision record |
| Active goal | Current machine-readable objective and work items | Long prose, generated status tables, durable rationale |
| Support tiers | Public support claims and proof pointers | Feature design, implementation sequencing |
| Policy ledgers | Exceptions, owners, coverage, review dates | Broad architecture or product strategy |

## Rules

1. One kind of truth belongs in one artifact.
2. One semantic artifact or implementation work item belongs in one PR unless a
   plan explicitly says otherwise.
3. Specs define behavior; plans define sequencing.
4. Proposals explain why; ADRs record decisions.
5. Active goals tell agents what to execute now.
6. Generated status is updated by tools, not by hand.
7. Public claims require support-tier proof or an equivalent status pointer.
8. Policy exceptions require an owner, reason, coverage, and review date.

## Required headers

Every new proposal, spec, ADR, and implementation plan should declare these
fields when they apply. Use `n/a` when a field is intentionally not applicable.

```text
Status:
Owner:
Created:
Linked proposal:
Linked specs:
Linked ADRs:
Linked plan:
Linked issues:
Linked PRs:
Support-tier impact:
Policy impact:
```

Existing legacy artifacts may use older header names, but new source-of-truth
artifacts should converge on this shape.

## Agent workflow

Agents must:

1. Read repo instructions, then this document.
2. Read `.perl-lsp/goals/active.toml`.
3. Choose exactly one ready or assigned work item.
4. Read the linked plan item.
5. Read the linked spec for acceptance.
6. Read linked ADRs for constraints.
7. Implement only the selected work item.
8. Run the proof commands listed for that item.
9. Update receipts, status, or policy ledgers only when the work item requires it.
10. Stop rather than guessing when source-of-truth artifacts are missing or contradictory.

## Stop conditions

Stop and report instead of inventing work when:

- the active goal is missing or stale;
- linked files do not exist;
- generated status is dirty and no generator/check command is listed;
- proof commands cannot run;
- unrelated staged changes exist;
- requested work conflicts with an ADR;
- a public claim lacks support-tier proof or an equivalent status pointer.

## Active goal lifecycle

The active goal manifest lives at:

```text
.perl-lsp/goals/active.toml
```

Set `status = "active"` when the lane is executable. Set `status = "paused"`
with a reason when no current implementation lane is selected.

Archive replaced manifests under:

```text
.perl-lsp/goals/archive/YYYY-MM-DD-<lane>.toml
```

Do not leave multiple active manifests.

## Closeout format

At the end of a lane, add or update:

```text
plans/<lane>/closeout.md
```

A closeout should include:

- what shipped;
- proof commands and receipts;
- linked PRs and CI runs;
- generated status, support-tier, and policy updates;
- deferred work;
- claim boundaries;
- the next lane recommendation.

## Common failure modes

### Spec becomes a task list

Move PR order to `plans/<lane>/implementation-plan.md` and keep the spec focused
on behavior, examples, proof, and claim boundaries.

### Plan becomes product rationale

Move why-oriented text to the proposal. Keep the plan focused on work items,
proof, dependencies, and rollback.

### Active goal becomes prose

Keep `.perl-lsp/goals/active.toml` machine-readable and link to prose docs
instead of copying long generated tables or narrative state.

### Generated status is hand-edited

Run the named generator or check command. If a generator is unavailable, record
that as a blocker or explicit proof limitation.

### Support claims drift

Require a support-tier impact header or equivalent status pointer for public
claims, and link to proof commands or receipts.

### Policy exceptions become silent debt

Every exception in `policy/*.toml` needs an owner, reason, `covered_by`, and
`review_after`; add `expires` when the exception is temporary.

### Mega PR

Split unrelated source-of-truth artifacts or implementation work items into
separate PRs unless the implementation plan explicitly requires bundling them.

## What good looks like

A new contributor or agent can arrive cold and answer:

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

If the repo answers those questions without chat history, the source-of-truth
system is working.
