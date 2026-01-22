# perl-symbol-table

Symbol table and scope management for the Perl LSP ecosystem.

## Features

- **Symbol tracking**: Track symbol definitions with metadata (name, kind, location, scope, documentation)
- **Reference tracking**: Track symbol usages for find-all-references
- **Scope management**: Hierarchical scope tracking with Perl scoping rules
- **LSP integration**: Designed for go-to-definition, find-references, rename refactoring

## Core Types

- `Symbol` - A symbol definition with metadata
- `SymbolReference` - A reference to a symbol
- `SymbolTable` - Central registry of symbols, references, and scopes
- `Scope` - A lexical scope boundary
- `ScopeKind` - Classification of scope types (Global, Package, Subroutine, Block, Eval)
- `ScopeId` - Unique identifier for a scope

## Usage

```rust
use perl_symbol_table::{Symbol, SymbolTable, ScopeKind};
use perl_symbol_types::SymbolKind;
use perl_position_tracking::SourceLocation;

let mut table = SymbolTable::new();

// Add a subroutine symbol
let symbol = Symbol {
    name: "process".to_string(),
    qualified_name: "MyPackage::process".to_string(),
    kind: SymbolKind::Subroutine,
    location: SourceLocation { start: 0, end: 100 },
    scope_id: table.current_scope(),
    declaration: None,
    documentation: Some("Process data".to_string()),
    attributes: vec![],
};

table.add_symbol(symbol);

// Create a scope for function body
let sub_scope = table.push_scope(
    ScopeKind::Subroutine,
    SourceLocation { start: 10, end: 90 }
);

// Add local variable
let var = Symbol {
    name: "data".to_string(),
    qualified_name: "data".to_string(),
    kind: SymbolKind::scalar(),
    location: SourceLocation { start: 20, end: 30 },
    scope_id: sub_scope,
    declaration: Some("my".to_string()),
    documentation: None,
    attributes: vec![],
};

table.add_symbol(var);

// Pop scope when done
table.pop_scope();
```

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.
