# CLAUDE.md

This file provides guidance to Claude Code when working with code in this crate.

## Crate Overview

`perl-ast-utils` is a **Tier 1 AST leaf crate** that provides shared AST range-lookup, insertion-position, and indentation helpers used by other LSP feature crates.

**Purpose**: Centralise generic AST and source-text helper logic (e.g., `find_node_at_range`, `find_statement_start`, `find_function_insert_position`, `get_indent_at`) so multiple LSP feature crates can reuse them without duplicating code.

**Version**: workspace (currently 0.12.3)

## Commands

```bash
cargo build -p perl-ast-utils          # Build this crate
cargo test -p perl-ast-utils           # Run tests
cargo clippy -p perl-ast-utils         # Lint
cargo doc -p perl-ast-utils --open     # View documentation
```

## Architecture

### Dependencies

- `perl-ast` — AST types (`Node`, `NodeKind`, `SourceLocation`)

### Key Functions

| Function | Purpose |
|----------|---------|
| `find_declaration_position` | Locate a suitable position to insert a new variable declaration |
| `find_statement_start` | Find the byte offset where a statement begins |
| `find_function_insert_position` | Find a position to insert a new subroutine |
| `find_node_at_range` | Walk the AST to find the node spanning a given byte range |
| `get_indent_at` | Determine the indentation level at a given position |

## Important Notes

- This is a single-responsibility microcrate — keep it focused on AST walking utilities.
- Depends only on `perl-ast`, NOT on `perl-parser-core`. This is the correct seam for IDE-facing AST consumers.
- Doctests are disabled (`doctest = false` in Cargo.toml).
- Dev-dependency `perl-ast` provides `Node`, `NodeKind`, `SourceLocation` for test helper construction.
