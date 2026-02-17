# perl-heredoc

Heredoc collection and normalization utilities for Perl source text.

## Scope

- Collects pending heredoc bodies from source text.
- Handles delimiter styles and quote kinds.
- Supports `<<~` indentation stripping and CRLF-safe terminator matching.

## Public Surface

- Data types: `PendingHeredoc`, `HeredocContent`, `CollectionResult`, `QuoteKind`.
- Function: `collect_all`.

## Workspace Role

Internal parser helper crate used by lexer/parser pipelines.

## License

MIT OR Apache-2.0.
