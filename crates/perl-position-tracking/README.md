# perl-position-tracking

UTF-8/UTF-16 position tracking and conversion for the Perl LSP ecosystem.

Part of [tree-sitter-perl-rs](https://github.com/EffortlessMetrics/perl-lsp).

## Public API

- **`ByteSpan`** / **`SourceLocation`** -- byte-offset span for parser and AST nodes
- **`LineStartsCache`** -- line-start index for offset-to-position conversion (borrows text)
- **`LineIndex`** -- owning line index with UTF-16 column support
- **`PositionMapper`** -- rope-backed mapper with incremental edit support and line-ending detection
- **`WirePosition`** / **`WireRange`** / **`WireLocation`** -- LSP wire-protocol position types
- **`Position`** / **`Range`** -- engine types with 1-based line/column and byte offset
- **`offset_to_utf16_line_col`** / **`utf16_line_col_to_offset`** -- standalone conversion functions
- **`LineEnding`** -- detected line-ending style (LF, CRLF, CR, Mixed)

Enable the `lsp-compat` feature for bidirectional `From` conversions with `lsp_types`.

## Example

```rust
use perl_position_tracking::{ByteSpan, LineStartsCache};

let source = "line 1\nline 2\nline 3";
let cache = LineStartsCache::new(source);

let span = ByteSpan::new(7, 13);
assert_eq!(span.slice(source), "line 2");

let (line, col) = cache.offset_to_position(source, span.start);
assert_eq!((line, col), (1, 0)); // 0-indexed line/UTF-16 column
```

## License

MIT OR Apache-2.0
