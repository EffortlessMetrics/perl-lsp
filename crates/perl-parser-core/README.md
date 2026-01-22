# perl-parser-core

Core parser engine providing AST types, parser infrastructure, and token stream utilities for the Perl LSP ecosystem.

## Purpose

This crate provides the foundational parsing components used by `perl-parser`, `perl-lsp`, and other workspace crates. It implements a recursive descent parser with error recovery, incremental parsing support, and position tracking for LSP integration.

## Key Modules

- **`ast`** - Abstract Syntax Tree definitions (`Node`, `NodeKind`, `SourceLocation`)
- **`parser`** - Recursive descent parser with error recovery (`Parser`)
- **`tokens`** - Token stream and trivia utilities (`TokenStream`, `Token`, `Trivia`)
- **`position`** - Position mapping and line indexing for UTF-16/UTF-8 conversion
- **`error`** - Error classification, recovery strategies, and parse results
- **`builtins`** - Perl builtin function signatures and metadata
- **`edit`** - Edit tracking for incremental parsing
- **`quote_parser`** - Parser for Perl quote and quote-like operators
- **`heredoc_collector`** - Heredoc content collector with FIFO ordering

## Usage

```rust
use perl_parser_core::{Parser, Node, NodeKind, ParseError};

// Parse Perl source code
let source = "sub hello { print 'world'; }";
let result = Parser::parse_source(source);

match result {
    Ok(ast) => {
        // Work with the AST
        println!("Parsed {} nodes", ast.len());
    }
    Err(err) => {
        // Handle parse errors with recovery context
        eprintln!("Parse error: {}", err);
    }
}
```

## Features

- Deterministic AST construction with error recovery
- Incremental parsing with edit tracking
- UTF-16/UTF-8 position conversion for LSP protocol compliance
- Comprehensive Perl syntax coverage
- Trivia-preserving parsing for formatting tools

## Internal Use

This crate is primarily intended for internal use within the perl-lsp workspace. The public API may change between minor versions. See the main `perl-parser` crate for the stable public interface.

## License

Licensed under either of:

- MIT License ([LICENSE-MIT](../../LICENSE-MIT) or http://opensource.org/licenses/MIT)
- Apache License, Version 2.0 ([LICENSE-APACHE](../../LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)

at your option.
