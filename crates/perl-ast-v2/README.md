# perl-ast-v2

Incremental-parsing-focused AST nodes for the Perl parser ecosystem.

This crate extracts the experimental `v2` AST surface from `perl-ast` so
incremental parsing consumers can depend on a smaller, more focused microcrate.

## Provided types

- `Node`
- `NodeKind`
- `NodeId`
- `NodeIdGenerator`
- `DiagnosticId`
- `MissingKind`
