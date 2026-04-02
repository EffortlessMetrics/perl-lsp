# perl-lsp-selection-range

Smart selection-range expansion for Perl source. It returns the nested regions
an editor should select as the user expands the current selection.

## Use this crate when

Use `perl-lsp-selection-range` if you need the selection-range algorithm itself.
It is a narrow helper crate, not the umbrella provider surface.

## Key exports

- `selection_ranges` - compute nested selection ranges for one or more cursors

## Example

```rust,ignore
use lsp_types::Position;
use perl_lsp_selection_range::selection_ranges;

let ranges = selection_ranges(source, &[Position::new(3, 8)]);
```

## Stack role

`perl-lsp` uses this crate for `textDocument/selectionRange`. It turns parsed
Perl structure into progressively larger editor selections.
