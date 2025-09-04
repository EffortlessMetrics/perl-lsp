# Rope Integration Guide

## Core Rope Modules

The perl-parser crate includes comprehensive Rope support for document management:

- **`textdoc.rs`**: UTF-16 aware text document handling with `ropey::Rope`
- **`position_mapper.rs`**: Centralized position mapping (CRLF/LF/CR line endings, UTF-16 code units, byte offsets)
- **`incremental_integration.rs`**: Bridge between LSP server and incremental parsing with Rope
- **`incremental_handler_v2.rs`**: Enhanced incremental document updates using Rope

## Position Conversion Features

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

## Line Ending Support

- **CRLF handling**: Proper Windows line ending support
- **Mixed line endings**: Robust detection and handling of mixed CRLF/LF/CR
- **UTF-16 emoji support**: Correct positioning with Unicode characters requiring surrogate pairs

## Development Guidelines

**Where to Make Rope Improvements**:
- **Production Code**: `/crates/perl-parser/src/` - All Rope enhancements should target this crate
- **Key Modules**: `textdoc.rs`, `position_mapper.rs`, `incremental_*.rs` modules
- **NOT Internal Test Harnesses**: Avoid modifying `/crates/tree-sitter-perl-rs/` or other internal test code

## Testing Commands

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