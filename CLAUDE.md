# CLAUDE.md

This file provides guidance to Claude Code when working with this repository.

**Latest Release**: v0.8.9 GA - Comprehensive PR Workflow Integration with Production-Stable AST Generation  
**API Stability**: See [docs/STABILITY.md](docs/STABILITY.md)

## Project Overview

This repository contains **five published crates** forming a complete Perl parsing ecosystem with comprehensive workspace refactoring capabilities:

### Published Crates (v0.8.9 GA)

1. **perl-parser** (`/crates/perl-parser/`) ⭐ **MAIN CRATE**
   - Native recursive descent parser with ~100% Perl 5 syntax coverage
   - 4-19x faster than legacy implementations (1-150 µs parsing)
   - True incremental parsing with <1ms LSP updates
   - Production-ready Rope integration for UTF-16/UTF-8 position conversion
   - Enterprise-grade workspace refactoring utilities

2. **perl-lsp** (`/crates/perl-lsp/`) ⭐ **LSP BINARY**
   - Standalone Language Server binary with production-grade CLI
   - Clean separation from parser logic for improved maintainability
   - Works with VSCode, Neovim, Emacs, and all LSP-compatible editors

3. **perl-lexer** (`/crates/perl-lexer/`)
   - Context-aware tokenizer with mode-based lexing
   - Handles slash disambiguation and Unicode identifiers

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

# Or Homebrew (macOS)
brew tap tree-sitter-perl/tap && brew install perl-lsp
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

## Key Features

- **~100% Perl Syntax Coverage**: Handles all modern Perl constructs including edge cases
- **Production-Ready LSP Server**: ~85% of LSP features functional with comprehensive workspace support
- **Enhanced Incremental Parsing**: <1ms updates with 70-99% node reuse efficiency
- **Comprehensive Testing**: 100% test pass rate (195 library tests, 33 LSP E2E tests, 19 DAP tests)
- **Unicode-Safe**: Full Unicode identifier and emoji support with proper UTF-8/UTF-16 handling
- **Enterprise Security**: Path traversal prevention, file completion safeguards
- **Cross-file Workspace Refactoring**: Enterprise-grade symbol renaming, module extraction, import optimization

## Architecture

### Crate Structure
- **Core Parser**: `/crates/perl-parser/` - parsing logic, LSP providers, Rope implementation
- **LSP Binary**: `/crates/perl-lsp/` - standalone server, CLI interface, protocol handling
- **Lexer**: `/crates/perl-lexer/` - tokenization, Unicode support
- **Test Corpus**: `/crates/perl-corpus/` - comprehensive test suite

### Parser Versions
- **v3 (Native)** ⭐ **RECOMMENDED**: ~100% coverage, 4-19x faster, production incremental parsing
- **v2 (Pest)**: ~99.996% coverage, legacy but stable
- **v1 (C-based)**: ~95% coverage, benchmarking only

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
cargo test                               # All tests
cargo test -p perl-parser               # Parser tests
cargo test -p perl-lsp                  # LSP integration tests
```

### Development
```bash
cargo clippy --workspace                # Lint workspace crates
cargo bench                             # Performance benchmarks
perl-lsp --stdio --log                  # Debug LSP server
```

## Documentation

- **[Commands Reference](docs/COMMANDS_REFERENCE.md)** - Comprehensive build/test commands
- **[LSP Implementation Guide](docs/LSP_IMPLEMENTATION_GUIDE.md)** - LSP server architecture
- **[LSP Development Guide](docs/LSP_DEVELOPMENT_GUIDE.md)** - Source threading and comment extraction
- **[Crate Architecture Guide](docs/CRATE_ARCHITECTURE_GUIDE.md)** - System design and components
- **[Incremental Parsing Guide](docs/INCREMENTAL_PARSING_GUIDE.md)** - Performance and implementation
- **[Security Development Guide](docs/SECURITY_DEVELOPMENT_GUIDE.md)** - Enterprise security practices
- **[Benchmark Framework](docs/BENCHMARK_FRAMEWORK.md)** - Cross-language performance analysis
- **[File Completion Guide](docs/FILE_COMPLETION_GUIDE.md)** - Enterprise-secure path completion

### Specialized Guides
- **[LSP Crate Separation](docs/LSP_CRATE_SEPARATION_GUIDE.md)** - v0.8.9 architectural improvements
- **[Workspace Navigation](docs/WORKSPACE_NAVIGATION_GUIDE.md)** - Enhanced cross-file features
- **[Rope Integration](docs/ROPE_INTEGRATION_GUIDE.md)** - Document management system
- **[Source Threading](docs/SOURCE_THREADING_GUIDE.md)** - Comment documentation extraction
- **[Position Tracking](docs/POSITION_TRACKING_GUIDE.md)** - UTF-16/UTF-8 position mapping
- **[Variable Resolution](docs/VARIABLE_RESOLUTION_GUIDE.md)** - Scope analysis system
- **[Import Optimizer](docs/IMPORT_OPTIMIZER_GUIDE.md)** - Advanced code actions

## Performance Targets ✅ **EXCEEDED**

| Operation | Target | Achieved | Status |
|-----------|--------|----------|--------|
| Simple Edits | <100µs | 65µs avg | ✅ Excellent |
| Moderate Edits | <500µs | 205µs avg | ✅ Very Good |
| Large Documents (100 stmt) | <1ms | 538µs avg | ✅ Good |
| Node Reuse Efficiency | ≥70% | 99.7% peak | ✅ Exceptional |
| Statistical Consistency | <1.0 CV | 0.6 CV | ✅ Excellent |
| Incremental Success Rate | ≥95% | 100% | ✅ Perfect |

## Current Status (v0.8.9)

✅ **Production Ready**:
- 100% test pass rate across all components
- Zero clippy warnings, consistent formatting
- Enterprise-grade LSP server with comprehensive features
- Production-stable incremental parsing with statistical validation
- Enhanced workspace navigation and PR workflow integration

**LSP Features (~85% functional)**:
- ✅ Syntax checking, diagnostics, completion, hover
- ✅ Workspace symbols, rename, code actions
- ✅ **Thread-safe semantic tokens** (2.826µs average, zero race conditions)
- ✅ Enhanced call hierarchy, go-to-definition, find references
- ✅ File path completion with enterprise security
- ✅ Debug Adapter Protocol (DAP) support

**Recent Enhancements (v0.8.9)**:
- ✅ Comprehensive S-expression generation with 50+ operators
- ✅ Enhanced AST traversal including ExpressionStatement support
- ✅ Production-ready workspace indexing and cross-file analysis
- ✅ Advanced code actions with parameter threshold validation
- ✅ Statistical performance testing infrastructure

## Contributing

1. **Parser improvements** → `/crates/perl-parser/src/`
2. **LSP features** → `/crates/perl-parser/src/` (provider logic)
3. **CLI enhancements** → `/crates/perl-lsp/src/` (binary interface)
4. **Testing** → Use existing comprehensive test infrastructure
5. **Security features** → Follow enterprise security practices (see [Security Development Guide](docs/SECURITY_DEVELOPMENT_GUIDE.md))

### Coding Standards
- Run `cargo clippy --workspace` before committing changes
- Use `cargo fmt` for consistent formatting
- Prefer `.first()` over `.get(0)` for accessing first element
- Use `.push(char)` instead of `.push_str("x")` for single characters
- Use `or_default()` instead of `or_insert_with(Vec::new)` for default values
- Avoid unnecessary `.clone()` on types that implement Copy
- Add `#[allow(clippy::only_used_in_recursion)]` for recursive tree traversal functions