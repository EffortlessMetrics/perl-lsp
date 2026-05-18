# perl-lsp Source-of-Truth and Specification System

This guide defines how `perl-lsp` encodes long-lived work across proposals,
specs, ADRs, implementation plans, active goals, status docs, and receipts. It
is an authoring contract for maintainers and agents; it is not a generated
status report.

## The Stack

Use this chain for durable lanes:

```text
Roadmap → Proposal → Specs → ADRs → Plan → Active goal → PRs → Receipts
```

| Layer | Owns | Storage | Must not do |
| --- | --- | --- | --- |
| Roadmap | release direction and active milestone | `docs/project/ROADMAP.md` | Detailed PR sequencing or generated metrics |
| Proposal / PRD | why the lane exists, user value, alternatives, success criteria, claim boundary | `docs/proposals/PLSP-PROP-*.md` | Behavior minutiae, proof command ownership, generated status state |
| Spec | behavior contract, acceptance, proof requirements, status interpretation, claim limits | `docs/specs/PLSP-SPEC-*.md` | Product motivation, roadmap framing, active queue ownership |
| ADR | durable architecture or operating decision | `docs/adr/PLSP-ADR-*.md` | Raw worklists, point-in-time metrics, temporary task tracking |
| Implementation plan | PR-sized sequence, proof commands, rollback, handoff state | `plans/<lane>/implementation-plan.md` | Product claims, durable decisions, generated status content |
| Active goal manifest | machine-readable current agent state | `.perl-lsp/goals/active.toml` | Prose-only strategy or duplicated generated truth |
| Status / support tiers | current truth and public claim proof | `docs/project/status/*.md` | Roadmap promises or implementation-order decisions |
| Policy ledgers | CI, lint, file, package, and exception receipts | `policy/*.toml`, `.ci/**` | Narrative rationale without an enforcing surface |
| Closeout / handoff | what happened, what remains, proof, and deferred risks | `plans/<lane>/closeout.md` or `docs/forensics/` | New product scope or generated status rewrites |

The Real Perl Editor Trust lane demonstrates this model through
`docs/proposals/PLSP-PROP-0001-real-perl-editor-trust.md`, the matching
`PLSP-SPEC-*` files, `PLSP-ADR-*` records, the
`plans/real-perl-editor-trust/` plan, and `.perl-lsp/goals/active.toml`.

## ID Naming

Use stable `PLSP-*` identifiers for lane-specific source-of-truth artifacts:

- Proposals: `PLSP-PROP-####-short-name.md`
- Specs: `PLSP-SPEC-####-short-name.md`
- ADRs: `PLSP-ADR-####-short-name.md`
- Plans: `plans/<lane>/README.md` and `plans/<lane>/implementation-plan.md`
- Active goal: `.perl-lsp/goals/active.toml`
- Archived goals: `.perl-lsp/goals/archive/YYYY-MM-DD-<lane>.toml`

Use the next available number in the relevant namespace. Do not guess issue
numbers in filenames or PR titles; use `#0000` when the issue tracker is not
available.

## Required Headers

Proposals should begin with:

```md
# PLSP-PROP-####: Title

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

Specs should begin with:

```md
# PLSP-SPEC-####: Title

Status:
Owner:
Linked proposal:
Linked ADRs:
Linked plan:
Status impact:
```

ADRs should begin with the local ADR status fields used in `docs/adr/` and must
include context, decision, considered options when relevant, and consequences.
For `PLSP-ADR-*` files, link back to the proposal, specs, plan, and status
surfaces affected by the decision.

Plans should identify lane status, linked proposal, linked specs, linked ADRs,
current work items, proof commands, rollback notes, and handoff or closeout
state. Active goals should follow the TOML shape documented in
`.perl-lsp/goals/README.md`.

## When to Create Each Artifact

Create a proposal when the work changes product direction, creates a new lane,
or ties multiple subsystems to one user-facing outcome. The proposal answers
"why this lane exists" and defines the claim boundary.

Create a spec when future PRs need a durable behavior contract. The spec answers
"what must be true" through acceptance criteria, proof requirements, valid and
invalid PR shapes, and claim limits.

Create an ADR when the project makes a durable architecture or operating
decision. The ADR answers "which option did we choose and why" so future work
can avoid re-litigating the decision.

Create or update a plan when the lane needs reviewable PR sequencing, rollback
notes, or handoff state. The plan answers "how do we land this safely" and
should be broken into small work items.

Update the active goal only when the operative lane or current machine-readable
agent state changes. Archive the previous manifest when it is no longer the
current lane instead of leaving stale active state in place.

## Status and Generated Truth

Generated and evidence-backed status docs are the current truth. Source-of-truth
artifacts should link to them rather than copying their tables, counts, or
support-tier matrices.

Preferred status pointers include:

- `docs/project/CURRENT_STATUS.md`
- `docs/project/ROADMAP.md`
- `docs/project/status/SUPPORT_TIERS.md`
- `docs/project/status/provider_confidence_matrix.md`
- `docs/project/status/semantic_scorecard.md`
- `docs/project/status/semantic_shadow_compare.md`
- `docs/project/status/ux_capability_dashboard.md`
- `features.toml`
- `policy/*.toml` and `.ci/**` enforcement receipts

If a metric appears outside the generated or evidence-backed source, treat it as
stale until reverified.

## Active Goals

`.perl-lsp/goals/active.toml` is for current execution state that agents can
consume without chat history. It should contain the lane ID, objective, active
work items, source-of-truth links, status pointers, and proof commands.

Active goals are not planning essays. Keep strategy and rationale in proposals,
specs, ADRs, and plans; keep machine-readable current state in the manifest.
When a lane closes or another lane becomes operative, move the prior manifest to
`.perl-lsp/goals/archive/` with a dated filename and write a new active manifest.

## PR Bodies and Review Boundaries

Keep documentation PRs focused on one artifact type or one source-of-truth
system concern. Use the project PR body shape:

```text
Problem: <one sentence>
Fix: <one sentence>
Verification: `<command>` passes
```

For docs-only source-of-truth PRs, `git diff --check` is the minimum validation.
Run existing docs or status checks when the changed files participate in those
checks. Do not implement semantic behavior, type inference, provider changes, or
status regeneration in a PR whose scope is only proposal/spec/ADR/plan cleanup.

## Agent Consumption Rules

Implementation agents should start with the narrowest artifact that matches the
assigned work:

1. Read the roadmap only for milestone framing.
2. Read the proposal for user value, alternatives, and non-goals.
3. Read the relevant specs for behavior, acceptance, proof, and claim limits.
4. Read ADRs for durable architecture or operating constraints.
5. Read the plan for the next PR-sized step, rollback, and handoff state.
6. Read the active goal for current machine-readable execution state.
7. Verify current metrics and receipts against status docs instead of trusting
   copied numbers in narrative docs.

Do not invent a new structure when an existing `PLSP-*` artifact should be
extended or linked. Do not jam a new lane into an older proposal when the work
has a distinct user outcome, semantic contract, or architecture decision.
