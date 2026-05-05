# perl-ast

AST (Abstract Syntax Tree) node definitions for the Perl parser ecosystem.

## Overview

`perl-ast` provides the typed node structures used to represent parsed Perl source code. It contains two AST modules:

- **`ast`** -- The primary AST used by `perl-parser`. Defines `Node` (kind + `SourceLocation`) and the `NodeKind` enum with 50+ variants covering declarations, expressions, control flow, regex, OO constructs, and error recovery nodes. Includes S-expression serialization via `to_sexp()` (implemented via a formatter boundary).
- **`v2`** -- An enhanced AST for incremental parsing, re-exported from `perl-ast-v2`. This surface is currently experimental while compatibility hardens toward v0.15. Nodes carry a unique `NodeId` and use `Range` (line/column) positions instead of byte offsets.

## Public API

Re-exports from `lib.rs`: `Node`, `NodeKind`, `SourceLocation`.

## Workspace Role

Tier 1 leaf crate. Depended on by `perl-parser-core`, `perl-tokenizer`, `perl-pragma`, and `perl-error`.

## Dependencies

- `perl-position-tracking` -- span and position types (`SourceLocation`, `Range`, `Position`)
- `perl-token` -- token definitions (`Token`, `TokenKind`) used in error recovery nodes

## License

MIT OR Apache-2.0


## AST compatibility contract

The compatibility contract and change policy are documented in `docs/reference/ast-contract.md`.
Any `NodeKind` addition must include parser, traversal, kind-name, S-expression, and semantic-decision coverage.
