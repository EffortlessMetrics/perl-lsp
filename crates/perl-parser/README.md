# perl-parser

> **Tree-sitter compatible**: node kinds, field names, and byte/point semantics (0-based; UTF-16 for LSP) match the Tree-sitter Perl grammar.

A high-performance Perl parser with full Language Server Protocol support.

## Features

- **100% Perl 5 syntax coverage**: Handles all edge cases including `m!pattern!`, indirect object syntax, and more
- **Language Server Protocol**: Full LSP implementation with advanced IDE features
- **Debug Adapter Protocol**: Integrated debugging support
- **Tree-sitter compatible**: Produces standard S-expression output
- **Performance**: 4-19x faster than traditional parsers

## Binaries

This crate provides two binaries:

- `perl-lsp`: Full-featured Language Server for Perl
- `perl-dap`: Debug Adapter for Perl debugging

## Usage

### As a library

```rust
use perl_parser::{Parser, ParseOptions};

let source = "my $x = 42;";
let options = ParseOptions::default();
let ast = Parser::parse(source, options)?;

println!("{}", ast.to_sexp());
```

### As a Language Server

```bash
# Install
cargo install perl-parser

# Run
perl-lsp --stdio
```

## LSP Features

- Syntax diagnostics
- Go to definition
- Find references
- Document symbols
- Signature help
- Semantic tokens
- Call hierarchy
- Inlay hints
- Test discovery and execution
- Code formatting

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.