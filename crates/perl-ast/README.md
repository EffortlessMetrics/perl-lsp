# perl-ast

AST (Abstract Syntax Tree) node definitions for the Perl parser ecosystem.

## Overview

`perl-ast` provides the typed node structures used to represent parsed Perl source code. It contains two AST modules:

- **`ast`** -- The primary AST used by `perl-parser`. Defines `Node` (kind + `SourceLocation`) and the `NodeKind` enum with 50+ variants covering declarations, expressions, control flow, regex, OO constructs, and error recovery nodes. Includes S-expression serialization via `to_sexp()`.
- **`v2`** -- A re-export of the extracted `perl-ast-v2` microcrate for incremental parsing experiments. Nodes carry a unique `NodeId` and use `Range` (line/column) positions instead of byte offsets. Adds `NodeIdGenerator`, `MissingKind`, `DiagnosticId`, and lightweight `ErrorRef` nodes. This surface is experimental until the incremental compatibility contract is finalized.

## Public API

Re-exports from `lib.rs`: `Node`, `NodeKind`, `SourceLocation`.

## Compatibility Contract

- Stable contract: `perl_ast::Node` and `perl_ast::NodeKind` (`ast` module).
- Experimental contract: `perl_ast::v2` (re-exported from `perl-ast-v2`).
- See [`docs/reference/ast-contract.md`](../../docs/reference/ast-contract.md) for the required coverage checklist when introducing a new `NodeKind` variant.

## Workspace Role

Tier 1 leaf crate. Depended on by `perl-parser-core`, `perl-tokenizer`, `perl-pragma`, and `perl-error`.

## Dependencies

- `perl-position-tracking` -- span and position types (`SourceLocation`, `Range`, `Position`)
- `perl-token` -- token definitions (`Token`, `TokenKind`) used in error recovery nodes

## License

MIT OR Apache-2.0
