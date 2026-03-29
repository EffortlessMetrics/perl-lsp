# perl-lsp-selection-range

[![Crates.io](https://img.shields.io/crates/v/perl-lsp-selection-range.svg)](https://crates.io/crates/perl-lsp-selection-range)
[![Documentation](https://docs.rs/perl-lsp-selection-range/badge.svg)](https://docs.rs/perl-lsp-selection-range)

Smart selection expansion for Perl editors and language servers.

## When to use this crate

Use `perl-lsp-selection-range` when you want
`textDocument/selectionRange` behavior for Perl source. It expands a cursor or
selection outward through useful syntax boundaries such as:

- string content -> full string -> expression
- hash key -> subscript -> full access expression
- identifier -> statement -> block -> enclosing function

## Quick example

```rust
use lsp_types::Position;
use perl_lsp_selection_range::selection_ranges;

let source = "my $value = foo($bar);\n";
let ranges = selection_ranges(source, &[Position::new(0, 4)]);
assert_eq!(ranges.len(), 1);
```

## Public API

- `selection_ranges`: returns nested LSP `SelectionRange` trees for positions

## License

MIT OR Apache-2.0
