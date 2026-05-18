# Repo source-of-truth system

This repo uses a linked source-of-truth stack so humans and agents can find the
right kind of truth without scraping chat history or treating every document as
a task list.

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
| Roadmap | Release direction, milestone framing, lane list | Detailed PR queue, generated metrics, proof receipts |
| Proposal | Why the lane exists, users, alternatives, success criteria | Behavior contract, PR sequence, generated status |
| Spec | Behavior, acceptance examples, proof requirements, claim boundaries | Product rationale, active queue, durable architecture decision |
| ADR | Durable architecture or operating decision, consequences, rejected alternatives | Task list, current metric state, implementation queue |
| Plan | PR order, work items, dependencies, proof commands, rollback | Product strategy, durable decision record, generated status truth |
| Active goal | Current machine-readable work, status pointers, proof commands, claim boundaries | Long prose, generated metrics, architecture rationale |
| Support tiers | Public claim proof, limitations, next promotion requirements | Feature design, PR sequencing |
| Policy ledgers | Exceptions, owners, coverage, review dates | Broad architecture, product rationale |

## Rules

1. One kind of truth belongs in one artifact.
2. One semantic artifact should land per PR unless the linked plan says otherwise.
3. Specs define behavior; plans define sequencing.
4. Proposals explain why; ADRs record durable decisions.
5. Active goals tell agents what to do now.
6. Generated status is updated by tools, not by hand.
7. Public claims require support-tier proof or an equivalent proof pointer.
8. Policy exceptions require an owner, reason, coverage, and review date.

## Canonical locations

| Question | Source of truth |
|---|---|
| Why are we doing this? | `docs/proposals/` |
| What must be true? | `docs/specs/` |
| What durable decision constrains the work? | `docs/adr/` |
| What PR lands next? | `plans/<lane>/implementation-plan.md` |
| What is active now? | `.perl-lsp/goals/active.toml` |
| What proves a public claim? | `docs/project/status/SUPPORT_TIERS.md` and linked receipts |
| What policy exceptions exist? | `policy/*.toml` |

## Required headers

New source-of-truth artifacts should include the applicable headers below. Use
`n/a` when a field does not apply.

### Proposals

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

### Specs

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

### ADRs

```text
Status:
Date:
Owner:
Linked proposal:
Linked specs:
Linked plan:
```

### Plans

```text
Status:
Owner:
Linked proposal:
Linked specs:
Linked ADRs:
Active goal:
```

## Agent workflow

Agents must:

1. Read `AGENTS.md` or `CLAUDE.md`.
2. Read this file.
3. Read `.perl-lsp/goals/active.toml`.
4. Read the linked implementation plan.
5. Read the linked proposal only for why.
6. Read the linked spec for acceptance and proof.
7. Read linked ADRs for constraints.
8. Inspect git status and recent commits.
9. Pick exactly one ready work item.
10. Implement only that work item.
11. Run the proof commands.
12. Update receipts, generated status, support tiers, or policy ledgers only when
    the work item requires it.
13. Commit one focused change and open one focused PR.

## Stop conditions

Stop and report instead of guessing when:

- the active goal is missing or stale;
- linked files do not exist;
- a linked spec or plan item is missing;
- proof commands cannot run and no substitute evidence is defined;
- generated status differs from committed status before the selected work starts;
- unrelated staged changes exist;
- requested work conflicts with an ADR;
- a public claim lacks support-tier proof.

## Active goal lifecycle

The active manifest lives at `.perl-lsp/goals/active.toml`.

Use:

```toml
status = "active"
```

for a selected lane. Use:

```toml
status = "paused"
reason = "No selected implementation lane."
```

when no work item should be executed.

When replacing a lane, archive the old manifest under
`.perl-lsp/goals/archive/YYYY-MM-DD-<lane>.toml` before writing the new active
manifest. Do not leave multiple active manifests.

## Closeout format

At the end of a lane, write `plans/<lane>/closeout.md` with:

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

Closeout prevents the next agent from rediscovering old work.

## Common failure modes

### Spec becomes a task list

Move PR order to `plans/<lane>/implementation-plan.md`; keep the spec focused on
behavior, examples, proof, and claim boundaries.

### Plan becomes product rationale

Move why to `docs/proposals/`; keep the plan focused on work items,
dependencies, proof commands, and rollback.

### Active goal becomes prose

Keep `.perl-lsp/goals/active.toml` machine-readable. Link to docs instead of
copying long generated tables or rationale.

### Generated status is hand-edited

Run the generator or checker named by the plan. Generated status is evidence,
not a narrative scratchpad.

### Support claims drift

Require support-tier impact on source-of-truth artifacts and link public claims
to `docs/project/status/SUPPORT_TIERS.md` or an equivalent receipt.

### Policy exceptions become silent debt

Every exception in `policy/*.toml` needs an owner, reason, `covered_by`, and
`review_after`; temporary exceptions should also include an expiry.

### Mega PR

Keep one semantic artifact or one implementation work item per PR unless the
plan explicitly authorizes a combined change.

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
