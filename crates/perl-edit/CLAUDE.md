# CLAUDE.md

This file provides guidance to Claude Code when working with code in this repository.

## Crate Overview

`perl-edit` is a **Tier 1 leaf crate** providing edit tracking primitives for incremental Perl parsing.

**Purpose**: Represents text edits with byte and line/column coordinates, computes positional shifts, and applies edits to positions and ranges.

**Version**: 0.9.0

## Commands

```bash
cargo build -p perl-edit             # Build this crate
cargo test -p perl-edit              # Run tests
cargo clippy -p perl-edit            # Lint
cargo doc -p perl-edit --open        # View documentation
```

## Architecture

### Dependencies

- `perl-position-tracking` -- provides `Position` and `Range` types

### Key Types

| Type | Purpose |
|------|---------|
| `Edit` | Single text edit with start/old-end/new-end byte offsets and positions |
| `EditSet` | Ordered collection of non-overlapping edits, sorted by start byte |

### Edit Structure

```rust
pub struct Edit {
    pub start_byte: usize,
    pub old_end_byte: usize,
    pub new_end_byte: usize,
    pub start_position: Position,
    pub old_end_position: Position,
    pub new_end_position: Position,
}
```

### Edit Methods

| Method | Description |
|--------|-------------|
| `byte_shift()` | Net byte offset change (`new_end_byte - old_end_byte`) |
| `line_shift()` | Net line number change |
| `affects_byte(byte)` | Whether a byte position is at or after the edit start |
| `overlaps_range(range)` | Whether a range overlaps this edit |
| `apply_to_position(pos)` | Shift a position; returns `None` if inside the edit |
| `apply_to_range(range)` | Shift a range; returns `None` if either endpoint is inside the edit |

### EditSet Methods

| Method | Description |
|--------|-------------|
| `add(edit)` | Insert an edit, maintaining sorted order by `start_byte` |
| `apply_to_position(pos)` | Apply all edits cumulatively to a position |
| `apply_to_range(range)` | Apply all edits cumulatively to a range |
| `affects_range(range)` | Whether any edit overlaps the given range |
| `byte_shift_at(byte)` | Cumulative byte shift from all edits ending before the given byte |
| `affected_ranges()` | Original ranges covered by the edits |
| `len()` / `is_empty()` | Edit count and emptiness check |
| `edits()` | Read-only slice of the underlying edits |

## Usage

```rust
use perl_edit::{Edit, EditSet};
use perl_position_tracking::Position;

let edit = Edit::new(
    10, 15, 17,
    Position::new(10, 2, 5),
    Position::new(15, 2, 10),
    Position::new(17, 2, 12),
);

assert_eq!(edit.byte_shift(), 2);

let mut edits = EditSet::new();
edits.add(edit);
```

## Important Notes

- Edits within an `EditSet` are maintained in sorted order by `start_byte`
- `apply_to_position` returns `None` for positions inside an edited region (invalidated)
- Positions before an edit are unchanged; positions after are shifted by the byte/line delta
- Column adjustment only applies when the position shares the same line as the edit end
- This crate has no `TextEdit` or `EditBuilder` types -- those names do not exist here
