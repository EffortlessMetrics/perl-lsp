# PLSP-SPEC-0034: Compiler-world contract

Status: draft
Owner: perl-lsp maintainers
Linked proposal: [PLSP-PROP-0001](../proposals/PLSP-PROP-0001-real-perl-editor-trust.md)
Linked program: compiler program — canonical current-main roadmap ([#2559](https://github.com/EffortlessMetrics/perl-lsp/issues/2559), authored in parallel)
Linked boundary ADR: HIR body / PIR-A / EIR boundary ADR ([#2564](https://github.com/EffortlessMetrics/perl-lsp/issues/2564), authored in parallel)
Linked specs:
- [PLSP-SPEC-0022](PLSP-SPEC-0022-module-path-authority.md)
- [PLSP-SPEC-0023](PLSP-SPEC-0023-ambient-inputs.md)
- [PLSP-SPEC-0024](PLSP-SPEC-0024-framework-fact-adapters.md)
- [PLSP-SPEC-0030](PLSP-SPEC-0030-compile-state-layers.md)
- [PLSP-SPEC-0032](PLSP-SPEC-0032-pir-a-places-effects-cfg.md)
- [PLSP-SPEC-0035](PLSP-SPEC-0035-executable-profile-and-eir.md)
Linked issues:
- [#2559](https://github.com/EffortlessMetrics/perl-lsp/issues/2559) — compiler program tracker
- [#2564](https://github.com/EffortlessMetrics/perl-lsp/issues/2564) — boundary ADR
- [#2425](https://github.com/EffortlessMetrics/perl-lsp/issues/2425) — corrected: compiler-owned cross-file world model (no parser-core cross-file resolution; SCCs not a topological compile_order)
Linked roadmap: [Compiler-backed LSP roadmap](../project/COMPILER_BACKED_LSP_ROADMAP.md)
Status impact: workspace fact substrate, snapshot/generation layer, module
resolution, cache identity, invalidation, compile-effect orchestration

## Purpose

Per-file facts are not enough: Perl's compile-time behavior is cross-file
(`use`/`require`/`@ISA`/`import`/generated symbols) and order-sensitive
(`BEGIN`, compile-time vs runtime `require`). This spec defines the
**compiler world** — the workspace-level orchestration layer that owns cross-file
state — as an **extension of the existing per-request semantic snapshot**, not a
new parallel system.

The repo already has the per-file fact shard
(`FileFactShard`, with `producer_schema_version` set from
`PRODUCER_SCHEMA_VERSION`,
[`crates/perl-workspace/src/semantic/facts.rs`](../../crates/perl-workspace/src/semantic/facts.rs)
and [`crates/perl-workspace/src/workspace/workspace_index.rs:1140`](../../crates/perl-workspace/src/workspace/workspace_index.rs))
and a generation-numbered, atomically-published, overlay-supporting,
one-snapshot-per-request semantic snapshot layer (the snapshot layer of
[#1601](https://github.com/EffortlessMetrics/perl-lsp/issues/1601), referenced at
[`workspace_index.rs:1150`](../../crates/perl-workspace/src/workspace/workspace_index.rs)).
The compiler world extends that snapshot with cross-file structure.

It corrects [#2425](https://github.com/EffortlessMetrics/perl-lsp/issues/2425):
cross-file resolution is owned by the **workspace/compiler orchestration layer**,
not rebuilt inside `perl-parser-core`; and module dependency cycles are modeled
as **strongly-connected components (SCCs)**, not flattened into a single
topological `compile_order`.

## Contract

### C1 — Extend the semantic snapshot, do not fork it

The compiler world is carried by the existing semantic snapshot: generation-
numbered, atomically published, overlay-aware, one-per-request. New cross-file
structures are additive members of that snapshot, stamped with the snapshot's
generation. There is no second, separately-versioned world object that can drift
from the published snapshot. A request reads exactly one snapshot generation for
all of its cross-file answers.

### C2 — Phase-labeled module graph (SCCs, not a compile_order)

The module graph records cross-file dependency edges, each labeled by **kind**
and **phase**:

```rust
pub enum ModuleEdgeKind {
    Use, Require, Parent, Base, Import, Generated,
}
pub enum CompileTimePhase {
    Compile,  // resolved at compile time (use, BEGIN-time require)
    Runtime,  // resolved at runtime (string require, runtime use of a name)
    Unknown,  // phase not statically provable → dynamic boundary
}
```

Cycles are first-class: the graph is condensed into **strongly-connected
components**. A mutually-recursive `use`/`require` cycle is one SCC, modeled
honestly, **not** an error and **not** forced into a linear `compile_order`. The
old "topological compile_order" framing is retired — a topo order does not exist
for cyclic module graphs, and pretending it does silently drops edges. Analyses
that need an order use the SCC-condensed DAG and treat each SCC as a unit.

### C3 — Compile-state snapshot

The world carries a compile-state snapshot: the cross-file projection of the
per-file compile-state layers ([PLSP-SPEC-0030](PLSP-SPEC-0030-compile-state-layers.md))
into a workspace view — visible symbols across files, resolved imports/exports,
`@ISA` composition, generated-member provenance — stamped with the snapshot
generation. Per-file facts feed it; it does not re-lower files.

### C4 — Dependency and invalidation graph

The world carries a dependency/invalidation graph mapping each file to the files
whose facts depend on it. When a file changes, only the dependent set is
invalidated and recomputed; the new world is published as a new generation
(atomic publish, C1). Invalidation is conservative across dynamic boundaries: a
file behind an `Unknown`-phase or dynamic edge invalidates its dependents
conservatively rather than assuming independence.

### C5 — Cache identity

Every cached cross-file result carries a **cache identity** = (snapshot
generation, the content hashes / `file_id`s of the inputs that produced it,
`producer_schema_version`, and the relevant model/effect version constants). A
cached result is reused only when its full identity matches; a `producer_schema_version`
or model-version change invalidates the cache. Cache identity is the freshness
contract for cross-file answers (analogous to the per-file
`content_hash`/`producer_schema_version` already on `FileFactShard`).

### C6 — Compile-effect algebra

Cross-file compile-time effects are classified and given a treatment, so the
world knows how to incorporate each effect rather than guessing:

```rust
pub enum EffectClass {
    SourceState,        // edits to source text / file set
    SemanticState,      // changes to derived semantic facts
    ResolutionState,    // module/symbol resolution changes
    SyntaxAffecting,    // compile-time effect that changes how later code parses
    ExternalCapability, // needs an external capability (filesystem, modules)
    ArbitraryCode,      // BEGIN/eval-time arbitrary code execution
}
pub enum EffectTreatment {
    ApplyMonotonically,     // fold into world state in source order
    ApplyToScopedSnapshot,  // apply within a scoped/overlay snapshot only
    ResolveInCompilerWorld, // resolve cross-file in this layer (not parser-core)
    RequestParserReplay,    // re-parse affected source under new syntax state
    AbstractlyEvaluate,     // model the effect abstractly within the profile
    Deny,                   // refuse (outside the executable profile)
    EmitDynamicBoundary,    // record an honest boundary instead of guessing
}
```

The pairing of class to treatment is the contract: e.g. a `SyntaxAffecting`
effect (a `use` that changes parsing of later code) is treated with
`RequestParserReplay`; an `ArbitraryCode` `BEGIN` block outside the profile is
`Deny` or `EmitDynamicBoundary`; a `ResolutionState` effect is
`ResolveInCompilerWorld`. No effect is silently ignored; every effect maps to a
treatment or to a dynamic boundary.

### C7 — Layer ownership: per-file vs cross-file

`perl-parser-core` makes **per-file** facts only (one `lower_ast` pass per file,
per [PLSP-SPEC-0030](PLSP-SPEC-0030-compile-state-layers.md) C1). The
**workspace/compiler orchestration** layer owns **all cross-file** work: the
module graph, SCC condensation, cross-file resolution, the compile-state
snapshot, invalidation, cache identity, and the effect algebra. Cross-file
resolution must not be rebuilt inside `perl-parser-core` (the corrected
[#2425](https://github.com/EffortlessMetrics/perl-lsp/issues/2425)). Module-path
authority follows [PLSP-SPEC-0022](PLSP-SPEC-0022-module-path-authority.md);
ambient inputs follow [PLSP-SPEC-0023](PLSP-SPEC-0023-ambient-inputs.md).

## Valid PR Shapes

Valid PRs under this spec include:

- adding the phase-labeled module-graph type with SCC condensation
- adding the compile-state snapshot projection over per-file facts
- adding the dependency/invalidation graph and generation-stamped publish
- adding cache identity and a freshness check for one cross-file result class
- adding one `EffectClass`/`EffectTreatment` pairing with tests
- documentation that keeps per-file vs cross-file ownership distinct

Every compiler-world PR must name the cross-file structure it touches, confirm it
extends the existing snapshot (not a fork), confirm no cross-file resolution is
added to `perl-parser-core`, and state its invalidation/cache-identity impact.

## Invalid PR Shapes

Invalid PRs include:

- rebuilding cross-file resolution inside `perl-parser-core` (corrected
  [#2425](https://github.com/EffortlessMetrics/perl-lsp/issues/2425))
- flattening a cyclic module graph into a single topological `compile_order`
- adding a second world object not stamped with the snapshot generation
- reusing a cached cross-file result without a full cache-identity match
- ignoring a compile-time effect instead of mapping it to a treatment or
  dynamic boundary
- running Perl, `perldoc`, or arbitrary `BEGIN`/`eval` code to resolve cross-file
  facts (outside the profile; see
  [PLSP-SPEC-0035](PLSP-SPEC-0035-executable-profile-and-eir.md))
- changing provider behavior from a world change alone

## Acceptance

A PR satisfies this spec when:

- cross-file structure is carried by the existing generation-numbered snapshot
- the module graph is phase-labeled and cycles are SCCs (no topo compile_order)
- invalidation is generation-stamped and conservative across dynamic boundaries
- cached results carry and check a full cache identity
- each compile-time effect maps to an `EffectTreatment` or a dynamic boundary
- `perl-parser-core` gains no cross-file resolution

## Proof Commands

Docs-only changes to this spec may use:

```bash
cargo xtask ci-hygiene check-doc-paths docs/specs
cargo xtask ci-hygiene check-doc-paths docs/project/status
git diff --check
```

Implementation PRs add focused tests for the touched cross-file structure and
run the owning crate's checks (for example `cargo test -p perl-workspace --locked`).

## Non-goals

- No provider behavior change from this spec alone.
- No cross-file resolution in `perl-parser-core`.
- No topological `compile_order` for cyclic graphs.
- No real-Perl execution, `perldoc`, or `BEGIN`/`eval` execution to resolve
  facts (see [PLSP-SPEC-0035](PLSP-SPEC-0035-executable-profile-and-eir.md)).
- No determinism or oracle claim beyond
  [PLSP-SPEC-0026](PLSP-SPEC-0026-determinism-receipt-v1.md) and
  [PLSP-SPEC-0027](PLSP-SPEC-0027-differential-real-perl-oracle.md).

## Claim Boundaries

This spec may claim that cross-file state lives in a generation-numbered,
atomically-published compiler world extending the semantic snapshot, with a
phase-labeled SCC module graph, dependency/invalidation graph, cache identity,
and a compile-effect algebra, and that per-file facts stay in `perl-parser-core`.
It may not claim the world is implemented, that any provider consumes it as live
behavior, or that any compile-time effect is executed, until separate code,
receipts, and status rows make that claim.
