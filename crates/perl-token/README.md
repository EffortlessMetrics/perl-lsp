# perl-token

Core token and token-kind definitions for Perl lexing/parsing.

## Scope

- Defines `Token` with source text and byte-span metadata.
- Defines `TokenKind` classification used by parser/token stream layers.
- Serves as the shared token contract across lexer/parser crates.

## Public Surface

- `Token`.
- `TokenKind`.

## Workspace Role

Foundational internal crate used by `perl-lexer`, `perl-tokenizer`, and parser crates.

## License

MIT OR Apache-2.0.
