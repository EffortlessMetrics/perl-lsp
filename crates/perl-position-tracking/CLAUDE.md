# CLAUDE.md

This file provides guidance to Claude Code when working with code in this repository.

## Crate Overview

`perl-position-tracking` is a **Tier 1 leaf crate** providing UTF-8/UTF-16 position tracking and conversion for LSP compatibility.

**Purpose**: UTF-8/UTF-16 position tracking and conversion for Perl LSP — critical infrastructure for accurate editor positioning.

**Version**: 0.8.8

## Commands

```bash
cargo build -p perl-position-tracking        # Build this crate
cargo test -p perl-position-tracking         # Run tests
cargo clippy -p perl-position-tracking       # Lint
cargo doc -p perl-position-tracking --open   # View documentation
```

## Architecture

### Dependencies

- `serde_json` - JSON serialization
- `ropey` - Rope data structure for efficient text handling
- `thiserror` - Error definitions
- `serde` - Serialization traits

### Features

| Feature | Purpose |
|---------|---------|
| `lsp-compat` | LSP type compatibility (optional) |

### Key Types

| Type | Purpose |
|------|---------|
| `Position` | Line/column position |
| `Span` | Range with start/end positions |
| `ByteOffset` | Byte offset in source |
| `Utf16Offset` | UTF-16 code unit offset |
| `LineIndex` | Line start byte offsets |

### Position Conversion

LSP uses UTF-16 code units; Rust uses UTF-8 bytes. This crate bridges them:

```rust
use perl_position_tracking::{LineIndex, Position};

let source = "hello 世界";
let index = LineIndex::new(source);

// Byte offset → LSP position
let pos = index.byte_to_position(6);  // Position { line: 0, character: 6 }

// LSP position → byte offset
let byte = index.position_to_byte(pos);
```

### UTF-16 Considerations

```rust
// "世界" is 2 chars in Rust, but 4 UTF-16 code units
// (each CJK char is 1 char but 2 UTF-16 units for supplementary plane)

// Accurate conversion is critical for:
// - Cursor positioning
// - Selection ranges
// - Diagnostic spans
```

## Critical Role

This crate is used by almost every other crate in the workspace because:

1. **Lexer** — tracks byte positions during tokenization
2. **Parser** — assigns spans to AST nodes
3. **Semantic Analyzer** — positions for symbols
4. **LSP Features** — all position-based operations

## Important Notes

- UTF-16 conversion must be symmetric (round-trip safe)
- Line endings are normalized (CR, LF, CRLF → LF internally)
- Performance is critical — called frequently during editing
- Test with multi-byte UTF-8 characters (emoji, CJK)
