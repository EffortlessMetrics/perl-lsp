# CLAUDE.md

This file provides guidance to Claude Code when working with code in this repository.

## Crate Overview

- **Crate**: `perl-tokenizer`
- **Version**: 0.9.1
- **Tier**: 2 (single-level internal dependencies)
- **Purpose**: Bridge between raw `perl-lexer` output and the recursive descent parser. Provides buffered token stream with lookahead, trivia preservation (comments, whitespace, POD), position tracking, and `__DATA__`/`__END__` marker utilities.

## Commands

```bash
cargo build -p perl-tokenizer        # Build
cargo test -p perl-tokenizer          # Run all tests (unit + integration)
cargo clippy -p perl-tokenizer        # Lint
cargo doc -p perl-tokenizer --open    # View documentation
```

## Architecture

### Dependencies

| Crate | Role |
|-------|------|
| `perl-lexer` | Raw tokenization (`PerlLexer`, `LexerMode`) |
| `perl-token` | Token/TokenKind definitions (re-exported) |
| `perl-error` | `ParseError`, `ParseResult` |
| `perl-position-tracking` | `Position`, `Range` |
| `perl-ast` | `v2::Node`, `NodeKind`, `NodeIdGenerator` |

Dev-only: `perl-tdd-support` (test helpers).

### Modules

| Module | Key Types | Purpose |
|--------|-----------|---------|
| `token_stream` | `TokenStream` | Buffered stream over `PerlLexer` with 3-token lookahead; skips whitespace/comments; resets lexer mode at statement boundaries |
| `token_wrapper` | `TokenWithPosition`, `PositionTracker` | Attach line/column positions to raw lexer tokens via binary-search over line starts |
| `trivia` | `Trivia`, `TriviaToken`, `TriviaLexer`, `TriviaCollector`, `NodeWithTrivia` | Classify whitespace, line comments, and POD as trivia; lexer wrapper that yields `(Token, Vec<TriviaToken>)` |
| `trivia_parser` | `TriviaParserContext`, `TriviaPreservingParser`, `format_with_trivia` | Parse source preserving trivia on AST nodes; context with full token + trivia buffer |
| `util` | `find_data_marker_byte_lexed`, `code_slice` | Locate `__DATA__`/`__END__` boundaries using the lexer |

### Public Re-exports

`Token`, `TokenKind` (from `perl-token`), `TokenStream`, `TokenWithPosition`, `Trivia`, `TriviaToken`, `TriviaParserContext`, `TriviaPreservingParser`.

## Usage Examples

```rust
use perl_tokenizer::{TokenKind, TokenStream};

// Create a token stream and iterate
let mut stream = TokenStream::new("my $x = 42;");
while let Ok(token) = stream.next() {
    if token.kind == TokenKind::Eof { break; }
    println!("{:?}: {}", token.kind, token.text);
}

// Lookahead for parser decisions
let mut s = TokenStream::new("$hash{key}");
if let (Ok(first), Ok(second)) = (s.peek(), s.peek_second()) {
    // Use first.kind and second.kind for disambiguation
}
```

```rust
use perl_tokenizer::util::code_slice;

// Strip __DATA__/__END__ section
let code = code_slice("print 1;\n__DATA__\nstuff");
assert_eq!(code, "print 1;\n");
```

## Important Notes

- `TokenStream` makes EOF sticky: once EOF is returned, future peeks continue to return EOF.
- `on_stmt_boundary()` clears all cached lookahead and resets the lexer to `ExpectTerm` mode.
- `TriviaLexer` leaks the source string to obtain a `'static` reference for the inner `PerlLexer`. This is a known limitation.
- `find_data_marker_byte` is deprecated in favor of `find_data_marker_byte_lexed`.
- Integration tests live in `tests/trivia_edge_cases.rs` covering POD, Unicode, CRLF, shebangs, and mixed whitespace.
