# CLAUDE.md

This file provides guidance to Claude Code when working with this repository.

**Latest Release**: v0.8.9+ GA - Enhanced Lexer Performance Optimizations with Production-Stable AST Generation  
**API Stability**: See [docs/STABILITY.md](docs/STABILITY.md)

## Project Overview

This repository contains **five published crates** forming a complete Perl parsing ecosystem with comprehensive workspace refactoring capabilities:

### Published Crates (v0.8.9 GA)

1. **perl-parser** (`/crates/perl-parser/`) ⭐ **MAIN CRATE**
   - Native recursive descent parser with ~100% Perl 5 syntax coverage
   - Superior performance for small files and incremental updates (1-150 µs parsing)
   - Production-ready incremental parsing with <1ms LSP updates
   - Enterprise-grade workspace refactoring and cross-file analysis

2. **perl-lsp** (`/crates/perl-lsp/`) ⭐ **LSP BINARY**
   - Standalone Language Server binary with production-grade CLI
   - Works with VSCode, Neovim, Emacs, and all LSP-compatible editors

3. **perl-lexer** (`/crates/perl-lexer/`)
   - Context-aware tokenizer with Unicode support
   - Performance-optimized (v0.8.8+) with 15-22% improvement

4. **perl-corpus** (`/crates/perl-corpus/`)
   - Comprehensive test corpus with property-based testing infrastructure

5. **perl-parser-pest** (`/crates/perl-parser-pest/`) ⚠️ **LEGACY**
   - Pest-based parser (v2 implementation), marked as legacy

## Quick Start

### Installation
```bash
# Install LSP server
cargo install perl-lsp

# Or quick install (Linux/macOS)
curl -fsSL https://raw.githubusercontent.com/EffortlessSteven/tree-sitter-perl/main/install.sh | bash
```

### Usage
```bash
# Run LSP server (for editors)
perl-lsp --stdio

# Build parser from source
cargo build -p perl-parser --release

# Run tests
cargo test
```

## Essential Commands

**AI tools can run bare `cargo build` and `cargo test`** - the `.cargo/config.toml` ensures correct behavior.

### Build & Install
```bash
# Build main components
cargo build -p perl-lsp --release        # LSP server
cargo build -p perl-parser --release     # Parser library

# Install globally
cargo install perl-lsp                   # From crates.io
cargo install --path crates/perl-lsp     # From source
```

### Testing
```bash
cargo test                               # Run all tests
cargo test -p perl-parser               # Parser library tests
cargo test -p perl-lsp                  # LSP server integration tests
cargo test -p perl-parser --test lsp_comprehensive_e2e_test -- --nocapture # Full E2E test
```

### Development
```bash
cargo clippy --workspace                # Lint all crates
cargo bench                             # Run performance benchmarks
perl-lsp --stdio --log                  # Run LSP server with logging
```

## Architecture

### Crate Structure
- **Core Parser**: `/crates/perl-parser/` - parsing logic, LSP providers, Rope implementation
- **LSP Binary**: `/crates/perl-lsp/` - standalone server, CLI interface, protocol handling
- **Lexer**: `/crates/perl-lexer/` - tokenization, Unicode support
- **Test Corpus**: `/crates/perl-corpus/` - comprehensive test suite

### Parser Versions
- **v3 (Native)** ⭐ **RECOMMENDED**: ~100% coverage, optimized for LSP/incremental parsing
- **v2 (Pest)**: ~99.996% coverage, legacy but stable
- **v1 (C-based)**: ~95% coverage, fastest for large files, benchmarking only

## Key Features

- **~100% Perl Syntax Coverage**: Handles all modern Perl constructs including edge cases
- **Production-Ready LSP Server**: ~87% of LSP features functional with comprehensive workspace support
- **Enhanced Incremental Parsing**: <1ms updates with 70-99% node reuse efficiency
- **Unicode-Safe**: Full Unicode identifier and emoji support with proper UTF-8/UTF-16 handling
- **Enterprise Security**: Path traversal prevention, file completion safeguards
- **Cross-file Workspace Refactoring**: Enterprise-grade symbol renaming, module extraction, import optimization

## Documentation

See the [docs/](docs/) directory for comprehensive documentation:

- **[Commands Reference](docs/COMMANDS_REFERENCE.md)** - Comprehensive build/test commands
- **[LSP Implementation Guide](docs/LSP_IMPLEMENTATION_GUIDE.md)** - LSP server architecture  
- **[LSP Development Guide](docs/LSP_DEVELOPMENT_GUIDE.md)** - Source threading and comment extraction
- **[Crate Architecture Guide](docs/CRATE_ARCHITECTURE_GUIDE.md)** - System design and components
- **[Incremental Parsing Guide](docs/INCREMENTAL_PARSING_GUIDE.md)** - Performance and implementation
- **[Security Development Guide](docs/SECURITY_DEVELOPMENT_GUIDE.md)** - Enterprise security practices
- **[Benchmark Framework](docs/BENCHMARK_FRAMEWORK.md)** - Cross-language performance analysis

### Specialized Guides
- **[Workspace Navigation](docs/WORKSPACE_NAVIGATION_GUIDE.md)** - Enhanced cross-file features
- **[Rope Integration](docs/ROPE_INTEGRATION_GUIDE.md)** - Document management system
- **[Source Threading](docs/SOURCE_THREADING_GUIDE.md)** - Comment documentation extraction
- **[Position Tracking](docs/POSITION_TRACKING_GUIDE.md)** - UTF-16/UTF-8 position mapping
- **[Variable Resolution](docs/VARIABLE_RESOLUTION_GUIDE.md)** - Scope analysis system
- **[File Completion Guide](docs/FILE_COMPLETION_GUIDE.md)** - Enterprise-secure path completion

## Development Guidelines

### Choosing a Crate
1. **For Any Perl Parsing**: Use `perl-parser` - fastest, most complete, production-ready
2. **For IDE Integration**: Install `perl-lsp` - includes full LSP feature support
3. **For Testing Parsers**: Use `perl-corpus` for comprehensive test suite
4. **For Legacy Migration**: Migrate from `perl-parser-pest` to `perl-parser`

### Development Locations
- **Parser & LSP**: `/crates/perl-parser/` - main development with production Rope implementation
- **LSP Server**: `/crates/perl-lsp/` - standalone LSP server binary (v0.8.9)
- **Lexer**: `/crates/perl-lexer/` - tokenization improvements
- **Test Corpus**: `/crates/perl-corpus/` - test case additions

## Current Status (v0.8.9)

✅ **Production Ready**:
- 100% test pass rate across all components (291+ tests passing)
- Zero clippy warnings, consistent formatting
- Enterprise-grade LSP server with comprehensive features
- Production-stable incremental parsing with statistical validation

**LSP Features (~87% functional)**:
- ✅ Syntax checking, diagnostics, completion, hover
- ✅ Workspace symbols, rename, code actions
- ✅ Thread-safe semantic tokens (2.826µs average, zero race conditions)
- ✅ Enhanced call hierarchy, go-to-definition, find references
- ✅ Code Lens with reference counts and resolve support
- ✅ File path completion with enterprise security
- ✅ Debug Adapter Protocol (DAP) support

## Contributing

1. **Parser improvements** → `/crates/perl-parser/src/`
2. **LSP features** → `/crates/perl-parser/src/` (provider logic)
3. **CLI enhancements** → `/crates/perl-lsp/src/` (binary interface)
4. **Testing** → Use existing comprehensive test infrastructure
5. **Security features** → Follow enterprise security practices

### Coding Standards
- Run `cargo clippy --workspace` before committing changes
- Use `cargo fmt` for consistent formatting
- Prefer `.first()` over `.get(0)` for accessing first element
- Use `.push(char)` instead of `.push_str("x")` for single characters
- Use `or_default()` instead of `or_insert_with(Vec::new)` for default values
- Avoid unnecessary `.clone()` on types that implement Copy
- Add `#[allow(clippy::only_used_in_recursion)]` for recursive tree traversal functions