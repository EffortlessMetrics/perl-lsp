# perl-symbol-types

Unified Perl symbol taxonomy for LSP tooling.

This crate provides a single, authoritative definition of Perl symbol kinds
used across the parser, semantic analyzer, workspace index, and LSP providers.

## Design Goals

- **Single source of truth**: All symbol classification flows through this crate
- **Perl semantics**: Distinguishes variables by sigil type (scalar/array/hash)
- **LSP compatibility**: Direct mapping to LSP protocol symbol kinds
- **Zero-cost abstractions**: Enum variants are `Copy` types with inline methods

## Usage

```rust
use perl_symbol_types::{SymbolKind, VarKind};

// Create symbol kinds
let scalar_var = SymbolKind::scalar();
let sub = SymbolKind::Subroutine;
let pkg = SymbolKind::Package;

// Get sigils
assert_eq!(scalar_var.sigil(), Some("$"));
assert_eq!(SymbolKind::array().sigil(), Some("@"));
assert_eq!(SymbolKind::hash().sigil(), Some("%"));

// Category predicates
assert!(scalar_var.is_variable());
assert!(sub.is_callable());
assert!(pkg.is_namespace());

// LSP protocol mapping
assert_eq!(SymbolKind::Subroutine.to_lsp_kind(), 12); // Function
assert_eq!(SymbolKind::Package.to_lsp_kind(), 2);     // Module
```

## LSP Symbol Kind Mapping

| Variant | LSP Kind | Number | Description |
|---------|----------|--------|-------------|
| `Package` | Module | 2 | Package declaration |
| `Class` | Class | 5 | OO class (Moose, Moo, class keyword) |
| `Role` | Interface | 8 | Role definition (Moose::Role) |
| `Subroutine` | Function | 12 | Standalone subroutine |
| `Method` | Method | 6 | OO method |
| `Variable(_)` | Variable | 13 | Variables (scalar, array, hash) |
| `Constant` | Constant | 14 | use constant or Readonly |
| `Import` | Module | 2 | Imported symbol |
| `Export` | Function | 12 | Exported symbol |
| `Label` | Key | 20 | Loop/block label |
| `Format` | Struct | 23 | format declaration |

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.
