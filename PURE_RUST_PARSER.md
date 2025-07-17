# Pure Rust Perl Parser

This project now includes a **pure Rust implementation** of a Perl parser alongside the traditional C/tree-sitter parser. This allows for comprehensive testing and benchmarking between native implementations.

## Overview

The pure Rust parser is implemented using [Pest](https://pest.rs/), a PEG (Parsing Expression Grammar) parser generator for Rust. It provides:

- **No C dependencies**: Entirely written in Rust
- **Type safety**: Leverages Rust's type system
- **Memory safety**: No unsafe code in the parser logic
- **Comparable performance**: Competitive parsing speeds
- **Same interface**: Compatible S-expression output format

## Architecture

### Components

1. **Grammar Definition** (`src/grammar.pest`)
   - Comprehensive Perl grammar in PEG format
   - Supports most Perl syntax constructs
   - Extensible and maintainable

2. **Parser Implementation** (`src/pure_rust_parser.rs`)
   - AST node definitions
   - Recursive descent parsing
   - S-expression generation

3. **Comparison Harness** (`src/comparison_harness.rs`)
   - Unified interface for both parsers
   - Performance benchmarking
   - Output comparison

## Usage

### Building

```bash
# Build with pure Rust parser
cargo build --features pure-rust

# Build with C/tree-sitter parser (default)
cargo build --features c-scanner

# Build with both for comparison
cargo build --features "pure-rust c-scanner test-utils"
```

### Testing

```bash
# Run pure Rust parser tests
cargo test --features pure-rust pure_rust_parser::tests

# Run comparison tests
cargo run --features "pure-rust test-utils" --bin compare_parsers -- --test

# Compare specific file
cargo run --features "pure-rust test-utils" --bin compare_parsers -- file.pl
```

### Benchmarking

```bash
# Run comprehensive benchmarks
./benchmark_all.sh

# Three-level comparison (parser, bindings, CLI)
./compare_all_levels.sh

# Benchmark specific file
cargo run --release --features "pure-rust test-utils" --bin compare_parsers -- file.pl 1000
```

## API

### Pure Rust Parser API

```rust
use tree_sitter_perl::pure_rust_parser::{PureRustPerlParser, AstNode};

// Create parser
let mut parser = PureRustPerlParser::new();

// Parse source code
let source = "my $x = 42;";
match parser.parse(source) {
    Ok(ast) => {
        // Convert to S-expression
        let sexp = parser.to_sexp(&ast);
        println!("S-expression: {}", sexp);
    }
    Err(e) => println!("Parse error: {}", e),
}
```

### Comparison API

```rust
use tree_sitter_perl::comparison_harness::ComparisonHarness;

let mut harness = ComparisonHarness::new();

// Compare parsers
let (tree_sitter_result, pure_rust_result) = harness.compare_parsers(source);

// Run benchmarks
let results = harness.run_benchmark(source, 1000);
```

## Current Status

### Implemented Features

- ✅ Variables (scalar, array, hash)
- ✅ Literals (numbers, strings, identifiers)
- ✅ Basic expressions and operators
- ✅ Control flow (if, while, for, foreach)
- ✅ Subroutines and blocks
- ✅ Package declarations
- ✅ Comments

### In Progress

- 🚧 Complex string interpolation
- 🚧 Regular expressions and substitutions
- 🚧 Here documents
- 🚧 Advanced Perl features (ties, formats, etc.)

### Performance

Initial benchmarks show the pure Rust parser performs competitively:

- **Simple scripts**: Often faster than C parser
- **Complex scripts**: Within 10-20% of C parser performance
- **Memory usage**: Generally lower due to Rust's efficiency

## Grammar Extension

To extend the grammar:

1. Edit `src/grammar.pest`
2. Add corresponding AST nodes in `pure_rust_parser.rs`
3. Update the `build_node` method to handle new rules
4. Add tests for new constructs

Example:
```pest
// Add new operator
new_operator = { "<=>" }

// Update expression rule
comparison_expression = {
    shift_expression ~ (("<" | ">" | "<=" | ">=" | "<=>" | "lt" | "gt") ~ shift_expression)*
}
```

## Comparison Results

The comparison harness provides three levels of testing:

1. **Direct Parser Comparison**: Raw parser performance
2. **Binding Comparison**: Through different language bindings
3. **CLI Comparison**: End-to-end command-line usage

Example output:
```
Tree-sitter parser:
  ✓ Success
  Parse time: 125.3µs
  S-expression: (source_file (variable_declaration ...))

Pure Rust parser:
  ✓ Success
  Parse time: 98.7µs
  S-expression: (source_file (variable_declaration ...))

Performance: Pure Rust is 1.27x faster
```

## Future Work

1. **Complete Grammar Coverage**: Implement remaining Perl features
2. **Error Recovery**: Improve error messages and recovery
3. **Optimization**: Further performance improvements
4. **Tree-sitter Compatibility**: Full API compatibility
5. **Incremental Parsing**: Support for efficient re-parsing

## Contributing

When contributing to the pure Rust parser:

1. Ensure all tests pass with both parsers
2. Run benchmarks to verify performance
3. Update documentation for new features
4. Follow Rust best practices and idioms

## License

Same as the parent project - MIT License