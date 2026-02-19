# perl-symbol-table

Symbol table and scope management for the Perl LSP ecosystem.

Part of [tree-sitter-perl-rs](https://github.com/EffortlessMetrics/perl-lsp).

## Public API

- **`SymbolTable`** -- central registry of symbols, references, and scopes with hierarchical lookup
- **`Symbol`** -- a symbol definition with name, qualified name, kind, location, scope, documentation, and attributes
- **`SymbolReference`** -- a reference to a symbol with usage context and write tracking
- **`Scope`** -- a lexical scope boundary (Global, Package, Subroutine, Block, Eval)
- **`ScopeKind`** -- classification of scope types
- **`ScopeId`** -- unique scope identifier (`usize`)

Re-exports `SymbolKind` and `VarKind` from `perl-symbol-types`.

## Features

| Feature | Purpose |
|---------|---------|
| `serde` | Optional serialization via `serde::Serialize` / `Deserialize` |

## Dependencies

- `perl-symbol-types` -- symbol kind taxonomy
- `perl-position-tracking` -- source location tracking

## License

Licensed under either of [Apache License, Version 2.0](http://www.apache.org/licenses/LICENSE-2.0)
or [MIT license](http://opensource.org/licenses/MIT) at your option.
