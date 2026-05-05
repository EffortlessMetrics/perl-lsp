# AST Compatibility Contract

`perl-ast` is in a stability-contract phase as the parser, semantic analyzer, and LSP layers converge on a durable AST surface.

## Contract tiers

- `perl_ast::ast::{Node, NodeKind}` is the **stable public AST** for the current parser pipeline.
- `perl_ast::v2` (re-exported from `perl-ast-v2`) is **experimental** and may evolve until the v0.15 stabilization window.
- `perl-ast-v2` remains a published microcrate so incremental parsing consumers can opt in directly while the API hardens.

## NodeKind change policy

No new `NodeKind` variant should land without explicitly updating and validating all required surfaces:

- Variant definition and fields
- `kind_name()`
- `ALL_KIND_NAMES`
- Child traversal helpers (`children`, `first_child`, `for_each_child`, `for_each_child_mut`)
- S-expression rendering (`to_sexp()` / formatter coverage) or an explicit non-renderable waiver
- Parser fixture coverage showing when the variant is emitted
- Semantic analyzer handling decision (handled, ignored, or deferred)

This policy is intentionally conservative: AST drift is expensive for parser, semantic, and LSP consumers.

## Contributor checklist for AST changes

When introducing or materially changing AST shapes, include:

1. Parser evidence (fixture or targeted test)
2. Traversal coverage
3. Kind-name coverage
4. S-expression coverage (or explicit waiver)
5. Semantic analyzer decision note

The goal is not maximal process; it is preserving a stable cross-crate contract as the project approaches v0.15.
