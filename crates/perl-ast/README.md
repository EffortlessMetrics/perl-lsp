# perl-ast

Abstract syntax tree node types for Perl parsing and analysis.

## Scope

- Defines the primary AST model used by `perl-parser`.
- Provides an experimental `v2` AST for incremental parsing flows.
- Re-exports common node and location types used across parser/LSP crates.

## Public Surface

- `ast` module for primary node structures.
- `v2` module for alternate AST representation.
- Re-exports: `Node`, `NodeKind`, `SourceLocation`.

## Workspace Role

`perl-ast` is a foundational internal crate used by parser, semantic analysis, and LSP components.

## License

MIT OR Apache-2.0.
