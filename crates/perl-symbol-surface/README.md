# perl-symbol-surface

Stable symbol projections derived from the Perl AST.

## Problem it solves

Many higher-level IDE features need the same answer: which declarations in this
AST correspond to user-visible symbols? This crate provides a stable projection
layer so navigation, rename, indexing, and semantic analysis do not each need
to re-implement raw AST pattern matching.

## Public API

- `extract_symbol_decls` walks the tree and returns declaration projections.
- `SymbolDecl` stores symbol kind, name, qualification, and spans.

## Example

```rust,ignore
use perl_symbol_surface::extract_symbol_decls;

let decls = extract_symbol_decls(&ast, Some("My::Package"));
for decl in decls {
    println!("{}", decl.qualified_name);
}
```

## Workspace role

This crate sits between syntax (`perl-ast`) and IDE consumers such as
`perl-semantic-analyzer`, `perl-workspace-index`, navigation, rename, and
workspace-symbol features.

## License

MIT OR Apache-2.0
