# Compiler-Program Implementation Plan

Status: planned
Owner: perl-lsp maintainers
Linked proposal: [PLSP-PROP-0002](../../docs/proposals/PLSP-PROP-0002-compiler-program.md)
Linked ADR: [PLSP-ADR-0005](../../docs/adr/PLSP-ADR-0005-hir-body-pir-eir-boundaries.md)
Linked specs:
- [PLSP-SPEC-0025](../../docs/specs/PLSP-SPEC-0025-pir-v0.md) — PIR-A data model
- [PLSP-SPEC-0030](../../docs/specs/PLSP-SPEC-0030-compile-state-layers.md) — compile state layers
Goal manifest: [.perl-lsp/goals/compiler-program.toml](../../.perl-lsp/goals/compiler-program.toml)
Tracker: [#2559](https://github.com/EffortlessMetrics/perl-lsp-swarm/issues/2559)
References: #2563 (context), #2564 (HIR-body ADR), #2565 (PIR-A/EIR)

## Purpose

Define the PR sequence for the compiler-program lane: Phase-1 orientation
documents, Phase-2 HIR body and PIR-A expansion, and the eventual compiler
world integration. This plan is a routing map, not a product-claim document.

## Current State

- L0–L6 HIR side graphs are `fixture-backed`; see
  [compiler facts](../../docs/project/status/compiler_facts.md).
- HIR body construct coverage: 25 `lowered`, 3 `dynamic_boundary`,
  19 `intentionally_skipped`, 23 `not_yet_modeled` of 70 tracked AST kinds.
  See [HIR lowering coverage](../../docs/project/status/hir_lowering.md).
- PIR v0 is `fixture-backed` (data-access, call, dynamic-boundary families);
  branch/loop/return and read-side lowering remain reserved/unimplemented.
- Provider cutover is `partial live`; broader live cutover is gated by
  [#8197](https://github.com/EffortlessMetrics/perl-lsp/issues/8197).

## Phase-1 Gate: Orientation Documents

Phase 1 is this PR. No code changes. Phase 2 may open only after Phase 1 is
merged and the ADR is accepted.

Phase 1 is complete when:

- [x] Proposal `PLSP-PROP-0002` is merged
- [x] Boundary ADR `PLSP-ADR-0005` is accepted and merged
- [x] This implementation plan is merged
- [x] Goal manifest `.perl-lsp/goals/compiler-program.toml` is merged
- All four artifacts are linked to each other and to tracker #2559
- No generated status surface is altered; no provider behavior is changed

Proof commands for Phase 1:

```bash
git diff --check
cargo xtask check-active-goal-manifest
cargo xtask ci-hygiene check-doc-paths docs/proposals
cargo xtask ci-hygiene check-doc-paths docs/adr
cargo xtask ci-hygiene check-doc-paths plans/compiler-program
```

## Phase-2 Gate: HIR Body and PIR-A Expansion

Phase 2 may open only after Phase 1 is complete.

Phase 2 is ready to open when:

- Phase 1 is merged and the ADR is accepted
- HIR lowering coverage reflects the current `not_yet_modeled` baseline from
  `cargo xtask metrics hir-coverage --check`
- PLSP-SPEC-0025 is current and the reserved operation families are named

Phase 2 is complete when:

- Key expression constructs (`Binary`, `Unary`, calls, method calls) are
  `lowered` in HIR coverage, with provenance and source anchors per C1–C5
- PIR-A has branch/loop/return/read-side lowering with receipts per
  PLSP-SPEC-0025
- Compiler world integration extends `SemanticSnapshot` with the full L0–L6
  fact graph (no new crate; extends existing workspace-analysis types)
- No provider cutover follows from Phase 2 PRs (cutover is gated by #8197)

### Phase-2 Expression Order

Within Phase 2, expressions-before-control-flow is the canonical ordering.
Rationale: expression lowering in HIR (Binary, Unary, calls) feeds the
context-propagation needed for correct branch/loop semantics in PIR-A. Starting
control-flow lowering before expression lowering is anchored produces
under-specified CFG edges and deferred context gaps.

The required order within Phase 2:

1. **HIR body — expression constructs** (L0, PLSP-SPEC-0030)
   - `Binary`, `Unary` shells with operand anchors
   - `Ternary` broadening (existing shell; add per-arm context facts)
   - Call context: argument-list and receiver-expression shells
   - Method call context: method anchor and receiver expression facts

2. **PIR-A — call and assignment side** (PLSP-SPEC-0025)
   - Read-side lowering: `LexicalRead`, `StashRead`
   - Assignment completion: full `Assign` facts including RHS expression anchors

3. **PIR-A — control-flow** (PLSP-SPEC-0025)
   - `Branch` family (if/unless/ternary)
   - `Loop` family (while/until/for/foreach)
   - `Return` family (explicit return, last/next/redo)

4. **Compiler world** (SemanticSnapshot extension; no new crate)
   - Extend `SemanticSnapshot` with full L0–L6 layer graph access
   - Abstract compile engine: query interface over compiler world
   - Provider bridge: connect compiler world facts to provider fact-source
     tracing (no live behavior until cutover gates are satisfied)

Each slice within Phase 2 should be a separate PR-sized work item in the goal
manifest, with its own `claim_boundary`, `files`, and `commands`.

## Work item: compiler-program-orientation

Status: active (this PR)
Lane: substrate
Linked proposal: [PLSP-PROP-0002](../../docs/proposals/PLSP-PROP-0002-compiler-program.md)
Linked ADR: [PLSP-ADR-0005](../../docs/adr/PLSP-ADR-0005-hir-body-pir-eir-boundaries.md)
Blocks: all Phase-2 slices
Blocked by: none

Goal

Author the four orientation documents — proposal, ADR, implementation plan,
goal manifest — so Phase-2 agents have durable routing contracts.

Production delta

Docs only: four new files. No code, no generated status update, no provider
behavior change.

Non-goals

No HIR shells, no PIR operations, no EIR types, no compiler world
implementation, no provider cutover.

Acceptance

All four orientation documents exist, are cross-linked, and reference tracker
#2559. Proof commands pass without workspace errors.

Proof commands

```bash
git diff --check
cargo xtask check-active-goal-manifest
cargo xtask ci-hygiene check-doc-paths docs/proposals
cargo xtask ci-hygiene check-doc-paths docs/adr
cargo xtask ci-hygiene check-doc-paths plans/compiler-program
```

Rollback

Revert this PR. Phase-2 slices that cite the ADR must be updated to cite
whatever replacement boundary record exists.

## Future Work Items (not yet open)

The following items become available after Phase 1 is merged. Each should be
filed as a separate PR-sized work item with its own goal manifest entry.

- `hir-body-binary-unary-shells` — L0 HIR shells for Binary and Unary
- `hir-body-call-context-shells` — L0 HIR shells for call argument-list and
  receiver expression anchors
- `pir-a-read-side-lowering` — PIR-A LexicalRead/StashRead families
- `pir-a-branch-lowering` — PIR-A Branch/if/ternary families
- `pir-a-loop-lowering` — PIR-A Loop/while/for/foreach families
- `pir-a-return-lowering` — PIR-A Return/last/next/redo families
- `compiler-world-snapshot-extension` — SemanticSnapshot extension with L0–L6
- `abstract-compile-engine` — query layer over compiler world (no live behavior)
