# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is a Tree-sitter parser for the Perl programming language. The active implementation is in `/crates/tree-sitter-perl-rs/` (Rust), while `/tree-sitter-perl/` contains legacy code used only for testing purposes. The project is finishing the transition to the new Rust implementation.

## Key Commands

### Build Commands
```bash
# Build with default features (rust-scanner)
cargo xtask build

# Build with specific scanner
cargo xtask build --rust-scanner
cargo xtask build --c-scanner
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

### Parser Generation
```bash
# Generate parser from grammar (if needed for testing)
cd tree-sitter-perl
npx tree-sitter generate
```

## Architecture Overview

### Project Structure
- **`/crates/tree-sitter-perl-rs/`**: The active Rust parser implementation - ALL new development happens here
- **`/tree-sitter-perl/`**: Legacy testing directory containing corpus tests and the original grammar.js
- **`/xtask/`**: Build automation tooling

### Key Components

1. **Dual Scanner Implementation**
   - C Scanner: Legacy implementation in `scanner.c`
   - Rust Scanner: New implementation in `rust_scanner.rs`
   - Controlled by feature flags: `rust-scanner` (default) vs `c-scanner`

2. **Parser Generation Flow**
   - `grammar.js` (in legacy directory) defines Perl syntax rules
   - Tree-sitter generates `parser.c` for testing
   - External scanner handles complex lexical analysis (heredocs, quotes, interpolation)

3. **Scanner Architecture**
   - Implements `PerlScanner` trait for polymorphic scanner support
   - Manages complex state: quote stacks, heredoc delimiters, interpolation contexts
   - Handles 40+ token types including complex Perl constructs

4. **Error Handling**
   - Comprehensive error types in `error.rs` and `error/` module
   - Supports parsing, scanner, and Unicode errors
   - Uses `thiserror` for ergonomic error definitions

5. **Unicode Support**
   - Full normalization support (NFC, NFD, NFKC, NFKD)
   - Unicode identifier validation
   - Proper handling of combining marks and whitespace

### Testing Strategy
- **Corpus Tests** (`tree-sitter-perl/test/corpus/`): Main integration tests using tree-sitter format from legacy directory
- **Unit Tests**: Component-level testing in Rust
- **Property Tests**: Edge case testing with `proptest`
- **Benchmarks**: Performance testing with `criterion`

## Development Guidelines

1. **Development Location**: ALL new development happens in `/crates/tree-sitter-perl-rs/`
2. **Scanner Development**: Focus on the Rust scanner implementation; C scanner is being phased out
3. **Testing**: Corpus tests are located in the legacy `tree-sitter-perl/test/corpus/` directory
4. **Feature Flags**: Use conditional compilation for scanner-specific code
5. **Error Handling**: Use the project's error types and maintain detailed error messages

## Current Status

The project is finishing the transition to a pure Rust implementation:
- The Rust scanner in `/crates/tree-sitter-perl-rs/` is the active implementation
- The `/tree-sitter-perl/` directory is maintained only for its test corpus
- Focus is on completing and optimizing the Rust implementation