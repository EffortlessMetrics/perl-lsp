# perl-lsp-selection-range

Smart selection-range expansion for Perl source text.

## Problem it solves

Editors can grow the current selection from a word to larger syntactic units,
but they need language-specific rules to do it well. This crate expands Perl
selections through strings, hash subscripts, statements, blocks, subroutines,
and whole-file scopes.

## Public API

- `selection_ranges` returns LSP `SelectionRange` chains for one or more
  cursor positions.

## Expansion model

- string content -> quoted string -> containing expression
- hash key -> `{key}` -> full access expression
- identifier -> trimmed line -> statement -> block -> subroutine -> file

## Example

```rust,ignore
use lsp_types::Position;
use perl_lsp_selection_range::selection_ranges;

let ranges = selection_ranges(source, &[Position::new(3, 12)]);
```

## Workspace role

`perl-lsp` uses this crate to implement `textDocument/selectionRange` without
embedding byte/UTF-16 mapping and span-expansion rules in the server runtime.

## License

MIT OR Apache-2.0
