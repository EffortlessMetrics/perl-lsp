# PLSP-ADR-0005: HIR-body / PIR-A / EIR boundary terminology

Status: accepted
Date: 2026-06-21
Owner: perl-lsp maintainers
Linked proposal: [PLSP-PROP-0002](../proposals/PLSP-PROP-0002-compiler-program.md)
Linked specs:
- [PLSP-SPEC-0025](../specs/PLSP-SPEC-0025-pir-v0.md)
- [PLSP-SPEC-0030](../specs/PLSP-SPEC-0030-compile-state-layers.md)
Linked plan: [plans/compiler-program/implementation-plan.md](../../plans/compiler-program/implementation-plan.md)
Linked roadmap: [Compiler-backed LSP roadmap](../project/COMPILER_BACKED_LSP_ROADMAP.md)

## Context

`perl-lsp` now has a fixture-backed compiler substrate whose layer set is
contracted in [PLSP-SPEC-0030](../specs/PLSP-SPEC-0030-compile-state-layers.md).
The next phase of substrate build-out requires distinguishing three related but
separate concerns:

1. **HIR body items** — HIR-layer shells for Perl expressions and control-flow
   constructs that are currently `not_yet_modeled` in
   [HIR lowering coverage](../project/status/hir_lowering.md). These are L0
   items in the PLSP-SPEC-0030 stack, not a new IR.

2. **PIR-A** — The evolution of the existing Tooling PIR
   (`crates/perl-parser-core/src/pir/`), contracted by
   [PLSP-SPEC-0025](../specs/PLSP-SPEC-0025-pir-v0.md). PIR-A adds branch,
   loop, return, and read-side lowering to the existing v0 data model while
   remaining a source-anchored tooling IR for editor features. It is not a
   runtime evaluator and does not execute Perl.

3. **EIR** — A future Execution IR that branches off PIR-A when the substrate
   is ready to support bounded compilation or runtime targets. EIR is a separate
   concern from tooling correctness. It is not part of the current compiler-
   substrate lane and has no crate, data model, or planning doc at this time.

Without a canonical record, agents and reviewers working on branch/loop/return
PIR expansion must guess whether they are extending PIR-A or creating EIR. That
confusion risks:
- premature EIR-shaped data model decisions embedded in what should stay PIR-A,
- HIR body shells mislabeled as a new IR layer, and
- PIR PRs drifting outside the PLSP-SPEC-0025 contract without a boundary ADR
  to cite.

## Decision

### HIR body items are L0 HIR, not a new IR

HIR shells for Perl expressions (`Binary`, `Unary`, `Ternary`, etc.) and
unmodeled control-flow constructs are L0 additions under
[PLSP-SPEC-0030 L0](../specs/PLSP-SPEC-0030-compile-state-layers.md#l0--hir-items).
Adding a new HIR shell for `Binary` or adding branch-arm facts to an existing
`BranchShell` is an L0 PR, not a PIR PR. It follows the PLSP-SPEC-0030 C1–C5
obligations and the Valid PR Shapes listed there.

HIR body items must:
- live in `crates/perl-parser-core/src/hir/`
- follow the `HirItem` / `HirKind` type vocabulary
- carry parser anchor, source range, recovery confidence, and package/scope context
- contribute to HIR lowering coverage (the `hir_lowering.md` generated surface)

HIR body items must not:
- introduce new crate-level types that duplicate PIR operation concerns
- claim to lower Perl expressions to machine targets or runtime representations
- skip the `not_yet_modeled` → `lowered` state transition that the generated
  coverage tracks

### PIR-A is the evolution of the existing tooling PIR

The crate `crates/perl-parser-core/src/pir/` and the data model contracted in
[PLSP-SPEC-0025](../specs/PLSP-SPEC-0025-pir-v0.md) is PIR-A. Future PRs that
add branch/loop/return lowering, read-side lowering (`LexicalRead`,
`StashRead`), or broader context propagation are PIR-A PRs. They must stay
within the PLSP-SPEC-0025 contract:

- source-anchored nodes for every source-derived operation
- dynamic-boundary preservation instead of guessing
- lowering receipts with `provider_behavior_changed = false`
- no provider cutover (gated separately by #8197)
- no Perl execution or runtime dependency

PIR-A occupies the "tooling IR" slot in the compiler-backed roadmap pipeline.
It may feed determinism receipts, differential oracle proof, and future provider
fact-source tracing. It does not evaluate Perl expressions.

The term "PIR-A" is the repo-native name for this layer. It is equivalent to
"Tooling PIR," "tooling IR," and "PIR v0 evolution" in other documents. Agents
and reviewers should prefer "PIR-A" when distinguishing from EIR.

### EIR is a future concern and must not be started without a separate ADR

EIR (Execution IR) is the layer in the roadmap that supports bounded
compilation and Rust-native runtime targets. It branches off PIR-A when:

- PIR-A has branch, loop, return, read-side, and context-propagation lowering
- a real-workspace provider receipt proves the tooling substrate is stable
- a separate ADR describes the EIR data model and crate structure

No EIR crate, EIR type, EIR lowering, or EIR-shaped PR is authorized by this
ADR or by PLSP-SPEC-0025. Any PR adding `EirNode`, `EirGraph`, `EirOp`, or
similar types is out of scope until a dedicated EIR ADR is accepted.

### Compiler world extends SemanticSnapshot

The "compiler world" concept in the roadmap refers to extending the existing
`SemanticSnapshot` (and related workspace-level structures in
`crates/perl-semantic-analyzer/`) with the full layered fact graph — scope,
stash, compile environment, import/export, effects, framework adapters — so
providers can query compiled facts instead of raw parse output.

Compiler world is not a new crate. It is the integration layer between the
compiler substrate (PLSP-SPEC-0030 L0–L6 + PIR-A) and the provider-facing
query surface. Compiler world PRs must:
- extend `SemanticSnapshot` or workspace-analysis types, not replace them
- use the provenance, confidence, and dynamic-boundary vocabulary already in
  `perl-semantic-facts`
- not change live provider behavior (provider cutover is gated by #8197)

### Layer order

The canonical layer order from parser output to providers is:

```
Parser AST
  -> HIR items (L0)
  -> HIR bodies (L0 — expressions + control-flow shells)
  -> HIR side graphs: ScopeGraph (L1) + StashGraph (L2) + CompileEnvironment (L3)
     + ImportSpec/ExportSet/VisibleSymbols (L4) + CompileEffect log (L5)
     + FrameworkFactGraph (L6)
  -> PIR-A (source-anchored tooling IR; evolves PLSP-SPEC-0025)
  -> Compiler world (SemanticSnapshot extended with full fact graph)
  -> Abstract compile engine (query layer over compiler world)
  -> LSP providers (fact-source-traced, receipt-gated)
```

EIR branches off PIR-A. The reference interpreter (future, language-compliance
target) branches off EIR. Neither is in the current lane.

## Preconditions for PIR-A Expansion

PIR-A branch/loop/return expansion may begin only after:
- HIR body shells for `If`, `While`, `For`, `Foreach` are broadened enough to
  anchor branch and loop lowering (the existing `BranchShell` and `LoopShell`
  shells count; deeper per-arm expression coverage comes in Phase 2 Expression
  slices)
- PLSP-SPEC-0025 is current and the reserved `Branch`, `Loop`, `Return`,
  `LexicalRead`, `StashRead` families are populated by PR

## Preconditions for EIR Start

EIR work may not begin until:
- PIR-A has documented branch/loop/return/read-side lowering coverage
- a dedicated EIR ADR is proposed, reviewed, and accepted
- the EIR ADR names the crate, data model, dependency contract, and claim limits

## Non-goals

This ADR does not authorize:

- creating an EIR crate or EIR data model
- adding HIR shells for currently `not_yet_modeled` constructs (those are
  governed by PLSP-SPEC-0030 L0 and the HIR lowering plan)
- adding PIR-A operations beyond what PLSP-SPEC-0025 already reserves
- changing provider behavior, live behavior, or provider cutover status
- creating a compiler world crate or abstract compile engine implementation
- claim movement in any generated status surface
- parser/corpus bucket movement or support-tier promotion

## Consequences

Positive consequences:

- agents and reviewers have a single ADR to cite when deciding whether a PIR PR
  is expanding PIR-A or starting EIR
- HIR body PRs and PIR-A PRs use distinct, named identities that can be tracked
  in the goal manifest and plan
- the compiler roadmap's "tooling IR" → "compiler world" → "providers" path is
  named in one place with the correct direction
- EIR cannot start accidentally — it requires its own ADR before any code

Tradeoffs:

- the distinction between HIR body expansion and PIR-A expansion requires agents
  to know which crate they are touching; this is a documentation load, not a
  technical coupling
- the "compiler world" label remains aspirational until a SemanticSnapshot
  extension PR proves it; the ADR names the intent but does not implement it

## Alternatives Considered

### Name PIR as a single artifact without the -A suffix

Rejected. "PIR" alone conflates the current tooling IR with a future execution
IR in roadmap discussions. The -A suffix lets the codebase distinguish the
layers without renaming existing types.

### Define EIR in this ADR

Rejected. EIR is premature. Defining its structure now would invite scope drift
in PIR-A PRs. EIR must wait for its own ADR, grounded in a proven PIR-A baseline.

### Treat "compiler world" as a new crate from the start

Rejected. The compiler world is an integration concern over the existing
SemanticSnapshot and compiler substrate, not a separate crate. A crate split may
follow after the integration is proven, but that is not today's decision.

## Follow-up Obligations

- Keep [PLSP-SPEC-0025](../specs/PLSP-SPEC-0025-pir-v0.md) as the data-model
  contract for PIR-A expansion PRs.
- Keep [PLSP-SPEC-0030](../specs/PLSP-SPEC-0030-compile-state-layers.md) as the
  contract for all L0–L6 HIR body and side-graph PRs.
- File a dedicated EIR ADR before any PR introduces EIR types or crates.
- Update the compiler capability status page when PIR-A slices land.
- Link this ADR from any implementation-plan entry that produces HIR body items,
  PIR-A operations, or compiler world integration work.

## Status Links

- [Compiler capability status](../project/COMPILER_CAPABILITY_STATUS.md)
- [Compiler facts substrate](../project/status/compiler_facts.md)
- [HIR lowering coverage](../project/status/hir_lowering.md)
- [Compiler-backed LSP roadmap](../project/COMPILER_BACKED_LSP_ROADMAP.md)
- [PIR v0 spec](../specs/PLSP-SPEC-0025-pir-v0.md)
- [Compile state layers spec](../specs/PLSP-SPEC-0030-compile-state-layers.md)

## Why ADR-worthy

This is a terminology and boundary decision. It fixes the canonical names for
HIR body items, PIR-A, and EIR, defines when each may start, and establishes
the layer order that compiler-substrate PRs must follow. Without it, HIR body
PRs and PIR-A PRs share no shared vocabulary and EIR can drift into PIR-A.
