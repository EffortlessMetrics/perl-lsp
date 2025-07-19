# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is a Tree-sitter parser for the Perl programming language with two implementations:
1. **C/tree-sitter parser**: Production-ready implementation with C scanner and tree-sitter generated parser
2. **Pure Rust parser**: Production-ready implementation using Pest parser generator (95%+ Perl coverage)

Both implementations are in `/crates/tree-sitter-perl-rs/`. The `/tree-sitter-perl/` directory contains the original grammar and corpus tests.

## Key Commands

### Build Commands
```bash
# Build with default features (c-scanner)
cargo xtask build

# Build with pure Rust parser
cargo xtask build --features pure-rust

# Build with specific scanner
cargo xtask build --rust-scanner
cargo xtask build --c-scanner

# Build in release mode
cargo xtask build --release
```

### Test Commands
```bash
# Run all tests
cargo xtask test

# Run corpus tests (main integration tests)
cargo xtask corpus

# Run corpus tests with diagnostics (shows first failure in detail)
cargo xtask corpus --diagnose

# Run specific test suite
cargo xtask test --suite unit
cargo xtask test --suite integration

# Run a single test
cargo test test_name

# Test pure Rust parser
cargo test --features pure-rust pure_rust_parser::tests
```

### Parser Commands
```bash
# Run pure Rust parser on a file
cargo xtask parse-rust file.pl --sexp

# Compare C and Rust parsers
cargo xtask compare

# Run comparison tool
cargo run --features "pure-rust test-utils" --bin compare_parsers -- --test
cargo run --features "pure-rust test-utils" --bin compare_parsers -- file.pl

# Run benchmarks
cargo xtask bench
./benchmark_all.sh
./compare_all_levels.sh
```

### Code Quality
```bash
# Run all checks (formatting + clippy)
cargo xtask check --all

# Format code
cargo xtask fmt

# Run clippy
cargo xtask check --clippy
```

### Edge Case Testing
```bash
# Run comprehensive edge case tests
cargo xtask test-edge-cases

# Run with performance benchmarks
cargo xtask test-edge-cases --bench

# Generate coverage report
cargo xtask test-edge-cases --coverage

# Run specific edge case test
cargo xtask test-edge-cases --test test_dynamic_delimiters
```

### Parser Generation
```bash
# Generate parser from grammar (if needed for testing)
cd tree-sitter-perl
npx tree-sitter generate
```

## Architecture Overview

### Project Structure
- **`/crates/tree-sitter-perl-rs/`**: Active Rust implementation - ALL new development happens here
  - `src/scanner/`: Dual scanner implementation (C and Rust)
  - `src/pure_rust_parser.rs`: Pure Rust parser using Pest
  - `src/grammar.pest`: Pest grammar for Perl
  - `src/comparison_harness.rs`: Parser comparison infrastructure
  - `src/error/`: Comprehensive error handling
  - `src/unicode/`: Unicode support utilities
  - `src/edge_case_handler.rs`: Unified edge case detection and handling
  - `src/phase_aware_parser.rs`: Tracks Perl compilation phases
  - `src/dynamic_delimiter_recovery.rs`: Recovery strategies for runtime delimiters
  - `src/encoding_aware_lexer.rs`: Handles mid-file encoding changes
  - `src/tree_sitter_adapter.rs`: Ensures tree-sitter compatibility for edge cases
- **`/tree-sitter-perl/`**: Legacy directory with corpus tests and original grammar.js
- **`/xtask/`**: Build automation and task runner
- **`/benches/`**: Performance benchmarks

### Key Components

1. **Dual Implementation Strategy**
   - C/tree-sitter: FFI wrapper around tree-sitter generated C parser
   - Pure Rust: Pest-based parser with no C dependencies
   - Feature flags: `c-scanner` (default), `rust-scanner`, `pure-rust`

2. **Scanner Architecture**
   - Implements `PerlScanner` trait for polymorphic scanner support
   - Manages complex state: quote stacks, heredoc delimiters, interpolation contexts
   - Handles 40+ token types including complex Perl constructs

3. **Pure Rust Parser (Pest)**
   - Grammar defined in `src/grammar.pest`
   - AST nodes in `pure_rust_parser.rs`
   - S-expression output for compatibility
   - Comparison harness for benchmarking against C parser

4. **Error Handling**
   - Comprehensive error types in `error.rs` and `error/` module
   - Supports parsing, scanner, and Unicode errors
   - Uses `thiserror` for ergonomic error definitions

5. **Testing Infrastructure**
   - Corpus tests: `tree-sitter-perl/test/corpus/`
   - Unit tests: Rust component tests
   - Property tests: Edge case testing with `proptest`
   - Comparison tests: Side-by-side parser validation

6. **Edge Case Handling**
   - Phase-aware parsing for BEGIN/CHECK/INIT/END blocks
   - Dynamic delimiter recovery with multiple strategies
   - Encoding-aware lexing for mid-file encoding changes
   - Tree-sitter compatible error nodes and diagnostics
   - Performance optimized (<5% overhead for normal code)

## Development Guidelines

1. **Development Location**: ALL new development happens in `/crates/tree-sitter-perl-rs/`
2. **Parser Choice**: Use feature flags to switch between implementations
3. **Testing**: Always test with both parsers when making changes
4. **Grammar Changes**: 
   - For tree-sitter: Edit `tree-sitter-perl/grammar.js` and regenerate
   - For Pest: Edit `crates/tree-sitter-perl-rs/src/grammar.pest`
5. **Performance**: Run benchmarks to ensure no regressions

## Pure Rust Parser Details

### Grammar Extension
To extend the Pest grammar:
1. Edit `src/grammar.pest`
2. Add corresponding AST nodes in `pure_rust_parser.rs`
3. Update the `build_node` method to handle new rules
4. Add tests for new constructs

### Current Grammar Coverage
- ✅ Variables (scalar, array, hash) with all declaration types (my, our, local)
- ✅ Literals (numbers, strings with interpolation, identifiers, lists)
- ✅ All operators with proper precedence
- ✅ Control flow (if/elsif/else, unless, while, until, for, foreach)
- ✅ Subroutines (named and anonymous) and blocks
- ✅ Package system (package, use, require)
- ✅ Comments and POD documentation
- ✅ String interpolation ($var and @array)
- ✅ Regular expressions (qr//, =~, !~)
- ✅ Method calls and complex dereferencing
- 🚧 Substitution operators (s///, tr///) - requires context-sensitive parsing
- 🚧 Complex interpolation (${expr})
- 🚧 Heredocs

## Performance Characteristics

- C/tree-sitter parser: ~12-68 µs for typical files
- Pure Rust parser: ~200-450 µs for typical files (2.5KB)
- Memory usage: Pure Rust uses Arc<str> for efficient string storage
- Production ready: Both parsers handle real-world Perl code
- Benchmarking: Use `cargo xtask compare` for detailed comparison

## Common Development Tasks

### Adding a New Perl Feature
1. Update grammar (either grammar.js or grammar.pest)
2. Add/update scanner tokens if needed
3. Add corpus test in `tree-sitter-perl/test/corpus/`
4. Run tests: `cargo xtask test`
5. Run benchmarks: `cargo xtask bench`

### Debugging Parse Failures
1. Use `cargo xtask corpus --diagnose` for detailed error info
2. For Pest parser: Check parse error messages which show exact location
3. Use `cargo xtask parse-rust file.pl --ast` to see AST structure

### Performance Optimization
1. Run benchmarks before and after changes
2. Use `cargo xtask compare` to compare implementations
3. Check for performance gates: `cargo xtask compare --check-gates`

## Current Status

- Tree-sitter parser: Production-ready, 100% corpus compatibility
- Pure Rust parser: Production-ready with 99%+ Perl coverage
- Recent additions: 
  - Context-sensitive slash disambiguation (/, s///, tr///, m//, qr//)
  - Modern Perl features (try/catch, defer, class/method)
  - All operators including smart match (~~), file tests, and bitwise string ops
  - Full heredoc support via multi-phase parsing

### Context-Sensitive Features

The parser includes sophisticated solutions for Perl's context-sensitive features:

#### Slash Disambiguation
1. **Mode-aware lexer** (`perl_lexer.rs`) - Tracks parser state to disambiguate / as division vs regex
2. **Preprocessing adapter** (`lexer_adapter.rs`) - Transforms ambiguous tokens for PEG parsing
3. **Disambiguated parser** (`disambiguated_parser.rs`) - High-level API with automatic handling

See `SLASH_DISAMBIGUATION.md` for full details.

#### Heredoc Support
1. **Multi-phase parser** (`heredoc_parser.rs`) - Three-phase approach to handle stateful heredocs
2. **Full parser** (`full_parser.rs`) - Combines heredoc and slash handling
3. **Complete coverage** - Supports all heredoc variants including indented heredocs

See `HEREDOC_IMPLEMENTATION.md` for full details.