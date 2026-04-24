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

/// Outcome of applying a batch of LSP text changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ApplyChangesOutcome {
    /// When true, at least one change could not be mapped with strong confidence
    /// (e.g. malformed range ordering or split multi-byte boundary), so callers
    /// should avoid parser incremental-edit fast paths for this batch.
    pub requires_conservative_parse: bool,
}

fn pos_leq(a: Position, b: Position) -> bool {
    (a.line, a.character) <= (b.line, b.character)
}

fn lsp_pos_to_char_with_alignment(rope: &Rope, pos: Position, enc: PosEnc) -> (usize, bool) {
    // Out-of-range line offsets are clamped to EOF and treated as ambiguous.
    if pos.line as usize >= rope.len_lines() {
        return (rope.len_chars(), false);
    }

    let line_char0 = rope.line_to_char(pos.line as usize);
    let line_slice = rope.line(pos.line as usize);
    let mut char_idx = 0usize;
    let mut offset = 0u32;
    let mut exact = true;

    match enc {
        PosEnc::Utf8 => {
            for ch in line_slice.chars() {
                let next = offset + ch.len_utf8() as u32;
                if next > pos.character {
                    exact = offset == pos.character;
                    break;
                }
                offset = next;
                char_idx += 1;
            }
        }
        PosEnc::Utf16 => {
            for ch in line_slice.chars() {
                let next = offset + ch.len_utf16() as u32;
                if next > pos.character {
                    exact = offset == pos.character;
                    break;
                }
                offset = next;
                char_idx += 1;
            }
        }
    }

    // Past-end columns are clamped to EOL and treated as ambiguous.
    let line_chars = line_slice.chars().count();
    if char_idx == line_chars && pos.character > offset {
        exact = false;
    }

    let target_char = (line_char0 + char_idx.min(line_chars)).min(rope.len_chars());
    (target_char, exact)
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
    lsp_pos_to_char_with_alignment(rope, pos, enc).0
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
        PosEnc::Utf8 => {
            // UTF-8: return byte count from start of line
            let line_slice = rope.line(line);
            line_slice.chars().take(col_chars).map(|c| c.len_utf8() as u32).sum()
        }
        PosEnc::Utf16 => {
            // UTF-16: return UTF-16 code unit count from start of line
            let line_slice = rope.line(line);
            line_slice.chars().take(col_chars).map(|c| c.len_utf16() as u32).sum()
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
    let _ = apply_changes_with_outcome(doc, changes, enc, None);
}

/// Apply LSP text changes while surfacing whether conservative parse fallback
/// should be used by higher layers.
///
/// `incoming_version` may be provided by callers that track didChange sequencing.
/// If it is stale (`<= doc.version`), the batch is ignored and marked conservative.
pub fn apply_changes_with_outcome(
    doc: &mut Doc,
    changes: &[TextDocumentContentChangeEvent],
    enc: PosEnc,
    incoming_version: Option<i32>,
) -> ApplyChangesOutcome {
    let mut requires_conservative_parse = false;

    if let Some(version) = incoming_version {
        if version <= doc.version {
            return ApplyChangesOutcome { requires_conservative_parse: true };
        }
    }

    for ch in changes {
        if let Some(r) = &ch.range {
            if !pos_leq(r.start, r.end) {
                requires_conservative_parse = true;
                continue;
            }

            // IMPORTANT: Rope::remove and Rope::insert use char indices, not byte offsets
            let (s, start_exact) = lsp_pos_to_char_with_alignment(&doc.rope, r.start, enc);
            let (e, end_exact) = lsp_pos_to_char_with_alignment(&doc.rope, r.end, enc);
            if !start_exact || !end_exact {
                requires_conservative_parse = true;
            }

            if s <= e {
                doc.rope.remove(s..e);
                doc.rope.insert(s, &ch.text);
            } else {
                requires_conservative_parse = true;
            }
        } else {
            // Full document replace
            doc.rope = Rope::from_str(&ch.text);
            requires_conservative_parse = true;
        }
    }

    ApplyChangesOutcome { requires_conservative_parse }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Regression test: delete emoji via LSP range should work correctly.
    /// This catches both the "rope uses chars" bug and UTF-16 boundary handling.
    #[test]
    fn test_delete_emoji_via_lsp_range() {
        // "hi 😀x\n" - emoji is 4 bytes, 2 UTF-16 units
        let mut doc = Doc { rope: Rope::from_str("hi \u{1F600}x\n"), version: 1 };

        // Delete the emoji: positions are in UTF-16 code units
        // "hi " = 3 chars/units, emoji = 2 units, so emoji is at [3, 5)
        let change = TextDocumentContentChangeEvent {
            range: Some(Range {
                start: Position { line: 0, character: 3 },
                end: Position { line: 0, character: 5 },
            }),
            range_length: None,
            text: String::new(),
        };

        apply_changes(&mut doc, &[change], PosEnc::Utf16);

        assert_eq!(doc.rope.to_string(), "hi x\n", "Emoji should be deleted correctly");
    }

    /// Test that position inside surrogate pair clamps correctly (before, not after).
    #[test]
    fn test_position_inside_surrogate_clamps_before() {
        let rope = Rope::from_str("hi \u{1F600}x");

        // Position 4 is "inside" the emoji (which spans [3, 5) in UTF-16)
        let pos = Position { line: 0, character: 4 };
        let char_idx = lsp_pos_to_char(&rope, pos, PosEnc::Utf16);

        // Should clamp to char index 3 (start of emoji), not 4 (after emoji)
        assert_eq!(char_idx, 3, "Position inside surrogate should clamp to start of char");
    }

    /// Test UTF-8 encoding handles multi-byte chars correctly.
    #[test]
    fn test_utf8_position_multi_byte() {
        let rope = Rope::from_str("hi \u{1F600}x");

        // In UTF-8, emoji is 4 bytes, so 'x' is at byte offset 7
        // "hi " = 3 bytes, emoji = 4 bytes, 'x' = byte 7
        let pos = Position { line: 0, character: 7 };
        let char_idx = lsp_pos_to_char(&rope, pos, PosEnc::Utf8);

        // Should be char index 4 (0='h', 1='i', 2=' ', 3=emoji, 4='x')
        assert_eq!(char_idx, 4, "UTF-8 byte offset should map to correct char");
    }

    /// Test byte_to_lsp_pos returns correct UTF-16 character count.
    #[test]
    fn test_byte_to_lsp_pos_utf16_emoji() {
        let rope = Rope::from_str("hi \u{1F600}x");

        // 'x' is at byte 7, char index 4
        let pos = byte_to_lsp_pos(&rope, 7, PosEnc::Utf16);

        // "hi " = 3 UTF-16 units, emoji = 2 UTF-16 units, so 'x' is at character 5
        assert_eq!(pos.line, 0);
        assert_eq!(pos.character, 5, "UTF-16 character should account for emoji surrogate pair");
    }

    /// Test roundtrip: byte -> lsp position -> byte
    #[test]
    fn test_roundtrip_with_emoji() {
        let rope = Rope::from_str("hi \u{1F600}x\nworld");

        // Test 'x' position (byte 7)
        let pos = byte_to_lsp_pos(&rope, 7, PosEnc::Utf16);
        let back = lsp_pos_to_byte(&rope, pos, PosEnc::Utf16);
        assert_eq!(back, 7, "Roundtrip should preserve byte offset");

        // Test 'w' position (byte 9, line 1)
        let pos2 = byte_to_lsp_pos(&rope, 9, PosEnc::Utf16);
        assert_eq!(pos2.line, 1);
        let back2 = lsp_pos_to_byte(&rope, pos2, PosEnc::Utf16);
        assert_eq!(back2, 9, "Roundtrip on second line should work");
    }

    #[test]
    fn test_apply_changes_marks_malformed_range_conservative() {
        let mut doc = Doc { rope: Rope::from_str("abcdef"), version: 1 };
        let changes = vec![TextDocumentContentChangeEvent {
            range: Some(Range {
                start: Position { line: 0, character: 4 },
                end: Position { line: 0, character: 2 },
            }),
            range_length: None,
            text: "X".to_string(),
        }];

        let outcome = apply_changes_with_outcome(&mut doc, &changes, PosEnc::Utf16, Some(2));
        assert!(outcome.requires_conservative_parse);
        assert_eq!(doc.rope.to_string(), "abcdef");
    }

    #[test]
    fn test_apply_changes_marks_stale_version_conservative() {
        let mut doc = Doc { rope: Rope::from_str("abc"), version: 4 };
        let changes = vec![TextDocumentContentChangeEvent {
            range: None,
            range_length: None,
            text: "zzz".to_string(),
        }];

        let outcome = apply_changes_with_outcome(&mut doc, &changes, PosEnc::Utf16, Some(4));
        assert!(outcome.requires_conservative_parse);
        assert_eq!(doc.rope.to_string(), "abc");
    }
}
