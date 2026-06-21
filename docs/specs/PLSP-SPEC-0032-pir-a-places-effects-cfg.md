# PLSP-SPEC-0032: PIR-A place, effect, and CFG contract

Status: draft
Owner: perl-lsp maintainers
Linked proposal: [PLSP-PROP-0001](../proposals/PLSP-PROP-0001-real-perl-editor-trust.md)
Linked program: compiler program — canonical current-main roadmap ([#2559](https://github.com/EffortlessMetrics/perl-lsp/issues/2559), authored in parallel)
Linked boundary ADR: HIR body / PIR-A / EIR boundary ADR ([#2564](https://github.com/EffortlessMetrics/perl-lsp/issues/2564), authored in parallel)
Linked specs:
- [PLSP-SPEC-0025](PLSP-SPEC-0025-pir-v0.md)
- [PLSP-SPEC-0030](PLSP-SPEC-0030-compile-state-layers.md)
- [PLSP-SPEC-0031](PLSP-SPEC-0031-context-and-operator-semantics.md)
- [PLSP-SPEC-0035](PLSP-SPEC-0035-executable-profile-and-eir.md)
Linked issues:
- [#2559](https://github.com/EffortlessMetrics/perl-lsp/issues/2559) — compiler program tracker
- [#2564](https://github.com/EffortlessMetrics/perl-lsp/issues/2564) — boundary ADR (PIR-A vs EIR)
- [#2194](https://github.com/EffortlessMetrics/perl-lsp/issues/2194) — corrected: PIR-A is an evolution of PIR v0, not a fresh PirFunction/BasicBlock rewrite
- [#2269](https://github.com/EffortlessMetrics/perl-lsp/issues/2269) — corrected: DualValue/value model belongs to EIR, not PIR-A
- [#2270](https://github.com/EffortlessMetrics/perl-lsp/issues/2270) — PIR Place model
Linked roadmap: [Compiler-backed LSP roadmap](../project/COMPILER_BACKED_LSP_ROADMAP.md)
Status impact: PIR fact substrate, PIR receipt schema, control-flow facts,
safe-delete / rename-safety analysis inputs

## Purpose

PIR-A ("PIR — analysis") is the static-analysis intermediate representation for
editor tooling: places, effects, and control flow with honest dynamic
boundaries. It is the **schema-versioned evolution of the existing tooling PIR
v0** ([PLSP-SPEC-0025](PLSP-SPEC-0025-pir-v0.md)), already implemented in
[`crates/perl-parser-core/src/pir/`](../../crates/perl-parser-core/src/pir/) with
its receipt version constant
[`PIR_RECEIPT_VERSION`](../../crates/perl-parser-core/src/pir/model.rs) at
[`pir/model.rs:20`](../../crates/perl-parser-core/src/pir/model.rs).

PIR-A is **not** a from-scratch `PirFunction`/`BasicBlock`/`Instruction`/
`Terminator` rewrite, and it is **not** the execution IR. It corrects three
issues:

- [#2194](https://github.com/EffortlessMetrics/perl-lsp/issues/2194): PIR-A
  extends the existing `PirGraph`/`PirNode`/`PirEdge` model under a bumped
  `PIR_RECEIPT_VERSION`; it does not replace it with a new function/block IR.
- [#2270](https://github.com/EffortlessMetrics/perl-lsp/issues/2270): PIR-A adds
  a real `PlaceKind` model, replacing the conflated `PirContext::Lvalue` variant.
- [#2269](https://github.com/EffortlessMetrics/perl-lsp/issues/2269): runtime
  value modeling (DualValue, cells, heaps) is **EIR's** concern
  ([PLSP-SPEC-0035](PLSP-SPEC-0035-executable-profile-and-eir.md)), not PIR-A's.

This spec is descriptive of the implemented PIR v0 substrate and prescriptive
for the PIR-A evolution. It changes no provider behavior.

## Contract

### C1 — PIR-A is a schema evolution, not a rewrite

PIR-A keeps PIR v0's identity model (`PirId`, internally deterministic per
receipt), source-anchor model (`PirSourceAnchor`, `PirAnchorKind`), and
graph/edge shape (`PirGraph`, `PirNode`, `PirEdge`). New analysis facts are
additive fields and additive operation/edge variants. Any change to the node,
edge, place, effect, or receipt shape **must** bump `PIR_RECEIPT_VERSION`
([`pir/model.rs:20`](../../crates/perl-parser-core/src/pir/model.rs)) and update
the receipt and any alignment proof. Reserved `PirOperation` families already
present (`Branch`, `Loop`, `Return`, `LexicalRead`, `StashRead` —
[`pir/model.rs:290`](../../crates/perl-parser-core/src/pir/model.rs)) are
populated by PIR-A passes without a model break.

### C2 — Place model

A place is a location that can be read, written, modified, aliased, or
localized. The conflated `PirContext::Lvalue` variant
([`pir/model.rs:152`](../../crates/perl-parser-core/src/pir/model.rs)) is retired
in favor of an explicit place kind:

```rust
pub enum PlaceKind {
    Lexical { name: LexicalName, scope: HirScopeId },
    PackageSlot { symbol: SymbolName },
    ArrayElement { base: PlaceId, index: PirId },   // index evaluated once
    HashElement  { base: PlaceId, key: PirId },     // key evaluated once
    GlobSlot { symbol: SymbolName, slot: GlobSlotKind }, // *foo{SCALAR}, etc.
    Dereferenced { reference: PirId },               // @$ref, %$ref, $$ref
    Dynamic { reason: String },                       // symbolic / runtime place → boundary
}
```

`AccessMode` (from [PLSP-SPEC-0031](PLSP-SPEC-0031-context-and-operator-semantics.md)
C1: `Read`, `Write`, `ReadModifyWrite`, `Alias`, `Localize`) annotates each place
use. A place whose location is not statically provable (symbolic reference,
runtime-computed slot name, dynamic package) is `PlaceKind::Dynamic` and must
also emit a dynamic-boundary node, never a guessed concrete place.

### C3 — Place operations

PIR-A models these operations over places. Each names how it touches the place:

| Operation | Meaning | Place evaluation |
| --- | --- | --- |
| `Read` | read the value at the place | place computed once |
| `Write` | store a value into the place | place computed once |
| `Modify` | read-modify-write (`+=`, `++`, `chomp`, `substr` lvalue) | **place evaluated exactly once**, then read and written |
| `Alias` | bind a name/element to the same place (`foreach`, `@_`, signature aliasing, `\` of an lvalue) | place computed once; subsequent access shares it |
| `Localize` | dynamic save of the place's current value (`local`) | place computed once; save recorded |
| `Restore` | dynamic restore at scope exit (the matching end of `Localize`) | same place as its `Localize` |
| `TakeReference` | `\PLACE` — produce a reference to the place without reading its value | place computed once; no read effect |

The "evaluate place once" rule for `Modify` is a hard contract: `$h{f()} += 1`
calls `f()` exactly once, and PIR-A must model the index/key sub-expression as a
single evaluated node shared by the read and write, not duplicated.

### C4 — Effect model

Each PIR-A node carries the effects it performs, so analyses (dead code, safe
delete, rename safety, purity) can reason without re-deriving them:

```rust
pub enum Effect {
    ReadPlace(PlaceId),
    WritePlace(PlaceId),
    AliasPlace { from: PlaceId, to: PlaceId },
    LocalizePlace(PlaceId),
    CallEffect { callee: PirCallee, may_be_pure: bool },
    DynamicBoundary { kind: PirDynamicBoundaryKind, reason: String },
}
```

Effects are conservative: when an operation's effect cannot be proven (call to an
unknown sub, dynamic dispatch, `eval`), it carries a `DynamicBoundary` effect and
must not be reported as pure or side-effect-free.

### C5 — CFG is a derived structural view

The control-flow graph is a **derived view over the node/edge model**, not a
separately authored block IR. PIR v0 already carries CFG edges (`PirEdge`,
`PirEdgeKind { Fallthrough, Branch, Loop, Return, DynamicExit, Unknown }` at
[`pir/model.rs:411`](../../crates/perl-parser-core/src/pir/model.rs) and
`PirGraph::edges` at
[`pir/model.rs:533`](../../crates/perl-parser-core/src/pir/model.rs)). PIR-A
computes basic blocks, dominators, and reachability **from** these edges on
demand. Blocks are a structural projection; they are not the storage model and do
not get their own authored identity that can drift from the node graph. Missing
or unprovable edges are `PirEdgeKind::Unknown` or `DynamicExit`, never dropped.

### C6 — Verifier rules

A PIR-A graph is well-formed only when a verifier can confirm:

- every `PirEdge` endpoint references a node in the same graph
- every `Modify` place is evaluated exactly once (no duplicated index/key node)
- every `Localize` has a matching `Restore` on every CFG path that leaves its
  scope, or a `DynamicExit` explaining why not
- every `PlaceKind::Dynamic` co-occurs with a dynamic-boundary node
- every source-derived node has a `PirSourceAnchor` per
  [PLSP-SPEC-0025](PLSP-SPEC-0025-pir-v0.md)
- the derived CFG (C5) is consistent with the stored edges (no block referencing
  a non-existent edge)
- the receipt's `PIR_RECEIPT_VERSION` matches the emitted shape

Verifier failures are receipt-visible, not silent.

## Valid PR Shapes

Valid PRs under this spec include:

- adding the `PlaceKind` / `PlaceId` model and migrating `PirContext::Lvalue`
  consumers to it, with a `PIR_RECEIPT_VERSION` bump
- populating one reserved `PirOperation` family (`Branch`, `Loop`, `Return`,
  `LexicalRead`, `StashRead`) into the graph
- adding place operations (`Modify`/`Alias`/`Localize`/`Restore`/`TakeReference`)
  with the evaluate-once rule and tests
- adding the `Effect` model to nodes
- adding a derived-CFG view (basic blocks, dominators) over existing edges
- adding verifier rules and receipt-visible verifier output
- documentation that keeps PIR-A distinct from EIR

Every PIR-A PR must name the operation/place/effect family it touches, the
`PIR_RECEIPT_VERSION` impact, the dynamic-boundary rule, and confirm it adds no
runtime value model and no provider behavior.

## Invalid PR Shapes

Invalid PRs include:

- replacing `PirGraph`/`PirNode`/`PirEdge` with a fresh `PirFunction`/
  `BasicBlock`/`Instruction`/`Terminator` IR (the corrected
  [#2194](https://github.com/EffortlessMetrics/perl-lsp/issues/2194))
- adding runtime values (`DualValue`, cells, heaps) to PIR-A (belongs to EIR;
  corrected [#2269](https://github.com/EffortlessMetrics/perl-lsp/issues/2269))
- duplicating a `Modify` place's index/key sub-expression instead of evaluating
  it once
- emitting a concrete `PlaceKind` for a symbolic/dynamic place instead of
  `Dynamic` + boundary
- authoring basic blocks as primary storage that can drift from the node graph
- changing node/edge/place/effect/receipt shape without a `PIR_RECEIPT_VERSION`
  bump
- changing provider behavior from a PIR-A change alone

## Acceptance

A PR satisfies this spec when:

- it extends the existing PIR model rather than replacing it, with a correct
  `PIR_RECEIPT_VERSION` bump where shape changes
- places use `PlaceKind` with an `AccessMode`, and `Lvalue`-as-context is gone
- `Modify`/`Alias`/`Localize`/`Restore`/`TakeReference` follow the evaluate-once
  and matched-restore rules
- effects are conservative and dynamic boundaries are preserved
- the CFG is a derived view consistent with stored edges
- the verifier confirms well-formedness and the receipt reports it

## Proof Commands

Docs-only changes to this spec may use:

```bash
cargo xtask ci-hygiene check-doc-paths docs/specs
cargo xtask ci-hygiene check-doc-paths docs/project/status
git diff --check
```

Implementation PRs must add focused PIR-A lowering and verifier tests and run:

```bash
cargo test -p perl-parser-core --locked
```

and, once a PIR receipt validator exists:

```bash
cargo xtask check-pir-receipts
```

## Non-goals

- No runtime value model, cells, heap, or execution in PIR-A (that is EIR;
  [PLSP-SPEC-0035](PLSP-SPEC-0035-executable-profile-and-eir.md)).
- No provider behavior change from this spec alone.
- No HIR replacement; HIR stays the canonical syntax/body surface.
- No determinism or oracle claim.
- No fresh function/block IR rewrite.

## Claim Boundaries

This spec may claim that PIR-A is a schema-versioned analysis evolution of PIR
v0 with a place model, effect model, derived CFG, and verifier. It may not claim
PIR-A is runtime-capable, executes Perl, models runtime values, replaces HIR, or
drives provider behavior until separate code, receipts, and status rows exist.
