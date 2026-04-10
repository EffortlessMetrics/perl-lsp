# CLAUDE.md

This file provides guidance to Claude Code when working with code in this crate.

## Crate Overview

`perl-symbol-surface` is the **projection layer** between the Perl syntax model (`perl-ast`) and IDE features (semantic analyzer, workspace index, navigation, rename, workspace symbols, call hierarchy).

**Purpose**: Derive stable, reusable symbol-bearing views from the AST so that every IDE feature uses the same extraction logic rather than each implementing its own AST pattern-match.

**Version**: workspace (currently 0.12.3)

## Commands

```bash
cargo build -p perl-symbol-surface          # Build this crate
cargo test -p perl-symbol-surface           # Run tests (11 tests)
cargo clippy -p perl-symbol-surface         # Lint
cargo doc -p perl-symbol-surface --open     # View documentation
cargo tree -p perl-symbol-surface           # Verify dep graph (no perl-parser-core)
```

## Architecture

### Dependencies

- `perl-ast` — AST node types (`Node`, `NodeKind`, `SourceLocation`)
- `perl-position-tracking` — `SourceLocation` span type
- `perl-symbol-types` — `SymbolKind`, `VarKind` (canonical taxonomy)

**NOT allowed**: `perl-parser-core`, `lsp-types`, or any LSP provider crate.

### Source Modules

| File | Purpose |
|------|---------|
| `lib.rs` | Re-exports `SymbolDecl` and `extract_symbol_decls`; module declarations |
| `decl.rs` | `SymbolDecl` struct + `extract_symbol_decls` function |

### Key Types

| Type | Module | Purpose |
|------|--------|---------|
| `SymbolDecl` | `decl` | Projected declaration site: kind, name, qualified_name, full_span, anchor_span, container |
| `extract_symbol_decls` | `decl` | Single-pass AST walker producing `Vec<SymbolDecl>` |

### What `SymbolDecl` Projects

| AST node | `SymbolKind` emitted |
|----------|---------------------|
| `Package { name, .. }` | `Package` |
| `Class { name, .. }` | `Class` |
| `Subroutine { name: Some(..), .. }` | `Subroutine` |
| `Method { name, .. }` | `Method` |
| `VariableDeclaration { variable, .. }` | `Variable(VarKind)` |
| `Use { module: "constant", args, .. }` | `Constant` |

Anonymous subroutines (`name: None`) are skipped.

### Package Context Propagation

The walker tracks the innermost `package` declaration in statement order:

- `Package { block: None, .. }` — updates context for all subsequent siblings
- `Package { block: Some(..), .. }` — scopes context to just that block
- `Class { .. }` — scopes context to the class body

### Fields of `SymbolDecl`

| Field | Type | Meaning |
|-------|------|---------|
| `kind` | `SymbolKind` | Unified classification |
| `name` | `String` | Bare, unqualified name |
| `qualified_name` | `String` | `Foo::bar` inside a package; equals `name` at top level |
| `full_span` | `(usize, usize)` | Byte range of the full declaration node |
| `anchor_span` | `Option<(usize, usize)>` | Byte range of the name token; `None` when AST lacks `name_span` |
| `container` | `Option<String>` | Enclosing package name; `None` at top level |
| `declarator` | `Option<String>` | Variable scope keyword: `"my"`, `"our"`, `"local"`, `"state"`; `None` for non-variable declarations. `"our"` means package-scoped (cross-file visible). |

### Future Phases

| Type | Status | Purpose |
|------|--------|---------|
| `SymbolRef` | Not yet | Reference sites: kind, name, span, is_write |
| `CallSite` | Not yet | Call sites: dispatch kind, callee_name, callee_span |

## Important Notes

- Depends only on `perl-ast`, `perl-position-tracking`, `perl-symbol-types` — this invariant MUST be maintained.
- Doctests are disabled (`doctest = false` in Cargo.toml).
- `anchor_span` is `None` for `Class` and `use constant` — the AST does not carry `name_span` for these nodes.
- The `Walk` function recurses into subroutine bodies to catch nested subs and closures.
- This crate does NOT do semantic analysis (scope tracking, cross-file resolution) — that is `perl-semantic-analyzer`'s domain.
