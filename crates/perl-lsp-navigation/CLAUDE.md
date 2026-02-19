# CLAUDE.md

This file provides guidance to Claude Code when working with code in this repository.

## Crate Overview

`perl-lsp-navigation` is a **Tier 2 LSP feature crate** providing workspace symbol search, type hierarchy, type definition, find references, and document links for Perl.

**Purpose**: LSP navigation providers consumed by `perl-lsp` request handlers for code navigation features.

**Version**: 0.9.0

## Commands

```bash
cargo build -p perl-lsp-navigation                        # Build this crate
cargo test -p perl-lsp-navigation                         # Run tests
cargo test -p perl-lsp-navigation --features lsp-compat   # Tests including lsp-compat gated code
cargo clippy -p perl-lsp-navigation                       # Lint
cargo doc -p perl-lsp-navigation --open                   # View documentation
```

## Architecture

### Dependencies

- `perl-parser-core` -- AST types, `Parser`, `PositionMapper`, `SourceLocation`
- `perl-semantic-analyzer` -- `SymbolExtractor`, `SymbolKind` for workspace symbol indexing
- `perl-workspace-index` -- (declared dependency, used by consuming crate for cross-file nav)
- `perl-position-tracking` -- `WireRange`, `WireLocation`, `WirePosition`, UTF-16 offset conversion
- `lsp-types` -- `Location`, `LocationLink`, `Uri` (re-exports `Location`)
- `serde` / `serde_json` -- serialization of workspace symbols and document links
- `url` -- `Url` type for document link resolution

### Modules

| Module | Key Exports | Purpose |
|--------|-------------|---------|
| `workspace_symbols` | `WorkspaceSymbolsProvider`, `WorkspaceSymbol` | Indexes documents, fuzzy/prefix/exact symbol search |
| `type_hierarchy` | `TypeHierarchyProvider`, `TypeHierarchyItem`, `TypeHierarchySymbolKind` | `@ISA` / `use parent` / `use base` hierarchy navigation |
| `type_definition` | `TypeDefinitionProvider` | Go-to-type-definition (gated on `lsp-compat`) |
| `references` | `find_references_single_file` | Same-file variable/subroutine reference finding |
| `document_links` | `compute_links` | Extracts `use`/`require` document links with deferred resolution |

### Features

| Feature | Purpose |
|---------|---------|
| `lsp-compat` | Enables `TypeDefinitionProvider::find_type_definition` and LSP type imports |
| `slow_tests` | Enable slow/expensive integration tests |

## Usage

```rust
use perl_lsp_navigation::{WorkspaceSymbolsProvider, TypeHierarchyProvider};
use perl_parser_core::Parser;
use std::collections::HashMap;

// Workspace symbol search
let mut provider = WorkspaceSymbolsProvider::new();
let mut parser = Parser::new("sub hello { }");
let ast = parser.parse()?;
provider.index_document("file:///test.pl", &ast, "sub hello { }");

let mut source_map = HashMap::new();
source_map.insert("file:///test.pl".to_string(), "sub hello { }".to_string());
let results = provider.search("hello", &source_map);

// Type hierarchy
let hierarchy = TypeHierarchyProvider::new();
let items = hierarchy.prepare(&ast, code, offset);
let supertypes = hierarchy.find_supertypes(&ast, &item);
let subtypes = hierarchy.find_subtypes(&ast, &item);

// Single-file references
use perl_lsp_navigation::find_references_single_file;
let refs = find_references_single_file(&ast, byte_offset);

// Document links
use perl_lsp_navigation::compute_links;
let links = compute_links(uri, text, &workspace_roots);
```

## Important Notes

- `TypeDefinitionProvider::find_type_definition` is only available with the `lsp-compat` feature flag
- `WorkspaceSymbolsProvider` requires a `source_map` (URI to source text) for offset-to-position conversion
- Document links use deferred resolution via a `data` field for `documentLink/resolve`
- Type hierarchy builds its index from `use parent`, `use base`, and `our @ISA` declarations
- `find_references_single_file` operates on a single AST; cross-file references need workspace indexing
- Some tests require `slow_tests` feature
