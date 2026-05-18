# perl-lsp Source-of-Truth Specification System

This guide defines how perl-lsp encodes long-lived work as linked source-of-truth
artifacts. Use it when creating or consuming proposal/spec/ADR/plan/goal lanes.
It is an authoring contract, not a status document.

For long-lived work, follow this path:

```text
Roadmap → Proposal → Specs → ADRs → Plan → Active goal → PRs → Receipts
```

The Real Perl Editor Trust lane is the reference implementation: its proposal,
four specs, two ADRs, implementation plan, status receipts, and active goal
manifest form one discoverable chain.

## Artifact Roles

| Layer | Owns | Storage | Must not own |
| --- | --- | --- | --- |
| Roadmap | Release direction and active milestone | `docs/project/ROADMAP.md` | PR-sized sequencing or generated metrics |
| Proposal / PRD | Why the lane exists, user value, alternatives, and claim boundary | `docs/proposals/PLSP-PROP-*.md` | Implementation order or generated status tables |
| Spec | Behavior contract, acceptance, proof requirements, status interpretation, and claim limits | `docs/specs/PLSP-SPEC-*.md` | Product motivation or PR order |
| ADR | Durable architecture or operating decision | `docs/adr/PLSP-ADR-*.md` | Raw worklists or point-in-time metric state |
| Implementation plan | PR-sized sequence, proof commands, rollback, and handoff state | `plans/<lane>/implementation-plan.md` | Product claims, behavior contracts, or durable decisions |
| Active goal manifest | Machine-readable current agent state | `.perl-lsp/goals/active.toml` | Prose-only strategy or generated status content |
| Status / support tiers | Current truth and public claim proof | `docs/project/status/*.md` | Lane motivation or implementation sequencing |
| Policy ledgers | CI, lint, file, package, and exception receipts | `policy/*.toml`, `.ci/**` | Narrative status or roadmap intent |
| Closeout / handoff | What happened, what remains, and proof | `plans/<lane>/closeout.md`, `docs/forensics/` | New durable policy without an ADR |

## ID Naming

Use `PLSP-*` IDs for lane artifacts that are part of the durable specification
system.

| Artifact | Pattern | Example |
| --- | --- | --- |
| Proposal | `PLSP-PROP-####-short-name.md` | `PLSP-PROP-0001-real-perl-editor-trust.md` |
| Spec | `PLSP-SPEC-####-short-name.md` | `PLSP-SPEC-0002-provider-confidence-receipts.md` |
| ADR | `PLSP-ADR-####-short-name.md` | `PLSP-ADR-0002-confidence-before-cutover.md` |
| Plan lane | `plans/<lane>/` | `plans/real-perl-editor-trust/` |
| Active goal | `.perl-lsp/goals/active.toml` | active lane manifest |

Numbers should be monotonic within the artifact family. Do not guess issue
numbers in artifact names. If an issue reference is needed in commits or PRs and
cannot be verified, use the repository's `#0000` placeholder convention.

## Required Headers

### Proposal Header

A proposal should start with these fields:

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

A proposal explains the user problem, affected users/surfaces, current evidence,
success criteria, alternatives, risks, and claim boundary.

### Spec Header

A spec should start with these fields:

```md
# PLSP-SPEC-####: Title

Status:
Owner:
Created:
Linked proposal:
Linked ADRs:
Linked plan:
Status surfaces:
Proof commands:
```

A spec defines the behavior contract, acceptance examples, required proof,
status interpretation, compatibility and fallback behavior, and claim limits.

### ADR Header

A PLSP ADR should start with these fields:

```md
# PLSP-ADR-####: Title

Status:
Date:
Linked proposal:
Linked specs:
Linked plan:
```

An ADR records context, decision, consequences, alternatives, and follow-up
links. It should be durable enough to guide future provider, parser, semantic,
workspace, or operations changes.

### Plan Header

A lane plan should identify the linked proposal/spec/ADR/status/goal artifacts
before the work queue:

```md
# Lane implementation plan

Status:
Lane:
Linked proposal:
Linked specs:
Linked ADRs:
Active goal:
Status sources:
```

Each work item should be PR-sized and include goal, production delta, non-goals,
acceptance, proof commands, and rollback.

### Active Goal Header

The active goal manifest is TOML. It should include stable lane metadata and
pointers before work items:

```toml
id = "plsp-lane-slug"
title = "Lane title"
status = "active"
owner = "codex-swarm"
created = "YYYY-MM-DD"

proposal = "docs/proposals/PLSP-PROP-####-short-name.md"
plan = "plans/<lane>/implementation-plan.md"
status_pointer = "docs/project/status/..."
```

Archive inactive manifests under `.perl-lsp/goals/archive/` when another lane
becomes the operative active goal.

## When to Create Each Artifact

- Create a proposal when the lane needs a durable explanation of why the work
  matters, who benefits, what alternatives were considered, and what claims are
  out of bounds.
- Create a spec when reviewers and agents need a behavior contract with
  acceptance examples, proof commands, fallback behavior, and claim limits.
- Create an ADR when a decision changes provider architecture, parser strategy,
  semantic model boundaries, release operations, CI policy, or another durable
  operating rule.
- Create or update a plan when the contract exists and the remaining work needs
  PR-sized sequencing, proof commands, rollback notes, and handoff state.
- Update the active goal manifest only when the lane is the current executable
  agent state. Do not use it as a general backlog.
- Add closeout or forensic notes when a lane closes, hands off, or leaves
  receipts that future maintainers must be able to audit.

## Linking Status Without Copying Generated Truth

Status docs and support tiers own current truth. Proposals, specs, ADRs, plans,
and active goals should link to those docs rather than copying generated tables
or point-in-time counts.

Preferred status links include:

- `docs/project/CURRENT_STATUS.md` for evidence-backed overall state
- `docs/project/ROADMAP.md` for canonical release direction
- `docs/project/status/SUPPORT_TIERS.md` for public support claims
- `docs/project/status/provider_confidence_matrix.md` for provider receipts
- `docs/project/status/semantic_scorecard.md` for semantic capability proof
- `docs/project/status/semantic_shadow_compare.md` for shadow comparison proof
- `docs/project/status/ux_capability_dashboard.md` for user-facing capability proof

If a generated metric appears stale or contradictory, refresh or verify it with
the owning command before making a claim. Do not hand-edit generated status
sections.

## Active Goals

Active goals are machine-readable manifests for the current lane. They should
let Codex, Claude, or another agent find the objective, current work item,
linked contracts, status pointers, and proof commands without chat history.

Use `.perl-lsp/goals/active.toml` for the operative lane and
`.perl-lsp/goals/archive/` for inactive manifests. The manifest should point to
proposals, specs, ADRs, plans, and status docs; it should not embed generated
metric tables or replace narrative design docs.

## PR Body Structure

Keep docs-system PRs focused. One semantic artifact per PR is preferred after
the source-of-truth scaffolding exists.

Use the repository PR body shape:

```text
Problem: <one sentence>
Fix: <one sentence>
Verification: `<command>` passes
```

For lane work, the body should mention the governing proposal/spec/ADR/plan when
that context helps reviewers, but it should not duplicate the artifact content.

## Agent Consumption Rules

When an agent starts long-lived lane work:

1. Read `docs/README.md` to find the source-of-truth stack.
2. Read the active goal manifest when the task says to continue the current
   lane.
3. Read the linked proposal for why and claim boundaries.
4. Read the linked specs for behavior and proof requirements.
5. Read linked ADRs for durable design or operating constraints.
6. Read the lane plan for PR order, proof commands, rollback, and handoff.
7. Read status docs and receipts for current truth; do not copy generated
   tables into new artifacts.
8. Keep each PR to one concern and cite the proof commands that define the
   review boundary.

For the source-of-truth stack itself, use these entry points:

- [proposals README](../proposals/README.md)
- [specs README](../specs/README.md)
- [ADR README](../adr/README.md)
- [plans README](../../plans/README.md)
- [active goals README](../../.perl-lsp/goals/README.md)
- [Real Perl Editor Trust plan](../../plans/real-perl-editor-trust/README.md)
