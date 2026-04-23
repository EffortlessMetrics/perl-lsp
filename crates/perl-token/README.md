# perl-token

Core token type definitions for the Perl parser ecosystem.

## Overview

`perl-token` is a Tier 1 leaf crate that defines the shared token types used
across the lexer, tokenizer, and parser crates. It has zero external
dependencies (only `std::sync::Arc`).

## Public API

- **`Token`** -- a token with `kind: TokenKind`, `text: Arc<str>`, `start: usize`, `end: usize`
- **`TokenKind`** -- enum classifying every Perl token: keywords, operators, delimiters, literals, sigils, and special tokens

## Usage

```rust
use perl_token::{Token, TokenKind};

let tok = Token::new(TokenKind::Identifier, "foo", 0, 3);
assert_eq!(tok.kind, TokenKind::Identifier);
```

## Workspace Role

Foundational crate consumed by `perl-lexer`, `perl-tokenizer`, `perl-parser-core`,
and downstream parser/LSP crates. Part of the
[tree-sitter-perl-rs](https://github.com/EffortlessMetrics/perl-lsp) workspace.

## License

MIT OR Apache-2.0
