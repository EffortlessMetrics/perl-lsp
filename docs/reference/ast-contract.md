# AST Compatibility Contract

This document defines the compatibility boundary between the primary AST API and the incremental AST experiment.

## Stability tiers

- **Stable public AST (`perl_ast::Node`, `perl_ast::NodeKind`)**
  - The `ast` module is the primary parser-facing and consumer-facing AST surface.
  - Treat this API as stability-contract surface as we move toward the v0.15 window.
- **Experimental AST (`perl_ast::v2`)**
  - `perl_ast::v2` is a re-export of the extracted `perl-ast-v2` microcrate.
  - It remains experimental and may change while incremental parsing contracts harden.

## NodeKind change policy

No new `NodeKind` variant should land without all of the following:

1. **Parser fixture coverage** (proof the variant is emitted in real parsing).
2. **Traversal coverage** (`children`, `first_child`, and traversal helpers).
3. **Kind-name coverage** (`kind_name` and `ALL_KIND_NAMES`).
4. **S-expression coverage** (`to_sexp`) or an explicit, documented non-renderable waiver.
5. **Semantic decision** (handled, intentionally ignored, or explicitly deferred).

## Contributor checklist

When adding or changing AST surface:

- Keep changes scoped to one concern.
- Add tests that fail if required `NodeKind` surfaces drift.
- Preserve existing `Node::to_sexp()` API compatibility when refactoring internals.
- Document any intentional compatibility exception in code comments and tests.
