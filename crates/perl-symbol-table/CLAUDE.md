# CLAUDE.md

This file provides guidance to Claude Code when working with code in this repository.

## Crate Overview

`perl-symbol-table` is a **Tier 1 leaf crate** providing symbol table and scope management for Perl LSP.

**Purpose**: Central data structure for tracking Perl symbols, references, and lexical scopes for IDE features like go-to-definition, find-all-references, and semantic highlighting.

**Version**: 0.9.0

## Commands

```bash
cargo build -p perl-symbol-table         # Build this crate
cargo test -p perl-symbol-table          # Run tests
cargo clippy -p perl-symbol-table        # Lint
cargo doc -p perl-symbol-table --open    # View documentation
```

## Architecture

### Dependencies

- `perl-symbol-types` - Symbol kind taxonomy (`SymbolKind`, `VarKind`)
- `perl-position-tracking` - Source location tracking (`SourceLocation`)

### Features

| Feature | Purpose |
|---------|---------|
| `serde` | Optional serialization support for `Symbol`, `SymbolReference`, `Scope`, `ScopeKind` |

### Key Types

| Type | Purpose |
|------|---------|
| `SymbolTable` | Central registry: symbols (by name), references (by name), scopes (by ID), scope stack, package context |
| `Symbol` | Definition with name, qualified_name, kind, location, scope_id, declaration, documentation, attributes |
| `SymbolReference` | Usage site with name, kind, location, scope_id, is_write flag |
| `Scope` | Lexical scope with id, parent, kind, location, and symbol name set |
| `ScopeKind` | Enum: Global, Package, Subroutine, Block, Eval |
| `ScopeId` | Type alias for `usize` |

### SymbolTable Methods

| Method | Signature | Purpose |
|--------|-----------|---------|
| `new()` | `-> Self` | Creates table with global scope (id=0, package "main") |
| `current_scope()` | `-> ScopeId` | Returns top of scope stack |
| `current_package()` | `-> &str` | Returns current package name |
| `set_current_package()` | `(String)` | Sets package context |
| `push_scope()` | `(ScopeKind, SourceLocation) -> ScopeId` | Creates child scope, pushes onto stack |
| `pop_scope()` | `()` | Pops current scope from stack |
| `add_symbol()` | `(Symbol)` | Registers symbol in table and scope's symbol set |
| `add_reference()` | `(SymbolReference)` | Adds usage reference |
| `find_symbol()` | `(&str, ScopeId, SymbolKind) -> Vec<&Symbol>` | Walks scope chain upward; also checks `our` variables |
| `find_references()` | `(&Symbol) -> Vec<&SymbolReference>` | Finds all references matching symbol name and kind |
| `all_symbols()` | `-> impl Iterator<Item = &Symbol>` | Iterates all symbols |
| `all_references()` | `-> impl Iterator<Item = &SymbolReference>` | Iterates all references |
| `get_scope()` | `(ScopeId) -> Option<&Scope>` | Looks up scope by ID |

## Usage

```rust
use perl_symbol_table::{Symbol, SymbolTable, ScopeKind, SymbolKind};
use perl_position_tracking::SourceLocation;

let mut table = SymbolTable::new();

// Add a symbol in global scope
table.add_symbol(Symbol {
    name: "foo".to_string(),
    qualified_name: "main::foo".to_string(),
    kind: SymbolKind::Subroutine,
    location: SourceLocation { start: 0, end: 10 },
    scope_id: table.current_scope(),
    declaration: None,
    documentation: None,
    attributes: vec![],
});

// Enter a subroutine scope
let sub_scope = table.push_scope(ScopeKind::Subroutine, SourceLocation { start: 10, end: 100 });

// Find symbol from inner scope (walks up scope chain)
let found = table.find_symbol("foo", sub_scope, SymbolKind::Subroutine);

// Exit scope
table.pop_scope();
```

## Important Notes

- `find_symbol` walks the scope chain upward and also checks `our`-declared variables across package scope
- Re-exports `SymbolKind` and `VarKind` from `perl-symbol-types` for convenience
- `current_scope()` uses `unwrap_or(&0)` so it falls back to global scope if the stack is empty
- All public fields on `SymbolTable` (`symbols`, `references`, `scopes`) use `HashMap` for O(1) lookup
- Used by the semantic analyzer and workspace indexer for reference resolution
