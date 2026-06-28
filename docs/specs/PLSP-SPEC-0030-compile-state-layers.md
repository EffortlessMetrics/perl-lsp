# PLSP-SPEC-0030: Compile state layers contract

Status: accepted
Owner: perl-lsp maintainers
Linked proposal: [PLSP-PROP-0001](../proposals/PLSP-PROP-0001-real-perl-editor-trust.md)
Linked specs:
- [PLSP-SPEC-0017](PLSP-SPEC-0017-fact-provenance-and-source-backing.md)
- [PLSP-SPEC-0022](PLSP-SPEC-0022-module-path-authority.md)
- [PLSP-SPEC-0023](PLSP-SPEC-0023-ambient-inputs.md)
- [PLSP-SPEC-0024](PLSP-SPEC-0024-framework-fact-adapters.md)
- [PLSP-SPEC-0025](PLSP-SPEC-0025-pir-v0.md)
- [PLSP-SPEC-0027](PLSP-SPEC-0027-differential-real-perl-oracle.md)
Linked roadmap: [Compiler-backed LSP roadmap](../project/COMPILER_BACKED_LSP_ROADMAP.md)
Linked plan: [Semantic substrate first-wave plan](../project/SEMANTIC_SUBSTRATE_FIRST_WAVE_PLAN.md)
Status impact: compiler fact substrate, compiler capability status, HIR lowering
coverage, semantic scorecard, provider cutover gating

## Purpose

The compiler-backed direction models Perl's compile-time behavior as an ordered
stack of Rust-native fact layers between the parser AST and any LSP provider. The
[compiler-backed roadmap](../project/COMPILER_BACKED_LSP_ROADMAP.md) names these
layers in prose; the [compiler fact substrate](../project/status/compiler_facts.md)
and [compiler capability status](../project/COMPILER_CAPABILITY_STATUS.md) track
their state. This spec is the durable contract that those prose and status
surfaces lean on: it fixes the layer set, the layering direction, the shared
fact obligations, and the claim limits, so reviewers and agents can decide
whether a compile-state PR is in-contract without chat history.

This spec is descriptive of the existing fixture-backed substrate and
prescriptive for future compile-state work. It does not cut any provider over,
broaden any provider behavior, or promote any layer's state.

## Current Implementation Status

The compile-state layers are implemented and fixture-backed in
`crates/perl-parser-core/src/hir/` and the neutral fact vocabulary in
`crates/perl-semantic-facts/`. Lowering is `perl_parser_core::hir::lower_ast`,
which produces a `HirFile` carrying every layer's facts:

| Layer | Canonical owner type(s) | `HirFile` surface |
| --- | --- | --- |
| L0 HIR items | `HirItem`, `HirKind` | `HirFile::items` |
| L1 Scope / pad | `ScopeGraph`, `ScopeFrame`, `ScopeKind`, `Binding`, `StorageClass`, `BindingReference` | `HirFile::scope_graph` |
| L2 Package / stash | `StashGraph`, `PackageStash`, `GlobSlot`, `PackageInheritanceEdge`, `ExportDeclaration`, `StashDynamicBoundary` | `HirFile::stash_graph` |
| L3 Compile environment | `CompileEnvironment`, `PragmaStateFact`, `PragmaEffect`, `IncRootFact`, `ModuleRequest`, `CompilePhaseBlock`, `DynamicBoundary` | `HirFile::compile_environment` |
| L4 Import / export / visible symbols | `ImportSpec`, `ExportSet`, `VisibleSymbol` (`perl-semantic-facts`); projections via `StashGraph::export_sets`, `CompileEnvironment::import_specs` | projected (method) |
| L5 Compile-time effects | `CompileEffect`, `CompileEffectKind`, `CompileEffectSourceKind`, `CompileEffectFactKind`, `COMPILE_EFFECT_MODEL_VERSION` | `HirFile::compile_effects()` (method) |
| L6 Framework adapters | `FrameworkAdapterRegistry`, `FrameworkFactGraph`, `FrameworkExportedSymbolFact` | `HirFile::framework_facts()` (method) |

Layers L0–L3 are stored fields on `HirFile`; layers L4–L6 are projected on
demand by the methods named above (`HirFile::compile_effects()`,
`HirFile::framework_facts()`, and the `StashGraph` / `CompileEnvironment`
projections), not stored fields. Adjacent source-backed tables (`PrototypeTable`,
`BarewordTable`) and the prototype/bareword fact classes are part of the same
substrate and follow the same shared obligations below.

Every layer above is `fixture-backed` per the
[compiler fact substrate](../project/status/compiler_facts.md); none is `live`.
This spec does not change that state.

## Contract

### C1 — Layer set and direction

The compile-state stack is the ordered layer set L0–L6 in the table above. Each
layer may depend only on layers at or below its index, plus the parser AST and
the neutral fact vocabulary in `perl-semantic-facts`. A higher layer must not be
a precondition for lowering a lower one. Lowering produces all layers from a
single `lower_ast` pass over one file; no layer requires running Perl, reading
ambient environment, or resolving across files at lowering time.

### C2 — Shared fact obligations

Every emitted compile-state fact must carry, where the owning type provides the
field:

- a source range or source anchor tying the fact to written code
- provenance under
  [PLSP-SPEC-0017](PLSP-SPEC-0017-fact-provenance-and-source-backing.md)
  (the HIR layers use `CompileProvenance` / `CompileConfidence`; the neutral
  vocabulary uses `Provenance` / `Confidence`)
- confidence
- a dynamic-boundary reason when the layer cannot model the behavior, instead of
  a guessed fact

A fact is source-backed only when it points to a workspace source construct.
Runtime-only, heuristic, stale, partial, or unanchored evidence must be a
dynamic boundary or a low-confidence non-exact fact, never exact provider
behavior.

### C3 — Determinism and versioning

Lowering the same source must produce the same layered facts. The compile-effect
log is the canonical determinism surface: `HirFile::compile_effects` returns
effects in stable source order with contiguous ordinals starting at `0`, and
every effect stamps `model_version == COMPILE_EFFECT_MODEL_VERSION`. Any change
to the effect record shape, ordering, or fact categories must bump
`COMPILE_EFFECT_MODEL_VERSION` and update the alignment proof.

### C4 — No provider cutover from this layer

Compile-state facts are proof data. No provider may consume a compile-state fact
as live behavior under this spec. Provider cutover is gated separately by the
provider-cutover lane and requires its own fact-source tracing, shadow
comparison, and real-workspace receipts. This spec may not be cited to authorize
live behavior, edit-producing behavior, support-tier promotion, or parser/corpus
bucket movement.

### C5 — Dynamic boundaries are first-class

Behavior the stack cannot model — string `eval`, dynamic `require`, symbolic
references, typeglob mutation, `AUTOLOAD`, unmodeled `BEGIN` side effects — must
be recorded as an explicit dynamic boundary (`DynamicBoundary`,
`StashDynamicBoundary`, `CompileEffectKind::EmitDynamicBoundary`, or a
boundary-classified `CompileEnvironmentBoundary`) with a reason. Uncertainty must
be represented, not erased.

## Layer Obligations

### L0 — HIR items

Normalized language constructs over the AST: packages, subs, methods, `use` /
`require`, lexical declarations, calls and method calls, blocks, barewords,
literals, dynamic boundaries. Each `HirItem` keeps a parser anchor, source range,
recovery confidence, and known package/scope context. HIR must not discard parser
recovery state; recovered or partial nodes keep `RecoveryConfidence` other than
`Parsed`.

### L1 — Scope and pad

Models Perl's lexical compilation state: `my`, `our`, `state`, `local`, package
boundaries, block scopes, sub scopes, signature variables, and pragma/feature
visibility. Outputs scope frames (`ScopeFrame` / `ScopeKind`), lexical and
package bindings (`Binding` / `StorageClass`), shadowing relationships, and local
reference facts (`BindingReference`). `local` is recorded as
`StorageClass::LocalizedPackage`; it is a localization fact, not a new lexical
binding. This layer is intended to unlock safer local rename, references,
completion, and diagnostics once cutover is separately proven — but unlocks
nothing live under this spec.

### L2 — Package and stash

Models package symbol tables as first-class state: package declarations, sub
declarations, `our` variables, typeglob slots (`GlobSlot`), simple aliases,
`@ISA` edges (`PackageInheritanceEdge`), constant subs, static `Exporter`
declarations (`ExportDeclaration`), and `AUTOLOAD` boundaries. Dynamic stash
mutation is a `StashDynamicBoundary`, not a guessed slot.

### L3 — Compile environment

Models compile-time effects of pragmas and module configuration: `strict`,
`warnings`, `feature`, `lib`, include roots (`IncRootFact`), module requests
(`ModuleRequest`), and phase blocks (`CompilePhaseBlock` / `CompilePhase`).
Pragma state is queryable lexically via `CompileEnvironment::pragma_state_at`.
Include-root and module-request facts are compiler-substrate data and must not
spawn Perl or read ambient environment; module-path authority follows
[PLSP-SPEC-0022](PLSP-SPEC-0022-module-path-authority.md) and ambient-input rules
follow [PLSP-SPEC-0023](PLSP-SPEC-0023-ambient-inputs.md).

### L4 — Import, export, visible symbols

Projects canonical `ImportSpec`, `ExportSet`, and `VisibleSymbol` values from the
lower layers. `visible_symbols_at` over these facts is the canonical proof that
the stack can answer "what is in scope here" without a provider. Static
`Exporter` / `Exporter::Tiny` cases are modeled; non-literal or runtime import
behavior is a dynamic boundary.

### L5 — Compile-time effect log

A bounded, ordered log linking source constructs to the compiler state mutations
they cause and the fact categories emitted. `CompileEffect` records the effect
kind, source kind, fact kind, range, scope, package context, dynamic-boundary
reason, provenance, confidence, and `model_version`. The log is the contract's
determinism and freshness surface (C3). Safe evaluation of modeled effects may be
added later; unsupported behavior must emit a dynamic boundary, never a guessed
fact.

### L6 — Framework adapters

Framework facts are emitted through the shared registry
(`FrameworkAdapterRegistry` / `FrameworkFactGraph`), never provider-local special
casing, and are governed in full by
[PLSP-SPEC-0024](PLSP-SPEC-0024-framework-fact-adapters.md). This layer sits above
the import/export layer and reuses its provenance and boundary rules.

## Valid PR Shapes

Valid PRs under this spec include:

- broadening one layer's fixtures (for example scope-shadow, typeglob,
  inheritance, pragma-visibility, or signature-variable fixtures)
- adding a new fact field with provenance, confidence, and an anchor to an
  existing layer
- adding a new `CompileEffectKind` / `CompileEffectFactKind` variant with a
  `COMPILE_EFFECT_MODEL_VERSION` bump and updated alignment proof
- adding a layer-local query (for example a pragma-state or visible-symbol
  query) that emits facts, not provider behavior
- recording a new dynamic-boundary class for behavior the stack cannot model
- documentation and status updates that keep this spec, the roadmap, and the
  compiler fact substrate aligned

Every compile-state PR must name the one layer it touches, the fact obligations
it preserves (C2), and the determinism impact (C3).

## Invalid PR Shapes

Invalid PRs include:

- a provider consuming a compile-state fact as live behavior under this spec
- a higher layer becoming a precondition for lowering a lower layer
- a fact emitted without a source anchor, provenance, or confidence where the
  owning type provides those fields
- runtime, heuristic, stale, or unanchored evidence surfaced as an exact fact
- erasing dynamic Perl uncertainty instead of emitting a dynamic boundary
- changing effect-record shape, ordering, or categories without a
  `COMPILE_EFFECT_MODEL_VERSION` bump
- running Perl, `perldoc`, DAP, or application code to discover compile-state
  facts at lowering time
- support-tier, parser-bucket, PIR, determinism, or corpus promotion claimed
  from a compile-state fixture alone

## Acceptance

A compile-state PR satisfies this spec when:

- it touches one named layer and keeps the L0–L6 dependency direction (C1)
- every new or changed fact preserves the shared obligations (C2)
- the compile-effect log stays deterministic and correctly versioned (C3)
- no provider behavior changes from the PR (C4)
- unmodeled behavior is recorded as a dynamic boundary with a reason (C5)
- fixtures or the alignment proof cover the change

## Proof Commands

Layer fixtures and the spec-alignment proof live in `perl-parser-core`:

```bash
cargo test -p perl-parser-core --test hir_tests --locked
cargo test -p perl-parser-core --test compile_state_layers_spec_alignment --locked
```

Coverage and scorecard freshness for compile-state lanes:

```bash
cargo xtask metrics hir-coverage --check
cargo xtask semantic-scorecard --check
cargo xtask semantic-shadow-compare --check
```

Docs-only changes to this spec may use:

```bash
cargo xtask ci-hygiene check-doc-paths docs/specs
cargo xtask ci-hygiene check-doc-paths docs/project/status
git diff --check
```

## Non-goals

- No provider behavior change from this spec alone.
- No provider cutover; that is gated separately.
- No support-tier, parser-bucket, PIR, or corpus promotion.
- No real-Perl execution, `perldoc`, DAP launch, or application probing at
  lowering time.
- No cross-file resolution requirement at lowering time.
- No erasure of dynamic Perl uncertainty.

## Claim Boundaries

This spec may claim that the compile-state layers form an ordered, single-pass,
deterministic fact stack with shared provenance, confidence, and dynamic-boundary
obligations. It may not claim that any layer is `live`, that any provider
consumes its facts in normal behavior, or that any layer is exact, edit-safe,
runtime-complete, or cross-file-resolved unless separate receipts, status rows,
and promotion gates make that claim.
