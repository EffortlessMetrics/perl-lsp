# perl-line-index

A tiny microcrate that provides a byte-oriented line index used by incremental parsing paths.

## API

- `LineIndex::new(&str)` builds a line-start cache.
- `LineIndex::byte_to_position(usize)` maps byte offset to `(line, column)`.
- `LineIndex::position_to_byte(line, column)` maps back to byte offsets.
- `LineIndex::position_to_byte_checked(line, column)` returns `None` if the
  column crosses the addressed line's byte boundary.
