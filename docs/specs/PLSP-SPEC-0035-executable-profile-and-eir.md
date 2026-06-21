# PLSP-SPEC-0035: Executable-profile charter and EIR contract

Status: draft
Owner: perl-lsp maintainers
Linked proposal: [PLSP-PROP-0001](../proposals/PLSP-PROP-0001-real-perl-editor-trust.md)
Linked program: compiler program — canonical current-main roadmap ([#2559](https://github.com/EffortlessMetrics/perl-lsp/issues/2559), authored in parallel)
Linked boundary ADR: HIR body / PIR-A / EIR boundary ADR ([#2564](https://github.com/EffortlessMetrics/perl-lsp/issues/2564), authored in parallel)
Linked specs:
- [PLSP-SPEC-0023](PLSP-SPEC-0023-ambient-inputs.md)
- [PLSP-SPEC-0027](PLSP-SPEC-0027-differential-real-perl-oracle.md)
- [PLSP-SPEC-0031](PLSP-SPEC-0031-context-and-operator-semantics.md)
- [PLSP-SPEC-0032](PLSP-SPEC-0032-pir-a-places-effects-cfg.md)
- [PLSP-SPEC-0034](PLSP-SPEC-0034-compiler-world.md)
Linked issues:
- [#2559](https://github.com/EffortlessMetrics/perl-lsp/issues/2559) — compiler program tracker
- [#2565](https://github.com/EffortlessMetrics/perl-lsp/issues/2565) — two-layer IR (PIR-A vs EIR)
- [#2413](https://github.com/EffortlessMetrics/perl-lsp/issues/2413) — corrected: interpreter foundations bounded by an executable profile; EIR distinct from PIR-A
- [#2269](https://github.com/EffortlessMetrics/perl-lsp/issues/2269) — corrected: runtime values use handle types, not `Rc<RefCell>` as the contract
Linked roadmap: [Compiler-backed LSP roadmap](../project/COMPILER_BACKED_LSP_ROADMAP.md)
Status impact: executable profile boundary, EIR contract, bounded compile-time
evaluation, differential-oracle thresholds

## Purpose

Some compiler-grade answers require *evaluating* Perl, not just analyzing it
(constant folding, modeled `BEGIN` effects, bounded compile-time `eval`). Doing
this safely requires two things this spec defines:

1. An **executable profile** — a versioned charter stating exactly which Perl is
   supported for evaluation, and where evaluation must fail closed.
2. An **EIR** (execution IR) contract — the runtime representation, kept
   **distinct from PIR-A** ([PLSP-SPEC-0032](PLSP-SPEC-0032-pir-a-places-effects-cfg.md)).

It corrects two issues:

- [#2413](https://github.com/EffortlessMetrics/perl-lsp/issues/2413):
  interpreter foundations are bounded by the executable profile and live in EIR,
  not in PIR-A.
- [#2269](https://github.com/EffortlessMetrics/perl-lsp/issues/2269): runtime
  values are referenced through **handle types** (`CellId`, `ValueId`,
  `RuntimeHeap`), not `Rc<RefCell<...>>` as the public contract.

This spec defines the profile and EIR contract. It does **not** ship an
interpreter, change provider behavior, or claim Perl compatibility.

## Contract — Executable profile

### C1 — The profile is versioned and explicit

The executable profile is a versioned document declaring what evaluation
supports. Every evaluation receipt stamps the profile version. A construct,
builtin, or capability is supported for evaluation **only if the profile lists
it**; everything else fails closed (C3). The profile declares:

- supported Perl version(s) and the supported subset of language constructs
- supported builtins (and the behavior contract for each)
- supported regex subset
- module policy (which modules may be loaded / modeled)
- filesystem policy, I/O policy, environment-variable policy
- dynamic-boundary behavior (what produces a boundary vs. an evaluation)
- warnings and exceptions behavior
- determinism guarantees
- differential-oracle agreement thresholds for the profiled subset

### C2 — Initial profile (v1)

The initial executable profile is deliberately narrow:

- **pure Perl only** — no XS, no source filters
- **static modules only** — modules whose facts are resolvable without running
  arbitrary code; no ambient module loading
- **no ambient I/O** — no filesystem, network, or environment reads during
  evaluation; ambient inputs follow
  [PLSP-SPEC-0023](PLSP-SPEC-0023-ambient-inputs.md)
- **bounded compile-time execution** — `BEGIN`/`eval` modeling is step- and
  time-bounded; unbounded or non-terminating evaluation fails closed
- **bounded regex** — a bounded regex subset; constructs outside it are a
  boundary
- **explicit failure at unsupported boundaries** — reaching any construct,
  builtin, module, or capability outside the profile produces an explicit
  failure / dynamic boundary, never a guessed result

### C3 — Fail closed at the boundary

Evaluation that reaches outside the profile **stops and reports** — it emits a
dynamic boundary (consistent with
[PLSP-SPEC-0030](PLSP-SPEC-0030-compile-state-layers.md) C5 and the compiler
world's `EmitDynamicBoundary`/`Deny` treatments in
[PLSP-SPEC-0034](PLSP-SPEC-0034-compiler-world.md) C6). It never fabricates a
value, never silently degrades to a partial result presented as exact, and never
loads or runs anything the profile does not list.

## Contract — EIR

### C4 — EIR is execution IR, distinct from PIR-A

EIR represents *running* Perl: runtime values, cells, stack frames, blocks and
terminators, exceptions, builtins, and regex execution. It is a **separate IR
from PIR-A**. PIR-A
([PLSP-SPEC-0032](PLSP-SPEC-0032-pir-a-places-effects-cfg.md)) models places,
effects, and control flow for *analysis*; EIR models *evaluation*. The two-layer
split is the corrected [#2565](https://github.com/EffortlessMetrics/perl-lsp/issues/2565)
and [#2413](https://github.com/EffortlessMetrics/perl-lsp/issues/2413): runtime
value modeling lives in EIR, never in PIR-A
([#2269](https://github.com/EffortlessMetrics/perl-lsp/issues/2269)).

### C5 — Runtime values are handles, not `Rc<RefCell>`

The EIR runtime addresses values and mutable cells through **handle types**, not
shared interior-mutability smart pointers, as the public contract:

```rust
pub struct ValueId(u32);    // a runtime value
pub struct CellId(u32);     // a mutable container (scalar/array/hash slot)
pub struct RuntimeHeap { /* owns the storage CellId/ValueId index into */ }
```

`Rc<RefCell<...>>` may appear as a private implementation detail, but the
contract — the type that crosses module/API boundaries and appears in receipts —
is the handle. This keeps EIR values serializable, comparable for the
differential oracle, and free of aliasing/borrow hazards as a public surface.
Aliasing (Perl's `\`, `foreach`, `@_`) is modeled as two names resolving to the
same `CellId`, not as a cloned `Rc`.

### C6 — EIR shape

EIR carries, at minimum:

- runtime values and cells via `ValueId` / `CellId` over a `RuntimeHeap`
- stack frames (call frames with `@_` aliasing, `wantarray` context per
  [PLSP-SPEC-0031](PLSP-SPEC-0031-context-and-operator-semantics.md))
- blocks with explicit terminators (return, die/throw, loop control, fallthrough)
- an exception model (`die`/`eval` unwinding, `$@`)
- a builtin contract for each profile-supported builtin
- a regex execution contract for the profile-supported regex subset

Every EIR evaluation that the profile permits is deterministic for the same
inputs and profile version, and is comparable against real Perl through the
differential oracle ([PLSP-SPEC-0027](PLSP-SPEC-0027-differential-real-perl-oracle.md))
within the profile's thresholds (C1).

## Valid PR Shapes

Valid PRs under this spec include:

- authoring or versioning the executable-profile charter
- adding the EIR value/cell/heap handle types
- adding stack frames, blocks/terminators, or the exception model to EIR
- adding one profile-supported builtin or the bounded regex contract
- adding bounded compile-time evaluation with step/time limits and fail-closed
  boundaries
- adding oracle thresholds for the profiled subset
- documentation that keeps EIR distinct from PIR-A and the profile explicit

Every EIR/profile PR must name the profile version, the construct/builtin it
adds, the fail-closed behavior at the boundary, and confirm runtime values use
handle types and that no runtime modeling leaks into PIR-A.

## Invalid PR Shapes

Invalid PRs include:

- evaluating constructs, builtins, modules, or capabilities not listed in the
  profile
- fabricating a value or presenting a partial evaluation as exact at a boundary
- unbounded or non-terminating compile-time evaluation
- exposing `Rc<RefCell<...>>` as the runtime value contract (corrected
  [#2269](https://github.com/EffortlessMetrics/perl-lsp/issues/2269))
- adding runtime values/cells/heap to PIR-A (corrected
  [#2269](https://github.com/EffortlessMetrics/perl-lsp/issues/2269) /
  [#2565](https://github.com/EffortlessMetrics/perl-lsp/issues/2565))
- ambient I/O, filesystem, network, env reads, XS, or source filters during
  evaluation
- making real-Perl execution an editor-runtime dependency (see
  [PLSP-SPEC-0027](PLSP-SPEC-0027-differential-real-perl-oracle.md))
- changing provider behavior from an EIR/profile change alone

## Acceptance

A PR satisfies this spec when:

- the profile version is stamped and the change stays within the declared subset
- unsupported boundaries fail closed with an explicit boundary, never a guess
- EIR is distinct from PIR-A and runtime values use `ValueId`/`CellId`/
  `RuntimeHeap` handles
- bounded evaluation respects step/time limits
- determinism and oracle thresholds for the profiled subset are stated
- tests cover the touched construct/builtin and its boundary behavior

## Proof Commands

Docs-only changes to this spec may use:

```bash
cargo xtask ci-hygiene check-doc-paths docs/specs
cargo xtask ci-hygiene check-doc-paths docs/project/status
git diff --check
```

Implementation PRs add focused evaluation/boundary tests and run the owning
crate's checks, plus differential-oracle checks when comparing against real Perl
(see [PLSP-SPEC-0027](PLSP-SPEC-0027-differential-real-perl-oracle.md) Proof
Commands).

## Non-goals

- No interpreter shipped from this spec alone.
- No provider behavior change from this spec alone.
- No Perl compatibility, full CPAN, or Rust-Perl-runtime-replacement claim.
- No evaluation outside the declared profile.
- No real-Perl editor-runtime dependency.
- No runtime value modeling in PIR-A.

## Claim Boundaries

This spec may claim that evaluation is governed by a versioned executable profile
that fails closed at its boundary, and that runtime behavior lives in an EIR
distinct from PIR-A using handle-typed values. It may not claim an interpreter
exists, that any Perl is evaluated by live editor requests, that Perl
compatibility is achieved, or that any provider behavior has changed, until
separate code, receipts, and status rows make that claim.
