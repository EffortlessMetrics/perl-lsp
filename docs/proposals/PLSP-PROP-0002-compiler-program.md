# PLSP-PROP-0002: Repo-native compiler-program contracts

Status: proposed
Owner: perl-lsp maintainers
Created: 2026-06-21
Target milestone: Compiler-program gate (tracker #2559)
Linked specs: #2563 (context), #2564 (HIR-body ADR), #2565 (PIR-A/EIR)
Linked ADRs: [PLSP-ADR-0005](../adr/PLSP-ADR-0005-hir-body-pir-eir-boundaries.md)
Linked plan: [plans/compiler-program/implementation-plan.md](../../plans/compiler-program/implementation-plan.md)
Support/status impact: compiler facts, HIR lowering coverage, PIR v0 substrate, provider cutover gating
Policy impact: HIR-body/PIR-A/EIR terminology becomes canonical; no generated status is altered here

## Problem

`perl-lsp` now has a fixture-backed compiler substrate: HIR items, scope/pad,
package/stash, compile environment, import/export, compile-time effects,
framework adapters, and PIR v0. The layers are contracted in
[PLSP-SPEC-0030](../specs/PLSP-SPEC-0030-compile-state-layers.md), but the
next level of the roadmap — compiler world, abstract compile engine, provider
bridge, and the distinction between tooling PIR and a future execution IR — has
no repo-native artifacts. Agents, reviewers, and contributors must rely on chat
history and external references to know what "Phase 2" means, which layer order
to follow, and where the HIR-body / PIR-A / EIR boundaries lie.

The result is that the substrate is built but not yet oriented toward a next
step. PRs can drift between HIR body expansion, PIR control-flow broadening,
and speculative execution-IR experiments without a durable boundary saying which
is which. That tension is already visible: PIR v0 is named "tooling IR" in one
place and "PIR" in another, and there is no canonical record clarifying whether
EIR (execution IR) is an evolution of PIR or a separate concern.

## Users and Surfaces

- Compiler-substrate engineers adding HIR body constructs or PIR operations
- Agents routing build work to HIR-body vs. PIR-A vs. EIR slices
- Reviewers deciding whether a PR is in-contract with the layer order
- Plan-reviewers setting the Phase-2 gate and expressions-before-control-flow order
- LSP provider engineers planning provider cutover sequences

## Current Evidence

Current facts live in generated or human-owned status docs. This proposal
links to those sources instead of duplicating their tables.

- [HIR lowering coverage](../project/status/hir_lowering.md) reports the
  current HIR construct coverage: 25 `lowered`, 3 `dynamic_boundary`,
  19 `intentionally_skipped`, 23 `not_yet_modeled` of 70 tracked AST kinds.
  Expressions (`Binary`, `Unary`) and several statement constructs remain
  `not_yet_modeled`.
- [Compiler facts](../project/status/compiler_facts.md) confirms the Tooling
  PIR layer is `fixture-backed` with HIR lowering for data-access, call, and
  dynamic-boundary families; branch/loop/return lowering remains out.
- [Compiler capability status](../project/COMPILER_CAPABILITY_STATUS.md) shows
  Tooling IR / PIR as `fixture-backed`; provider cutover as `partial live`
  gated on [#8197](https://github.com/EffortlessMetrics/perl-lsp/issues/8197).
- [PLSP-SPEC-0025 (PIR v0)](../specs/PLSP-SPEC-0025-pir-v0.md) contracts the
  existing PIR data model and lowering; the `Branch`, `Loop`, `Return`,
  `LexicalRead`, `StashRead` families are reserved but not yet populated.
- [PLSP-SPEC-0030 (Compile state layers)](../specs/PLSP-SPEC-0030-compile-state-layers.md)
  contracts L0–L6 with no PIR-A or EIR layer defined; those belong in the
  next layer's contract, not in the PLSP-SPEC-0030 revision.

The compiler roadmap ([COMPILER_BACKED_LSP_ROADMAP.md](../project/COMPILER_BACKED_LSP_ROADMAP.md))
names "tooling IR" in the pipeline but does not define HIR-body expansion
ordering, SemanticSnapshot extension, compiler world, abstract compile engine,
or the EIR branch-off point. This proposal provides the missing orientation.

## Success Criteria

- A canonical ADR records the HIR-body / PIR-A / EIR terminology and boundary
  rules so any future IR-adjacent PR can be checked against it without chat context.
- A proposal records the lane motivation, user surfaces, and non-goals so
  reviewers know what the compiler-program lane is for.
- An implementation plan records the Phase-2 gate and expressions-before-
  control-flow ordering so agents can execute the next slice without ambiguity.
- A goal manifest records the active tracker (#2559) so the swarm can route
  work to this lane by manifest query.
- No generated status is altered; no provider behavior is changed; no PIR
  operations are added; no EIR crate is created.

## Proposed Shape

The lane is organized as four repo-native artifacts:

**Proposal (this document)**: records why the lane exists, the user surfaces,
and the product motivation. Does not encode PR sequence or generated metrics.

**Boundary ADR (PLSP-ADR-0005)**: records the durable terminology decision —
PIR-A is the evolution of the existing tooling PIR (`crates/perl-parser-core/src/pir/`);
EIR is a future execution IR that branches off PIR-A later; HIR body items are
the HIR-layer shells for expressions and control-flow, not a new IR. Agents and
reviewers may cite this ADR when deciding whether a PIR PR is in-contract.

**Implementation plan (plans/compiler-program/)**: records the Phase-2 gate
and the expressions-before-control-flow ordering within Phase 2. This is the
PR-sequence map: what must be true before Phase 2 opens, which slices run in
what order, and how to confirm each slice is complete.

**Goal manifest (.perl-lsp/goals/compiler-program.toml)**: records the active
tracker (#2559) and current work item so the swarm can route agents by manifest
query without scraping chat.

## Alternatives Considered

### Extend PLSP-SPEC-0030 with a PIR-A and EIR layer definition

Rejected. PLSP-SPEC-0030 contracts the compile-state stack (L0–L6). PIR-A and
EIR sit above and downstream of that stack; adding them to PLSP-SPEC-0030
would conflate HIR-to-provider lowering with IR-for-execution concerns. The
correct place for PIR-A/EIR boundary rules is a focused ADR.

### Put all lane guidance in the compiler roadmap

Rejected. The roadmap is a prose design document, not a durable decision
record. Agents cannot verify whether a PIR PR is in-contract by reading
roadmap prose. A dedicated ADR and plan provide checkable, single-purpose
artifacts that the roadmap can link to.

### Defer all orientation documents until EIR work is imminent

Rejected. The HIR-body and PIR-A expansion slices are already running
(branch/loop/return lowering is next per the PIR v0 spec). Without a boundary
ADR, those PRs must guess whether they are expanding PIR-A or creating EIR.
The ADR is needed now to prevent terminology drift.

## Non-goals

- No new HIR body shells (expressions, control-flow items)
- No new PIR operations (branch, loop, return, read-side)
- No EIR crate or EIR data model
- No compiler world or abstract compile engine implementation
- No provider cutover or live behavior change
- No generated status alteration
- No parser/corpus bucket movement
- No release-lineage sync claim

## Evidence Plan

Docs-only check:

```bash
git diff --check
cargo xtask check-active-goal-manifest
cargo xtask ci-hygiene check-doc-paths docs/proposals
cargo xtask ci-hygiene check-doc-paths docs/adr
cargo xtask ci-hygiene check-doc-paths plans/compiler-program
```

## Exit Criteria

The lane can close when all of these are true:

- PLSP-ADR-0005 is accepted and merged with the HIR-body/PIR-A/EIR boundary rules
- Implementation plan names the Phase-2 gate and expression-ordering
- Goal manifest points at tracker #2559
- No generated status, provider behavior, or PR sequence is altered by the docs-only PR
- Agents and reviewers can cite ADR-0005 to decide whether a future IR PR is in-contract

## Claim Boundary

This proposal defines lane orientation and document locations. It does not add
HIR body shells, add PIR operations, create EIR, change provider behavior, alter
generated status, claim parser movement, or authorize provider cutover. Those
changes require their own specs, plans, receipts, and PR-sized proof.
