# CLAUDE.md

This file provides guidance to Claude Code when working with code in this repository.

## Crate Overview

`perl-workspace-index` is a **Tier 3 central orchestration crate** providing workspace indexing and cross-file navigation.

**Purpose**: Workspace indexing and refactoring orchestration for Perl — enables cross-file navigation, references, and workspace-wide operations.

**Version**: 0.8.8

## Commands

```bash
cargo build -p perl-workspace-index        # Build this crate
cargo test -p perl-workspace-index         # Run tests
cargo clippy -p perl-workspace-index       # Lint
cargo doc -p perl-workspace-index --open   # View documentation
```

## Architecture

### Dependencies

- `perl-parser-core` - Parsing engine
- `perl-position-tracking` - Position handling
- `perl-symbol-types` - Symbol taxonomy
- `perl-uri` - URI handling

### Features

| Feature | Purpose |
|---------|---------|
| `workspace` | Full workspace support (default) |
| `lsp-compat` | LSP type compatibility |

### Main Modules

| File | Size | Purpose |
|------|------|---------|
| `workspace/workspace_index.rs` | 128KB | Core indexing engine |
| `workspace/document_store.rs` | - | Document caching |
| `workspace/workspace_rename.rs` | - | Rename orchestration |

### Key Types

| Type | Purpose |
|------|---------|
| `WorkspaceIndex` | Central index of all workspace symbols |
| `FileIndex` | Index for a single file |
| `DocumentStore` | Cache of parsed documents |
| `SymbolRef` | Cross-file symbol reference |

### Dual Indexing Pattern (PR #122)

Symbols are indexed under both qualified and bare names:

```rust
// When indexing a symbol like "MyPackage::my_function"
// Index under bare name
file_index.references
    .entry("my_function".to_string())
    .or_default()
    .push(symbol_ref.clone());

// Index under qualified name
file_index.references
    .entry("MyPackage::my_function".to_string())
    .or_default()
    .push(symbol_ref);
```

This enables both:
- `MyPackage::my_function()` → finds definition
- `my_function()` (after `use MyPackage`) → finds definition

## Usage

```rust
use perl_workspace_index::{WorkspaceIndex, DocumentStore};

// Create workspace index
let mut index = WorkspaceIndex::new();

// Index a file
index.index_file(uri, source)?;

// Find definition
let definitions = index.find_definitions("my_function")?;

// Find references
let references = index.find_references("MyPackage::helper")?;

// Workspace-wide rename
let edits = index.rename("old_name", "new_name")?;
```

### Document Store

```rust
// Cache parsed documents
let store = DocumentStore::new();
store.update(uri, source);

// Get cached document
if let Some(doc) = store.get(uri) {
    // Use cached parse result
}
```

## Important Notes

- The 128KB `workspace_index.rs` is the largest file in the codebase
- Indexing is incremental (only re-index changed files)
- Memory usage scales with workspace size
- Thread-safe for concurrent access
