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
## Stability

`perl-ast-v2` is intentionally published for incremental parsing integration
experiments, but remains pre-stability and may evolve until promoted by the
project [AST compatibility contract](../../docs/reference/ast-contract.md).
