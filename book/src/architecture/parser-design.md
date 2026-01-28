# Parser Design

The perl-parser crate implements a recursive descent parser for Perl 5 syntax.

## Key Features

- Near-complete Perl 5 syntax coverage (~100%)
- Tree-sitter compatible output
- Incremental parsing support
- Robust error recovery
- Context-aware lexing

## Architecture

The parser follows a multi-stage pipeline:

1. **Lexical Analysis**: Context-aware tokenization
2. **Parsing**: Recursive descent with error recovery
3. **AST Construction**: Build abstract syntax tree
4. **Serialization**: Output S-expressions for Tree-sitter compatibility

See the [LSP Implementation Guide](../lsp/implementation-guide.md) for integration details.
