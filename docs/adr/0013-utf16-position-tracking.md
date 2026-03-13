# ADR-0013: UTF-16 Position Tracking with Symmetric Conversion

**Status**: Accepted
**Date**: 2025-01-20
**Decision Makers**: Perl LSP Architecture Team
**Related**: [POSITION_TRACKING_GUIDE.md](../reference/POSITION_TRACKING_GUIDE.md), [ROPE_INTEGRATION_GUIDE.md](../reference/ROPE_INTEGRATION_GUIDE.md)

## Context

The Language Server Protocol (LSP) specification requires all position values to be expressed in UTF-16 code units, not UTF-8 bytes or Unicode code points. This creates a fundamental mismatch with Rust's native string representation, which is UTF-8 based.

### Problem Statement

1. **Protocol Requirement**: LSP clients (VSCode, Neovim, etc.) send and expect positions in UTF-16 code units
2. **Rust Native Encoding**: Rust strings are UTF-8, creating a conversion requirement
3. **Multi-byte Characters**: Emoji and CJK characters may require multiple UTF-16 code units (surrogate pairs)
4. **Security Concerns**: Improper conversion can lead to boundary violations and position spoofing
5. **Round-trip Accuracy**: Conversions must be symmetric to maintain position integrity

### Encoding Complexity Examples

| Character | UTF-8 Bytes | UTF-16 Code Units | LSP Position Delta |
|-----------|-------------|-------------------|-------------------|
| `A` | 1 | 1 | 1 |
| `é` | 2 | 1 | 1 |
| `世` | 3 | 1 | 1 |
| `🦀` | 4 | 2 (surrogate pair) | 2 |

## Decision

**We implement symmetric UTF-16 position conversion with boundary validation through a dedicated `PositionTracker` system integrated with the Rope data structure.**

### Core Architecture

```rust
/// Position tracking with UTF-16 support and security validation
pub struct PositionTracker {
    source: String,
    line_starts: LineStartsCache,
}

impl PositionTracker {
    /// SECURE: Convert UTF-8 offset to UTF-16 position with boundary validation
    pub fn convert_utf8_to_utf16_position(&self, text: &str, utf8_offset: usize) -> u32;
    
    /// SECURE: Convert UTF-16 position to UTF-8 offset with boundary validation
    pub fn convert_utf16_to_utf8_position(&self, text: &str, utf16_pos: u32) -> usize;
}
```

### Security Features

1. **Boundary Validation**: All conversions check input bounds before processing
2. **Symmetric Operations**: UTF-8 ↔ UTF-16 conversions use identical validation logic
3. **Overflow Prevention**: Arithmetic operations include safe bounds checking
4. **Fractional Handling**: Proper handling of positions within multi-byte sequences

### Implementation Components

| Component | Purpose | Location |
|-----------|---------|----------|
| `PositionTracker` | Core position conversion | `perl-parser/src/position_tracker.rs` |
| `LineStartsCache` | O(log n) line lookups | `perl-parser/src/line_starts_cache.rs` |
| `textdoc.rs` | Rope integration | `perl-parser/src/textdoc.rs` |
| `position_mapper.rs` | Centralized mapping | `perl-parser/src/position_mapper.rs` |

### Rope Integration

The system integrates with the `ropey` crate for efficient document manipulation:

```rust
use ropey::Rope;
use crate::textdoc::{Doc, PosEnc, lsp_pos_to_byte, byte_to_lsp_pos};

// Convert LSP positions (UTF-16) to byte offsets
let byte_offset = lsp_pos_to_byte(&doc.rope, pos, PosEnc::Utf16);

// Convert byte offsets to LSP positions
let lsp_pos = byte_to_lsp_pos(&doc.rope, byte_offset, PosEnc::Utf16);
```

## Consequences

### Positive

1. **Protocol Compliance**: Full LSP specification compliance for position handling
2. **Unicode Safety**: Correct handling of all Unicode characters including emoji and CJK
3. **Security**: Prevents position spoofing and boundary violation attacks
4. **Performance**: O(log n) position lookups via Rope's B-tree structure
5. **Round-trip Accuracy**: Symmetric conversion ensures position integrity

### Negative

1. **Complexity**: Additional conversion layer between Rust and LSP positions
2. **Performance Overhead**: Conversion operations add ~10-50μs per position lookup
3. **Testing Burden**: Requires extensive edge case testing for Unicode scenarios

### Mitigations

- LineStartsCache provides O(log n) lookups to minimize conversion overhead
- Comprehensive test suite covers edge cases including surrogate pairs
- Mutation testing validates security properties

## Testing

### Test Commands

```bash
# Run position tracking tests
cargo test -p perl-parser --test parser_context -- test_utf16_position_mapping

# Test UTF-16 security enhancements
cargo test -p perl-parser --test mutation_hardening_tests -- utf16_security

# Test symmetric conversion
cargo test -p perl-parser parser_context_tests::test_symmetric_position_conversion
```

### Security Test Coverage

```rust
#[test]
fn test_utf16_security_validation() {
    let text = "Test with 🦀 emoji and 🌍 symbols";
    let tracker = PositionTracker::new(text.to_string());

    // Test boundary conditions
    assert_eq!(tracker.convert_utf8_to_utf16_position(text, 0), 0);
    
    // Test overflow protection
    let overflow_result = tracker.convert_utf8_to_utf16_position(text, usize::MAX);
    assert!(overflow_result <= text.chars().count() as u32);

    // Test symmetric conversion
    for i in 0..=text.len() {
        let utf16_pos = tracker.convert_utf8_to_utf16_position(text, i);
        let back_to_utf8 = tracker.convert_utf16_to_utf8_position(text, utf16_pos);
        assert_eq!(back_to_utf8, i);
    }
}
```

## References

- [Position Tracking Guide](../reference/POSITION_TRACKING_GUIDE.md)
- [Rope Integration Guide](../reference/ROPE_INTEGRATION_GUIDE.md)
- [LSP Specification - Text Documents](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#textDocuments)
- [PR #153: Security Enhancements](https://github.com/EffortlessMetrics/perl-lsp/pull/153)
