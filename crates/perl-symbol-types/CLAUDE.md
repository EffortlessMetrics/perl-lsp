# CLAUDE.md

This file provides guidance to Claude Code when working with code in this repository.

## Crate Overview

`perl-symbol-types` is a **Tier 1 leaf crate** providing unified Perl symbol taxonomy for LSP tooling.

**Purpose**: Unified Perl symbol taxonomy for LSP tooling — defines symbol kinds, visibility, and metadata types.

**Version**: 0.8.8

## Commands

```bash
cargo build -p perl-symbol-types         # Build this crate
cargo test -p perl-symbol-types          # Run tests
cargo clippy -p perl-symbol-types        # Lint
cargo doc -p perl-symbol-types --open    # View documentation
```

## Architecture

### Dependencies

- `serde` - Serialization (optional)

### Key Types

| Type | Purpose |
|------|---------|
| `SymbolKind` | Kind of symbol (variable, sub, package, etc.) |
| `Visibility` | Symbol visibility (lexical, package, global) |
| `SigilType` | Variable sigil ($, @, %, *, &) |
| `DeclarationKind` | How symbol was declared (my, our, local, state) |

### Symbol Kinds

```rust
pub enum SymbolKind {
    // Variables
    Scalar,      // $x
    Array,       // @arr
    Hash,        // %hash
    Glob,        // *foo

    // Code
    Subroutine,  // sub foo {}
    Method,      // Method in a class
    Constant,    // use constant

    // Structural
    Package,     // package Foo;
    Module,      // .pm file
    Class,       // class Foo {} (Perl 5.38+)

    // Other
    Label,       // LABEL:
    Format,      // format NAME =
    TypeGlob,    // typeglob assignment
}
```

### Visibility Levels

```rust
pub enum Visibility {
    Lexical,    // my - visible in current scope
    Package,    // our - visible in current package
    Local,      // local - dynamic scope
    State,      // state - persistent lexical
    Global,     // No declaration - fully qualified
}
```

## Usage

```rust
use perl_symbol_types::{SymbolKind, Visibility, SigilType};

let symbol = Symbol {
    name: "count".to_string(),
    kind: SymbolKind::Scalar,
    visibility: Visibility::Lexical,
    sigil: SigilType::Scalar,
    // ...
};

// Pattern matching on kind
match symbol.kind {
    SymbolKind::Subroutine => { /* ... */ },
    SymbolKind::Scalar | SymbolKind::Array | SymbolKind::Hash => {
        // Variable
    },
    _ => { /* ... */ },
}
```

## Important Notes

- This crate defines types only — no logic
- Used consistently across semantic analyzer and LSP providers
- Changes affect symbol reporting throughout the system
