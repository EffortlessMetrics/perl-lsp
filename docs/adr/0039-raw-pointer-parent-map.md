# ADR-0039: Raw-Pointer Parent Map for AST Upward Traversal

**Status**: Accepted
**Date**: 2026-03-18
**Decision Makers**: Perl LSP Architecture Team
**Related**: [ADR-0012](0012-error-handling-strategy.md), [ADR-0031](0031-async-runtime-concurrent-dispatch.md), [ADR-0034](0034-custom-lsp-runtime.md)

## Context

Several editor features need to walk *up* the parsed Perl AST, not just down from the root.
Declaration lookup, scope analysis, and context-sensitive navigation all need efficient parent
access after parsing has already produced a tree of child-owned nodes.

The current implementation solves this by building a `ParentMap`:

- `crates/perl-semantic-analyzer/src/analysis/declaration.rs` defines
  `ParentMap = FxHashMap<*const Node, *const Node>`
- `DeclarationProvider::build_parent_map()` populates the map during AST traversal
- `crates/perl-lsp-rs/src/state/document.rs` stores the map alongside each parsed document
- `crates/perl-lsp-rs/src/runtime/mod.rs` provides manual `Send`/`Sync` impls for `LspServer`
  because `DocumentState` contains raw pointers through `ParentMap`

This is an unusual design in a Rust codebase because the project deliberately accepts a narrow raw
pointer escape hatch inside an otherwise strongly synchronized runtime.

### Problem Statement

The project needed a way to support fast upward AST traversal without introducing one of the
following costs:

1. intrusive parent pointers inside every AST node
2. repeated root-to-leaf rescans to recover ancestry information
3. heavy reference-counted interior links that complicate ownership and mutation
4. broad unsafe traversal code spread across navigation features

The code already used this design, but the rationale and guardrails were not captured as an ADR.
That made the choice look like an isolated oddity instead of an intentional architecture decision.

## Decision

**The project will keep parent relationships in a sidecar `ParentMap` keyed by raw AST node
pointers, and will treat the map as a synchronized, document-scoped cache rather than embedding
parent links directly into AST nodes.**

### Chosen Architecture

| Concern | Decision |
|---|---|
| Parent lookup representation | `FxHashMap<*const Node, *const Node>` sidecar map |
| Ownership model | AST remains child-owned; parent links are external metadata |
| Lifetime scope | Parent maps live only as long as the parsed document snapshot that created them |
| Thread-safety boundary | Access is mediated through document storage protected by synchronization |
| Validation strategy | Debug assertions check freshness, suspicious emptiness, and parent-map cycles |

### Why This Was Chosen

1. **Upward traversal is a hot-path requirement.**
   Declaration and scope features need O(1) parent lookups once a document has been parsed.

2. **The AST should stay structurally simple.**
   Embedding parent pointers in every node would make the tree more invasive, harder to evolve,
   and more tightly coupled to runtime navigation needs.

3. **Document snapshots already provide a natural lifetime boundary.**
   The parser, AST, document text, and parent map are rebuilt together on refresh, which gives the
   raw-pointer map a bounded and understandable validity window.

4. **Unsafe surface area is minimized.**
   Rather than letting many features improvise ancestry recovery, the project centralizes the raw
   pointer representation in one map type plus narrow runtime synchronization assumptions.

## Alternatives Considered

### Option 1: Store parent pointers directly in AST nodes

**Pros**:
- Parent traversal would be immediate and explicit on every node
- No extra sidecar map to build after parsing

**Cons**:
- Makes AST ownership more complex and more self-referential
- Couples parser data structures to LSP/navigation concerns
- Increases risk when cloning, rebuilding, or incrementally replacing trees

**Decision**: Rejected.

### Option 2: Recompute ancestry on demand from the root

**Pros**:
- Avoids raw pointers entirely
- Keeps AST representation purely child-directed

**Cons**:
- Too expensive for latency-sensitive editor operations
- Repeats work across declaration, scope, and navigation features
- Makes performance dependent on tree depth and repeated searches

**Decision**: Rejected.

### Option 3: Use reference-counted bidirectional links (`Rc`/`Weak`, `Arc`/`Weak`)

**Pros**:
- Avoids naked raw pointers in the API surface
- Encodes ancestry in the object graph itself

**Cons**:
- Adds allocation and graph-management overhead
- Complicates parser-owned tree construction
- Poor fit for document snapshots that are frequently rebuilt wholesale

**Decision**: Rejected.

### Option 4: Keep a raw-pointer sidecar parent map

**Pros**:
- O(1) parent lookup after a single O(n) build pass
- Preserves a simple child-owned AST
- Keeps the unusual memory model localized and reviewable
- Fits snapshot-style document rebuilds already used by the LSP runtime

**Cons**:
- Requires explicit synchronization and lifetime discipline
- Forces manual `Send`/`Sync` reasoning at the server boundary
- Needs documentation so contributors understand why raw pointers are present

**Decision**: Accepted.

## Consequences

### Positive

- **Fast scope walking** for declaration and semantic features.
- **Clear separation of concerns** between parser data structures and editor navigation metadata.
- **Localized unsafety**: the raw-pointer model is confined to parent-map construction and
  synchronized document storage.
- **Debug-time guardrails**: the current implementation already checks for stale versions, empty
  maps in suspicious cases, and cycles.

### Negative / Trade-offs

- **Manual thread-safety reasoning** is required because raw pointers prevent automatic
  `Send`/`Sync` derivation.
- **Snapshot invalidation matters**: features must not keep using an old parent map after document
  refresh.
- **Contributor surprise**: reviewers may reasonably ask why a Rust project uses raw pointers for a
  core navigation path.

## Guardrails

This decision depends on the following implementation rules remaining true:

1. Parent maps are rebuilt whenever the AST snapshot is rebuilt.
2. Parent-map access remains scoped to synchronized document state.
3. Debug assertions continue to detect stale provider use and obvious structural corruption.
4. Production code does not widen unsafe access patterns beyond the current narrow boundary.

If these assumptions stop being true, this ADR must be revisited.

## Revisit Triggers

Review this ADR if any of the following happen:

- the AST representation gains a safe and cheap way to store parent links directly
- incremental parsing starts preserving parent relationships more efficiently in-tree
- raw-pointer synchronization becomes a recurring source of bugs or review friction
- declaration/scope performance no longer benefits materially from cached parent links

## References

- `crates/perl-semantic-analyzer/src/analysis/declaration.rs`
- `crates/perl-lsp-rs/src/state/document.rs`
- `crates/perl-lsp-rs/src/runtime/mod.rs`
