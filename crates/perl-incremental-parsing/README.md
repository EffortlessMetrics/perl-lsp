# perl-incremental-parsing

Incremental parsing infrastructure for efficient re-parsing of Perl code in response to document edits.

## Purpose

This crate provides efficient incremental parsing capabilities for Perl code, enabling Language Server Protocol features to respond quickly to document edits by reusing portions of the previous parse tree. The incremental parser minimizes re-parsing overhead by identifying which portions of the AST are affected by an edit and only re-parsing the modified regions.

## Key Features

- **Efficient Re-parsing**: Reuses unaffected subtrees from previous parses to minimize overhead
- **Edit Change Detection**: Identifies which portions of the AST are affected by document edits
- **LSP Integration**: Enables fast document update responses for Language Server Protocol features
- **Selective Parsing**: Only re-parses modified regions and their dependent nodes
- **Performance Optimized**: Designed for sub-millisecond incremental parse updates

## Usage

```rust
use perl_incremental_parsing::incremental;
use perl_parser_core::Parser;

// Initial parse
let source = "sub foo { return 42; }";
let mut parser = Parser::new(source);
let ast = parser.parse();

// After edit, incrementally reparse only affected portions
// (specific APIs depend on incremental module implementation)
```

## Documentation

For detailed API documentation, see [docs.rs/perl-incremental-parsing](https://docs.rs/perl-incremental-parsing).

## License

Licensed under either of:

- MIT License
- Apache License, Version 2.0

at your option.
