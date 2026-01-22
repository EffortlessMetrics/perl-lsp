# perl-refactoring

Refactoring operations for Perl code, including inline expansion, code movement, and extraction.

## Purpose

This crate provides refactoring and modernization utilities for Perl code, enabling automated code transformations that improve code quality while preserving behavior. It supports workspace-wide refactoring operations for cross-file symbol changes.

## Key Features

- **Import Optimization**: Organize and optimize Perl module imports
- **Code Modernization**: Apply Perl best practices and modernization patterns
- **Refactoring Operations**: Support for extract, inline, and move code transformations
- **Workspace Refactoring**: Cross-file refactoring with workspace-wide symbol tracking
- **Symbol-Aware**: Integrates with workspace index for accurate symbol resolution

## Usage

```rust
use perl_refactoring::refactor;

// Apply refactoring operations to Perl code
// (specific APIs depend on refactor module implementation)
```

## Features

- `default`: Basic refactoring functionality
- `modernize`: Enable code modernization patterns
- `workspace_refactor`: Enable workspace-wide refactoring operations

## Documentation

For detailed API documentation, see [docs.rs/perl-refactoring](https://docs.rs/perl-refactoring).

## License

Licensed under either of:

- MIT License
- Apache License, Version 2.0

at your option.
