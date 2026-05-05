# AST Compatibility Contract

This document defines the compatibility tiers for AST surfaces in the `perl-lsp`
workspace.

## Surface tiers

- `perl_ast::Node` and `perl_ast::NodeKind` (from `perl-ast::ast`) are the
  primary parser AST and should be treated as the stability-contract surface as
  `perl-ast` approaches `v0.15`.
- `perl_ast::v2` is an extracted, re-exported microcrate (`perl-ast-v2`) for
  incremental-parsing consumers and remains experimental until the `v0.15.0`
  compatibility window is declared complete.

## Required updates when adding or changing `NodeKind`

No `NodeKind` variant change is complete without all of the following:

1. `kind_name()` coverage.
2. `ALL_KIND_NAMES` coverage.
3. Child traversal coverage (`children()`, `first_child()`,
   `for_each_child()`, `for_each_child_mut()`).
4. S-expression coverage (`to_sexp()` / `to_sexp_inner()`) or an explicit,
   documented "not renderable" waiver.
5. Parser fixture coverage proving when the variant is emitted.
6. Semantic analyzer handling decision (handled, intentionally ignored, or
   explicitly deferred).

## Contributor checklist

When introducing a new AST construct, include evidence in the PR that answers:

- Parser: when is it emitted?
- Traversal: what are its children?
- S-expression: how is it rendered?
- Semantic analyzer: how is it treated?
- LSP-facing spans: does it expose the required symbol/name anchor(s)?
- Tests: which fixture or unit test demonstrates the behavior?
