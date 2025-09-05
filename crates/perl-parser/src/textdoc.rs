//! Rope-based text document handling for LSP with UTF-16 aware position mapping
//!
//! This module provides efficient document management using the `ropey` crate for
//! O(log n) insertions, deletions, and position conversions. It handles the conversion
//! between LSP's UTF-16 based positions and Rust's UTF-8 strings, ensuring proper
//! handling of Unicode characters including emojis and multi-byte sequences.
//!
//! ## Key Features
//! - **Efficient Edits**: O(log n) performance for document modifications
//! - **UTF-16 Compliance**: Proper LSP position mapping for Unicode text
//! - **Incremental Updates**: Support for LSP TextDocumentContentChangeEvent
//! - **Position Safety**: Boundary-checked conversions with graceful clamping

use lsp_types::{Position, Range, TextDocumentContentChangeEvent};
use ropey::Rope;

/// Document state using Rope for efficient text operations
/// 
/// The `Doc` struct stores document content in a Rope data structure,
/// providing O(log n) performance for edits while maintaining UTF-8/UTF-16
/// position mapping capabilities for LSP compliance.
#[derive(Clone)]
pub struct Doc {
    /// Rope-backed document content for efficient edits and slicing
    pub rope: Rope,
    /// Document version number for LSP synchronization
    pub version: i32,
}

/// Position encoding format for LSP compatibility
/// 
/// LSP uses UTF-16 code units for positions, while Rust strings are UTF-8.
/// This enum determines how position conversions are performed.
#[derive(Clone, Copy)]
pub enum PosEnc {
    /// UTF-16 encoding (LSP standard) - counts UTF-16 code units
    Utf16,
    /// UTF-8 encoding (Rust native) - counts UTF-8 bytes  
    Utf8,
}

/// Convert LSP position to byte offset with UTF-16/UTF-8 encoding support
/// 
/// This function handles the conversion from LSP Position (line, character)
/// to a byte offset in the Rope, accounting for UTF-16 vs UTF-8 encoding
/// differences. Unicode characters like emojis are handled correctly.
/// 
/// # Arguments
/// * `rope` - The rope containing the document text
/// * `pos` - LSP position with 0-based line and character indices
/// * `enc` - Whether to interpret character positions as UTF-16 or UTF-8
/// 
/// # Returns
/// Byte offset clamped to valid rope boundaries
pub fn lsp_pos_to_byte(rope: &Rope, pos: Position, enc: PosEnc) -> usize {
    let line = pos.line as usize;
    let col = pos.character as usize;

    if line >= rope.len_lines() {
        return rope.len_bytes();
    }

    let line_char0 = rope.line_to_char(line);

    match enc {
        PosEnc::Utf8 => {
            let line_chars = rope.line(line).len_chars();
            let char_idx = col.min(line_chars);
            rope.char_to_byte(line_char0 + char_idx)
        }
        PosEnc::Utf16 => {
            let line_slice = rope.line(line);
            let mut cu = 0usize;
            for (i, ch) in line_slice.chars().enumerate() {
                if cu >= col {
                    return rope.char_to_byte(line_char0 + i);
                }
                cu += ch.len_utf16();
            }
            // Clamp to end of line
            rope.char_to_byte(line_char0 + line_slice.len_chars())
        }
    }
}

/// Convert byte offset to LSP position with UTF-16/UTF-8 encoding support
/// 
/// This function performs the reverse conversion from a byte offset in the Rope
/// back to an LSP Position, ensuring proper character counting for the specified
/// encoding format.
/// 
/// # Arguments
/// * `rope` - The rope containing the document text
/// * `byte` - Byte offset to convert (will be clamped to rope bounds)
/// * `enc` - Whether to count characters as UTF-16 or UTF-8
/// 
/// # Returns
/// LSP Position with 0-based line and character indices
pub fn byte_to_lsp_pos(rope: &Rope, byte: usize, enc: PosEnc) -> Position {
    let byte = byte.min(rope.len_bytes());
    let char_idx = rope.byte_to_char(byte);
    let line = rope.char_to_line(char_idx);
    let line_char0 = rope.line_to_char(line);
    let col_chars = char_idx - line_char0;

    let character = match enc {
        PosEnc::Utf8 => col_chars as u32,
        PosEnc::Utf16 => {
            let line_slice = rope.line(line);
            let mut cu = 0u32;
            for (i, ch) in line_slice.chars().enumerate() {
                if i >= col_chars {
                    break;
                }
                cu += ch.len_utf16() as u32;
            }
            cu
        }
    };

    Position { line: line as u32, character }
}

/// Convert LSP range to byte offset pair
/// 
/// Converts both start and end positions of an LSP Range to byte offsets
/// for efficient rope operations.
/// 
/// # Arguments
/// * `rope` - The rope containing the document text
/// * `range` - LSP range with start and end positions
/// * `enc` - Position encoding format
/// 
/// # Returns
/// Tuple of (start_byte, end_byte) clamped to rope bounds
pub fn range_to_bytes(rope: &Rope, range: &Range, enc: PosEnc) -> (usize, usize) {
    let s = lsp_pos_to_byte(rope, range.start, enc);
    let e = lsp_pos_to_byte(rope, range.end, enc);
    (s.min(rope.len_bytes()), e.min(rope.len_bytes()))
}

/// Apply incremental LSP text changes to a Rope-backed document
/// 
/// Processes an array of LSP TextDocumentContentChangeEvent objects,
/// applying them to the document's rope in sequence. Supports both
/// full document replacement and ranged incremental edits.
/// 
/// # Arguments
/// * `doc` - Mutable document to modify
/// * `changes` - Array of LSP change events to apply
/// * `enc` - Position encoding for interpreting ranges
/// 
/// # Behavior
/// - Changes without ranges replace the entire document
/// - Changes with ranges perform incremental edits at specified positions
/// - All position calculations respect UTF-16/UTF-8 encoding differences
/// - Invalid ranges are safely clamped to document boundaries
pub fn apply_changes(doc: &mut Doc, changes: &[TextDocumentContentChangeEvent], enc: PosEnc) {
    for ch in changes {
        if let Some(r) = &ch.range {
            let (s, e) = range_to_bytes(&doc.rope, r, enc);
            if s <= doc.rope.len_bytes() && e <= doc.rope.len_bytes() && s <= e {
                doc.rope.remove(s..e);
                doc.rope.insert(s, &ch.text);
            }
        } else {
            // Full document replace
            doc.rope = Rope::from_str(&ch.text);
        }
    }
}

// Make PosEnc Clone
