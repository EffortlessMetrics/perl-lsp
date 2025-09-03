# Position Tracking Guide

## Overview

The enhanced position tracking system provides accurate line/column mapping for LSP compliance with production-ready performance and comprehensive Unicode support.

## Features

- **O(log n) Position Mapping**: Efficient binary search-based position lookups using LineStartsCache
- **LSP-Compliant UTF-16 Support**: Accurate character counting for multi-byte Unicode characters and emoji
- **Multi-line Token Support**: Proper position tracking for tokens spanning multiple lines (strings, comments, heredocs)
- **Line Ending Agnostic**: Handles CRLF, LF, and CR line endings consistently across platforms
- **Production-Ready Integration**: Seamless integration with parser context and LSP server for real-time editing
- **Comprehensive Testing**: 8 specialized test cases covering Unicode, CRLF, multiline strings, and edge cases

## API Reference

### Core PositionTracker methods
```rust
impl PositionTracker {
    /// Create from source text with line start caching
    pub fn new(source: String) -> Self;
    
    /// Convert byte offset to Position with UTF-16 support  
    pub fn byte_to_position(&self, byte_offset: usize) -> Position;
}

// LineStartsCache for O(log n) lookups
impl LineStartsCache {
    /// Build cache with CRLF/LF/CR line ending support
    pub fn new(text: &str) -> Self;
    
    /// Convert byte offset to (line, utf16_column)
    pub fn offset_to_position(&self, text: &str, offset: usize) -> (u32, u32);
}
```

## Usage Guide

### Using PositionTracker in Parser Context
```rust
use crate::parser_context::ParserContext;

// Create parser with automatic position tracking
let ctx = ParserContext::new(source);

// Access accurate token positions
let token = ctx.current_token().unwrap();
let range = token.range();
println!("Token at line {}, column {}", range.start.line, range.start.column);
```

## Testing

### Test Commands
```bash
# Run position tracking tests
cargo test -p perl-parser --test parser_context -- test_multiline_positions
cargo test -p perl-parser --test parser_context -- test_utf16_position_mapping
cargo test -p perl-parser --test parser_context -- test_crlf_line_endings

# Test with specific edge cases
cargo test -p perl-parser parser_context_tests::test_multiline_string_token_positions
```

## Integration with LSP

The position tracking system is fully integrated with:
- LSP position conversion (UTF-16 ↔ UTF-8)
- Multi-line token handling
- Real-time editing support
- Cross-platform line ending support

This ensures accurate position reporting for all LSP features including hover, go-to-definition, and diagnostics.