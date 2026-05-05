# AST Compatibility Contract

This document defines compatibility expectations for the AST crates while
`perl-ast` moves toward a more explicit public stability contract.

## Surfaces and stability tier

- `perl_ast::ast` (`Node`, `NodeKind`, `SourceLocation`) is the primary parser AST
  surface and is treated as the stable public contract.
- `perl_ast::v2` (re-export of `perl-ast-v2`) is an experimental compatibility tier
  intended for incremental parsing consumers.
- `perl-ast-v2` is published for focused integration work, but its API is
  considered pre-stability until explicitly promoted.

## NodeKind change gate

No new `NodeKind` variant should land without all of the following:

- Child traversal coverage (`children`, `first_child`, immutable traversal,
  mutable traversal).
- Kind-name coverage (`kind_name`, `ALL_KIND_NAMES`).
- S-expression coverage, or an explicit "not renderable" decision documented in
  tests.
- Parser fixture coverage proving when the variant is emitted.
- Semantic analyzer decision: handled, intentionally ignored, or explicitly
  deferred.

## Contributor checklist (AST behavior changes)

Before opening a PR that adds or changes an AST node shape, verify:

1. Parser emission is tested with a fixture.
2. Traversal behavior is covered.
3. S-expression output expectation is covered.
4. Semantic analyzer handling decision is encoded in tests/docs.
5. `perl-ast` and `perl-ast-v2` test suites pass.
