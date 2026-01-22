# perl-semantic-analyzer

Semantic analysis for Perl code, including symbol resolution and type inference.

## Purpose

This crate provides semantic analysis and symbol extraction for Perl code, enabling advanced IDE features like symbol navigation, type inference, and dead code detection. It builds on the parser to provide deep understanding of Perl code semantics.

## Key Features

- **Symbol Extraction**: Extract and categorize symbols (subs, packages, variables, etc.)
- **Scope Analysis**: Analyze lexical scoping and variable visibility
- **Type Inference**: Infer types for Perl variables and expressions
- **Dead Code Detection**: Identify unused code and unreachable branches
- **Declaration Analysis**: Track symbol declarations and their locations
- **Workspace Integration**: Integrates with workspace index for cross-file analysis

## Usage

```rust
use perl_semantic_analyzer::analysis;

// Perform semantic analysis on Perl code
// (specific APIs depend on analysis module implementation)
```

## Features

- Platform-specific features are automatically disabled on WASM targets
- Dead code detection and indexing are available on non-WASM platforms

## Documentation

For detailed API documentation, see [docs.rs/perl-semantic-analyzer](https://docs.rs/perl-semantic-analyzer).

## License

Licensed under either of:

- MIT License
- Apache License, Version 2.0

at your option.
