# CLAUDE.md

This file provides guidance to Claude Code when working with code in this repository.

## Crate Overview

`perl-symbol-table` is a **Tier 2 utility crate** providing symbol table and scope management for Perl LSP.

**Purpose**: Symbol table and scope management for Perl LSP — stores and retrieves symbols within lexical scopes.

**Version**: 0.8.8

## Commands

```bash
cargo build -p perl-symbol-table         # Build this crate
cargo test -p perl-symbol-table          # Run tests
cargo clippy -p perl-symbol-table        # Lint
cargo doc -p perl-symbol-table --open    # View documentation
```

## Architecture

### Dependencies

- `perl-symbol-types` - Symbol taxonomy
- `perl-position-tracking` - Position handling

### Features

| Feature | Purpose |
|---------|---------|
| `serde` | Optional serialization support |

### Key Types

| Type | Purpose |
|------|---------|
| `SymbolTable` | Collection of symbols with scope hierarchy |
| `Scope` | Single lexical scope |
| `ScopeId` | Identifier for a scope |
| `SymbolEntry` | Symbol with its scope context |

### Scope Hierarchy

```rust
// Perl scopes nest:
// - File scope (package main)
//   - Package scope
//     - Subroutine scope
//       - Block scope (if, while, etc.)
//         - Nested block scope
```

## Usage

```rust
use perl_symbol_table::{SymbolTable, Scope};

let mut table = SymbolTable::new();

// Enter new scope
let scope_id = table.push_scope();

// Add symbol to current scope
table.insert("$x", symbol_info);

// Lookup symbol (searches up scope chain)
if let Some(symbol) = table.lookup("$x") {
    println!("Found: {:?}", symbol);
}

// Exit scope
table.pop_scope();
```

### Scope Queries

```rust
// Find all symbols visible at a position
let visible = table.visible_at(position);

// Find symbol declaration
let decl = table.find_declaration("$x", position);

// Get enclosing scope
let scope = table.scope_at(position);
```

## Important Notes

- Scope hierarchy mirrors Perl's lexical scoping rules
- Symbol shadowing is handled correctly
- Used by semantic analyzer for reference resolution
