# CLAUDE.md

This file provides guidance to Claude Code when working with code in this repository.

## Crate Overview

`perl-token` is a **Tier 1 leaf crate** providing token type definitions for the Perl parser.

**Purpose**: Token definitions for Perl parser — defines all token kinds, categories, and associated metadata.

**Version**: 0.8.8

## Commands

```bash
cargo build -p perl-token            # Build this crate
cargo test -p perl-token             # Run tests
cargo clippy -p perl-token           # Lint
cargo doc -p perl-token --open       # View documentation
```

## Architecture

### Dependencies

**None** — uses only `std::sync::Arc` from the standard library.

This is a pure definition crate with no external dependencies.

### Key Types

| Type | Purpose |
|------|---------|
| `TokenKind` | Enum of all Perl token types |
| `Token` | Token with kind, span, and text |
| `Span` | Byte range in source |

### Token Categories

Tokens are organized into categories:

- **Keywords**: `if`, `elsif`, `unless`, `while`, `for`, etc.
- **Operators**: `+`, `-`, `*`, `/`, `=~`, `!~`, etc.
- **Delimiters**: `(`, `)`, `{`, `}`, `[`, `]`
- **Literals**: Numbers, strings, regex
- **Sigils**: `$`, `@`, `%`, `*`, `&`
- **Special**: Heredoc markers, POD, comments

## Usage

```rust
use perl_token::{Token, TokenKind, Span};

// Token kinds are exhaustive
match token.kind {
    TokenKind::Identifier => { /* ... */ },
    TokenKind::ScalarSigil => { /* ... */ },
    // ...
}
```

## Important Notes

- Changes to token kinds affect all lexer and parser code
- Keep the enum variants organized by category
- Document new token kinds thoroughly
