# CLAUDE.md

This file provides guidance to Claude Code when working with code in this repository.

## Crate Overview

`perl-heredoc` is a **Tier 1 leaf crate** that collects heredoc bodies from raw Perl source bytes.

**Purpose**: Given a queue of `PendingHeredoc` declarations and source bytes, walks lines from a starting offset, matches terminators, and returns span-based content with indentation stripping and CRLF normalization.

**Version**: 0.9.0

## Commands

```bash
cargo build -p perl-heredoc          # Build this crate
cargo test -p perl-heredoc           # Run tests
cargo clippy -p perl-heredoc         # Lint
cargo doc -p perl-heredoc --open     # View documentation
```

## Architecture

### Dependencies

- `perl-position-tracking` -- provides `ByteSpan` (re-exported as `Span`)

### Key Types and Functions

| Item | Kind | Purpose |
|------|------|---------|
| `collect_all` | `pub fn` | Entry point: processes `VecDeque<PendingHeredoc>` in FIFO order against `&[u8]` source |
| `PendingHeredoc` | `pub struct` | Declaration info: `label` (`Arc<str>`), `allow_indent`, `quote` (`QuoteKind`), `decl_span` |
| `HeredocContent` | `pub struct` | Result per heredoc: `segments` (per-line `ByteSpan`s), `full_span`, `terminated` flag |
| `CollectionResult` | `pub struct` | Aggregate result: `contents`, `terminators_found`, `next_offset` |
| `QuoteKind` | `pub enum` | `Unquoted`, `Single`, `Double`, `Backtick` |

### Internal helpers (private)

- `collect_one` -- collects a single heredoc body, handles `<<~` indent stripping
- `next_line_bounds` -- line iterator over `&[u8]` with CRLF support
- `split_leading_ws` -- splits leading spaces/tabs from a byte slice
- `strip_trailing_cr` -- drops trailing `\r` for terminator comparison
- `common_prefix_len` -- byte-level common prefix for indent stripping

### Processing Flow

1. Caller creates `PendingHeredoc` entries during lexing when `<<LABEL` is encountered
2. `collect_all` pops entries from the `VecDeque` in FIFO order
3. For each entry, `collect_one` scans lines from the current byte offset
4. Terminator matching: leading whitespace is stripped, trailing CR is removed, remainder compared to label
5. For `<<~` (indented) heredocs, terminator's leading whitespace becomes the baseline; that prefix is stripped from all content lines
6. Unterminated heredocs return `terminated: false` with whatever content was found

## Usage

```rust
use perl_heredoc::{collect_all, PendingHeredoc, QuoteKind};
use perl_position_tracking::ByteSpan;
use std::collections::VecDeque;
use std::sync::Arc;

let src = b"Hello world\nEND\n";
let mut pending = VecDeque::new();
pending.push_back(PendingHeredoc {
    label: Arc::from("END"),
    allow_indent: false,
    quote: QuoteKind::Double,
    decl_span: ByteSpan { start: 0, end: 0 },
});

let result = collect_all(src, 0, pending);
assert!(result.terminators_found[0]);
```

## Important Notes

- Multiple heredocs can be stacked (FIFO queue); `collect_all` processes them sequentially
- Indented heredocs (`<<~`) strip the common leading whitespace prefix based on the terminator line
- Terminator comparison is case-sensitive and exact after whitespace/CR stripping
- All spans are byte-level (`ByteSpan`), not character-level
- The crate operates on `&[u8]` slices, not `&str`, for maximum flexibility
