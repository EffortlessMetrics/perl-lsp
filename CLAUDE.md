# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is a **Pure Rust Perl Parser** built with Pest parser generator for Rust 2024. The parser:
- Uses Pest PEG grammar for parsing Perl 5 syntax
- Outputs tree-sitter compatible S-expressions
- Has no C dependencies (pure Rust implementation)
- Achieves 95%+ Perl syntax coverage with comprehensive edge case handling

The main implementation is in `/crates/tree-sitter-perl-rs/`. Legacy tree-sitter files in `/tree-sitter-perl/` are kept for reference only.

## Key Commands

### Build Commands
```bash
# Build the Pure Rust parser (canonical)
cargo xtask build --features pure-rust

# Build in release mode
cargo xtask build --features pure-rust --release

# Build from crate directory
cd crates/tree-sitter-perl-rs
cargo build --features pure-rust
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
cargo test --features pure-rust
```

### Parser Commands
```bash
# Parse a Perl file with Pure Rust parser
cargo xtask parse-rust file.pl --sexp

# Parse from stdin
echo 'print "Hello"' | cargo run --features pure-rust --bin parse-rust -- -

# Run directly from crate
cd crates/tree-sitter-perl-rs
cargo run --features pure-rust --bin parse-rust -- script.pl

# Run benchmarks
cargo bench --features pure-rust
cargo xtask bench
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
- **`/crates/tree-sitter-perl-rs/`**: Pure Rust Perl parser implementation
  - `src/pure_rust_parser.rs`: Main Pest-based parser
  - `src/grammar.pest`: Complete Perl 5 PEG grammar
  - `src/error/`: Error handling and diagnostics
  - `src/unicode/`: Unicode identifier support
  - `src/edge_case_handler.rs`: Heredoc edge case detection
  - `src/phase_aware_parser.rs`: BEGIN/END block handling
  - `src/tree_sitter_adapter.rs`: S-expression output formatting
  - `src/lib.rs`: Public API and exports
- **`/xtask/`**: Development automation tools
- **`/docs/`**: Architecture and design documentation
- **`/benches/`**: Performance benchmarks

### Key Components

1. **Pest Parser Architecture**
   - PEG grammar in `grammar.pest` defines all Perl syntax
   - Recursive descent parsing with packrat optimization
   - Zero-copy parsing with `&str` slices
   - Feature flag: `pure-rust` enables the Pest parser

2. **AST Generation**
   - Strongly typed AST nodes in `pure_rust_parser.rs`
   - Arc<str> for efficient string storage
   - Tree-sitter compatible node types
   - Position tracking for all nodes

3. **S-Expression Output**
   - `to_sexp()` method produces tree-sitter format
   - Compatible with existing tree-sitter tools
   - Preserves all position information
   - Error nodes for unparseable constructs

4. **Edge Case Handling**
   - Comprehensive heredoc support (99% coverage)
   - Phase-aware parsing for BEGIN/END blocks
   - Dynamic delimiter detection and recovery
   - Clear diagnostics for unparseable constructs

5. **Testing Strategy**
   - Grammar tests for each Perl construct
   - Edge case tests with property testing
   - Performance benchmarks
   - Integration tests for S-expression output

   - Encoding-aware lexing for mid-file encoding changes
   - Tree-sitter compatible error nodes and diagnostics
   - Performance optimized (<5% overhead for normal code)

## Development Guidelines

1. **Primary Parser**: Pure Rust Pest parser is the canonical implementation
2. **Development Location**: ALL new development in `/crates/tree-sitter-perl-rs/`
3. **Grammar Changes**: Edit `crates/tree-sitter-perl-rs/src/grammar.pest`
4. **Testing**: Use `cargo test --features pure-rust`
5. **Performance**: Run benchmarks to ensure no regressions
6. **Legacy C Code**: C parser/scanner exist only for benchmarking comparison

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
- ✅ Substitution operators (s///, tr///) via context-sensitive parsing
- ✅ Complex interpolation (${expr})
- ✅ Heredocs with multi-phase parsing

## Performance Characteristics

- Pure Rust parser: ~200-450 µs for typical files (2.5KB)
- Memory usage: Arc<str> for zero-copy string storage
- Production ready: Handles real-world Perl code
- Predictable: ~180 µs/KB parsing speed
- Legacy C parser: ~12-68 µs (kept for benchmark reference only)

## Common Development Tasks

### Adding a New Perl Feature
1. Update `src/grammar.pest` with new syntax rules
2. Add corresponding AST nodes in `pure_rust_parser.rs`
3. Update `build_node()` method to handle new constructs
4. Add tests in `tests/` directory
5. Run tests: `cargo test --features pure-rust`
6. Run benchmarks: `cargo bench --features pure-rust`

### Debugging Parse Failures
1. Use `cargo xtask corpus --diagnose` for detailed error info
2. For Pest parser: Check parse error messages which show exact location
3. Use `cargo xtask parse-rust file.pl --ast` to see AST structure

### Performance Optimization
1. Run benchmarks before and after changes
2. Use `cargo xtask compare` to compare implementations
3. Check for performance gates: `cargo xtask compare --check-gates`

## Current Status

**Pure Rust Pest Parser**: Production-ready with 99%+ Perl coverage
- Complete Perl 5 syntax support
- Tree-sitter compatible S-expression output
- Context-sensitive features (slash disambiguation, heredocs)
- Modern Perl features (try/catch, defer, class/method)
- All operators including smart match (~~), file tests, bitwise ops
- Comprehensive edge case handling system

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

#### Edge Case Handling
1. **Edge case handler** (`edge_case_handler.rs`) - Unified detection and recovery system
2. **Phase-aware parsing** (`phase_aware_parser.rs`) - Handles BEGIN/CHECK/INIT/END blocks
3. **Dynamic recovery** (`dynamic_delimiter_recovery.rs`) - Multiple strategies for runtime delimiters
4. **Tree-sitter adapter** (`tree_sitter_adapter.rs`) - Ensures 100% AST compatibility

See `docs/EDGE_CASES.md` for comprehensive documentation.