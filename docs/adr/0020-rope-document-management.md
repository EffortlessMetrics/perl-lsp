# ADR-0020: Rope-Based Document Management

**Status**: Accepted
**Date**: 2025-01-20
**Decision Makers**: Perl LSP Architecture Team
**Related**: [ROPE_INTEGRATION_GUIDE.md](../reference/ROPE_INTEGRATION_GUIDE.md), [CRATE_ARCHITECTURE_GUIDE.md](../reference/CRATE_ARCHITECTURE_GUIDE.md)

## Context

LSP servers require efficient text document management with frequent position conversions between different coordinate systems:

1. **LSP Protocol**: Uses UTF-16 code units for position encoding
2. **Parser Operations**: Work with byte offsets for text slicing
3. **Editor Updates**: Send UTF-16 positions, receive UTF-16 positions
4. **Large Files**: Perl files can be thousands of lines with complex Unicode

### Performance Requirements

| Operation | Target | Typical |
|-----------|--------|---------|
| Position conversion | <1ms | <100µs |
| Line lookup | O(log n) | O(log n) |
| Character access | O(log n) | O(1) amortized |
| Edit application | O(log n) | O(log n) |

### Problem Statement

Naive string-based document management causes:
- O(n) position conversions for each LSP operation
- Memory copying on every edit
- Poor performance with large files
- Unicode handling complexity

## Decision

**We use `ropey::Rope` data structure for O(log n) position lookups with comprehensive UTF-16/UTF-8 compatibility, enabling sub-millisecond position conversions and scalable document management.**

### Core Rope Modules

| Module | Purpose |
|--------|---------|
| `textdoc.rs` | UTF-16 aware text document handling with `ropey::Rope` |
| `position_mapper.rs` | Centralized position mapping (CRLF/LF/CR, UTF-16, byte offsets) |
| `incremental_integration.rs` | Bridge between LSP server and incremental parsing |
| `incremental_handler_v2.rs` | Enhanced incremental document updates |

### Position Conversion API

```rust
use crate::textdoc::{Doc, PosEnc, lsp_pos_to_byte, byte_to_lsp_pos};
use ropey::Rope;

// Create document with Rope
let mut doc = Doc { rope: Rope::from_str(content), version };

// Convert LSP positions (UTF-16) to byte offsets 
let byte_offset = lsp_pos_to_byte(&doc.rope, pos, PosEnc::Utf16);

// Convert byte offsets to LSP positions
let lsp_pos = byte_to_lsp_pos(&doc.rope, byte_offset, PosEnc::Utf16);
```

### Performance Characteristics

```rust
// Sub-millisecond position conversions
// Typical conversions complete in <100µs

// O(log n) line ending lookups using Rope's internal B-tree
// Memory efficiency via gap buffer techniques
// Incremental updates - only affected ranges re-parsed
```

### Line Ending Support

- **CRLF handling**: Proper Windows line ending support with automatic detection
- **Mixed line endings**: Robust detection and handling of mixed CRLF/LF/CR
- **UTF-16 emoji support**: Correct positioning with surrogate pairs
- **Performance**: O(log n) line ending lookups

### LSP Provider Integration

```rust
pub fn my_lsp_provider(doc: &Doc, range: Range) -> Result<Vec<MyResult>, Error> {
    // Convert LSP positions to byte offsets using Rope
    let start_byte = lsp_pos_to_byte(&doc.rope, range.start, PosEnc::Utf16)?;
    let end_byte = lsp_pos_to_byte(&doc.rope, range.end, PosEnc::Utf16)?;
    
    // Work with byte offsets for parser operations
    let text_slice = doc.rope.byte_slice(start_byte..end_byte);
    
    // Process and convert results back to LSP positions
    let results = process_text_slice(text_slice)?;
    
    results.into_iter().map(|result| {
        let lsp_pos = byte_to_lsp_pos(&doc.rope, result.byte_offset, PosEnc::Utf16)?;
        Ok(MyResult { position: lsp_pos, ..result })
    }).collect()
}
```

## Consequences

### Positive

- **Sub-millisecond Conversions**: Typical operations complete in <100µs
- **Scalable for Large Files**: O(log n) performance regardless of file size
- **Memory Efficient**: Gap buffer techniques minimize memory usage
- **Unicode Safe**: All position conversions handle multibyte sequences correctly
- **Incremental Updates**: Only affected text ranges re-parsed during edits

### Negative

- **Learning Curve**: Rope semantics differ from simple string operations
- **API Complexity**: Position encoding must be explicitly specified
- **Debugging Overhead**: Rope state harder to inspect than plain strings

### Mitigations

- Comprehensive ROPE_INTEGRATION_GUIDE.md documentation
- Clear API with explicit position encoding types
- Helper functions for common conversion patterns

## Performance Metrics

| Operation | Complexity | Typical Time |
|-----------|------------|--------------|
| LSP to byte | O(log n) | <100µs |
| Byte to LSP | O(log n) | <100µs |
| Line count | O(1) | <1µs |
| Char at position | O(log n) | <10µs |
| Insert text | O(log n) | <1ms |
| Delete text | O(log n) | <1ms |

Where n = document length in characters.

## References

- [ROPE_INTEGRATION_GUIDE.md](../reference/ROPE_INTEGRATION_GUIDE.md) - Complete integration guide
- [CRATE_ARCHITECTURE_GUIDE.md](../reference/CRATE_ARCHITECTURE_GUIDE.md) - perl-parser crate documentation
- [UTF16_POSITION_TRACKING.md](0013-utf16-position-tracking.md) - Related ADR on UTF-16 handling
- [ropey crate](https://docs.rs/ropey/) - Underlying Rope implementation
