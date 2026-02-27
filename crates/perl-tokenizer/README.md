# perl-tokenizer

Token stream, trivia preservation, and utility functions for the Perl parser pipeline.

## Overview

`perl-tokenizer` bridges the raw `perl-lexer` output and the recursive descent parser. It provides a buffered token stream with up to three-token lookahead, trivia-preserving lexing and parsing for code formatting, position tracking wrappers, and helpers for locating `__DATA__`/`__END__` markers.

## Public API

- `TokenStream` -- parser-oriented stream with `peek`, `peek_second`, `peek_third`, and statement-boundary reset
- `TokenWithPosition` / `PositionTracker` -- attach line/column info to lexer tokens
- `Trivia`, `TriviaToken`, `TriviaLexer` -- classify and collect comments, whitespace, and POD as trivia
- `TriviaPreservingParser`, `TriviaParserContext` -- parse source while preserving trivia on AST nodes
- `find_data_marker_byte_lexed`, `code_slice` -- locate `__DATA__`/`__END__` boundaries
- Re-exports `Token` and `TokenKind` from `perl-token`

## Workspace Role

Internal Tier 2 crate consumed by `perl-parser-core` and downstream tooling. Not intended for standalone use outside the workspace.

## License

MIT OR Apache-2.0
