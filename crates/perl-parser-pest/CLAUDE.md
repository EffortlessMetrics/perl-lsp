# CLAUDE.md

This file provides guidance to Claude Code when working with code in this repository.

## Crate Overview

`perl-parser-pest` is a **legacy Tier 7 crate** providing the v2 Pest-based Perl parser.

**Purpose**: Legacy Pest-based Perl parser (v2) — maintained as a learning tool and compatibility reference. NOT in the default gate.

**Version**: 0.8.8

## Commands

```bash
cargo build -p perl-parser-pest          # Build this crate
cargo test -p perl-parser-pest           # Run tests
cargo clippy -p perl-parser-pest         # Lint
cargo doc -p perl-parser-pest --open     # View documentation
```

## Architecture

### Dependencies

- `pest`, `pest_derive` - PEG parser generator
- `stacker` - Stack overflow protection
- `thiserror` - Error definitions
- `regex` - Pattern matching
- `unicode-ident` - Unicode support
- `once_cell`, `lazy_static` - Lazy initialization

### Features

| Feature | Purpose |
|---------|---------|
| `serde` | Serialization support (optional) |

### Parser Versions

| Version | Implementation | Status |
|---------|----------------|--------|
| v1 | C-based (Tree-sitter) | Benchmarking only |
| **v2** | **Pest PEG (this crate)** | **Legacy** |
| v3 | Native recursive descent | Current |

### Grammar File

The Pest grammar is defined in a `.pest` file:

```pest
// perl.pest
program = { SOI ~ statement* ~ EOI }
statement = { expression ~ ";" | control_structure | ... }
expression = { term ~ (operator ~ term)* }
// ...
```

## Why Legacy?

The Pest parser was replaced by the native v3 parser because:

1. **Performance** — Native parser is faster
2. **Error Recovery** — Better error handling in v3
3. **Flexibility** — Easier to add Perl-specific features
4. **LSP Features** — Better integration with IDE features

## Usage

```rust
use perl_parser_pest::Parser;

let source = "my $x = 42;";
let result = Parser::parse(source);

match result {
    Ok(ast) => { /* process AST */ },
    Err(e) => { /* handle parse error */ },
}
```

## Important Notes

- **NOT in default gate** — excluded from `just ci-gate`
- Used for benchmarking comparisons with v3
- Kept for learning and reference
- Do not add new features to this parser
- See `perl-parser` for the current parser
