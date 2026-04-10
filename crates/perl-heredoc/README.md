# perl-heredoc

Heredoc collection and normalization for Perl source text.

## Overview

Collects heredoc bodies from raw source bytes given a queue of pending
declarations. Supports all four Perl quoting styles (`<<EOF`, `<<'EOF'`,
`<<"EOF"`, `` <<`EOF` ``) and the indented heredoc syntax (`<<~EOF`)
introduced in Perl 5.26, including common-prefix whitespace stripping.

CRLF line endings are normalized during terminator matching so the
collector works identically on Windows and Unix sources.

## Public API

- `collect_all` -- process a `VecDeque<PendingHeredoc>` against source bytes, returning `CollectionResult`
- `PendingHeredoc` -- declaration info: label, quoting style, indentation flag, source span
- `HeredocContent` -- collected body: per-line `ByteSpan` segments, full span, terminated flag
- `CollectionResult` -- all collected contents, terminator-found flags, next byte offset
- `QuoteKind` -- enum: `Unquoted`, `Single`, `Double`, `Backtick`


## Coverage

The crate's behavior is already covered by
[`tests/comprehensive_unit_tests.rs`](tests/comprehensive_unit_tests.rs),
which exercises the public API and the main heredoc edge cases:

- terminated heredoc collection
- indented heredoc stripping (`<<~`)
- unterminated heredoc reporting
- CRLF handling
- FIFO behavior across multiple pending heredocs
- exact label matching

## Workspace Role

Internal Tier 1 leaf crate in the
[tree-sitter-perl-rs](https://github.com/EffortlessMetrics/perl-lsp)
workspace. Used by lexer and parser pipelines to resolve heredoc bodies
after scanning heredoc declarations.

## License

MIT OR Apache-2.0
