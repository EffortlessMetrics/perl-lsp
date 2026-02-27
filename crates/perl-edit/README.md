# perl-edit

Edit tracking primitives for incremental Perl parsing.

## Overview

`perl-edit` provides types and algorithms for tracking text edits to Perl source
code and computing their positional effects. It tracks both byte offsets and
line/column coordinates, making it suitable for incremental parsing and LSP
document synchronization.

## Public API

- **`Edit`** -- a single text edit with start/old-end/new-end byte offsets and
  positions. Methods: `byte_shift()`, `line_shift()`, `affects_byte()`,
  `overlaps_range()`, `apply_to_position()`, `apply_to_range()`.
- **`EditSet`** -- an ordered collection of edits, sorted by start position.
  Methods: `add()`, `apply_to_position()`, `apply_to_range()`,
  `affects_range()`, `byte_shift_at()`, `affected_ranges()`, `len()`,
  `is_empty()`, `edits()`.

## Workspace Role

Tier 1 leaf crate in the `tree-sitter-perl-rs` workspace. Depends only on
`perl-position-tracking` for `Position` and `Range` types. Used by incremental
parsing and document update pipelines.

## License

MIT OR Apache-2.0
