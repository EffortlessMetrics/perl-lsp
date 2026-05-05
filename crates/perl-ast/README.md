# perl-ast

AST (Abstract Syntax Tree) node definitions for the Perl parser ecosystem.

## Overview

`perl-ast` provides the typed node structures used to represent parsed Perl source code. It contains two AST modules:

- **`ast`** -- The primary AST used by `perl-parser`. Defines `Node` (kind + `SourceLocation`) and the `NodeKind` enum with 50+ variants covering declarations, expressions, control flow, regex, OO constructs, and error recovery nodes.
- **`v2`** -- Re-export of the extracted `perl-ast-v2` microcrate for incremental parsing use-cases. This compatibility tier is experimental while the workspace hardens incremental parser behavior.

S-expression serialization remains available through `Node::to_sexp()` for tree-sitter-compatible snapshots, but rendering policy is treated as a compatibility/debug adapter rather than AST semantics.

## Compatibility contract

The canonical AST contract lives in [`docs/reference/ast-contract.md`](../../docs/reference/ast-contract.md).

When adding or changing a `NodeKind`, update and test all required surfaces:

- kind naming (`kind_name`, `ALL_KIND_NAMES`)
- child traversal (`children`, `first_child`, `for_each_child`, `for_each_child_mut`)
- S-expression rendering coverage (or explicit documented waiver)
- parser fixture coverage
- semantic analyzer handling decision

## Public API

Re-exports from `lib.rs`: `Node`, `NodeKind`, `SourceLocation`, and experimental `v2`.

## Workspace Role

Tier 1 leaf crate. Depended on by `perl-parser-core`, `perl-tokenizer`, `perl-pragma`, and `perl-error`.

## Dependencies

- `perl-position-tracking` -- span and position types (`SourceLocation`, `Range`, `Position`)
- `perl-token` -- token definitions (`Token`, `TokenKind`) used in error recovery nodes

## License

MIT OR Apache-2.0
