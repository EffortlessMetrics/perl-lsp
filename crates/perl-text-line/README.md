# perl-text-line

`perl-text-line` provides single-line text helpers used by Perl parser/LSP workflows:

- locate the line bounds for a byte cursor in UTF-8 source text
- compute identifier-character boundaries for simple scanner checks
- skip contiguous ASCII whitespace while scanning within a line

The crate is intentionally tiny and side-effect free so it can be reused by
feature crates with strict boundaries.
