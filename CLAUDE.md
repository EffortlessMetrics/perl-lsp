# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

**Latest Release**: v0.8.9 GA - Comprehensive PR Workflow Integration with Production-Stable AST Generation and Enhanced Workspace Navigation  
**API Stability**: See [docs/STABILITY.md](docs/STABILITY.md) for guarantees

## Project Overview

This repository contains **four published crates** forming a complete Perl parsing ecosystem:

#### 1. **perl-parser** (`/crates/perl-parser/`) ⭐ **MAIN CRATE**
- Native recursive descent parser with ~100% Perl 5 syntax coverage
- 4-19x faster than legacy implementations (1-150 µs parsing)
- True incremental parsing with Rope-based document management (<1ms LSP updates)
- LSP server binary (`perl-lsp`) with full feature set
- See [docs/INCREMENTAL_PARSING_GUIDE.md](docs/INCREMENTAL_PARSING_GUIDE.md) for details

#### 2. **perl-lexer** (`/crates/perl-lexer/`) 
- Context-aware tokenizer with mode-based lexing
- Handles slash disambiguation at lexing phase
- Zero dependencies, used by perl-parser

#### 3. **perl-corpus** (`/crates/perl-corpus/`)
- Comprehensive test corpus with property-based testing infrastructure
- Feature: `ci-fast` for conditional test execution

#### 4. **perl-parser-pest** (`/crates/perl-parser-pest/`) ⚠️ **LEGACY**
- Pest-based parser (~99.995% coverage) - use perl-parser instead

## Key Features

### Incremental Parsing with Rope-based Document Management
- Production-ready incremental parsing with <1ms LSP updates
- Rope-based text management for UTF-16/UTF-8 position conversion  
- Subtree reuse with 70-90% cache hit ratios
- See [docs/INCREMENTAL_PARSING_GUIDE.md](docs/INCREMENTAL_PARSING_GUIDE.md) for details

### Enhanced Workspace Navigation (v0.8.9)
- Production-stable workspace navigation with comprehensive AST traversal
- ExpressionStatement support for complete symbol coverage
- Enhanced code actions and refactoring with parameter validation
- Cross-file call analysis and workspace-wide symbol resolution
- 100% test coverage (195 library, 33 LSP E2E, 19 DAP tests)
- See [docs/WORKSPACE_NAVIGATION_GUIDE.md](docs/WORKSPACE_NAVIGATION_GUIDE.md) for details

### LSP Server (`perl-lsp` binary) ✅ **PRODUCTION READY**
- **~85% of LSP features work** (all advertised capabilities functional)
- Advanced syntax checking and diagnostics with hash key context detection
- Code completion (variables, 150+ built-ins, file paths) with comment documentation  
- Enhanced workspace features: symbols, rename, code actions, semantic tokens
- Incremental parsing with <1ms real-time editing performance
- File path completion with enterprise security safeguards
- **Debug Adapter Protocol (DAP)** support with full debugging flow
- **Performance**: <50ms for all operations, works with all LSP-compatible editors
- See [docs/LSP_IMPLEMENTATION_GUIDE.md](docs/LSP_IMPLEMENTATION_GUIDE.md) for details

## Quick Start

**AI tools can run bare `cargo build` and `cargo test` commands** - the `.cargo/config.toml` ensures correct behavior.

## Essential Commands

### Quick Commands
```bash
# Build everything
cargo build --all

# Run all tests  
cargo xtask test

# Run LSP server
perl-lsp --stdio

# Install LSP server globally
cargo install --path crates/perl-lsp
```

### Key Testing
```bash
# Main test suites
cargo test -p perl-parser                              # Parser tests
cargo test -p perl-parser --test lsp_comprehensive_e2e_test  # LSP tests
cargo xtask corpus                                     # Integration tests

# Incremental parsing (enable with --features incremental)
cargo test -p perl-parser incremental_v2::tests --features incremental
```

**For complete command reference, see [docs/COMMANDS_REFERENCE.md](docs/COMMANDS_REFERENCE.md)**

## Development Guidelines

### LSP Development
- Use source-aware constructors for enhanced documentation support
- Thread source text through provider constructors
- Add comprehensive tests (unit, integration, user story)
- See [docs/LSP_IMPLEMENTATION_GUIDE.md](docs/LSP_IMPLEMENTATION_GUIDE.md) for detailed guidance

### Key Development Resources
- **Incremental Parsing**: [docs/INCREMENTAL_PARSING_GUIDE.md](docs/INCREMENTAL_PARSING_GUIDE.md)
- **File Path Completion**: [docs/FILE_COMPLETION_GUIDE.md](docs/FILE_COMPLETION_GUIDE.md)
- **Import Optimization**: [docs/IMPORT_OPTIMIZER_GUIDE.md](docs/IMPORT_OPTIMIZER_GUIDE.md)
- **Position Tracking**: [docs/POSITION_TRACKING_GUIDE.md](docs/POSITION_TRACKING_GUIDE.md)
- **Architecture**: [docs/ARCHITECTURE_OVERVIEW.md](docs/ARCHITECTURE_OVERVIEW.md)

## Current Parser Status

| Parser | Coverage | Performance | Status |
|--------|----------|-------------|---------|
| v3 Native (recommended) | 100% | 4-19x faster | ✅ Production ready |
| v2 Pest | 99.996% | ~200-450 µs | ✅ Legacy support |
| v1 C | ~95% | ~12-68 µs | ⚠️ Benchmarking only |

### v3 Native Parser ⭐ **RECOMMENDED**
- **100% Perl syntax coverage** including all edge cases
- **Enhanced workspace navigation** with comprehensive AST traversal
- **Production-stable LSP server** (~85% features functional)
- **Advanced features**: incremental parsing, file completion, import optimization
- Handles all complex cases: regex delimiters, indirect object syntax, modern Perl features

**Performance**: 1-150 µs parsing, <1ms LSP updates, <50ms operations

## Documentation Index

### Development Guides
- **Architecture**: [docs/ARCHITECTURE_OVERVIEW.md](docs/ARCHITECTURE_OVERVIEW.md)
- **Commands**: [docs/COMMANDS_REFERENCE.md](docs/COMMANDS_REFERENCE.md) 
- **LSP Development**: [docs/LSP_IMPLEMENTATION_GUIDE.md](docs/LSP_IMPLEMENTATION_GUIDE.md)

### Feature Guides
- **Incremental Parsing**: [docs/INCREMENTAL_PARSING_GUIDE.md](docs/INCREMENTAL_PARSING_GUIDE.md)
- **Workspace Navigation**: [docs/WORKSPACE_NAVIGATION_GUIDE.md](docs/WORKSPACE_NAVIGATION_GUIDE.md)
- **File Completion**: [docs/FILE_COMPLETION_GUIDE.md](docs/FILE_COMPLETION_GUIDE.md)
- **Import Optimization**: [docs/IMPORT_OPTIMIZER_GUIDE.md](docs/IMPORT_OPTIMIZER_GUIDE.md)
- **Position Tracking**: [docs/POSITION_TRACKING_GUIDE.md](docs/POSITION_TRACKING_GUIDE.md)

### Reference
- **Parser Comparison**: [docs/PARSER_COMPARISON.md](docs/PARSER_COMPARISON.md)
- **Variable Resolution**: [docs/VARIABLE_RESOLUTION_GUIDE.md](docs/VARIABLE_RESOLUTION_GUIDE.md)
- **Scope Analyzer**: [docs/SCOPE_ANALYZER_REFERENCE.md](docs/SCOPE_ANALYZER_REFERENCE.md)