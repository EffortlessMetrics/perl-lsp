# CLAUDE.md

This file provides guidance to Claude Code when working with code in this crate.

## Crate Overview

- **Tier**: 1 (leaf crate, no internal workspace dependencies)
- **Purpose**: UTF-8/UTF-16 position tracking, byte-span types, and line-index caching for the Perl LSP ecosystem. Bridges the gap between Rust byte offsets and LSP UTF-16 code-unit positions.
- **Version**: 0.9.1

## Commands

```bash
cargo build -p perl-position-tracking        # Build
cargo test -p perl-position-tracking         # Run tests
cargo clippy -p perl-position-tracking       # Lint
cargo doc -p perl-position-tracking --open   # View docs
```

## Architecture

### Dependencies

| Dependency | Purpose |
|------------|---------|
| `ropey` | Rope data structure for efficient incremental text editing in `PositionMapper` |
| `serde` / `serde_json` | Serialization for position/span types and JSON LSP helpers |
| `thiserror` | Error type definitions |
| `lsp-types` (optional, `lsp-compat` feature) | Bidirectional `From` conversions with `lsp_types::Position`, `Range`, `Location` |

### Modules

| Module | File | Key Exports |
|--------|------|-------------|
| `span` | `src/span.rs` | `ByteSpan`, `SourceLocation` (type alias) |
| `position` | `src/position.rs` | `Position` (engine, 1-based line/col), `Range` |
| `line_index` | `src/line_index.rs` | `LineStartsCache` (borrows text), `LineIndex` (owns text) |
| `convert` | `src/convert.rs` | `offset_to_utf16_line_col`, `utf16_line_col_to_offset` |
| `mapper` | `src/mapper.rs` | `PositionMapper`, `LineEnding`, `apply_edit_utf8`, `json_to_position`, `position_to_json`, `newline_count`, `last_line_column_utf8` |
| `wire` | `src/wire.rs` | `WirePosition`, `WireRange`, `WireLocation`, plus `lsp-compat` `From` impls |

### Key Types

| Type | Description |
|------|-------------|
| `ByteSpan` | Half-open `[start, end)` byte range. Supports `contains`, `overlaps`, `intersection`, `union`, `slice`. |
| `SourceLocation` | Type alias for `ByteSpan` (backward compat). |
| `Position` | Engine type: byte offset + 1-based line/column. Supports `advance` over text. |
| `Range` | Engine type: start/end `Position` pair with `contains_byte`, `overlaps`, `extend`, `span_to`. |
| `LineStartsCache` | Cached line-start byte offsets. Converts offsets to 0-based line + UTF-16 column. Works with `&str` or `Rope`. |
| `LineIndex` | Owning variant of line index. Stores text and line starts together. |
| `PositionMapper` | Rope-backed mapper: LSP pos to byte offset, byte offset to LSP pos, incremental edits, line-ending detection. |
| `WirePosition` | LSP wire type: 0-based line + UTF-16 character. |
| `WireRange` | LSP wire type: start/end `WirePosition`. |
| `WireLocation` | LSP wire type: URI + `WireRange`. |
| `LineEnding` | Enum: `Lf`, `CrLf`, `Cr`, `Mixed`. |

## Usage Examples

### Byte-offset spans

```rust
use perl_position_tracking::ByteSpan;

let span = ByteSpan::new(7, 13);
let text = span.slice("line 1\nline 2\nline 3"); // "line 2"
```

### Offset to LSP position (via cache)

```rust
use perl_position_tracking::LineStartsCache;

let cache = LineStartsCache::new("line 1\nline 2");
let (line, col) = cache.offset_to_position("line 1\nline 2", 7);
// line=1, col=0 (0-indexed, UTF-16 units)
```

### Rope-backed incremental mapper

```rust
use perl_position_tracking::PositionMapper;

let mut mapper = PositionMapper::new("hello world");
mapper.apply_edit(6, 11, "Rust");
assert_eq!(mapper.text(), "hello Rust");
```

## Important Notes

- UTF-16 conversion must be round-trip safe (byte -> LSP position -> byte).
- `LineStartsCache` handles CR, LF, and CRLF line endings.
- `PositionMapper` re-detects `LineEnding` after each edit.
- Engine `Position` uses 1-based line/column; wire `WirePosition` uses 0-based line + UTF-16 character.
- The `lsp-compat` feature gates `From` impls between wire types and `lsp_types` -- it is optional to avoid pulling in `lsp-types` for non-LSP consumers.
- Performance-sensitive: called on every keystroke during editing.
