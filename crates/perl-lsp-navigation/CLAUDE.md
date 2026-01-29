# CLAUDE.md

This file provides guidance to Claude Code when working with code in this repository.

## Crate Overview

`perl-lsp-navigation` is a **Tier 4 LSP feature crate** providing go-to definition, find references, and document symbols.

**Purpose**: LSP navigation providers for Perl — enables code navigation features like go-to-definition, find-references, and document outline.

**Version**: 0.8.8

## Commands

```bash
cargo build -p perl-lsp-navigation       # Build this crate
cargo test -p perl-lsp-navigation        # Run tests
cargo clippy -p perl-lsp-navigation      # Lint
cargo doc -p perl-lsp-navigation --open  # View documentation
```

## Architecture

### Dependencies

- `perl-parser-core` - AST access
- `perl-semantic-analyzer` - Symbol resolution
- `perl-workspace-index` - Cross-file navigation
- `perl-position-tracking` - Position handling

### Features

| Feature | Purpose |
|---------|---------|
| `lsp-compat` | LSP type compatibility |
| `slow_tests` | Enable slow integration tests |

### LSP Capabilities

| Capability | Method |
|------------|--------|
| Go to Definition | `textDocument/definition` |
| Go to Declaration | `textDocument/declaration` |
| Go to Type Definition | `textDocument/typeDefinition` |
| Go to Implementation | `textDocument/implementation` |
| Find References | `textDocument/references` |
| Document Symbols | `textDocument/documentSymbol` |
| Workspace Symbols | `workspace/symbol` |
| Hover | `textDocument/hover` |

### Navigation Targets

| Symbol Type | Navigation |
|-------------|------------|
| Variable | Declaration site |
| Subroutine | `sub foo {}` definition |
| Method | Method definition (may cross packages) |
| Package | `package Foo;` declaration |
| Module | `.pm` file |

## Usage

```rust
use perl_lsp_navigation::NavigationProvider;

let provider = NavigationProvider::new(analyzer, workspace);

// Go to definition
let locations = provider.definition(document, position)?;

// Find references
let references = provider.references(document, position, include_declaration)?;

// Document symbols
let symbols = provider.document_symbols(document)?;

// Hover information
let hover = provider.hover(document, position)?;
```

### Cross-File Navigation

Uses dual indexing from `perl-workspace-index`:

```rust
// Both qualified and bare names work:
// MyPackage::helper() → finds definition
// helper() (after use MyPackage) → finds definition
```

## Important Notes

- Cross-file navigation requires workspace indexing
- Hover combines type info, documentation, and signature
- Document symbols power the outline view
- Some tests are slow (use `slow_tests` feature)
