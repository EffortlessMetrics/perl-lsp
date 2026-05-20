//! Targeted coverage tests for uncovered branches in perl-position-tracking.
//!
//! Each test covers a specific function or branch identified by source analysis.

use perl_position_tracking::{
    ByteSpan, LineIndex, LineStartsCache, Position, PositionMapper, Range, WirePosition, WireRange,
};

// ─── ByteSpan: hash and Serialize/Deserialize derivation ────────────────────

#[test]
fn byte_span_hash_is_consistent() {
    use std::collections::HashSet;
    let a = ByteSpan::new(0, 5);
    let b = ByteSpan::new(0, 5);
    let c = ByteSpan::new(1, 5);
    let mut set = HashSet::new();
    set.insert(a);
    set.insert(b);
    set.insert(c);
    assert_eq!(set.len(), 2);
}

#[test]
fn byte_span_serde_round_trip_non_zero() -> Result<(), serde_json::Error> {
    let span = ByteSpan::new(10, 20);
    let json = serde_json::to_string(&span)?;
    let back: ByteSpan = serde_json::from_str(&json)?;
    assert_eq!(span, back);
    Ok(())
}

// ─── ByteSpan: try_slice out-of-bounds returns None ─────────────────────────

#[test]
fn byte_span_try_slice_oob_at_start_returns_none() {
    let span = ByteSpan::new(5, 50); // end past source length
    assert!(span.try_slice("short").is_none());
}

// ─── Position: advance_char with multi-byte character ───────────────────────

#[test]
fn position_advance_char_multibyte_increments_byte_and_column() {
    let mut pos = Position::new(0, 1, 1);
    pos.advance_char('日'); // 3-byte UTF-8, 1 UTF-16 unit
    assert_eq!(pos.byte, 3);
    assert_eq!(pos.line, 1);
    assert_eq!(pos.column, 2);
}

#[test]
fn position_advance_char_emoji_increments_byte_by_four() {
    let mut pos = Position::new(0, 1, 1);
    pos.advance_char('💖'); // 4-byte UTF-8
    assert_eq!(pos.byte, 4);
    assert_eq!(pos.line, 1);
    assert_eq!(pos.column, 2);
}

// ─── Range: is_empty when non-empty ─────────────────────────────────────────

#[test]
fn range_is_empty_false_for_non_empty() {
    let start = Position::new(0, 1, 1);
    let end = Position::new(5, 1, 6);
    let r = Range::new(start, end);
    assert!(!r.is_empty());
}

#[test]
fn range_is_empty_true_when_bytes_equal() {
    let p = Position::new(7, 2, 3);
    let r = Range::empty(p);
    assert!(r.is_empty());
}

// ─── Range: len ─────────────────────────────────────────────────────────────

#[test]
fn range_len_returns_byte_difference() {
    let start = Position::new(3, 1, 4);
    let end = Position::new(10, 1, 11);
    let r = Range::new(start, end);
    assert_eq!(r.len(), 7);
}

#[test]
fn range_len_empty_is_zero() {
    let p = Position::new(5, 1, 6);
    let r = Range::empty(p);
    assert_eq!(r.len(), 0);
}

// ─── Range: contains (position) ─────────────────────────────────────────────

#[test]
fn range_contains_position_inside_returns_true() {
    let start = Position::new(0, 1, 1);
    let end = Position::new(10, 1, 11);
    let r = Range::new(start, end);
    let mid = Position::new(5, 1, 6);
    assert!(r.contains(mid));
}

#[test]
fn range_contains_position_at_end_returns_false() {
    let start = Position::new(0, 1, 1);
    let end = Position::new(10, 1, 11);
    let r = Range::new(start, end);
    // end is exclusive
    assert!(!r.contains(end));
}

// ─── LineStartsCache: CR-only line endings ──────────────────────────────────

#[test]
fn line_starts_cache_cr_only_position_round_trip() {
    let src = "abc\rdef\rghi";
    let cache = LineStartsCache::new(src);
    // Line starts: 0, 4 (after \r), 8 (after \r)
    assert_eq!(cache.offset_to_position(src, 4), (1, 0));
    assert_eq!(cache.offset_to_position(src, 8), (2, 0));
    // Round-trip
    assert_eq!(cache.position_to_offset(src, 1, 0), 4);
    assert_eq!(cache.position_to_offset(src, 2, 0), 8);
}

#[test]
fn line_starts_cache_position_to_offset_strips_cr_from_line_end() {
    // The inner CRLF-strip loop: ensure CR at line end is stripped when computing
    // the line extent for position_to_offset.
    let src = "ab\rcd";
    let cache = LineStartsCache::new(src);
    // Character 0 on line 1 should be at byte 3 ('c')
    let off = cache.position_to_offset(src, 1, 0);
    assert_eq!(off, 3);
    // Character 1 on line 1 should be at byte 4 ('d')
    let off2 = cache.position_to_offset(src, 1, 1);
    assert_eq!(off2, 4);
}

// ─── LineIndex: position_to_offset last-line path (no trailing newline) ─────

#[test]
fn line_index_position_to_offset_last_line_no_newline() {
    let idx = LineIndex::new("abc".to_string());
    // Single line, no newline: last-line path in position_to_offset
    assert_eq!(idx.position_to_offset(0, 2), Some(2));
    assert_eq!(idx.position_to_offset(0, 3), Some(3)); // end of string
}

#[test]
fn line_index_position_to_offset_crlf_text() {
    let idx = LineIndex::new("ab\r\ncd".to_string());
    // 'c' is at byte 4 (a=0, b=1, \r=2, \n=3, c=4)
    assert_eq!(idx.position_to_offset(1, 0), Some(4));
    assert_eq!(idx.position_to_offset(1, 1), Some(5));
}

// ─── PositionMapper: byte_to_lsp_pos with mid-char byte offset ──────────────

#[test]
fn mapper_byte_to_lsp_pos_mid_multibyte_char_snaps_to_char_start() {
    // 'a' = 1 byte, '😀' = 4 bytes, 'b' = 1 byte
    // Bytes: a=0, 😀=1..5, b=5
    let mapper = PositionMapper::new("a😀b");
    // Mid-char byte offsets 2, 3, 4 all fall inside 😀
    // The branch `current_byte + ch_len > byte_in_line` triggers and breaks,
    // yielding column 1 (past 'a', before emoji).
    let pos2 = mapper.byte_to_lsp_pos(2);
    let pos3 = mapper.byte_to_lsp_pos(3);
    let pos4 = mapper.byte_to_lsp_pos(4);
    assert_eq!(pos2.line, 0);
    assert_eq!(pos2.character, 1);
    assert_eq!(pos3.line, 0);
    assert_eq!(pos3.character, 1);
    assert_eq!(pos4.line, 0);
    assert_eq!(pos4.character, 1);
}

// ─── PositionMapper: lsp_pos_to_byte with line beyond rope length ────────────

#[test]
fn mapper_lsp_pos_to_byte_line_beyond_end_returns_none() {
    let mapper = PositionMapper::new("hello");
    assert!(mapper.lsp_pos_to_byte(WirePosition { line: 99, character: 0 }).is_none());
}

// ─── PositionMapper: slice clamping ─────────────────────────────────────────

#[test]
fn mapper_slice_empty_range_returns_empty() {
    let mapper = PositionMapper::new("hello world");
    let s = mapper.slice(3, 3);
    assert!(s.is_empty());
}

#[test]
fn mapper_slice_past_end_clamps() {
    let mapper = PositionMapper::new("abc");
    // Both start and end clamp to len_bytes (3)
    let s = mapper.slice(5, 5);
    assert!(s.is_empty());
}

// ─── WirePosition: new constructor ──────────────────────────────────────────

#[test]
fn wire_position_new_sets_fields() {
    let wp = WirePosition::new(3, 7);
    assert_eq!(wp.line, 3);
    assert_eq!(wp.character, 7);
}

#[test]
fn wire_position_default_is_origin() {
    let wp = WirePosition::default();
    assert_eq!(wp.line, 0);
    assert_eq!(wp.character, 0);
}

// ─── WireRange: whole_document with non-empty text ──────────────────────────

#[test]
fn wire_range_whole_document_multiline_text() {
    let src = "abc\ndef";
    let wr = WireRange::whole_document(src);
    assert_eq!(wr.start.line, 0);
    assert_eq!(wr.start.character, 0);
    assert_eq!(wr.end.line, 1);
    assert_eq!(wr.end.character, 3); // "def" has 3 chars
}

// ─── convert: offset_to_utf16_line_col with empty text ──────────────────────

#[test]
fn offset_to_utf16_line_col_empty_text_returns_origin() {
    let (line, col) = perl_position_tracking::offset_to_utf16_line_col("", 0);
    assert_eq!((line, col), (0, 0));
}

#[test]
fn offset_to_utf16_line_col_beyond_empty_text_clamps() {
    let (line, col) = perl_position_tracking::offset_to_utf16_line_col("", 99);
    assert_eq!((line, col), (0, 0));
}

// ─── convert: utf16_line_col_to_offset col=0 fast path ─────────────────────

#[test]
fn utf16_line_col_to_offset_col_zero_returns_line_start() {
    let text = "hello\nworld\nfoo";
    assert_eq!(perl_position_tracking::utf16_line_col_to_offset(text, 0, 0), 0);
    assert_eq!(perl_position_tracking::utf16_line_col_to_offset(text, 1, 0), 6);
    assert_eq!(perl_position_tracking::utf16_line_col_to_offset(text, 2, 0), 12);
}

// ─── LineStartsCache: new_rope with single-line and multi-line ropes ─────────

#[test]
fn line_starts_cache_new_rope_single_line() {
    let rope = ropey::Rope::from_str("hello");
    let cache = LineStartsCache::new_rope(&rope);
    let (line, col) = cache.offset_to_position_rope(&rope, 3);
    assert_eq!(line, 0);
    assert_eq!(col, 3);
}

#[test]
fn line_starts_cache_new_rope_multi_line() {
    let rope = ropey::Rope::from_str("abc\ndef\n");
    let cache = LineStartsCache::new_rope(&rope);
    // 'd' is at byte 4
    let (line, col) = cache.offset_to_position_rope(&rope, 4);
    assert_eq!(line, 1);
    assert_eq!(col, 0);
}
