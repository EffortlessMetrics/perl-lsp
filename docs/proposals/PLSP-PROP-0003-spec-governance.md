# PLSP-PROP-0003: Spec-governance via cargo-allow

Status: proposed
Owner: perl-lsp maintainers
Created: 2026-07-10
Target milestone: Spec-governance S0 (tracker #3586)
Linked specs: none yet (S2/S3 will add a spec-system contract spec)
Linked ADRs: none yet
Linked plan: [plans/spec-governance/implementation-plan.md](../../plans/spec-governance/implementation-plan.md)
Support/status impact: none in S0 — no generated status is read or altered
Policy impact: none in S0 — `.allow/` config and `.perl-lsp/goals/` manifests
are out of scope until S1

## Problem

`perl-lsp`'s spec system (`docs/reference/SPEC_TEMPLATE.md`,
`.spec/<issue#>-<slug>/` bundles, `spec-builder.js`, spec-planner/red-tdd/
builder prompts) has grown its own ad hoc structural conventions: file-path
identity, informal linking between context.md/acceptance.md/checklist.md, and
no machine-checkable graph of proposal -> spec -> plan -> closeout. Structural
drift (duplicate spec IDs, missing linked artifacts, orphaned goal manifests)
is currently caught only by human/agent review, not by a dedicated validator.

`cargo-allow` v0.1.10 is already installed in this repo for its default job —
the source-exception ledger (`policy/allow.toml`) — and ships a second, opt-in
`spec-system` profile purpose-built for exactly this class of problem:
identity, typed links, lifecycle, ownership, and closeout edges over a
markdown-based spec graph. Today that profile is unconfigured here
(`cargo-allow doctor --profile spec-system` reports `.allow/profiles/
spec-system.toml` missing) and no perl-lsp spec bundle is registered with it.

Without a durable migration contract, future PRs that touch the spec system
have no shared understanding of which structural checks belong to
`cargo-allow` versus which stay repo-specific in xtask, and no phased,
reversible path from "unconfigured" to "narrowly blocking."

## Users and Surfaces

- spec-planner and spec-builder.js, which author `.spec/` and (post-S3) native
  bundle content
- red-tdd and builder agents, which read spec bundles to implement
- Reviewers and orchestrator routing logic, which need a queryable structural
  graph of active spec work
- CI, which will eventually run an advisory (then narrowly-blocking) spec-graph
  check alongside the existing source-exception ledger check

## Success Criteria

- A durable implementation plan records the phase train (S0-S6), the
  responsibility split between `cargo-allow` (structural graph) and existing
  tooling (content authoring, semantics, proof execution, live GitHub state),
  and the cutover/rollback order (advisory -> shadow -> narrow-block).
- A tracking issue (#3586) exists so agents can route follow-up phases without
  re-deriving the plan from chat history.
- No `.allow/` config, no `.perl-lsp/goals/` manifest, and no code changes
  ship in S0 — this proposal and its linked plan are orientation only.

## Proposed Shape

**Proposal (this document)**: records why a structural spec-graph validator is
worth adopting, which surfaces it serves, and what stays out of scope for the
first phase.

**Implementation plan (`plans/spec-governance/implementation-plan.md`)**:
records the S0-S6 phase train, the responsibility-split table, the legacy
`.spec` read-only-import posture, and the governance clause for the
long-running operating goal.

**Goal manifest** (deferred to S1): `cargo-allow`'s `spec-system` profile is
itself a candidate registrant once `.allow/profiles/spec-system.toml` exists;
S0 intentionally does not create `.perl-lsp/goals/spec-governance.toml` so
that the exact shape can be taken from `cargo-allow init --profile
spec-system --dry-run` output at S1 time rather than guessed here.

## Alternatives Considered

### Write a bespoke perl-lsp spec-graph linter in xtask

Rejected for S0. `cargo-allow` already ships a generic markdown-graph
validator with identity/link/lifecycle checks and receipt output; duplicating
that in xtask would be maintenance debt for a solved problem. xtask keeps
owning what is genuinely repo-specific (lane IDs, WIP caps, hazard-class
completeness, RTK-prefix policy) rather than re-implementing generic graph
validation.

### Configure `.allow/profiles/spec-system.toml` and register bundles in this PR

Rejected. This is a docs-only control-plane PR; the profile config and the
first registered graph belong to S1, after the exact `init --dry-run` shape
is captured as source of truth rather than hand-authored here.

### Migrate the whole `.spec` tree onto cargo-allow-native identity immediately

Rejected. S3 defines the native bundle mapping (context.md -> proposal,
acceptance.md -> spec, checklist.md -> implementation_plan, closeout.md ->
closeout) for *new* bundles only. The historical `.spec` tree stays read-only
except for currently-active, goal-linked, or frequently-referenced specs
(see the plan's "Legacy .spec posture" section) — a mass rewrite is out of
scope for the entire phase train, not just S0.

## Non-goals

- No `.allow/profiles/spec-system.toml` creation or configuration
- No `.perl-lsp/goals/` manifest for this lane
- No registration of any `.spec/` bundle with `cargo-allow`
- No CI job (advisory or blocking) added in S0
- No change to spec-planner, spec-builder.js, red-tdd, or builder prompts
- No change to `docs/project/status/**` generated surfaces
- No touch to PR #3579's branch (already merged to `main` as of this PR's base)

## Evidence Plan

Docs-only check:

```bash
git diff --check
cargo xtask ci-hygiene check-doc-paths docs/proposals
cargo xtask ci-hygiene check-doc-paths plans/spec-governance
```

## Exit Criteria

S0 (this proposal + the linked plan) can be considered complete when:

- This proposal and `plans/spec-governance/implementation-plan.md` are merged
  and cross-linked
- Tracker #3586 exists and is referenced by both documents
- No code, `.allow/` config, or `.perl-lsp/goals/` manifest changed
- S1 can open by running `cargo-allow init --profile spec-system --dry-run`
  and using its output as the source of truth for the profile config shape

## Claim Boundary

This proposal defines lane orientation and the migration contract only. It
does not configure `cargo-allow`, register any spec-graph artifacts, change
CI, alter spec-authoring tooling, or claim any structural check is enforced
today. Those changes require S1-S6 PRs with their own proof.
