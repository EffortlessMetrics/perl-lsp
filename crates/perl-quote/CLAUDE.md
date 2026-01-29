# CLAUDE.md

This file provides guidance to Claude Code when working with code in this repository.

## Crate Overview

`perl-quote` is a **Tier 1 leaf crate** providing quote operator handling for Perl parsing.

**Purpose**: Quote operator handling — parses and validates Perl's complex quoting constructs (`q//`, `qq//`, `qw//`, `qr//`, `qx//`, heredocs).

**Version**: 0.8.8

## Commands

```bash
cargo build -p perl-quote            # Build this crate
cargo test -p perl-quote             # Run tests
cargo clippy -p perl-quote           # Lint
cargo doc -p perl-quote --open       # View documentation
```

## Architecture

### Dependencies

**None** — pure Rust with no external dependencies.

### Quote Operators

| Operator | Purpose | Interpolation |
|----------|---------|---------------|
| `q//` | Single-quoted string | No |
| `qq//` | Double-quoted string | Yes |
| `qw//` | Word list | No |
| `qr//` | Compiled regex | Yes |
| `qx//` | Command execution | Yes |
| `tr///` | Transliteration | No |
| `y///` | Transliteration (alias) | No |
| `s///` | Substitution | Yes/No |
| `m//` | Match | Yes |

### Delimiter Handling

Perl allows various delimiters for quote operators:

```perl
q/string/          # Forward slash
q{string}          # Braces (paired)
q[string]          # Brackets (paired)
q<string>          # Angle brackets (paired)
q(string)          # Parentheses (paired)
q!string!          # Any non-alphanumeric
```

### Key Functions

```rust
// Check if a character is a valid quote delimiter
is_valid_delimiter(ch: char) -> bool

// Find matching closing delimiter
matching_delimiter(open: char) -> Option<char>

// Parse quote operator content
parse_quoted_content(input: &str, delimiter: char) -> Result<...>
```

## Usage

```rust
use perl_quote::{QuoteKind, parse_quote};

let result = parse_quote("qq{hello $world}");
match result.kind {
    QuoteKind::DoubleQuote => { /* interpolated */ },
    QuoteKind::SingleQuote => { /* literal */ },
    // ...
}
```

## Important Notes

- Nested delimiters (e.g., `q{a{b}c}`) require balanced counting
- Escape sequences differ by quote type
- Heredoc handling is in `perl-heredoc` crate
