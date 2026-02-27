# CLAUDE.md

This file provides guidance to Claude Code when working with code in this crate.

## Crate Overview

- **Tier**: 1 (leaf crate, no internal workspace dependencies)
- **Purpose**: Single source of truth for Perl symbol classification across the perl-lsp ecosystem
- **Version**: 0.9.1

## Commands

```bash
cargo build -p perl-symbol-types         # Build
cargo test -p perl-symbol-types          # Run tests
cargo clippy -p perl-symbol-types        # Lint
cargo doc -p perl-symbol-types --open    # View docs
```

## Architecture

### Dependencies

- `serde` (with `derive` feature) -- serialization support for all types

### Downstream Consumers

- `perl-symbol-table` -- symbol table storage
- `perl-workspace-index` -- workspace-wide symbol indexing
- `perl-semantic-analyzer` -- semantic analysis

### Key Types (all in `src/lib.rs`)

| Type | Purpose |
|------|---------|
| `VarKind` | Variable sigil classification: `Scalar`, `Array`, `Hash` |
| `SymbolKind` | Unified symbol taxonomy: `Package`, `Class`, `Role`, `Subroutine`, `Method`, `Variable(VarKind)`, `Constant`, `Import`, `Export`, `Label`, `Format` |

### Key Methods on `SymbolKind`

| Method | Returns | Description |
|--------|---------|-------------|
| `to_lsp_kind()` | `u32` | Generic LSP symbol kind mapping (all variables map to 13) |
| `to_lsp_kind_document_symbol()` | `u32` | Richer mapping distinguishing `$`=13, `@`=18, `%`=19 |
| `sigil()` | `Option<&str>` | Returns sigil for variable kinds, `None` otherwise |
| `is_variable()` | `bool` | True for any `Variable(_)` variant |
| `is_callable()` | `bool` | True for `Subroutine` or `Method` |
| `is_namespace()` | `bool` | True for `Package`, `Class`, or `Role` |
| `scalar()` / `array()` / `hash()` | `Self` | Convenience constructors |

## Usage

```rust
use perl_symbol_types::{SymbolKind, VarKind};

let var = SymbolKind::Variable(VarKind::Scalar);
assert_eq!(var.sigil(), Some("$"));
assert!(var.is_variable());

// LSP protocol mapping
assert_eq!(SymbolKind::Subroutine.to_lsp_kind(), 12);

// Richer document symbol mapping
assert_eq!(SymbolKind::Variable(VarKind::Array).to_lsp_kind_document_symbol(), 18);
```

## Important Notes

- This crate defines **types and classification logic only** -- no parsing or analysis
- All types derive `Copy`, `Eq`, `Hash`, `Serialize`, `Deserialize`
- Doctests are disabled (`doctest = false` in Cargo.toml); examples in doc comments are for documentation only
- Changes to symbol variants or LSP mappings affect symbol reporting across the entire workspace
