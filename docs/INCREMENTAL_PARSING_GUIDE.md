# Incremental Parsing Guide

## Overview

The native parser includes **production-ready incremental parsing** with **Rope-based document management** for efficient real-time LSP editing.

## Architecture

- **IncrementalDocument**: High-performance document state with subtree caching and Rope integration
- **Rope-based Text Management**: Efficient UTF-16/UTF-8 position conversion using `ropey` crate
- **Subtree Reuse**: Container nodes reuse unchanged AST subtrees from cache  
- **Metrics Tracking**: Detailed performance metrics (reused vs reparsed nodes)
- **Content-based Caching**: Hash-based subtree matching for common patterns
- **Position-based Caching**: Range-based subtree matching with precise Rope position tracking

## Rope Integration

The perl-parser crate includes comprehensive Rope support for document management:

**Core Rope Modules**:
- **`textdoc.rs`**: UTF-16 aware text document handling with `ropey::Rope`
- **`position_mapper.rs`**: Centralized position mapping (CRLF/LF/CR line endings, UTF-16 code units, byte offsets)
- **`incremental_integration.rs`**: Bridge between LSP server and incremental parsing with Rope
- **`incremental_handler_v2.rs`**: Enhanced incremental document updates using Rope

**Position Conversion Features**:
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

**Line Ending Support**:
- **CRLF handling**: Proper Windows line ending support
- **Mixed line endings**: Robust detection and handling of mixed CRLF/LF/CR
- **UTF-16 emoji support**: Correct positioning with Unicode characters requiring surrogate pairs

## Performance Targets

- **<1ms updates** for small edits (single token changes) with Rope optimization
- **<2ms updates** for moderate edits (function-level changes) with subtree reuse
- **Cache hit ratios** of 70-90% for typical editing scenarios
- **Memory efficient** with LRU cache eviction, Arc<Node> sharing, and Rope's piece table architecture

## API Usage

### Basic Incremental Parsing API
```rust
// Create incremental document with Rope support
let mut doc = IncrementalDocument::new(source)?;

// Apply single edit (automatically uses Rope for position tracking)
let edit = IncrementalEdit::new(start_byte, end_byte, new_text);
doc.apply_edit(edit)?;

// Apply multiple edits in batch (Rope handles position adjustments)
let mut edits = IncrementalEditSet::new();
edits.add(edit1);
edits.add(edit2);
doc.apply_edits(&edits)?;

// Performance metrics with Rope-optimized parsing
println!("Parse time: {:.2}ms", doc.metrics.last_parse_time_ms);
println!("Nodes reused: {}", doc.metrics.nodes_reused);
println!("Nodes reparsed: {}", doc.metrics.nodes_reparsed);
```

### Advanced Incremental Parsing (IncrementalParserV2)

**Key Features**:
- **Smart Node Reuse**: Automatically detects which AST nodes can be preserved across edits
- **Metrics Tracking**: Provides detailed statistics on reused vs reparsed nodes
- **Simple Value Edit Detection**: Optimized for common scenarios like number/string changes
- **Fallback Mechanisms**: Graceful degradation to full parsing when needed

**Usage Example**:
```rust
// Basic incremental parsing with IncrementalParserV2
use perl_parser::{incremental_v2::IncrementalParserV2, edit::Edit, position::Position};

let mut parser = IncrementalParserV2::new();

// Initial parse
let tree1 = parser.parse("my $x = 42;")?;
println!("Initial: Reparsed={}, Reused={}", parser.reparsed_nodes, parser.reused_nodes);

// Apply edit (change "42" to "4242")
parser.edit(Edit::new(8, 10, 12, /* position data */));
let tree2 = parser.parse("my $x = 4242;")?;
println!("After edit: Reparsed={}, Reused={}", parser.reparsed_nodes, parser.reused_nodes);
// Expected output: Reparsed=1, Reused=3 (only the Number node needs reparsing)
```

## LSP Integration

- **Document Management**: LSP server uses Rope for all document state (`textdoc::Doc`)
- **Position Conversion**: Automatic UTF-16 ↔ UTF-8 conversion via `position_mapper::PositionMapper`
- **Incremental Updates**: Enable via `PERL_LSP_INCREMENTAL=1` environment variable
- **Change Application**: Efficient change processing using `textdoc::apply_changes()`
- **Fallback Mechanisms**: Graceful degradation to full parsing when incremental parsing fails
- **Testing**: Comprehensive integration tests with async LSP harness and Rope-based position validation

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

# Test the example implementation
cargo run -p perl-parser --example test_incremental_v2 --features incremental

# Run comprehensive incremental tests
cargo test -p perl-parser --test incremental_integration_test --features incremental

# Run all incremental-related tests
cargo test -p perl-parser incremental --features incremental
```