# perl-lsp Source-of-Truth System

This guide defines how `perl-lsp` authors and consumes long-lived work. It is a
contract for source-of-truth artifacts, not a project status page. Use it when a
change needs a lane that outlives one pull request or affects public claims,
provider behavior, parser/semantic contracts, architecture, release direction, or
agent handoff.

The stack is:

```text
Roadmap → Proposal → Specs → ADRs → Plan → Active goal → PRs → Receipts
```

Each layer has one job. Do not duplicate generated status tables, metric counts,
or support-tier state across layers; link to the generated status surface instead.

## Artifact Roles

| Layer | Owns | Storage | Must not do |
| --- | --- | --- | --- |
| Roadmap | Release direction and active milestone | `docs/project/ROADMAP.md` | Replace lane proposals, specs, or plans |
| Proposal / PRD | User problem, success criteria, alternatives, risks, and claim boundary | `docs/proposals/PLSP-PROP-*.md` | Define PR order or copy generated metrics |
| Spec | Behavior contract, acceptance examples, proof requirements, status interpretation, and claim limits | `docs/specs/PLSP-SPEC-*.md` | Justify product motivation or sequence implementation PRs |
| ADR | Durable architecture or operating decision with consequences | `docs/adr/PLSP-ADR-*.md` | Become a raw worklist or point-in-time metric report |
| Implementation plan | PR-sized sequence, proof commands, rollback, dependencies, and handoff state | `plans/<lane>/implementation-plan.md` | Redefine behavior contracts or architecture decisions |
| Active goal manifest | Machine-readable current lane state, active work items, pointers, and proof commands | `.perl-lsp/goals/active.toml` | Store prose-only strategy or generated status content |
| Status / support tiers | Current truth and evidence-backed claim proof | `docs/project/status/*.md` | Carry durable design rationale or implementation sequencing |
| Policy ledgers | CI, lint, file/package exceptions, and enforcement receipts | `policy/*.toml`, `.ci/**` | Replace specs or status docs |
| Closeout / handoff | What happened, what remains, and proof | `plans/<lane>/closeout.md` or `docs/forensics/` | Reopen the lane's source-of-truth contracts |

## ID Naming

Use `PLSP-*` IDs for the lane source-of-truth stack:

- Proposals: `PLSP-PROP-####-short-name.md`
- Specs: `PLSP-SPEC-####-short-name.md`
- ADRs: `PLSP-ADR-####-short-name.md`
- Plan directories: `plans/<kebab-case-lane>/`
- Active goal IDs: `plsp-<kebab-case-lane>` in `.perl-lsp/goals/active.toml`

Choose the next available number within the artifact family. Do not guess issue
numbers in titles or file names; use `#0000` in PR titles when the issue tracker
is unavailable.

## Required Headers

New PLSP source-of-truth artifacts should start with enough metadata for agents
and reviewers to route them without reading the full file.

### Proposal header

```md
# PLSP-PROP-####: Title

Status: Proposed | Accepted | Superseded | Completed
Owner: lane or team name
Created: YYYY-MM-DD
Related specs:
Related ADRs:
Related plan:
Current status:
```

A proposal must define why the lane exists, who benefits, success criteria,
non-goals, alternatives considered, risks, and the claim boundary.

### Spec header

```md
# PLSP-SPEC-####: Title

Status: Proposed | Accepted | Superseded | Completed
Proposal: docs/proposals/PLSP-PROP-####-short-name.md
Related ADRs:
Related plan:
Status sources:
```

A spec must define the behavior contract, acceptance examples, proof commands or
receipt requirements, fallback behavior, status interpretation, and what the
spec does not claim.

### ADR header

```md
# PLSP-ADR-####: Title

Status: Proposed | Accepted | Superseded | Deprecated
Date: YYYY-MM-DD
Related proposal:
Related specs:
Related plan:
```

An ADR must capture context, decision, alternatives, consequences, and migration
or compatibility notes when relevant.

### Plan header

```md
# Lane Title Implementation Plan

Status: Active | Planned | Completed | Deferred
Proposal:
Specs:
ADRs:
Active goal:
Status sources:
```

A plan must break work into PR-sized units and include proof commands, rollback,
and handoff state for each unit.

## When to Create Which Artifact

Create a proposal when the work needs product motivation, user value, success
criteria, alternatives, or a claim boundary that should outlive a single PR.

Create a spec when reviewers need a stable behavior contract: inputs, outputs,
acceptance examples, proof requirements, fallback behavior, and limits on what
`perl-lsp` may claim.

Create an ADR when the work makes a durable architecture or operating decision
that future maintainers should not rediscover through commit history.

Create an implementation plan when the lane is too large for one PR and needs a
sequenced path with proof commands and rollback notes.

Create or update an active goal manifest only when the lane becomes the current
machine-readable execution state for agents.

## Linking Status Without Copying Truth

Generated status docs and support-tier dashboards are the current truth for
metrics and receipts. Source-of-truth artifacts should link to them instead of
copying their tables.

Preferred status surfaces include:

- `docs/project/CURRENT_STATUS.md`
- `docs/project/ROADMAP.md`
- `docs/project/status/SUPPORT_TIERS.md`
- `docs/project/status/provider_confidence_matrix.md`
- `docs/project/status/semantic_scorecard.md`
- `docs/project/status/semantic_shadow_compare.md`
- `docs/project/status/ux_capability_dashboard.md`

If a number, count, or support claim matters, verify it against its status source
before citing it. Do not hand-edit generated status sections.

## Active Goals

`.perl-lsp/goals/active.toml` is the current machine-readable lane manifest. It
should identify the active lane, objective, end state, current work items,
source-of-truth links, status pointers, and proof commands.

Archive an old manifest under `.perl-lsp/goals/archive/` when it is no longer the
operative lane and historical state should be preserved. Do not change the active
goal just because a proposal or spec was added; change it when execution state
moves.

## PR Body Structure

For source-of-truth documentation PRs, keep the PR body short and evidence-based:

```text
Problem: <one sentence>
Fix: <one sentence>
Verification: `git diff --check` passes
```

Use a focused title with the repository title convention, for example:

```text
docs: expose perl-lsp source-of-truth stack (#0000)
docs(proposal): add semantic receiver facts proposal (#0000)
docs(spec): add receiver fact contract (#0000)
docs(adr): require receiver facts before method completion cutover (#0000)
```

One semantic documentation artifact per PR keeps review scope clear. A PR that
exposes the system may touch front-door/index docs together, but should not also
add a new proposal, spec, ADR, plan, or active-goal change.

## Agent Consumption Rules

Codex, Claude, and other implementation agents should consume the stack in this
order:

1. Read `docs/project/ROADMAP.md` for release direction.
2. Read the lane proposal for motivation, success criteria, alternatives, and
   claim boundary.
3. Read linked specs for behavior contracts, acceptance, proof, and fallback
   boundaries.
4. Read linked ADRs for durable decisions and constraints.
5. Read `plans/<lane>/implementation-plan.md` for PR-sized sequence, rollback,
   and handoff state.
6. Read `.perl-lsp/goals/active.toml` only when current execution state matters.
7. Verify current truth in `docs/project/status/*.md` and policy ledgers instead
   of trusting copied metrics.
8. Land one focused PR and report exact proof commands.

Agents must not implement semantic/type-engine behavior while authoring proposal,
spec, ADR, or source-of-truth-discovery PRs unless the assigned task explicitly
asks for implementation.

## Current Example

The Real Perl Editor Trust lane demonstrates the pattern:

- `docs/proposals/PLSP-PROP-0001-real-perl-editor-trust.md`
- `docs/specs/PLSP-SPEC-0001-parser-compatibility-bucket-closeout.md`
- `docs/specs/PLSP-SPEC-0002-provider-confidence-receipts.md`
- `docs/specs/PLSP-SPEC-0003-real-workspace-editor-baseline.md`
- `docs/specs/PLSP-SPEC-0004-corpus-receipt-freshness.md`
- `docs/adr/PLSP-ADR-0001-generated-status-is-control-plane.md`
- `docs/adr/PLSP-ADR-0002-confidence-before-cutover.md`
- `plans/real-perl-editor-trust/implementation-plan.md`
- `.perl-lsp/goals/active.toml`
