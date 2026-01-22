# perl-workspace-index

Workspace-wide symbol indexing for cross-file navigation and refactoring.

## Purpose

This crate provides workspace indexing and refactoring orchestration for Perl code, enabling fast cross-file symbol lookup and workspace-wide operations. It maintains a centralized index of symbols across all files in a workspace for efficient navigation and refactoring.

## Key Features

- **Workspace-Wide Indexing**: Index symbols across all files in a workspace
- **Dual Indexing Architecture**: Index symbols under both qualified and bare forms for comprehensive reference coverage
- **Document Storage**: Efficient caching and management of workspace documents
- **Symbol Lookup**: Fast symbol resolution across files with dual pattern matching
- **Workspace Rename**: Cross-file rename operations with reference tracking
- **Thread-Safe**: Uses parking_lot for efficient concurrent access
- **Performance Optimized**: Designed for large codebases with thousands of files

## Usage

```rust
use perl_workspace_index::workspace_index;

// Create and use a workspace index for cross-file operations
// (specific APIs depend on workspace module implementation)
```

## Features

- `default`: Basic workspace indexing functionality
- `workspace`: Enable full workspace features
- `lsp-compat`: Enable LSP types integration for LSP feature support

## Benchmarks

The crate includes benchmarks for workspace indexing performance. Run with:

```bash
cargo bench --features workspace
```

## Documentation

For detailed API documentation, see [docs.rs/perl-workspace-index](https://docs.rs/perl-workspace-index).

## License

Licensed under either of:

- MIT License
- Apache License, Version 2.0

at your option.
