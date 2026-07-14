# CLAUDE.md (perl-line-index)

## Role

Single-purpose line/column indexing utility: maps byte offsets to `(line,
column)` and back, using cached line-start byte offsets.

## Owns

- `LineIndex` -- built from source text once via `new(text: &str)`.
- `byte_to_position(byte) -> (line, column)` -- byte-column conversion.
- `position_to_byte(line, column) -> Option<usize>` -- byte-column reverse
  conversion, returning `None` for out-of-range lines/columns.
- `position_to_byte_utf16(text, line, column) -> Option<usize>` -- reverse
  conversion for UTF-16 code-unit columns (what LSP `Position.character`
  actually uses).
- `position_to_byte_checked` -- a checked variant of the reverse conversion.

## Does not own

- Building or maintaining the source text itself -- `LineIndex` is built
  once from a `&str` snapshot; callers own re-indexing on edits.
- Any parser or AST types -- this crate has no dependencies at all.

## Neighbors

- Upstream: none (zero-dependency leaf crate).
- Downstream: `perl-parser` (the only current in-workspace consumer).

## Read first

- `src/lib.rs` -- the entire crate; small enough to read in full.

## Focused validation

`cargo test -p perl-line-index`. `tests/line_index_roundtrip.rs` and
`tests/prop_line_index_roundtrip.rs` (property-based) cover
byte<->position round-tripping; `tests/utf16_position.rs` covers the
UTF-16 conversion path specifically -- LSP position math bugs are the
highest-cost failure mode for this crate, since they translate into wrong
cursor/range placement in the editor.

## Review hotspots

`position_to_byte_utf16` -- surrogate-pair handling (`char::len_utf16()`)
is the subtle part; any change here should be checked against the
property test, not just the example-based ones.

## Claim boundary

Describes the conversion API as authored and its round-trip guarantees as
exercised by the property tests. Does not assert behavior for malformed or
non-UTF-8 input -- the API takes `&str`, which Rust already guarantees is
valid UTF-8.
