# perl-tokenizer

Token stream, wrapper, and trivia utilities for Perl parser pipelines.

## Scope

- Provides parser-oriented token stream access patterns.
- Preserves/handles trivia (comments and whitespace) when needed.
- Supplies wrappers and parser context helpers for structured token handling.

## Public Surface

- `TokenStream`, `TokenWithPosition`.
- `Trivia`, `TriviaToken`.
- `TriviaPreservingParser`, `TriviaParserContext`.
- Re-exports `Token`, `TokenKind` from `perl-token`.

## Workspace Role

Internal infrastructure crate used by parser-core and related tooling.

## License

MIT OR Apache-2.0.
