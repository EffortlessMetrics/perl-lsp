# CLAUDE.md

This file provides guidance to Claude Code when working with code in this repository.

## Crate Overview

`perl-parser` is the **central hub crate** providing the main Perl parser interface. It aggregates and re-exports functionality from most other crates in the workspace.

**Purpose**: Native Perl parser (v3) with Tree-sitter-compatible kinds/fields/points and stable byte↔UTF-16 conversions.

**Version**: 0.8.8

## Commands

```bash
cargo build -p perl-parser            # Build this crate
cargo build -p perl-parser --release  # Build optimized
cargo test -p perl-parser             # Run tests
cargo clippy -p perl-parser           # Lint
cargo doc -p perl-parser --open       # View documentation
```

## Architecture

### Role in Workspace

This is a **Tier 6 composition crate** that aggregates most workspace functionality:

- Re-exports from `perl-parser-core` (parsing engine)
- Re-exports from `perl-semantic-analyzer` (semantic analysis)
- Re-exports from `perl-workspace-index` (workspace indexing)
- Re-exports from `perl-refactoring` (refactoring utilities)
- Re-exports from all `perl-lsp-*` providers

### Key Internal Dependencies

**Core Dependencies**:
- `perl-lexer` - Tokenization
- `perl-parser-core` - Core parsing engine
- `perl-semantic-analyzer` - Semantic analysis
- `perl-workspace-index` - Cross-file indexing
- `perl-refactoring` - Refactoring support
- `perl-incremental-parsing` - Incremental updates

**LSP Provider Dependencies**:
- `perl-lsp-providers` - Provider aggregation
- `perl-lsp-code-actions` - Quick fixes
- `perl-lsp-completion` - Auto-completion
- `perl-lsp-diagnostics` - Error reporting
- `perl-lsp-inlay-hints` - Type hints
- `perl-lsp-navigation` - Go-to definitions
- `perl-lsp-rename` - Rename refactoring
- `perl-lsp-semantic-tokens` - Syntax highlighting
- `perl-lsp-tooling` - External tool integration

### Main Modules

| Module | Purpose |
|--------|---------|
| `lib.rs` | Main library entry and re-exports |
| `engine.rs` | Core parsing engine interface |
| `tokens.rs` | Token management |
| `incremental.rs` | Incremental parsing support |
| `builtins.rs` | Built-in function handling |
| `analysis.rs` | Parser analysis integration |
| `workspace.rs` | Workspace indexing interface |
| `refactor.rs` | Refactoring integration |
| `ide.rs` | IDE/LSP integration |
| `tooling.rs` | External tooling integration |
| `tdd.rs` | Test-driven development support |

## Features

| Feature | Purpose |
|---------|---------|
| `test-performance` | Enable performance testing |
| `workspace` | Workspace indexing support |
| `lsp-compat` | LSP type compatibility |

## Usage Patterns

### Basic Parsing

```rust
use perl_parser::Parser;

let source = "print 'Hello, World!';";
let result = Parser::parse(source);
```

### With Semantic Analysis

```rust
use perl_parser::{Parser, SemanticAnalyzer};

let source = "my $x = 42;";
let ast = Parser::parse(source)?;
let analysis = SemanticAnalyzer::analyze(&ast)?;
```

## Testing

The crate has extensive tests covering:
- Parser behavior and edge cases
- UTF-16 position handling
- Integration with semantic analysis
- Workspace indexing

## Important Notes

- This crate is the main entry point for most Perl parsing needs
- For lower-level parsing access, use `perl-parser-core` directly
- For specific LSP features, consider using the individual `perl-lsp-*` crates
