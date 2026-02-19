# perl-symbol-types

Unified Perl symbol taxonomy for LSP tooling.

Part of the [perl-lsp](https://github.com/EffortlessMetrics/tree-sitter-perl-rs) workspace.

## Public API

- **`VarKind`** -- Variable sigil classification: `Scalar` (`$`), `Array` (`@`), `Hash` (`%`).
- **`SymbolKind`** -- Canonical symbol taxonomy: `Package`, `Class`, `Role`, `Subroutine`, `Method`, `Variable(VarKind)`, `Constant`, `Import`, `Export`, `Label`, `Format`.

Key methods on `SymbolKind`: `to_lsp_kind()`, `to_lsp_kind_document_symbol()`, `sigil()`, `is_variable()`, `is_callable()`, `is_namespace()`, plus convenience constructors `scalar()`, `array()`, `hash()`.

## Usage

```rust
use perl_symbol_types::{SymbolKind, VarKind};

let var = SymbolKind::scalar();
assert_eq!(var.sigil(), Some("$"));
assert!(var.is_variable());
assert_eq!(SymbolKind::Subroutine.to_lsp_kind(), 12); // LSP Function
```

## License

Licensed under MIT OR Apache-2.0 at your option.
