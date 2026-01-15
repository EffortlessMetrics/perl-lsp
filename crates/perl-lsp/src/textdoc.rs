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

/// Convert LSP position to char index with UTF-16/UTF-8 encoding support
///
/// This function handles the conversion from LSP Position (line, character)
/// to a char index in the Rope, accounting for UTF-16 vs UTF-8 encoding
/// differences. Unicode characters like emojis are handled correctly.
///
/// # Arguments
/// * `rope` - The rope containing the document text
/// * `pos` - LSP position with 0-based line and character indices
/// * `enc` - Whether to interpret character positions as UTF-16 or UTF-8
///
/// # Returns
/// Char index clamped to valid rope boundaries
pub fn lsp_pos_to_char(rope: &Rope, pos: Position, enc: PosEnc) -> usize {
    // Handle edge case: if line is beyond document end, clamp to end
    if pos.line as usize >= rope.len_lines() {
        return rope.len_chars();
    }

    let line_char0 = rope.line_to_char(pos.line as usize);
    let line_slice = rope.line(pos.line as usize);

    let col_chars = match enc {
        PosEnc::Utf8 => pos.character as usize,
        PosEnc::Utf16 => {
            let mut char_idx = 0usize;
            let mut utf16_units = 0u32;

            for ch in line_slice.chars() {
                if utf16_units >= pos.character {
                    break;
                }
                utf16_units += ch.len_utf16() as u32;
                char_idx += 1;
            }
            char_idx
        }
    };

    // Clamp to line boundaries
    let line_chars = line_slice.chars().count();
    let clamped_col = col_chars.min(line_chars);
    let target_char = line_char0 + clamped_col;

    target_char.min(rope.len_chars())
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
    rope.char_to_byte(lsp_pos_to_char(rope, pos, enc))
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

/// Convert LSP range to char index pair
///
/// Converts both start and end positions of an LSP Range to char indices
/// for rope operations. Ropey's `remove` and `insert` methods operate on
/// char indices, not byte offsets.
///
/// # Arguments
/// * `rope` - The rope containing the document text
/// * `range` - LSP range with start and end positions
/// * `enc` - Position encoding format
///
/// # Returns
/// Tuple of (start_char, end_char) clamped to rope bounds
pub fn range_to_chars(rope: &Rope, range: &Range, enc: PosEnc) -> (usize, usize) {
    let s = lsp_pos_to_char(rope, range.start, enc);
    let e = lsp_pos_to_char(rope, range.end, enc);
    (s.min(rope.len_chars()), e.min(rope.len_chars()))
}

/// Convert LSP range to byte offset pair
///
/// Converts both start and end positions of an LSP Range to byte offsets.
/// Use `range_to_chars` for rope operations like `remove` and `insert`.
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
///
/// # Note
/// Ropey's `remove` and `insert` operate on **char indices**, not byte offsets.
/// This function correctly converts LSP positions to char indices for rope operations.
pub fn apply_changes(doc: &mut Doc, changes: &[TextDocumentContentChangeEvent], enc: PosEnc) {
    for ch in changes {
        if let Some(r) = &ch.range {
            // IMPORTANT: Rope::remove and Rope::insert use char indices, not byte offsets
            let (s, e) = range_to_chars(&doc.rope, r, enc);
            if s <= e {
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
