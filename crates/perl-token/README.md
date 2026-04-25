# perl-token

Core token type definitions for the Perl parser ecosystem.

## Overview

`perl-token` is a Tier 1 leaf crate that defines the shared token types used
across the lexer, tokenizer, and parser crates. It has zero external
dependencies (only `std::sync::Arc`).

## Public API

- **`Token`** -- a token with `kind: TokenKind`, `text: Arc<str>`, `start: usize`, `end: usize`
- **`TokenRef<'src>`** -- borrowed token view with `kind`, `text: &'src str`, `start`, `end` for allocation-sensitive paths
- **`TokenKind`** -- enum classifying every Perl token: keywords, operators, delimiters, literals, sigils, and special tokens

## Usage

```rust
use perl_token::{Token, TokenKind, TokenRef};

let tok = Token::new(TokenKind::Identifier, "foo", 0, 3);
assert_eq!(tok.kind, TokenKind::Identifier);

let borrowed = TokenRef::new(TokenKind::Identifier, "foo", 0, 3);
let owned = borrowed.to_owned_token();
assert_eq!(owned, tok);
```

Borrowed view from owned tokens:

```rust
use perl_token::{Token, TokenKind};

let tok = Token::new(TokenKind::My, "my", 0, 2);
let borrowed = tok.as_ref_token();
assert_eq!(borrowed.text, "my");
```

## Benchmark scorecard

Run the token allocation scorecard benchmark:

```bash
cargo bench -p perl-token --bench token_scorecard
```

The benchmark group includes:
- `token/borrowed_construction` (no `Arc` allocation)
- `token/owned_construction` (`Arc<str>` allocation path)
- `token/borrowed_to_owned_conversion` (explicit conversion cost)

## Workspace Role

Foundational crate consumed by `perl-lexer`, `perl-tokenizer`, `perl-parser-core`,
and downstream parser/LSP crates. Part of the
[tree-sitter-perl-rs](https://github.com/EffortlessMetrics/perl-lsp) workspace.

## License

MIT OR Apache-2.0
