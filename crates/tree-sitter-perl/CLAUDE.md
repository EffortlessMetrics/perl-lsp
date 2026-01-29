# CLAUDE.md

This file provides guidance to Claude Code when working with code in this repository.

## Crate Overview

`tree-sitter-perl` is a **Tier 7 validation harness crate** for Tree-sitter Perl integration.

**Purpose**: Tree-sitter Perl validation harness (internal only) — provides benchmarking, validation, and comparison tools.

**Version**: 0.8.3
**Edition**: 2024
**publish**: false (not published to crates.io)

## Commands

```bash
cargo build -p tree-sitter-perl          # Build this crate
cargo test -p tree-sitter-perl           # Run tests
cargo clippy -p tree-sitter-perl         # Lint
cargo doc -p tree-sitter-perl --open     # View documentation
```

### Binaries

```bash
# Run benchmarks
cargo run -p tree-sitter-perl --bin bench_parser

# Parse single file
cargo run -p tree-sitter-perl --bin parse_file -- input.pl

# Additional utilities
cargo run -p tree-sitter-perl --bin <utility>
```

## Architecture

### Dependencies

**Internal (local paths)**:
- `perl-lexer` - Tokenization
- `perl-parser` - Native v3 parser

**External**:
- `tree-sitter` - Tree-sitter runtime
- `proptest` - Property-based testing
- `walkdir` - Directory traversal

**Parser Implementations**:
- `pest`, `pest_derive` - PEG parser
- `logos` - Lexer generator
- `chumsky` - Parser combinator

### Features

| Feature | Purpose |
|---------|---------|
| `pure-rust` | Pure Rust implementation (default) |
| `pure-rust-standalone` | Standalone pure Rust |
| `token-parser` | Token-based parsing |
| `test-utils` | Testing utilities |
| `c-scanner` | C scanner integration |
| `rust-scanner` | Rust scanner integration |

### Purpose

This crate is for **internal validation and benchmarking**:

1. **Parser Comparison** — Compare v3 native with tree-sitter
2. **Benchmarking** — Performance measurement across implementations
3. **Corpus Validation** — Test against tree-sitter test corpus
4. **Compatibility** — Ensure AST compatibility

### Corpus Location

```
tree-sitter-perl/test/corpus/
```

## Usage

```rust
use tree_sitter_perl::validate;

// Validate parser output against tree-sitter
let result = validate::compare_parsers(source)?;

if result.matches {
    println!("Parsers agree");
} else {
    println!("Differences: {:?}", result.differences);
}
```

### Benchmarking

```rust
use tree_sitter_perl::bench;

// Benchmark parsing performance
let results = bench::run_benchmarks(&corpus)?;

for result in results {
    println!("{}: {:?}", result.parser, result.duration);
}
```

## Important Notes

- **NOT published** — internal use only
- Used for parser validation, not production
- Contains multiple parser implementations for comparison
- Test corpus in `tree-sitter-perl/test/corpus/`
