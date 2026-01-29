# CLAUDE.md

This file provides guidance to Claude Code when working with code in this repository.

## Crate Overview

`tree-sitter-perl-c` is a **Tier 7 legacy benchmarking crate** providing Tree-sitter Perl grammar with C scanner.

**Purpose**: Tree-sitter Perl grammar with C scanner (legacy implementation) — used for benchmarking against native Rust parser.

**Version**: 0.8.3

## Commands

```bash
cargo build -p tree-sitter-perl-c        # Build this crate
cargo test -p tree-sitter-perl-c         # Run tests
cargo clippy -p tree-sitter-perl-c       # Lint
cargo doc -p tree-sitter-perl-c --open   # View documentation
```

### Binaries

```bash
# Run C parser benchmark
cargo run -p tree-sitter-perl-c --bin bench_parser_c

# Parse with C parser
cargo run -p tree-sitter-perl-c --bin parse_c -- input.pl
```

## Architecture

### Dependencies

- `tree-sitter` - Tree-sitter runtime
- `proptest` - Property-based testing
- `walkdir` - Directory traversal
- `thiserror` - Error definitions
- `serde` - Serialization

### Build Dependencies

- `cc` - C compilation
- `bindgen` - C binding generation

### Features

| Feature | Purpose |
|---------|---------|
| `c-scanner` | C scanner (default) |
| `test-utils` | Testing utilities |

### C Scanner

The C scanner handles Perl's complex lexical grammar:

```c
// scanner.c
// Handles heredocs, regex delimiters, quote-like operators
```

Building requires C toolchain:

```bash
# Linux
apt install build-essential

# macOS
xcode-select --install
```

## Usage

```rust
use tree_sitter_perl_c::Parser;

let mut parser = Parser::new();
parser.set_language(tree_sitter_perl_c::language())?;

let tree = parser.parse(source, None)?;
let root = tree.root_node();
```

### Benchmarking

```rust
use tree_sitter_perl_c::bench;

// Compare C parser with v3 native
let c_result = bench::parse_with_c(source)?;
let native_result = perl_parser::parse(source)?;

// Measure performance difference
```

## Important Notes

- Requires C compiler for building
- Used for benchmarking only
- Legacy implementation — native v3 is current
- C scanner handles edge cases differently than Rust
