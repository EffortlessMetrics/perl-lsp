# CLAUDE.md

This file provides guidance to Claude Code when working with code in this repository.

## Crate Overview

`perl-tokenizer` is a **Tier 2 utility crate** providing token stream and utilities for the Perl parser.

**Purpose**: Token stream and utilities for Perl parser — provides higher-level tokenization abstractions over the raw lexer.

**Version**: 0.8.8

## Commands

```bash
cargo build -p perl-tokenizer        # Build this crate
cargo test -p perl-tokenizer         # Run tests
cargo clippy -p perl-tokenizer       # Lint
cargo doc -p perl-tokenizer --open   # View documentation
```

## Architecture

### Dependencies

- `perl-lexer` - Raw tokenization
- `perl-token` - Token definitions
- `perl-error` - Error types
- `perl-position-tracking` - Position tracking
- `perl-ast` - AST types

### Key Types

| Type | Purpose |
|------|---------|
| `TokenStream` | Iterator over tokens with lookahead |
| `TokenBuffer` | Buffered token storage |
| `TokenCursor` | Position in token stream |

### Token Stream Features

```rust
use perl_tokenizer::TokenStream;

let mut stream = TokenStream::new(source);

// Peek ahead without consuming
let next = stream.peek()?;
let two_ahead = stream.peek_nth(2)?;

// Consume token
let token = stream.next()?;

// Check for specific token
if stream.check(TokenKind::Semicolon) {
    stream.advance();
}
```

### Lookahead

The tokenizer provides lookahead for parser decisions:

```rust
// Determine if this is a hash or block
match (stream.peek(), stream.peek_nth(1)) {
    (Some(TokenKind::LeftBrace), Some(TokenKind::RightBrace)) => {
        // Empty hash {}
    },
    (Some(TokenKind::LeftBrace), Some(TokenKind::Identifier)) => {
        // Could be block or hash - need more context
    },
    // ...
}
```

## Usage

```rust
use perl_tokenizer::{TokenStream, Tokenizer};

let source = "my $x = 42;";
let tokenizer = Tokenizer::new(source);
let mut stream = TokenStream::from(tokenizer);

while let Some(token) = stream.next() {
    println!("{:?}", token);
}
```

## Important Notes

- Built on `perl-lexer` for raw tokenization
- Provides parser-friendly abstractions
- Handles token buffering for backtracking
