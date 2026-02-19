# CLAUDE.md

This file provides guidance to Claude Code when working with code in this repository.

## Crate Overview

`perl-token` is a **Tier 1 leaf crate** providing token type definitions for the Perl parser.

**Purpose**: Defines `Token` and `TokenKind` -- the shared token contract used by lexer, tokenizer, and parser crates.

**Version**: 0.9.0

## Commands

```bash
cargo build -p perl-token            # Build this crate
cargo test -p perl-token             # Run tests
cargo clippy -p perl-token           # Lint
cargo doc -p perl-token --open       # View documentation
```

## Architecture

### Dependencies

**None** -- uses only `std::sync::Arc` from the standard library.

This is a pure definition crate with no external dependencies.

### Key Types

| Type | Purpose |
|------|---------|
| `TokenKind` | Enum of all Perl token types (~80 variants) |
| `Token` | Token with kind, `Arc<str>` text, and byte start/end positions |

### Token Categories

`TokenKind` variants are organized into categories:

- **Keywords** (34): `My`, `Sub`, `If`, `While`, `For`, `Package`, `Use`, `Class`, `Method`, `Try`, `Catch`, etc.
- **Operators** (47): `Assign`, `Plus`, `Arrow`, `FatArrow`, `Match`, `SmartMatch`, `Range`, `Ellipsis`, etc.
- **Delimiters** (8): `LeftParen`, `RightParen`, `LeftBrace`, `RightBrace`, `LeftBracket`, `RightBracket`, `Semicolon`, `Comma`
- **Literals** (14): `Number`, `String`, `Regex`, `Substitution`, `HeredocStart`, `HeredocBody`, `DataMarker`, etc.
- **Identifiers/Sigils** (6): `Identifier`, `ScalarSigil`, `ArraySigil`, `HashSigil`, `SubSigil`, `GlobSigil`
- **Special** (2): `Eof`, `Unknown`

## Usage

```rust
use perl_token::{Token, TokenKind};

let tok = Token::new(TokenKind::ScalarSigil, "$", 0, 1);
assert_eq!(tok.kind, TokenKind::ScalarSigil);

// TokenKind is Copy + Eq, suitable for match arms
match tok.kind {
    TokenKind::Identifier => { /* ... */ },
    TokenKind::ScalarSigil => { /* ... */ },
    _ => {}
}
```

## Important Notes

- Changes to `TokenKind` variants propagate to all lexer and parser crates
- Keep enum variants organized by category with doc comments
- `Token.text` uses `Arc<str>` for cheap cloning during lookahead and buffering
- No `Span` struct -- positions are stored as `start`/`end` fields on `Token`
