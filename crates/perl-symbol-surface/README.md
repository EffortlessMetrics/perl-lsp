# perl-symbol-surface

[![Crates.io](https://img.shields.io/crates/v/perl-symbol-surface.svg)](https://crates.io/crates/perl-symbol-surface)
[![Documentation](https://docs.rs/perl-symbol-surface/badge.svg)](https://docs.rs/perl-symbol-surface)

Stable symbol-bearing projections derived from the Perl AST.

## When to use this crate

Use `perl-symbol-surface` when you want a reusable symbol view over Perl AST
nodes without reimplementing tree walking in each feature crate. It sits
between raw syntax (`perl-ast`) and IDE features such as navigation, rename,
workspace symbols, and call hierarchy.

This crate is useful when you need declaration extraction but do not want to
depend on the larger parser/runtime stack.

## Quick example

```rust,ignore
use perl_symbol_surface::extract_symbol_decls;

let decls = extract_symbol_decls(&ast, Some("My::Package"));
assert!(!decls.is_empty());
```

## Public API

- `extract_symbol_decls`: single-pass declaration extraction
- `SymbolDecl`: stable declaration view with spans and qualified names

## License

MIT OR Apache-2.0
