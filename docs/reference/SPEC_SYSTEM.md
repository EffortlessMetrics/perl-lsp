# perl-lsp Source-of-Truth and Spec System

This guide defines how `perl-lsp` encodes long-lived work across proposals,
specs, ADRs, implementation plans, active goals, status receipts, and PRs. It is
an authoring contract for maintainers and agents; it is not a generated status
report.

## The Stack

Use this chain for long-lived lanes:

```text
Roadmap → Proposal → Specs → ADRs → Plan → Active goal → PRs → Receipts
```

| Layer | Owns | Storage |
| --- | --- | --- |
| Roadmap | Release direction and active milestone | [`docs/project/ROADMAP.md`](../project/ROADMAP.md) |
| Proposal / PRD | Why the lane exists, user value, alternatives, claim boundary | [`docs/proposals/PLSP-PROP-*.md`](../proposals/) |
| Spec | Behavior contract, acceptance, proof requirements, claim limits | [`docs/specs/PLSP-SPEC-*.md`](../specs/) |
| ADR | Durable architecture or operating decision | [`docs/adr/PLSP-ADR-*.md`](../adr/) |
| Implementation plan | PR-sized sequence, proof commands, rollback, handoff state | [`plans/<lane>/implementation-plan.md`](../../plans/) |
| Active goal manifest | Machine-readable current agent state | [`.perl-lsp/goals/active.toml`](../../.perl-lsp/goals/active.toml) |
| Status / support tiers | Current truth and public claim proof | [`docs/project/status/*.md`](../project/status/) |
| Policy ledgers | CI, lint, file, package, and exception receipts | [`policy/*.toml`](../../policy/), [`.ci/**`](../../.ci/) |
| Closeout / handoff | What happened, what remains, proof | `plans/<lane>/closeout.md` or [`docs/forensics/`](../forensics/) |

## Artifact Roles

### Roadmap

The roadmap names release direction, milestones, and sequencing intent. It
should point to lanes and truth sources, not duplicate proposal bodies or status
dashboards.

### Proposal / PRD

A proposal explains why a lane should exist: the user problem, affected surfaces,
success criteria, considered alternatives, non-goals, and claim boundary. Use a
proposal when the work changes product direction or ties multiple subsystems to a
user-visible outcome.

A proposal must not own PR ordering, generated metric state, or durable
architecture decisions. Link to specs, ADRs, plans, and status docs when they
exist.

See [`docs/proposals/README.md`](../proposals/README.md).

### Spec

A spec defines what must be true. It owns behavior contracts, acceptance
criteria, proof requirements, status interpretation, and claim limits. Use a spec
when reviewers and future agents need a durable contract across more than one PR.

A spec must not carry broad product motivation, roadmap framing, active queue
ownership, or PR sequencing.

See [`docs/specs/README.md`](../specs/README.md).

### ADR

An ADR records a durable decision about architecture, policy, or operating model.
Use an ADR when a decision should continue to guide implementation after the
current lane is complete.

An ADR must not become a raw worklist, a point-in-time metric document, or a PR
queue. Link to proposals, specs, plans, and generated status instead.

See [`docs/adr/README.md`](../adr/README.md).

### Implementation Plan

A plan converts the proposal/spec/ADR stack into PR-sized work. It owns work-item
order, proof commands, rollback notes, blockers, and handoff state.

Plans live under `plans/<lane>/`. They are not specs and should not redefine the
behavior contract. Link to generated status docs and receipts instead of copying
status tables.

See [`plans/README.md`](../../plans/README.md) and
[`plans/real-perl-editor-trust/README.md`](../../plans/real-perl-editor-trust/README.md).

### Active Goal Manifest

The active goal manifest records current machine-readable lane state for agents:
lane ID, objective, active work items, linked artifacts, status pointers, and
proof commands.

The active manifest lives at [`.perl-lsp/goals/active.toml`](../../.perl-lsp/goals/active.toml).
Archive old manifests under `.perl-lsp/goals/archive/` when changing the active
lane. Do not make agents infer active state only from chat or prose.

See [`.perl-lsp/goals/README.md`](../../.perl-lsp/goals/README.md).

### Status, Support Tiers, and Receipts

Generated status and support-tier documents are the current-truth surface for
metrics, readiness, confidence, and public claims. Link to them; do not copy
large generated tables into proposals, specs, ADRs, or plans.

Common status surfaces include:

- [`docs/project/CURRENT_STATUS.md`](../project/CURRENT_STATUS.md)
- [`docs/project/status/SUPPORT_TIERS.md`](../project/status/SUPPORT_TIERS.md)
- [`docs/project/status/provider_confidence_matrix.md`](../project/status/provider_confidence_matrix.md)
- [`docs/project/status/semantic_scorecard.md`](../project/status/semantic_scorecard.md)
- [`docs/project/status/semantic_shadow_compare.md`](../project/status/semantic_shadow_compare.md)
- [`docs/project/status/ux_capability_dashboard.md`](../project/status/ux_capability_dashboard.md)

### Policy Ledgers

Policy TOMLs and `.ci/**` files record enforceable exceptions, gates, and
receipts. Specs and ADRs may explain the policy meaning, but the ledger remains
the enforcement source.

## ID Naming

Use `PLSP-*` IDs for the lane-level source-of-truth system:

| Artifact | Pattern | Example |
| --- | --- | --- |
| Proposal | `PLSP-PROP-####-short-name.md` | `PLSP-PROP-0001-real-perl-editor-trust.md` |
| Spec | `PLSP-SPEC-####-short-name.md` | `PLSP-SPEC-0002-provider-confidence-receipts.md` |
| ADR | `PLSP-ADR-####-short-name.md` | `PLSP-ADR-0001-generated-status-is-control-plane.md` |
| Plan directory | `plans/<lane>/` | `plans/real-perl-editor-trust/` |
| Active goal ID | `plsp-<lane>` | `plsp-real-perl-editor-trust` |

Keep IDs stable after review. If an artifact is superseded, add status and links
rather than renumbering history.

## Required Headers

Use simple front matter fields at the top of each durable artifact. Plain
Markdown key/value headers are sufficient unless the local README defines a more
specific template.

### Proposal headers

```md
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

```md
Status:
Owner:
Linked proposal:
Linked ADRs:
Linked plan:
Status impact:
```

### ADR headers

```md
Status:
Date:
Decision owner:
Linked proposal:
Linked specs:
Linked plan:
```

### Plan work-item headers

```md
Status:
Linked proposal:
Linked spec:
Linked ADR:
Blocks:
Blocked by:
```

Active goals use TOML fields documented in
[`.perl-lsp/goals/README.md`](../../.perl-lsp/goals/README.md).

## Choosing the Right Artifact

Create or update a proposal when the question is "why should this lane exist?"
or "what user value and claim boundary justify this multi-PR direction?"

Create or update a spec when the question is "what behavior, proof, and claim
boundary must implementations satisfy?"

Create or update an ADR when the question is "what durable decision should
future maintainers keep following?"

Create or update a plan when the question is "what PR-sized sequence lands this
safely, with proof and rollback?"

Create or update the active goal manifest when the question is "what should
agents treat as the current executable lane state?"

Update status/support-tier surfaces through the existing generation or status
workflow when the question is "what is true now?"

## Linking Status Without Copying Generated Truth

When an artifact needs current evidence:

1. Link to the generated status or support-tier document.
2. State how the artifact interprets that evidence.
3. Avoid copying generated tables, counts, or large dashboards.
4. If a number is required in prose, re-run the status command and cite the
   truth source in the same change.

This keeps proposals, specs, ADRs, and plans durable while generated status stays
free to change.

## PR Body Shape

Use short PR bodies that identify the layer being changed and its verification:

```md
Problem: <one sentence>
Fix: <one sentence>
Verification: `<command>` passes
```

For docs-only source-of-truth PRs, `git diff --check` is the minimum proof.
Run existing docs or status checks when the changed artifact touches generated
status, support tiers, or policy ledgers.

## How Agents Should Consume the Stack

Implementation agents should read from durable intent toward executable work:

1. Check upstream commits and current workspace state.
2. Read the roadmap only for milestone context.
3. Read the proposal for user value and non-goals.
4. Read linked specs for behavior contracts and proof requirements.
5. Read linked ADRs for durable decisions.
6. Read the implementation plan for the next PR-sized work item.
7. Read `.perl-lsp/goals/active.toml` for current machine-readable lane state.
8. Verify generated status through linked status docs and commands before making
   factual claims.

Do not infer missing contracts from chat history. If the stack lacks an artifact,
add the smallest durable artifact in its proper layer before implementing broad
behavior changes.
