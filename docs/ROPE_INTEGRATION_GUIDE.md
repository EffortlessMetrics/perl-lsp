# Rope Integration Guide

*Diataxis Framework*:
- **Tutorials**: Getting started with Rope integration
- **How-to Guides**: Specific implementation tasks  
- **Reference**: API documentation and specifications
- **Explanation**: Design decisions and concepts

## Overview (*Diataxis: Explanation* - Design concepts)

The perl-parser crate includes comprehensive Rope support for efficient document management, enabling sub-millisecond position conversions and scalable text manipulation for large Perl files. Rope integration provides the foundation for LSP performance with UTF-16/UTF-8 compatibility.

## Core Rope Modules (*Diataxis: Reference* - API specifications)

The perl-parser crate includes comprehensive Rope support for document management:

- **`textdoc.rs`**: UTF-16 aware text document handling with `ropey::Rope`
- **`position_mapper.rs`**: Centralized position mapping (CRLF/LF/CR line endings, UTF-16 code units, byte offsets)
- **`incremental_integration.rs`**: Bridge between LSP server and incremental parsing with Rope
- **`incremental_handler_v2.rs`**: Enhanced incremental document updates using Rope

## Getting Started (*Diataxis: Tutorial* - Learning-oriented guidance)

### Basic Rope Usage

```rust
// Create a simple document with Rope
use ropey::Rope;
use crate::textdoc::Doc;

let content = "my $var = 'hello world';\nprint $var;\n";
let rope = Rope::from_str(content);
let mut doc = Doc { rope, version: 1 };

// Navigate through the document
let line_count = doc.rope.len_lines();
let char_count = doc.rope.len_chars();
println!("Document has {} lines and {} characters", line_count, char_count);
```

## Position Conversion API (*Diataxis: Reference* - API specifications)

```rust
// UTF-16/UTF-8 position conversion
use crate::textdoc::{Doc, PosEnc, lsp_pos_to_byte, byte_to_lsp_pos};
use ropey::Rope;

// Create document with Rope
let mut doc = Doc { rope: Rope::from_str(content), version };

// Convert LSP positions (UTF-16) to byte offsets 
let byte_offset = lsp_pos_to_byte(&doc.rope, pos, PosEnc::Utf16);

// Convert byte offsets to LSP positions
let lsp_pos = byte_to_lsp_pos(&doc.rope, byte_offset, PosEnc::Utf16);
```

## Advanced Features (*Diataxis: Explanation* - Understanding the capabilities)

### Line Ending Support
- **CRLF handling**: Proper Windows line ending support with automatic detection
- **Mixed line endings**: Robust detection and handling of mixed CRLF/LF/CR within single files
- **UTF-16 emoji support**: Correct positioning with Unicode characters requiring surrogate pairs
- **Performance**: O(log n) line ending lookups using Rope's internal B-tree structure

### Performance Characteristics
- **Sub-millisecond position conversions**: Typical conversions complete in <100µs
- **Memory efficiency**: Rope uses gap buffer techniques for optimal memory usage
- **Incremental updates**: Only affected text ranges are re-parsed during edits
- **Unicode safety**: All position conversions handle multibyte UTF-8 sequences correctly

## Implementation Guide (*Diataxis: How-to Guide* - Step-by-step implementation tasks)

### Integrating Rope with LSP Providers

```rust
// Example: Adding Rope support to a new LSP provider
use crate::textdoc::{Doc, PosEnc};
use lsp_types::{Position, Range};

pub fn my_lsp_provider(doc: &Doc, range: Range) -> Result<Vec<MyResult>, Error> {
    // Convert LSP positions to byte offsets using Rope
    let start_byte = lsp_pos_to_byte(&doc.rope, range.start, PosEnc::Utf16)?;
    let end_byte = lsp_pos_to_byte(&doc.rope, range.end, PosEnc::Utf16)?;
    
    // Work with byte offsets for parser operations
    let text_slice = doc.rope.byte_slice(start_byte..end_byte);
    
    // Process the text and convert results back to LSP positions
    let results = process_text_slice(text_slice)?;
    
    // Convert results back to LSP format
    results.into_iter().map(|result| {
        let lsp_pos = byte_to_lsp_pos(&doc.rope, result.byte_offset, PosEnc::Utf16)?;
        Ok(MyResult { position: lsp_pos, ..result })
    }).collect()
}
```

### Handling Document Updates

```rust
// Example: Processing incremental document changes
use crate::incremental_handler_v2::IncrementalHandler;

pub fn handle_document_change(
    doc: &mut Doc, 
    changes: Vec<lsp_types::TextDocumentContentChangeEvent>
) -> Result<(), Error> {
    for change in changes {
        if let Some(range) = change.range {
            // Convert LSP range to Rope indices
            let start_idx = doc.rope.line_to_char(range.start.line as usize) + range.start.character as usize;
            let end_idx = doc.rope.line_to_char(range.end.line as usize) + range.end.character as usize;
            
            // Apply the change using Rope's efficient editing
            doc.rope.remove(start_idx..end_idx);
            doc.rope.insert(start_idx, &change.text);
            
            // Update document version
            doc.version += 1;
        } else {
            // Full document replacement
            doc.rope = Rope::from_str(&change.text);
            doc.version += 1;
        }
    }
    Ok(())
}
```

## Development Guidelines (*Diataxis: How-to Guide* - Development practices)

**Where to Make Rope Improvements**:
- **Production Code**: `/crates/perl-parser/src/` - All Rope enhancements should target this crate
- **Key Modules**: `textdoc.rs`, `position_mapper.rs`, `incremental_*.rs` modules
- **NOT Internal Test Harnesses**: Avoid modifying `/crates/tree-sitter-perl-rs/` or other internal test code

### Best Practices

1. **Position Conversion**: Always use the provided helper functions instead of manual calculations
2. **Error Handling**: Rope operations can fail with invalid positions - always handle errors gracefully
3. **Performance**: Cache position conversions when processing multiple operations on the same range
4. **Unicode Safety**: Never assume single-byte characters - use Rope's Unicode-aware methods

## Testing and Validation (*Diataxis: How-to Guide* - Testing procedures)

### Comprehensive Test Suite

```bash
# Test Rope-based position mapping
cargo test -p perl-parser position_mapper

# Test incremental parsing with Rope integration  
cargo test -p perl-parser incremental_integration_test

# Test UTF-16 position conversion with multibyte characters
cargo test -p perl-parser multibyte_edit_test

# Test LSP document changes with Rope
cargo test -p perl-lsp lsp_comprehensive_e2e_test
```