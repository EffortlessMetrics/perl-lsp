# PLSP-SPEC-0025: PIR v0 contract

Status: accepted
Owner: perl-lsp maintainers
Linked proposal: [PLSP-PROP-0001](../proposals/PLSP-PROP-0001-real-perl-editor-trust.md)
Linked specs:
- [PLSP-SPEC-0015](PLSP-SPEC-0015-real-perl-editor-trust-v1-boundary.md)
- [PLSP-SPEC-0017](PLSP-SPEC-0017-fact-provenance-and-source-backing.md)
- [PLSP-SPEC-0022](PLSP-SPEC-0022-module-path-authority.md)
- [PLSP-SPEC-0023](PLSP-SPEC-0023-ambient-inputs.md)
- [PLSP-SPEC-0024](PLSP-SPEC-0024-framework-fact-adapters.md)
Linked ADRs:
- [PLSP-ADR-0002](../adr/PLSP-ADR-0002-confidence-before-cutover.md)
Linked roadmap: [Compiler-backed LSP roadmap](../project/COMPILER_BACKED_LSP_ROADMAP.md)
Owner issue: [#8196](https://github.com/EffortlessMetrics/perl-lsp/issues/8196)
Status impact: compiler capability status, compiler facts, future determinism
receipts, future differential oracle receipts

## Current Implementation Status

Tooling PIR is currently planned. The compiler substrate already has HIR,
scope/pad facts, stash/package facts, compile environment, module resolution,
import/export facts, generated-member facts, framework-adapter boundaries, and
compile-time effect facts. PIR v0 is the next substrate layer that turns those
facts into a source-anchored tooling IR.

This spec defines the PIR v0 contract. It does not add PIR code, provider
behavior, cache behavior, determinism claims, or real-Perl execution.

## Contract

PIR is a Perl intermediate representation for editor tooling and static
analysis. It is not bytecode, not a runtime, and not an evaluator.

PIR v0 must:

- preserve source anchors for every node that comes from source
- preserve dynamic-boundary links instead of guessing exact behavior
- preserve scalar, list, void, and lvalue context where known
- model lexical and stash reads and writes
- model calls and method calls without executing Perl
- model branches, loops, assignment, returns, and control-flow edges
- emit receipts that explain what lowered, what fell back, and what was blocked
- remain provider-neutral until separate provider promotion proof exists

PIR v0 must not:

- evaluate arbitrary Perl
- run Perl, `perldoc`, DAP, or application code
- replace HIR as the canonical syntax tree
- promote rename, safe delete, diagnostics, references, or semantic tokens
- hide ambient inputs or dynamic boundaries
- claim determinism from lowering alone

## Target Code Shape

Future code may introduce types with this shape:

```rust
pub struct PirNode {
    pub id: PirId,
    pub source_anchor: Option<SourceRange>,
    pub operation: PirOperation,
    pub context: PirContext,
    pub dynamic_boundary: Option<DynamicBoundaryId>,
}

pub enum PirContext {
    Scalar,
    List,
    Void,
    Lvalue,
    Unknown,
}

pub enum PirOperation {
    LexicalRead { name: LexicalName },
    LexicalWrite { name: LexicalName },
    StashRead { symbol: SymbolName },
    StashWrite { symbol: SymbolName },
    Assign,
    Call { callee: PirCallee },
    MethodCall { receiver: PirReceiver, method: PirMethod },
    Branch { condition: PirId },
    Loop { condition: Option<PirId> },
    Return,
    DynamicBoundary { kind: DynamicBoundaryKind },
}
```

The exact Rust names may differ. The semantics above are the contract.

## Required Node Identity

Every PIR node must have a stable `PirId` inside one lowering receipt. IDs do
not need to be stable across versions or unrelated workspaces, but a receipt
must be internally deterministic for the same source, compiler environment, and
configuration.

Node identity must support:

- source-anchor lookup
- parent/child or edge lookup where modeled
- dynamic-boundary lookup
- receipt explanations for lowered, fallback, and blocked nodes

## Source Anchors

Source-derived PIR nodes must preserve a source anchor. A source anchor is the
workspace file and range that caused the PIR node to exist.

Generated, framework, or ambient facts may produce PIR nodes only when their
provenance is explicit:

- `ExplicitSource` nodes anchor to source text
- `SourceBackedGenerated` nodes anchor to the framework declaration, not a fake
  generated method body
- `GeneratedNoSource` nodes remain blocked or receipt-only
- `DynamicBoundary` nodes point to the boundary range when available
- `AmbientInput` nodes report the ambient input class
- `Unknown` nodes remain fallback, blocked, or explanation-only

## Context Model

PIR v0 must model these contexts:

- scalar
- list
- void
- lvalue
- unknown

Unknown context is allowed when the compiler substrate cannot prove context
without executing Perl. Unknown context must be visible in receipts and must not
be silently promoted to scalar or list.

## Data Access Model

PIR v0 must distinguish:

- lexical reads
- lexical writes
- stash/package reads
- stash/package writes
- typeglob or symbolic access boundaries
- dynamic dereference boundaries

Lexical and stash operations must preserve the source range that caused the
read or write when available.

Symbolic references, non-literal typeglob mutation, dynamic package names, and
runtime stash mutation must become dynamic-boundary nodes unless a separate
proof models the class.

## Call Model

PIR v0 must model:

- direct subroutine calls
- package-qualified calls
- method calls
- constructor-shaped calls
- dynamic callee boundaries
- dynamic receiver boundaries
- dynamic method-name boundaries

Calls may link to source-backed callee facts only when the existing fact source,
confidence, freshness, and provenance contracts allow it. Calls must not infer
runtime dispatch as exact proof.

## Control-Flow Model

PIR v0 must support a first control-flow graph for:

- assignment
- branches
- loops
- returns
- fallthrough
- dynamic-boundary exits

CFG edges may be conservative. Missing or unknown edges must be represented as
unknown or dynamic, not dropped silently.

## Lowering Receipts

Every PIR lowering receipt must report:

- schema or receipt version
- source file or workspace fixture identity
- lowering mode
- node count
- edge count where CFG exists
- lowered operation counts
- context counts
- source-anchor coverage
- dynamic-boundary counts by kind
- unsupported construct counts
- stale or ambient inputs that affected lowering
- whether provider behavior changed

Provider behavior must remain unchanged unless a separate provider promotion PR
uses PIR proof and updates support claims.

## Valid PR Shapes

Valid PRs under this spec include:

- adding PIR structs with no provider behavior
- lowering one operation family into PIR
- adding source-anchor preservation tests
- adding dynamic-boundary preservation tests
- adding context propagation for one expression family
- adding CFG or call graph v1 receipts
- adding real-workspace PIR lowering receipts
- adding documentation that keeps PIR separate from determinism and runtime

Every PIR PR must name the operation class, source-anchor rule, dynamic-boundary
rule, fallback rule, receipt, and explicit non-goal for provider behavior.

## Invalid PR Shapes

Invalid PRs include:

- using PIR lowering to promote provider behavior without a provider ledger row
- evaluating arbitrary Perl during lowering
- treating real Perl as an editor dependency
- hiding unknown context or dynamic boundaries
- claiming determinism from PIR node coverage alone
- replacing HIR with PIR in one broad cutover
- adding retained PIR caches without key, cap, eviction, pressure counter, and
  cleanup tests
- bundling PIR code with unrelated parser, provider, or refactor changes

## Acceptance

A PIR PR satisfies this spec when:

- the touched operation class is named and scoped
- source anchors are preserved or explicitly absent with a reason
- dynamic boundaries are preserved and counted
- unknown context is visible
- receipts describe lowered, fallback, and blocked constructs
- tests cover the touched operation class
- support tiers and provider cutover claims remain unchanged unless a separate
  support-review PR promotes them

## Proof Commands

Docs-only changes to this spec may use:

```bash
cargo xtask ci-hygiene check-doc-paths docs/specs
cargo xtask ci-hygiene check-doc-paths docs/project/status
cargo xtask check-provider-confidence-matrix
cargo xtask check-support-claims
cargo xtask check-provider-promotion-ledger
git diff --check
```

Implementation PRs must add focused PIR lowering tests for the operation class
and run the relevant crate checks. Once a PIR validator exists, implementation
PRs must also run:

```bash
cargo xtask check-pir-receipts
```

## Non-goals

- No provider behavior change from this spec alone.
- No PIR code from this spec alone.
- No broad compiler/runtime implementation claim.
- No determinism receipt claim.
- No differential real-Perl oracle claim.
- No retained PIR cache contract.
- No real Perl execution.

## Claim Boundaries

This spec may claim that PIR v0 has a source-anchor, context,
dynamic-boundary, call/control-flow, and receipt contract. It may not claim PIR
is implemented, provider-backed, deterministic, runtime-capable, or
conformance-proven until separate code, receipts, and status rows prove those
claims.
