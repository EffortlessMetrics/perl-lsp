# Issue Research / Plan Review Desk

**Status**: Active doctrine (introduced 2026-05-30)
**Related**: [PIPELINE_GATES.md](./PIPELINE_GATES.md) | [CLAUDE.md](../../CLAUDE.md) | [LIVE_SIGNALS_VS_LABELS.md](./LIVE_SIGNALS_VS_LABELS.md) | [LANE_BOUNDARIES.md](./LANE_BOUNDARIES.md)

> For the umbrella framing — what an Octopus Cluster is and the vocabulary these docs share — see [OCTOPUS_CLUSTER.md](./OCTOPUS_CLUSTER.md).

---

## Mission

The Issue Research / Plan Review Desk turns raw observations into trustworthy
execution surface. Its output is **not code** — it is a builder-ready work
order that a builder can pick up without rediscovering the problem.

```text
raw observation
  → verified from source
  → deduped against existing issues
  → improved into a builder-ready plan
  → labeled with the right gate
  → periodically re-verified against main
```

The desk sits **between scouts and builders**. Scouts discover. Builders
implement. The desk makes sure the thing handed to a builder is real, current,
scoped, and testable.

A builder opening a `builder-ready` issue should know, without re-investigating:

- what is broken
- where to look
- what *not* to touch
- what test proves the fix
- what risk exists
- what issue/PR dependencies matter

---

## Where the desk sits in the pipeline

This desk is not a new pipeline. It is the **operational doctrine for Gates 1–2**
([PIPELINE_GATES.md](./PIPELINE_GATES.md)) viewed as a continuous stream of
issue work rather than a one-shot pass:

| Gate | Desk responsibility |
|------|---------------------|
| **1. Identify** | Research findings, verify from source, dedupe, file accurate issues |
| **2. Spec** | Improve weak issues, run plan/architecture review, promote to `builder-ready`, sequence into waves |

The desk uses the **existing** agents and labels. It does not invent a parallel
state machine. The scout → accuracy-scout → research-verifier → plan-review
chain in CLAUDE.md *is* the desk's verification pipeline; this document adds the
quality bar, the dedupe discipline, the wave planning, and the operating rules
that keep that pipeline honest.

---

## Core rule: issue by issue

The same principle as PR-by-PR. Read-only inventory may batch. **Consequential
issue mutations are issue by issue.**

Read-only batching is allowed:

- inventory issues by label / state
- search labels
- compare possible duplicates
- inspect source, tests, docs, receipts
- inspect PR links and CI/check state

Mutating actions are issue by issue:

- file an issue
- rewrite an issue body
- close an issue
- mark a duplicate
- add or remove labels
- promote to `builder-ready`
- link PRs
- mark stale / superseded

Batch discovery is fine. Batch mutation is not. Every consequential edit is
deliberate, evidence-backed, and reversible-in-principle.

---

## Issue lifecycle

The desk tracks each issue through explicit states. Most states map directly to
**existing** labels (see the reconciliation table below); a few are proposed
refinements that must be created before agents rely on them.

```text
candidate     → a raw finding, not yet verified
researched    → claims grounded against source
filed         → exists as a GitHub issue
needs-plan-review → in the verification + plan-review pipeline
plan-reviewed → spec refined and approved (PIPELINE_GATES.md's Gate-2 exit label)
architecture-reviewed → structural fit confirmed (when relevant)
builder-ready → desk-level issue-quality marker; see reconciliation note below
in-build      → builder working (PR exists)
implemented   → PR merged
superseded    → replaced by other work
duplicate     → folded into another issue
stale         → invalidated by main moving
blocked       → waiting on a dependency or authority
```

**Reconciliation note (Gate-2 exit label):** [PIPELINE_GATES.md](./PIPELINE_GATES.md)
defines Gate 2's exit condition as the `plan-reviewed` label — spec-planner (which
creates the impl branch and `.spec/` checklist) runs *within* Gate 2, after
plan-review, so Gate 2 can close out on `plan-reviewed` alone. This desk additionally
tracks `builder-ready` as a supplementary issue-quality marker (all ten work-order
sections answered and reviewed) for issues that flow through the lighter
`research_finding.yml` intake rather than spec-planner's impl-branch path. Treat
`plan-reviewed` as authoritative for the Gate 2 → Gate 3 transition; `builder-ready`
is advisory quality signal, not a second required gate.

### Label reconciliation (existing vs proposed)

The authoritative label catalog is [LIVE_SIGNALS_VS_LABELS.md](./LIVE_SIGNALS_VS_LABELS.md)
and the label tables in [CLAUDE.md](../../CLAUDE.md). The desk **uses what
exists** and treats the rest as proposals:

| Desk concept | Existing label to use now | Proposed refinement (not yet created) |
|--------------|---------------------------|----------------------------------------|
| Raw finding needs triage | `swarm-discovered` | `needs-triage`, `needs-research` |
| In verification + plan-review | `needs-plan-review` | — |
| Spec approved | `plan-reviewed` | — |
| Structural fit confirmed | `architecture-reviewed` | `needs-architecture-review` (routing flag) |
| Ready to build | `builder-ready` | — |
| Blocks other work | `structural-blocker` | generic `blocked` |
| Needs a reproduction | — (note in body) | `needs-repro` |
| Needs acceptance tests | — (note in body) | `needs-acceptance-tests` |
| Needs dependency map | — (note in body) | `needs-dependency-map` |
| Invalidated by main | close with `state_reason: not_planned` | `stale` |
| Replaced by other work | close, cross-link the replacement | `superseded` |
| Folded into another issue | close with `state_reason: duplicate` | — |

**Rule:** do not apply a label that does not exist — GitHub silently drops
unknown labels from issue templates and API calls, producing a no-op that looks
like routing. Proposed labels must be created by a maintainer **and** added to
[LIVE_SIGNALS_VS_LABELS.md](./LIVE_SIGNALS_VS_LABELS.md) before any agent uses
them. Until then, use the existing label and record the finer state in the issue
body.

---

## The builder-ready quality bar

An issue is `builder-ready` only when every section below is answered — even if
the answer is "none" or "not applicable." For tiny issues this can be compact,
but no section is silently omitted.

```md
## Problem
## Current evidence
## Reproduction / example
## Suspected root area
## Fix plan
## Acceptance tests
## Non-goals
## Dependencies / sequencing
## Risk / rollback
## Verification notes
```

The [`builder_ready.yml`](../../.github/ISSUE_TEMPLATE/builder_ready.yml) issue
template captures exactly these sections.

### `builder-ready` is earned, not self-declared

Filing an issue with the builder-ready template does **not** grant the
`builder-ready` label. The template captures the *structure* a builder-ready
issue must have; the *label* is earned by passing Gate-2 review (plan-review,
and architecture-review when relevant). A freshly filed issue enters at
`needs-plan-review`.

This is the anti-pattern the desk exists to prevent: a "builder-ready" issue
with no acceptance test, an unreviewed architecture decision, or a stale
assumption. `builder-ready` means builder-ready.

---

## Dynamic workflows

Do not force every issue through the same path. Classify the issue first, then
pick the smallest safe workflow.

| Classification | Workflow |
|----------------|----------|
| New finding | research → dedupe → file |
| Weak existing issue | plan-review → improve body → label |
| Duplicate candidate | compare evidence → close/link only if truly duplicate |
| Stale / superseded | verify against main → close or update |
| Architecture-sensitive | architecture-review before `builder-ready` |
| Missing reproduction | repro-review before `builder-ready` |
| Missing acceptance tests | acceptance-review before `builder-ready` |
| Scope too broad | split into smaller issues |
| Builder-ready | add dependency map and wave priority |
| Blocked by dependency | record the blocker, sequence behind it |
| Wrong premise | correct the body or close with evidence |

---

## Research rules

### Use source, not summaries

Treat scout summaries as **hypotheses**, not facts. Verify from:

- source files
- tests
- docs and receipts
- GitHub issue/PR bodies
- changed-file lists
- CI / check output
- current `main`

### Do not file on vibes

Before filing, search existing open issues, search closed issues if the finding
is likely old, search recent PRs, and check whether `main` already fixed it or
whether the behavior is genuinely unsupported/dynamic rather than a bug.

### Dedupe by failure mode, not by surface

Never mark a duplicate based only on similar title, same file, same broad theme,
same label, same scout wave, or same helper module. Use the **semantic**
overlap:

- same failure mode
- same source surface
- same intended fix
- same acceptance tests
- same user-visible behavior

Shared base branches or shared helper modules are *coordination* signals, not
duplication. The correct verdict for overlapping-but-distinct work is usually
**sequence-both** or **split**, not **close as duplicate**.

---

## Review gates

The desk enforces four gates before an issue is `builder-ready`.

### Gate 1 — Research

An issue may move from `candidate` to `filed` only when the source surface is
identified, an existing-issue search is done, current `main` is checked, and the
claim has at least one concrete artifact.

### Gate 2 — Plan review

An issue may move to `plan-reviewed` only when the problem is specific, the fix
plan is plausible, acceptance tests are concrete, non-goals are explicit, and
dependencies are listed.

### Gate 3 — Architecture review

Required when the issue touches crate boundaries, public AST/API shape, LSP/DAP
protocol behavior, CI policy, workflow routing, storage/worktree cleanup, or
cross-lane status receipts. This is the architecture-reviewer's pass in Gate 2
(see [CLAUDE.md](../../CLAUDE.md) and [LANE_BOUNDARIES.md](./LANE_BOUNDARIES.md)).

### Gate 4 — Builder-ready

An issue may get `builder-ready` only when it is plan-reviewed,
architecture-reviewed (if needed), has concrete acceptance tests, a known
conflict surface, and a known rollback/rollout mode — and its labels match
reality.

---

## Builder wave planning

The desk does not only improve individual issues; it groups `builder-ready`
issues into **safe waves** ordered by dependency and conflict surface.

```md
## Wave name
## Issues included
## Why these belong together
## Dependency order
## Conflict surfaces
## Suggested builders
## Tests likely needed
## Risk
## Stop conditions
```

A keystone issue is one that other builders depend on — for example, a shared
classification or API-placement issue that gates downstream migration work.
Keystones get architecture review first and land before their dependents.

---

## Operating model

### Read-only agents may run wide

Inventory, search, source grounding, duplicate detection, staleness review, and
label audits are cheap read-only passes. Run them broadly.

### Mutating agents run narrow

Body rewrites, label changes, closures, duplicate-marking, PR linking, and
`builder-ready` promotion happen **one issue at a time**, each backed by
evidence.

### No heavy polling

Use point-in-time GitHub snapshots. Prefer targeted REST reads when cheaper. Do
not set GraphQL watchers or poll on a tight loop.

### Verify filings

An agent may *claim* it filed an issue. Verify the issue exists before relying on
it. If GitHub tooling fails mid-pass, recover the issue body and re-file via the
available API path — do not pretend a filing succeeded. (This is lane doctrine,
not an ad-hoc rescue: scout filings have failed silently when `gh` was absent
from the environment.)

---

## Issue templates

Two GitHub-Forms templates support the desk's entry points:

| Template | File | Entry label | Use |
|----------|------|-------------|-----|
| Research Finding | [`research_finding.yml`](../../.github/ISSUE_TEMPLATE/research_finding.yml) | `swarm-discovered`, `needs-triage` | A raw observation or hypothesis to verify and dedupe before it enters plan review |
| Builder-Ready Plan | [`builder_ready.yml`](../../.github/ISSUE_TEMPLATE/builder_ready.yml) | `swarm-discovered`, `needs-plan-review` | A fully structured work-order proposal entering the verification + plan-review pipeline |

The two templates differ by **depth and intent**, not just fields: a research
finding is a hypothesis at the `candidate`/`researched` stage; a builder-ready
plan is a decided work order awaiting Gate-2 confirmation. Neither template
grants `builder-ready` on filing — that label is earned through review.

---

## Tooling

`cargo xtask issue-plan` hosts the desk's tooling. Everything ships
**report-only first** (instrument before enforcement), mirroring the file-policy
rollout posture (`policy/non-rust-allowlist.toml` is `advisory` until its checker
lands).

### Available

| Command | Purpose |
|---------|---------|
| `cargo xtask issue-plan audit` | Report-only. Flags `builder-ready` issues whose body is missing a required work-order section (acceptance, reproduction, root area, non-goals, dependencies, risk, verification), `builder-ready` on a closed issue, stale routing-label contradictions (`needs-plan-review` co-present with a later `builder-ready`/`plan-reviewed` sign-off), and `#0000` placeholder references. Reads a `--fixture` JSON array or live `gh issue list`. Always exits 0; writes `target/receipts/issue-plan-audit.json`. |

### Proposed (not yet implemented)

Do not reference these as if they exist.

| Command | Purpose |
|---------|---------|
| `cargo xtask issue-plan promote <n> --to builder-ready` | Validate required fields before *suggesting* labels (no GitHub mutation at first) |
| `cargo xtask issue-plan dedupe --label <l>` | Report overlap by shared files, failure mode, and acceptance tests with a distinct/sequence/split/merge/duplicate recommendation |
| `cargo xtask issue-plan stale` | Report issues mentioning files removed from `main`, issues whose acceptance tests already exist, or whose linked PR merged |
| `cargo xtask issue-plan wave --label builder-ready --max-conflict-risk low` | Output safe groupings, dependency order, and conflict files |

---

## What this lane prevents

The desk exists to stop low-friction wrongness:

- a curator confidently closing distinct work as duplicate
- a scout filing a plan with wrong premises
- a builder starting from an unreviewed spec
- issue labels drifting from reality
- an agent claiming a filing that did not happen
- duplicate fragments splitting the same work
- `main` moving while issue plans stay stale
- acceptance tests missing until implementation time

The posture that works: verify by primary artifacts, correct the plan before
building, and let the issue tracker become a load-bearing map rather than a pile
of notes.

---

## See Also

- [PIPELINE_GATES.md](./PIPELINE_GATES.md) — the 7-gate model; the desk operationalizes Gates 1–2
- [LIVE_SIGNALS_VS_LABELS.md](./LIVE_SIGNALS_VS_LABELS.md) — authoritative label catalog; live truth vs label bookkeeping
- [LANE_BOUNDARIES.md](./LANE_BOUNDARIES.md) — lane ownership and non-overlap rules
- [CLAUDE.md](../../CLAUDE.md) — orchestrator routing model and the canonical label tables
- [`.github/ISSUE_TEMPLATE/builder_ready.yml`](../../.github/ISSUE_TEMPLATE/builder_ready.yml) — the builder-ready work-order form
- [`.github/ISSUE_TEMPLATE/research_finding.yml`](../../.github/ISSUE_TEMPLATE/research_finding.yml) — the research-finding intake form
