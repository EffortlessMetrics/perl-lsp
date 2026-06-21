# PLSP-SPEC-0031: Context and operator-semantics contract

Status: draft
Owner: perl-lsp maintainers
Linked proposal: [PLSP-PROP-0001](../proposals/PLSP-PROP-0001-real-perl-editor-trust.md)
Linked program: compiler program — canonical current-main roadmap ([#2559](https://github.com/EffortlessMetrics/perl-lsp/issues/2559), authored in parallel)
Linked boundary ADR: HIR body / PIR-A / EIR boundary ADR ([#2564](https://github.com/EffortlessMetrics/perl-lsp/issues/2564), authored in parallel)
Linked specs:
- [PLSP-SPEC-0025](PLSP-SPEC-0025-pir-v0.md)
- [PLSP-SPEC-0030](PLSP-SPEC-0030-compile-state-layers.md)
- [PLSP-SPEC-0032](PLSP-SPEC-0032-pir-a-places-effects-cfg.md)
Linked issues:
- [#2559](https://github.com/EffortlessMetrics/perl-lsp/issues/2559) — compiler program tracker
- [#2563](https://github.com/EffortlessMetrics/perl-lsp/issues/2563) — context model: ValueContext x EvaluationDemand x AccessMode
- [#2275](https://github.com/EffortlessMetrics/perl-lsp/issues/2275) — corrected: context is not a five-way enum with `Boolean`
Linked roadmap: [Compiler-backed LSP roadmap](../project/COMPILER_BACKED_LSP_ROADMAP.md)
Status impact: compiler fact substrate, HIR context constraints, PIR-A context
resolution, context-dependent diagnostics, future executable profile

## Purpose

Perl evaluation is governed by three independent axes that prior work conflated
into a single "context" enum. The implemented PIR v0 context type
(`PirContext { Scalar, List, Void, Lvalue, Unknown }`,
[`crates/perl-parser-core/src/pir/model.rs:145`](../../crates/perl-parser-core/src/pir/model.rs))
folds *value shape*, *boolean testing*, and *assignment-target-ness* into one
flat set. That conflation is the root cause behind
[#2563](https://github.com/EffortlessMetrics/perl-lsp/issues/2563) and
[#2275](https://github.com/EffortlessMetrics/perl-lsp/issues/2275): `Boolean`
and `Lvalue` are not values of the same axis as `Scalar`/`List`, and treating
them so produces wrong context propagation for `&&`, `||`, `//`, assignment, and
list operators.

This spec fixes the three-axis context model as a durable design contract and
defines a declarative operator-semantics table that downstream HIR-body and
PIR-A work must honor. It is a contract for *what context means* and *how
operators propagate it*; it does not implement evaluation, change provider
behavior, or claim runtime support.

## Contract

### C1 — Three orthogonal axes, not one enum

Context is the product of three axes, each resolved independently:

```rust
/// The wantarray-observable value shape an expression is evaluated for.
pub enum ValueContext {
    Scalar,
    List,
    Void,
    Unknown, // not statically provable without evaluation
}

/// What the surrounding expression demands of the produced value.
pub enum EvaluationDemand {
    Value,          // the value itself is consumed
    TruthTest,      // only the boolean truth of the value is consumed
    DefinednessTest // only `defined`-ness is consumed (the `//` LHS demand)
}

/// How a place is accessed (the lvalue axis).
pub enum AccessMode {
    Read,
    Write,
    ReadModifyWrite,
    Alias,     // `foreach`, `@_`, `\` of an lvalue, sub signature aliasing
    Localize,  // `local` — dynamic save/restore of a place
}
```

The full evaluation environment of any expression node is the tuple
`(ValueContext, EvaluationDemand, AccessMode)`. These axes are independent: a
scalar expression may be demanded as a value, as a truth test, or as a
definedness test; any of those may occur in a read or a write access mode.

### C2 — Boolean is a demand, not a context

`Boolean` is **not** a `ValueContext`. The condition of `if`, `while`, `unless`,
`?:`, `and`/`or`/`not`, and the LHS of `&&`/`||` is evaluated in **scalar
`ValueContext` with `EvaluationDemand::TruthTest`**. Any model that adds
`Boolean` as a sibling of `Scalar`/`List` (as the corrected
[#2275](https://github.com/EffortlessMetrics/perl-lsp/issues/2275) proposed) is
out of contract. The truth test is observed by analysis as a demand on a scalar
value; it does not change `wantarray`.

### C3 — Lvalue is an access mode / place, not a context

`Lvalue` is **not** a `ValueContext` either. Being an assignment target is a
property of *a place under an `AccessMode`* (`Write`, `ReadModifyWrite`,
`Alias`, or `Localize`), resolved by the place model in
[PLSP-SPEC-0032](PLSP-SPEC-0032-pir-a-places-effects-cfg.md). The current
`PirContext::Lvalue` variant
([`pir/model.rs:152`](../../crates/perl-parser-core/src/pir/model.rs)) is the
conflation this spec retires: an lvalue expression still has a `ValueContext`
(usually scalar or list) and is described by an `AccessMode`, not by a fourth
context value.

### C4 — HIR retains constraints; analysis resolves

HIR body nodes (the HIR-body graph from
[#2564](https://github.com/EffortlessMetrics/perl-lsp/issues/2564)) record
context **constraints**, not resolved contexts. A constraint is "this node's
`ValueContext` is the surrounding context" or "this node imposes scalar
`TruthTest` on its operand." PIR-A analysis resolves the actual
`(ValueContext, EvaluationDemand, AccessMode)` top-down from the outermost
demand inward:

- Statement context is `Void` value-context, `Value` demand, unless the
  statement is itself a condition.
- Each operator propagates context to its operands per the OperatorSemantics
  table (C5).
- For an **unprototyped** call, the argument list is flattened in **list**
  value-context (one flat list); the callee does not constrain caller-side
  argument context.
- For a call with a **signature** or **prototype**, argument contexts are bound
  in the callee per its declared shape, not flattened.
- A subroutine's `return` (and final expression) is evaluated in the **caller's**
  value-context (the `wantarray` of the call site), recorded as a constraint and
  resolved when the call site is known; `Unknown` when the call site is dynamic.

Unprovable context resolves to `ValueContext::Unknown` and must remain visible,
never silently promoted to `Scalar` or `List` (consistent with
[PLSP-SPEC-0025](PLSP-SPEC-0025-pir-v0.md) and
[PLSP-SPEC-0030](PLSP-SPEC-0030-compile-state-layers.md) C5).

### C5 — Declarative operator semantics

Each operator is defined by a row stating: the value-context and demand it
imposes on each operand, the rule for its own result context, evaluation order,
short-circuit behavior, and whether it produces a place. The table below is the
contract; an implementation may encode it as data, but the semantics are fixed
here.

Legend — `surrounding` = the context demanded of this operator by its parent;
`scalar/TruthTest` = scalar value-context with truth-test demand; `place` =
yields an assignable place under the stated `AccessMode`.

| Operator | LHS context/demand | RHS context/demand | Result context | Eval order | Short-circuit | Place rule |
| --- | --- | --- | --- | --- | --- | --- |
| `&&` / `and` | scalar / `TruthTest` | `surrounding` / `Value` | `surrounding` | L then R | skip R if LHS false | result is last evaluated operand's place if both in lvalue position (rare); otherwise not a place |
| `\|\|` / `or` | scalar / `TruthTest` | `surrounding` / `Value` | `surrounding` | L then R | skip R if LHS true | as `&&` |
| `//` (defined-or) | scalar / `DefinednessTest` | `surrounding` / `Value` | `surrounding` | L then R | skip R if LHS defined | **returns the last evaluated operand value, NOT a normalized boolean** |
| `=` (scalar assign) | `place` / `Write` (or `ReadModifyWrite` for `op=`) | scalar / `Value` | scalar (the assigned value); returns LHS place in list/lvalue use | R then L (value computed, then stored) | none | LHS is a place |
| `=` (list assign) | `place` list / `Write` | list / `Value` | scalar = count of RHS list in scalar context; list = the LHS list in list context | R then L | none | LHS is a list of places |
| `==`, `eq`, `<`, `cmp`, `<=>`, … | scalar / `Value` | scalar / `Value` | scalar (boolean-valued) | L then R | none | not a place |
| `,` (comma) in list ctx | list / `Value` | list / `Value` | list (flattened) | L then R | none | not a place |
| `,` (comma) in scalar ctx | scalar / `Value` (discarded) | scalar / `Value` | scalar = RHS value | L then R | none | not a place |
| `? :` (ternary) | scalar / `TruthTest` | both arms `surrounding` / `Value` | `surrounding` | cond, then one arm | only chosen arm evaluated | result is a place iff both arms are places |
| `?` `:` chained list op (`map`, `grep`, `sort`) block | n/a | block in list / `Value`; list args in list / `Value` | list (scalar = count) | block per element | none | not a place |
| named unary / list builtin | per prototype; default list / `Value` | — | per builtin | left to right | none | rarely a place (e.g. `substr` lvalue is) |
| `local EXPR` | `place` / `Localize` | — | the localized place's value | — | none | EXPR is a place; save/restore is a runtime effect |

Notes that the table encodes and that downstream work must not contradict:

- `&&`, `||`, `//` **return their last evaluated operand value**, not a
  normalized boolean. Only the LHS is in a truth/definedness *demand*; the
  operator's own result is the surviving operand in the surrounding context.
- List assignment has the well-known dual result: count in scalar context, the
  LHS list in list context. This is a result-context rule, not two operators.
- The comma operator's meaning is context-dependent: list-flattening in list
  context, C-style "evaluate-and-discard-left" in scalar context.

## Worked Corrections

This spec supersedes the framing in the corrected issues:

- [#2563](https://github.com/EffortlessMetrics/perl-lsp/issues/2563): context is
  the three-axis product `ValueContext x EvaluationDemand x AccessMode`, not a
  single flat enum.
- [#2275](https://github.com/EffortlessMetrics/perl-lsp/issues/2275): the
  PIR-context-on-edges proposal listed `Boolean` and `LValue` as context values.
  Per C2/C3 those are a demand and an access mode respectively; the edge context
  carries `ValueContext` only, with demand and access mode recorded on the node.

## Valid PR Shapes

Valid PRs under this spec include:

- introducing `ValueContext`, `EvaluationDemand`, and `AccessMode` types in the
  compiler-substrate crates with no provider behavior change
- recording context **constraints** on HIR body nodes
- implementing top-down context resolution for one operator family with tests
  drawn directly from the OperatorSemantics table
- adding the unprototyped-flatten / signature-bind / caller-return rules (C4)
- migrating `PirContext::Lvalue` consumers to the `AccessMode` place model
- documentation that keeps the three axes distinct

Every context PR must name the operator(s) or node family it touches, the row(s)
of the OperatorSemantics table it implements, and confirm `Boolean`/`Lvalue` are
not added as `ValueContext` values.

## Invalid PR Shapes

Invalid PRs include:

- adding `Boolean` or `Lvalue`/`LValue` as a `ValueContext` variant
- resolving `Unknown` context to `Scalar`/`List` without proof
- normalizing `&&`/`||`/`//` results to a boolean
- flattening signature/prototype-bound argument lists, or binding unprototyped
  argument lists in the callee
- resolving callee `return` context independent of the call site
- changing provider behavior from a context change alone

## Acceptance

A PR satisfies this spec when:

- context is modeled on three orthogonal axes
- `Boolean` is a scalar `TruthTest` demand and `Lvalue` is an `AccessMode`/place
- the touched operator(s) match their OperatorSemantics row for operand context,
  demand, result context, eval order, short-circuit, and place rule
- `&&`/`||`/`//` return the last evaluated value, not a normalized boolean
- unprovable context stays `Unknown` and visible
- tests cover the touched operator/node family

## Proof Commands

Docs-only changes to this spec may use:

```bash
cargo xtask ci-hygiene check-doc-paths docs/specs
cargo xtask ci-hygiene check-doc-paths docs/project/status
git diff --check
```

Implementation PRs must add focused context-resolution tests for the touched
operator family and run the owning crate's checks (for example
`cargo test -p perl-parser-core --locked`).

## Non-goals

- No evaluation or runtime from this spec alone.
- No provider behavior change from this spec alone.
- No EIR or executable-profile claim (see
  [PLSP-SPEC-0035](PLSP-SPEC-0035-executable-profile-and-eir.md)).
- No determinism or oracle claim.
- No `wantarray` semantics for `Boolean` or `Lvalue`.

## Claim Boundaries

This spec may claim that context is a three-axis model and that operators
propagate context per the declarative table. It may not claim that context
resolution is implemented, provider-backed, runtime-capable, or proven against
real Perl until separate code, receipts, and status rows exist.
