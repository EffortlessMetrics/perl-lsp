# perl-symbol-types

Unified Perl symbol taxonomy for LSP tooling.

This crate provides foundational type definitions for Perl symbol classification used across the perl-lsp ecosystem:
- `SymbolKind` enum for different Perl symbol types (packages, subroutines, variables, etc.)
- `VarKind` enum for variable types (scalar, array, hash)
- LSP protocol symbol kind mappings

## Usage

```rust
use perl_symbol_types::{SymbolKind, VarKind};

// Create symbol kinds
let scalar_var = SymbolKind::scalar();
let sub = SymbolKind::Subroutine;
let pkg = SymbolKind::Package;

// Get sigils for variables
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

## Symbol Types

| Variant | LSP Kind | Description |
|---------|----------|-------------|
| `Package` | Module | Package declaration |
| `Class` | Class | OO class (Moose, Moo, class keyword) |
| `Role` | Interface | Role definition (Moose::Role) |
| `Subroutine` | Function | Standalone subroutine |
| `Method` | Method | OO method |
| `Variable(_)` | Variable | Variables (scalar, array, hash) |
| `Constant` | Constant | use constant or Readonly |
| `Import` | Module | Imported symbol |
| `Export` | Function | Exported symbol |
| `Label` | Key | Loop/block label |
| `Format` | Struct | format declaration |

## Features

- **Single source of truth**: Canonical symbol types for all perl-lsp crates
- **Perl semantics**: Proper handling of sigil-based variable types ($, @, %)
- **LSP compatibility**: Direct mapping to LSP protocol symbol kinds
- **Zero-cost abstractions**: Copy types with inline methods
- **Document symbol support**: Richer variable type distinctions via `to_lsp_kind_document_symbol()`

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](../../LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](../../LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.
