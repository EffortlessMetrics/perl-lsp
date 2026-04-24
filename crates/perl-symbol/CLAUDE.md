# CLAUDE.md

This file provides guidance to Claude Code when working with code in this crate.

## Crate Overview

- **Crate**: `perl-symbol` (NEW published crate, Wave B collapse of 4 `perl-symbol-*` satellites)
- **Version**: workspace (currently 0.12.4)
- **Tier**: Tier 2+ (depends on `perl-ast`; consumed by `perl-workspace-index`, `perl-semantic-analyzer`, `perl-lsp`, `perl-lsp-rename`, `perl-lsp-performance`)
- **Purpose**: Unified Perl symbol model for the entire perl-lsp ecosystem — taxonomy, cursor extraction, search indexing, and AST surface projection.

## Commands

```bash
cargo build -p perl-symbol         # Build
cargo test -p perl-symbol          # Run tests
cargo clippy -p perl-symbol        # Lint
cargo doc -p perl-symbol --open    # View docs
```

## Architecture

### Allowed Dependencies

- `perl-ast` — AST node types (used by `surface` module only)
- `serde` — serialization support for `types` (SymbolKind/VarKind derive)

### NOT Allowed (architectural invariant from `perl-symbol-surface`)

> **NOT allowed** (for the `surface` module in particular, and for the crate
> as a whole): `perl-parser-core`, `lsp-types`, or any LSP provider crate.

This invariant keeps the `surface` module a clean projection layer between the
Perl syntax model and IDE features. Violating it would re-introduce the
dependency inversion that ADR-0041 explicitly consolidated away.

### Downstream Consumers

- `perl-workspace-index` — workspace-wide symbol indexing (uses `types`)
- `perl-semantic-analyzer` — semantic analysis (uses `types`)
- `perl-lsp` — LSP server (uses `cursor`)
- `perl-lsp-rename` — rename provider (uses `cursor`)
- `perl-lsp-performance` — caching / perf (uses `index`)

## Modules

### `types` — Symbol taxonomy

Key types: `SymbolKind`, `VarKind`. Re-exported at crate root for ergonomics
(`perl_symbol::SymbolKind`, `perl_symbol::VarKind`).

| Type | Purpose |
|------|---------|
| `VarKind` | Variable sigil classification: `Scalar`, `Array`, `Hash` |
| `SymbolKind` | Unified symbol taxonomy: `Package`, `Class`, `Role`, `Subroutine`, `Method`, `Variable(VarKind)`, `Constant`, `Import`, `Export`, `Label`, `Format` |

Key methods on `SymbolKind`:

| Method | Returns | Description |
|--------|---------|-------------|
| `to_lsp_kind()` | `u32` | Generic LSP symbol kind mapping (all variables map to 13) |
| `to_lsp_kind_document_symbol()` | `u32` | Richer mapping distinguishing `$`=13, `@`=18, `%`=19 |
| `sigil()` | `Option<&str>` | Returns sigil for variable kinds, `None` otherwise |
| `is_variable()` / `is_callable()` / `is_namespace()` | `bool` | Category predicates |
| `scalar()` / `array()` / `hash()` | `Self` | Convenience constructors |

### `cursor` — Cursor-based extraction

Key type: `CursorSymbolKind`. Helpers: `extract_symbol_from_source`,
`get_symbol_range_at_position`, `byte_offset_utf16`, `token_under_cursor`,
`is_modchar`, `is_word_boundary`.

### `index` — Symbol search

Key type: `SymbolIndex` (trie + inverted index). API: `new`, `add_symbol`,
`search_prefix`, `search_fuzzy`.

### `surface` — AST projection

Key types: `SymbolDecl`, `extract_symbol_decls`. Walks a `perl_ast::Node` and
emits a flat `Vec<SymbolDecl>` with kind, name, qualified name, spans, and
container metadata.

## Important Notes

- Doctests are disabled (`doctest = false` in Cargo.toml); examples in doc
  comments are for documentation only.
- All `types` values derive `Copy`, `Eq`, `Hash`, `Serialize`, `Deserialize`.
- Changes to `SymbolKind` variants or LSP mappings affect symbol reporting
  across the entire workspace — verify against `perl-lsp-rs` snapshot tests.
- The `surface` module depends on `perl-ast` and MUST NOT take on any LSP or
  parser-core dependencies.
- `facade_api_completeness.rs` (tests/) guards the public API surface against
  accidental breakage; update it if a new public item is added.
