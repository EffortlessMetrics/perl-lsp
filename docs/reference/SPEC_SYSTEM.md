# Repo source-of-truth system

This repo uses a linked source-of-truth stack so humans and agents can find the
right kind of truth in the right artifact instead of reconstructing context from
chat history or stale status notes.

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
| Roadmap | Release direction, milestone framing, lane inventory | Detailed PR queue, live generated metrics, proof receipts |
| Proposal | Why the lane exists, affected users and surfaces, alternatives, risks, success criteria | Behavior contract, implementation sequence, current generated status |
| Spec | Required behavior, acceptance examples, proof requirements, claim boundaries | Product rationale, active queue, PR order |
| ADR | Durable architecture or operating decision, context, consequences, rejected alternatives | Task list, current metric state, implementation queue |
| Plan | PR order, work items, dependencies, proof commands, rollback, handoff | Product motivation, durable architecture, generated status truth |
| Active goal | Current machine-readable objective, active work items, proof commands, status pointers | Long prose, generated metrics, durable design rationale |
| Support tiers | Public support claims, evidence, limitations, next promotion proof | Feature design, roadmap, implementation sequencing |
| Policy ledgers | Exceptions, CI/policy intent, owner, reason, coverage, review/expiry | Broad architecture, product rationale |

## Rules

1. One kind of truth belongs in one artifact.
2. One semantic artifact or one implementation work item belongs in one PR unless a plan explicitly says otherwise.
3. Specs define behavior; plans define sequencing.
4. Proposals explain why; ADRs record durable decisions.
5. Active goals tell agents what to do now.
6. Generated status is updated by tools, not by hand.
7. Public claims require support-tier proof or an equivalent evidence pointer.
8. Policy exceptions require owner, reason, coverage, and review date.

## Required headers

Every new proposal, spec, ADR, and plan should include the applicable source-of-truth links. Use `n/a` when a link is not applicable.

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

Existing legacy artifacts may use older headers. When touching them for lane work,
prefer moving them toward this shape instead of duplicating their truth elsewhere.

## Agent workflow

Agents must:

1. Read repo instructions (`AGENTS.md` for implementation agents; `CLAUDE.md` for the orchestrator).
2. Read this file.
3. Read `.perl-lsp/goals/active.toml`.
4. Read the linked implementation plan.
5. Read the linked proposal only for why.
6. Read the linked spec for acceptance.
7. Read linked ADRs for constraints.
8. Inspect git status and recent commits.
9. Select exactly one ready or active work item.
10. Implement only that item.
11. Run the listed proof commands or record why a command is unavailable.
12. Update status, receipts, or policy ledgers only when the selected work item requires it.
13. Commit one focused change and open one focused PR.

## Stop conditions

Stop and report instead of guessing when:

- the active goal is missing or stale;
- linked proposal, spec, ADR, or plan files do not exist;
- the selected work item is missing or contradictory;
- proof commands cannot run and no substitute evidence is defined;
- generated status differs from committed status before the work starts;
- unrelated staged changes exist;
- requested work conflicts with an ADR;
- a public claim lacks support-tier proof.

## Active goal lifecycle

The active manifest lives at:

```text
.perl-lsp/goals/active.toml
```

Use `status = "active"` for a current execution lane. Use `status = "paused"`
with a reason when no lane is selected. Archive replaced goals under:

```text
.perl-lsp/goals/archive/YYYY-MM-DD-<lane>.toml
```

Do not leave multiple active manifests.

## Closeout format

At the end of a lane, add or update:

```text
plans/<lane>/closeout.md
```

A closeout should record what shipped, proof commands, receipts, PRs, generated
status updates, support-tier or policy updates, deferred work, claim boundaries,
and the next lane recommendation.

## Common failure modes

### Spec becomes a task list

Move PR order to `plans/<lane>/implementation-plan.md`; keep the spec focused on
behavior, examples, proof, and claim boundaries.

### Plan becomes product rationale

Move why and alternatives to the proposal; keep the plan focused on work items,
proof commands, dependencies, and rollback.

### Active goal becomes prose

Keep `.perl-lsp/goals/active.toml` machine-readable and link to prose artifacts.
Do not copy generated tables into the manifest.

### Agent hand-edits generated status

Run the generator or checker named by the plan. If the command cannot run, record
that explicitly instead of editing generated output by hand.

### Support claims drift

Add or update support-tier evidence before broadening README, release, or user
claims.

### Policy exceptions become silent debt

Every policy exception needs owner, reason, `covered_by`, and `review_after`; add
an expiry for temporary exceptions.

### Mega PR

Split by semantic artifact or by one implementation work item. Do not bundle
proposal, spec, ADR, plan, active goal, runtime changes, and policy updates unless
the selected plan item explicitly requires that shape.

## What good looks like

A contributor or agent can arrive cold and answer:

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
