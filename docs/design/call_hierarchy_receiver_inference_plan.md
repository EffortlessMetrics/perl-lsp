# Call Hierarchy Receiver-Inference Plan

## Goal

Improve call hierarchy resolution for Perl OO code without promising full Perl type inference.

The target outcome is reliable method call hierarchy for common code patterns, while preserving current behavior of omitting unresolved or ambiguous dynamic dispatch.

## Baseline Assumptions

- Cross-file incoming/outgoing call hierarchy resolution already uses the workspace index as the primary fast path.
- Current gaps are concentrated around method-call identity when the receiver class is not explicit.
- Dynamic dispatch (`AUTOLOAD`, symbolic refs, indirect/dynamic notation) remains partially unresolved today.

## Design Principle

For call hierarchy, we only need to answer two questions at each call site:

1. What method name is being called?
2. What receiver class(es) are plausible here?

This can be solved by receiver-class inference and method lookup, not general whole-program type inference.

## Scope

### In scope

- Intraprocedural receiver-class inference inside one callable.
- Inheritance-aware method lookup.
- Lightweight return hints for constructor/factory patterns.
- Bounded dynamic fallback (`AUTOLOAD`, constant-folded method-name strings).

### Out of scope

- Arbitrary symbolic refs requiring runtime evaluation.
- Runtime-generated package names and symbol-table mutation.
- Full-program type inference or guaranteed completeness for dynamic Perl metaprogramming.

## Proposed Phases

### Phase 1: Intraprocedural receiver inference (80/20)

Add a request-scoped receiver environment that tracks variable -> candidate class set within the enclosing callable.

Seed facts from common patterns:

- `my $x = Foo->new(...)`
- `my $x = bless $ref, 'Foo'`
- typed signatures/lexicals where parser support exists
- aliasing (`$y = $x`)
- branch joins (union class sets)
- reassignment (kill/narrow previous facts)

**Acceptance rule:** only emit a resolved method target when class candidate resolution is confirmed.

### Phase 2: Inheritance-aware lookup

Build method lookup over class hierarchy with explicit support for:

- direct method definitions by package/class
- `@ISA`
- `use parent`
- `use base`
- `SUPER::method`

When multiple plausible classes map to different definitions, treat as ambiguous and omit.

### Phase 3: Dynamic fallback (bounded)

Add controlled fallback behavior:

- If normal method lookup fails, check package/inherited `AUTOLOAD`.
- Resolve dynamic method names only when constant propagation yields a literal string.
- Leave unresolved when values are non-constant or too dynamic.

### Phase 4: Interprocedural return hints (lightweight)

Add index-time return hints for simple factory/constructor wrappers:

- direct `return Foo->new(...)`
- direct `return bless ..., 'Foo'`
- unique constructor/factory wrappers

Do not introduce general fixed-point or whole-program analysis.

## Repository Placement

### `crates/perl-lsp/src/call_hierarchy_provider.rs`

- Keep AST walk syntax-first.
- Add structured call-site facts for method calls (receiver expression kind, method name, enclosing callable context).
- Attach receiver hints needed by resolver policy.

### `crates/perl-lsp/src/runtime/language/hierarchy.rs`

- Keep as orchestration/resolution boundary.
- Build request-scoped receiver environment.
- Resolve candidate receiver classes against workspace facts.
- Materialize outgoing/incoming `CallHierarchyItem`s only from confirmed targets.
- Preserve canonical identity in serialized `CallHierarchyItem.data` for round-trip stability.

### `crates/perl-workspace-index`

Extend semantic facts (without persisting full ASTs):

- method tables by class/package
- inheritance edges
- `AUTOLOAD` availability markers
- lightweight return hints

Keep `DocumentStore` as open-document overlay only; do not promote it to a durable workspace semantic cache.

## Testing Plan

Add/expand tests in `crates/perl-lsp/tests/lsp_call_hierarchy_tests.rs` and workspace-index coverage for:

1. `bless` receiver inference.
2. constructor receiver inference.
3. alias propagation.
4. branch joins.
5. inherited method resolution.
6. `SUPER::method` behavior.
7. `AUTOLOAD` fallback behavior.
8. constant-folded dynamic method names.
9. ambiguous receiver sets (omit, do not guess).
10. closed-file workspace-index resolution vs open-document dirty-buffer overlay.

## Implementation Order

1. Receiver inference inside one callable.
2. Inheritance-aware method lookup.
3. Bounded dynamic fallback (`AUTOLOAD` + constant method-name resolution).
4. Lightweight interprocedural return hints.

This order provides early user value with controlled blast radius.


## Suggested PR Slices

### PR 1: Resolver identity and intraprocedural receiver facts

- Add internal callable identity scaffolding used by hierarchy handlers.
- Add receiver-class fact collection for local constructor/bless/alias patterns.
- Add outgoing-call tests that confirm receiver-qualified method targets.

### PR 2: Method-aware incoming indexing

- Add method-aware callable/reference keying in workspace index.
- Plumb incoming call hierarchy resolution to method-aware index queries.
- Add cross-file incoming method tests, including ambiguity safeguards.

### PR 3: Inheritance graph integration

- Index `@ISA`, `use parent`, `use base` edges.
- Add lookup walk across inheritance chain and `SUPER::` handling.
- Add inherited method hierarchy tests.

### PR 4: Dynamic bounded fallback + return hints

- Add `AUTOLOAD` fallback where normal lookup fails.
- Add constant string propagation for dynamic method names.
- Add lightweight factory/constructor return hints.
- Add tests for bounded dynamic behavior and unresolved ambiguity policy.

## Non-goal Commitment

Do not claim complete support for arbitrary dynamic Perl dispatch. Prefer omission over incorrect invented targets when ambiguity remains.
