# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is a Tree-sitter parser for the Perl programming language, implementing both a traditional C-based parser and a modern Rust implementation. The project is in active migration from C to Rust.

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
# In tree-sitter-perl directory
cd tree-sitter-perl
npx tree-sitter generate
```

## Architecture Overview

### Project Structure
- **`/crates/tree-sitter-perl-rs/`**: Main Rust parser implementation (primary focus)
- **`/tree-sitter-perl/`**: Legacy C implementation and tree-sitter files
- **`/xtask/`**: Build automation tooling

### Key Components

1. **Dual Scanner Implementation**
   - C Scanner: Legacy implementation in `scanner.c`
   - Rust Scanner: New implementation in `rust_scanner.rs`
   - Controlled by feature flags: `rust-scanner` (default) vs `c-scanner`

2. **Parser Generation Flow**
   - `grammar.js` defines Perl syntax rules
   - Tree-sitter generates `parser.c`
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
- **Corpus Tests** (`test/corpus/`): Main integration tests using tree-sitter format
- **Unit Tests**: Component-level testing in Rust
- **Property Tests**: Edge case testing with `proptest`
- **Benchmarks**: Performance testing with `criterion`

## Development Guidelines

1. **Primary Development Focus**: Work in `/crates/tree-sitter-perl-rs/` for new features
2. **Scanner Development**: When modifying scanner behavior, update both C and Rust implementations if C scanner support is still active
3. **Testing**: Always add corpus tests for new grammar features in `tree-sitter-perl/test/corpus/`
4. **Feature Flags**: Use conditional compilation for scanner-specific code
5. **Error Handling**: Use the project's error types and maintain detailed error messages

## Migration Status

The project is migrating from C to Rust:
- Phase 1: Dual scanner support (current)
- Phase 2: Performance optimization
- Phase 3: Rust scanner as default
- Phase 4: C scanner removal

Currently, both scanners are maintained, with Rust scanner as the default.