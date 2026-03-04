//! Comprehensive unit tests for perl-position-tracking.
//!
//! Covers: ByteSpan, LineStartsCache, LineIndex, PositionMapper, WirePosition,
//! WireRange, WireLocation, Position, Range, convert functions, and mapper utilities.

use perl_position_tracking::{
    ByteSpan, LineEnding, LineIndex, LineStartsCache, PositionMapper, SourceLocation, WireLocation,
    WirePosition, WireRange,
};
use perl_tdd_support::must_some;

// ─── ByteSpan ────────────────────────────────────────────────────────────────

#[test]
fn byte_span_new_and_accessors() {
    let span = ByteSpan::new(3, 7);
    assert_eq!(span.start, 3);
    assert_eq!(span.end, 7);
    assert_eq!(span.len(), 4);
    assert!(!span.is_empty());
}

#[test]
fn byte_span_empty_at_zero() {
    let span = ByteSpan::empty(0);
    assert!(span.is_empty());
    assert_eq!(span.len(), 0);
    assert_eq!(span.start, 0);
    assert_eq!(span.end, 0);
}

#[test]
fn byte_span_empty_at_nonzero() {
    let span = ByteSpan::empty(42);
    assert!(span.is_empty());
    assert_eq!(span.start, 42);
}

#[test]
fn byte_span_whole() {
    let src = "hello world";
    let span = ByteSpan::whole(src);
    assert_eq!(span.start, 0);
    assert_eq!(span.end, 11);
    assert_eq!(span.slice(src), "hello world");
}

#[test]
fn byte_span_contains_boundary() {
    let span = ByteSpan::new(5, 10);
    assert!(!span.contains(4));
    assert!(span.contains(5)); // inclusive start
    assert!(span.contains(9));
    assert!(!span.contains(10)); // exclusive end
}

#[test]
fn byte_span_contains_span_exact() {
    let outer = ByteSpan::new(5, 10);
    assert!(outer.contains_span(ByteSpan::new(5, 10))); // exact match
    assert!(outer.contains_span(ByteSpan::new(5, 5))); // empty at start
    assert!(outer.contains_span(ByteSpan::new(10, 10))); // empty at end
}

#[test]
fn byte_span_overlaps_adjacent_is_false() {
    let a = ByteSpan::new(0, 5);
    let b = ByteSpan::new(5, 10);
    assert!(!a.overlaps(b));
    assert!(!b.overlaps(a));
}

#[test]
fn byte_span_overlaps_one_byte() {
    let a = ByteSpan::new(0, 6);
    let b = ByteSpan::new(5, 10);
    assert!(a.overlaps(b));
    assert!(b.overlaps(a));
}

#[test]
fn byte_span_intersection_none() {
    let a = ByteSpan::new(0, 5);
    let b = ByteSpan::new(5, 10);
    assert_eq!(a.intersection(b), None);
}

#[test]
fn byte_span_intersection_partial() {
    let a = ByteSpan::new(0, 8);
    let b = ByteSpan::new(3, 12);
    assert_eq!(a.intersection(b), Some(ByteSpan::new(3, 8)));
}

#[test]
fn byte_span_union_disjoint() {
    let a = ByteSpan::new(0, 5);
    let b = ByteSpan::new(10, 15);
    assert_eq!(a.union(b), ByteSpan::new(0, 15));
}

#[test]
fn byte_span_slice_and_try_slice() {
    let src = "abcdefghij";
    let span = ByteSpan::new(2, 5);
    assert_eq!(span.slice(src), "cde");
    assert_eq!(span.try_slice(src), Some("cde"));

    let oob = ByteSpan::new(0, 100);
    assert_eq!(oob.try_slice(src), None);
}

#[test]
fn byte_span_to_range() {
    let span = ByteSpan::new(1, 4);
    let r: std::ops::Range<usize> = span.to_range();
    assert_eq!(r, 1..4);
}

#[test]
fn byte_span_from_range() {
    let span: ByteSpan = (3..9).into();
    assert_eq!(span.start, 3);
    assert_eq!(span.end, 9);
}

#[test]
fn byte_span_from_tuple_roundtrip() {
    let span = ByteSpan::new(7, 14);
    let t: (usize, usize) = span.into();
    assert_eq!(t, (7, 14));
    let back: ByteSpan = t.into();
    assert_eq!(back, span);
}

#[test]
fn byte_span_display() {
    assert_eq!(format!("{}", ByteSpan::new(0, 42)), "0..42");
}

#[test]
fn byte_span_default_is_empty_at_zero() {
    let span = ByteSpan::default();
    assert_eq!(span.start, 0);
    assert_eq!(span.end, 0);
    assert!(span.is_empty());
}

#[test]
fn source_location_alias() {
    let loc: SourceLocation = ByteSpan::new(1, 2);
    assert_eq!(loc.start, 1);
    assert_eq!(loc.end, 2);
}

// ─── Position (engine type) ──────────────────────────────────────────────────

#[test]
fn position_start() {
    let pos = perl_position_tracking::Position::start();
    assert_eq!(pos.byte, 0);
    assert_eq!(pos.line, 1);
    assert_eq!(pos.column, 1);
}

#[test]
fn position_advance_ascii() {
    let mut pos = perl_position_tracking::Position::start();
    pos.advance("abc");
    assert_eq!(pos.byte, 3);
    assert_eq!(pos.line, 1);
    assert_eq!(pos.column, 4);
}

#[test]
fn position_advance_with_newlines() {
    let mut pos = perl_position_tracking::Position::start();
    pos.advance("ab\ncd\nef");
    assert_eq!(pos.byte, 8);
    assert_eq!(pos.line, 3);
    assert_eq!(pos.column, 3);
}

#[test]
fn position_advance_char_newline() {
    let mut pos = perl_position_tracking::Position::start();
    pos.advance_char('x');
    assert_eq!(pos.column, 2);
    pos.advance_char('\n');
    assert_eq!(pos.line, 2);
    assert_eq!(pos.column, 1);
}

#[test]
fn position_advance_multibyte_utf8() {
    let mut pos = perl_position_tracking::Position::start();
    // é is 2 bytes, 世 is 3 bytes, 😀 is 4 bytes
    pos.advance("é世😀");
    assert_eq!(pos.byte, 2 + 3 + 4);
    assert_eq!(pos.column, 4); // 3 chars + initial column 1
}

#[test]
fn position_display() {
    let pos = perl_position_tracking::Position::new(10, 3, 7);
    assert_eq!(format!("{pos}"), "3:7");
}

// ─── Range (engine type) ─────────────────────────────────────────────────────

#[test]
fn range_empty() {
    let pos = perl_position_tracking::Position::new(5, 1, 6);
    let r = perl_position_tracking::Range::empty(pos);
    assert!(r.is_empty());
    assert_eq!(r.len(), 0);
}

#[test]
fn range_contains_byte() {
    let r = perl_position_tracking::Range::new(
        perl_position_tracking::Position::new(10, 2, 1),
        perl_position_tracking::Position::new(20, 3, 1),
    );
    assert!(r.contains_byte(10));
    assert!(r.contains_byte(19));
    assert!(!r.contains_byte(20));
    assert!(!r.contains_byte(9));
}

#[test]
fn range_contains_position() {
    let r = perl_position_tracking::Range::new(
        perl_position_tracking::Position::new(10, 2, 1),
        perl_position_tracking::Position::new(20, 3, 1),
    );
    let inside = perl_position_tracking::Position::new(15, 2, 6);
    let outside = perl_position_tracking::Position::new(25, 4, 1);
    assert!(r.contains(inside));
    assert!(!r.contains(outside));
}

#[test]
fn range_overlaps_symmetric() {
    let a = perl_position_tracking::Range::new(
        perl_position_tracking::Position::new(0, 1, 1),
        perl_position_tracking::Position::new(10, 1, 11),
    );
    let b = perl_position_tracking::Range::new(
        perl_position_tracking::Position::new(5, 1, 6),
        perl_position_tracking::Position::new(15, 2, 5),
    );
    assert!(a.overlaps(&b));
    assert!(b.overlaps(&a));
}

#[test]
fn range_no_overlap_adjacent() {
    let a = perl_position_tracking::Range::new(
        perl_position_tracking::Position::new(0, 1, 1),
        perl_position_tracking::Position::new(10, 1, 11),
    );
    let b = perl_position_tracking::Range::new(
        perl_position_tracking::Position::new(10, 2, 1),
        perl_position_tracking::Position::new(20, 2, 11),
    );
    assert!(!a.overlaps(&b));
}

#[test]
fn range_extend() {
    let mut r = perl_position_tracking::Range::new(
        perl_position_tracking::Position::new(5, 1, 6),
        perl_position_tracking::Position::new(10, 1, 11),
    );
    let other = perl_position_tracking::Range::new(
        perl_position_tracking::Position::new(2, 1, 3),
        perl_position_tracking::Position::new(15, 2, 5),
    );
    r.extend(&other);
    assert_eq!(r.start.byte, 2);
    assert_eq!(r.end.byte, 15);
}

#[test]
fn range_span_to() {
    let a = perl_position_tracking::Range::new(
        perl_position_tracking::Position::new(10, 2, 1),
        perl_position_tracking::Position::new(20, 3, 1),
    );
    let b = perl_position_tracking::Range::new(
        perl_position_tracking::Position::new(5, 1, 6),
        perl_position_tracking::Position::new(30, 4, 1),
    );
    let s = a.span_to(&b);
    assert_eq!(s.start.byte, 5);
    assert_eq!(s.end.byte, 30);
}

#[test]
fn range_display() {
    let r = perl_position_tracking::Range::new(
        perl_position_tracking::Position::new(0, 1, 1),
        perl_position_tracking::Position::new(10, 2, 5),
    );
    assert_eq!(format!("{r}"), "1:1-2:5");
}

#[test]
fn range_from_source_location() {
    let loc = SourceLocation::new(3, 7);
    let r: perl_position_tracking::Range = loc.into();
    assert_eq!(r.start.byte, 3);
    assert_eq!(r.end.byte, 7);
}

// ─── LineStartsCache ─────────────────────────────────────────────────────────

#[test]
fn line_starts_cache_single_line() {
    let src = "hello world";
    let cache = LineStartsCache::new(src);
    let (line, col) = cache.offset_to_position(src, 0);
    assert_eq!(line, 0);
    assert_eq!(col, 0);
    let (line, col) = cache.offset_to_position(src, 5);
    assert_eq!(line, 0);
    assert_eq!(col, 5);
}

#[test]
fn line_starts_cache_multi_line_lf() {
    let src = "abc\ndef\nghi";
    let cache = LineStartsCache::new(src);

    assert_eq!(cache.offset_to_position(src, 0), (0, 0));
    assert_eq!(cache.offset_to_position(src, 3), (0, 3));
    assert_eq!(cache.offset_to_position(src, 4), (1, 0));
    assert_eq!(cache.offset_to_position(src, 8), (2, 0));
}

#[test]
fn line_starts_cache_crlf() {
    let src = "ab\r\ncd\r\nef";
    let cache = LineStartsCache::new(src);

    assert_eq!(cache.offset_to_position(src, 0), (0, 0));
    assert_eq!(cache.offset_to_position(src, 4), (1, 0));
    assert_eq!(cache.offset_to_position(src, 8), (2, 0));
}

#[test]
fn line_starts_cache_cr_only() {
    let src = "ab\rcd\ref";
    let cache = LineStartsCache::new(src);

    assert_eq!(cache.offset_to_position(src, 0), (0, 0));
    assert_eq!(cache.offset_to_position(src, 3), (1, 0));
    assert_eq!(cache.offset_to_position(src, 6), (2, 0));
}

#[test]
fn line_starts_cache_offset_beyond_end_clamps() {
    let src = "short";
    let cache = LineStartsCache::new(src);
    // Offset beyond end should be clamped
    let (line, col) = cache.offset_to_position(src, 999);
    assert_eq!(line, 0);
    assert_eq!(col, 5);
}

#[test]
fn line_starts_cache_utf16_column_bmp() {
    // BMP characters: 1 UTF-16 code unit each
    let src = "aéb"; // é is U+00E9 (1 UTF-16 code unit, 2 UTF-8 bytes)
    let cache = LineStartsCache::new(src);

    // After 'a': col 1 (UTF-16)
    assert_eq!(cache.offset_to_position(src, 1), (0, 1));
    // After 'é' (byte offset 3): col 2 (UTF-16)
    assert_eq!(cache.offset_to_position(src, 3), (0, 2));
    // After 'b' (byte offset 4): col 3 (UTF-16)
    assert_eq!(cache.offset_to_position(src, 4), (0, 3));
}

#[test]
fn line_starts_cache_utf16_column_supplementary() {
    // 😀 is U+1F600 — 4 UTF-8 bytes, 2 UTF-16 code units
    let src = "a😀b";
    let cache = LineStartsCache::new(src);

    assert_eq!(cache.offset_to_position(src, 0), (0, 0)); // before 'a'
    assert_eq!(cache.offset_to_position(src, 1), (0, 1)); // before 😀
    assert_eq!(cache.offset_to_position(src, 5), (0, 3)); // before 'b' (1 + 2)
    assert_eq!(cache.offset_to_position(src, 6), (0, 4)); // after 'b'
}

#[test]
fn line_starts_cache_position_to_offset_roundtrip() {
    let src = "hello\nworld\n!";
    let cache = LineStartsCache::new(src);

    for byte in 0..src.len() {
        if !src.is_char_boundary(byte) {
            continue;
        }
        let (line, col) = cache.offset_to_position(src, byte);
        let back = cache.position_to_offset(src, line, col);
        assert_eq!(back, byte, "roundtrip failed for byte {byte}: line={line}, col={col}");
    }
}

#[test]
fn line_starts_cache_position_to_offset_past_last_line() {
    let src = "abc\ndef";
    let cache = LineStartsCache::new(src);
    // Line index beyond available lines returns text.len()
    let off = cache.position_to_offset(src, 99, 0);
    assert_eq!(off, src.len());
}

#[test]
fn line_starts_cache_position_to_offset_utf16_emoji() {
    // 😀 is 2 UTF-16 code units
    let src = "a😀b\ncd";
    let cache = LineStartsCache::new(src);

    // Column 0 on line 0 → byte 0
    assert_eq!(cache.position_to_offset(src, 0, 0), 0);
    // Column 1 (after 'a') → byte 1
    assert_eq!(cache.position_to_offset(src, 0, 1), 1);
    // Column 3 (after emoji, 1 + 2 UTF-16 units) → byte 5
    assert_eq!(cache.position_to_offset(src, 0, 3), 5);
}

// ─── LineIndex ───────────────────────────────────────────────────────────────

#[test]
fn line_index_single_line() {
    let idx = LineIndex::new("hello".to_string());
    assert_eq!(idx.offset_to_position(0), (0, 0));
    assert_eq!(idx.offset_to_position(5), (0, 5));
}

#[test]
fn line_index_multi_line() {
    let idx = LineIndex::new("abc\ndef\nghi".to_string());
    assert_eq!(idx.offset_to_position(0), (0, 0));
    assert_eq!(idx.offset_to_position(4), (1, 0));
    assert_eq!(idx.offset_to_position(6), (1, 2));
    assert_eq!(idx.offset_to_position(8), (2, 0));
}

#[test]
fn line_index_position_to_offset_valid() {
    let idx = LineIndex::new("abc\ndef\nghi".to_string());
    assert_eq!(idx.position_to_offset(0, 0), Some(0));
    assert_eq!(idx.position_to_offset(1, 0), Some(4));
    assert_eq!(idx.position_to_offset(2, 2), Some(10));
}

#[test]
fn line_index_position_to_offset_out_of_range() {
    let idx = LineIndex::new("abc\ndef".to_string());
    assert_eq!(idx.position_to_offset(99, 0), None);
}

#[test]
fn line_index_utf16_roundtrip() {
    // 😀: 4 UTF-8 bytes, 2 UTF-16 code units
    let idx = LineIndex::new("a😀b".to_string());

    // offset 0 → (0, 0)
    assert_eq!(idx.offset_to_position(0), (0, 0));
    // offset 1 (before emoji) → (0, 1)
    assert_eq!(idx.offset_to_position(1), (0, 1));
    // offset 5 (after emoji) → (0, 3)  [1 + 2 UTF-16 units]
    assert_eq!(idx.offset_to_position(5), (0, 3));

    // Reverse: (0, 3) → offset 5
    assert_eq!(idx.position_to_offset(0, 3), Some(5));
}

#[test]
fn line_index_utf16_mid_character_returns_none() {
    // 😀 occupies 2 UTF-16 code units, so offset in the middle is invalid
    let idx = LineIndex::new("😀".to_string());
    // UTF-16 offset 1 is in the middle of the surrogate pair
    assert_eq!(idx.position_to_offset(0, 1), None);
}

#[test]
fn line_index_range() {
    let idx = LineIndex::new("abc\ndef".to_string());
    let (start, end) = idx.range(0, 7);
    assert_eq!(start, (0, 0));
    assert_eq!(end, (1, 3));
}

// ─── convert functions ───────────────────────────────────────────────────────

#[test]
fn offset_to_utf16_line_col_ascii() {
    let text = "abc\ndef\nghi";
    assert_eq!(perl_position_tracking::offset_to_utf16_line_col(text, 0), (0, 0));
    assert_eq!(perl_position_tracking::offset_to_utf16_line_col(text, 4), (1, 0));
    assert_eq!(perl_position_tracking::offset_to_utf16_line_col(text, 6), (1, 2));
}

#[test]
fn offset_to_utf16_line_col_beyond_end() {
    let text = "abc";
    let (line, col) = perl_position_tracking::offset_to_utf16_line_col(text, 999);
    assert_eq!(line, 0);
    assert_eq!(col, 3);
}

#[test]
fn offset_to_utf16_line_col_at_trailing_newline() {
    let text = "abc\n";
    let (line, col) = perl_position_tracking::offset_to_utf16_line_col(text, text.len());
    assert_eq!(line, 1);
    assert_eq!(col, 0);
}

#[test]
fn offset_to_utf16_line_col_emoji() {
    // 😀 is 4 UTF-8 bytes, 2 UTF-16 code units
    let text = "a😀b";
    // After 'a': offset 1 → col 1
    assert_eq!(perl_position_tracking::offset_to_utf16_line_col(text, 1), (0, 1));
    // After emoji: offset 5 → col 3 (1 + 2)
    assert_eq!(perl_position_tracking::offset_to_utf16_line_col(text, 5), (0, 3));
}

#[test]
fn utf16_line_col_to_offset_ascii() {
    let text = "abc\ndef\nghi";
    assert_eq!(perl_position_tracking::utf16_line_col_to_offset(text, 0, 0), 0);
    assert_eq!(perl_position_tracking::utf16_line_col_to_offset(text, 1, 0), 4);
    assert_eq!(perl_position_tracking::utf16_line_col_to_offset(text, 1, 2), 6);
}

#[test]
fn utf16_line_col_to_offset_past_end() {
    let text = "abc";
    assert_eq!(perl_position_tracking::utf16_line_col_to_offset(text, 99, 0), text.len());
}

#[test]
fn utf16_line_col_to_offset_emoji() {
    let text = "a😀b";
    // Col 3 (1 + 2 UTF-16 units for emoji) → byte 5
    assert_eq!(perl_position_tracking::utf16_line_col_to_offset(text, 0, 3), 5);
}

#[test]
fn convert_roundtrip_every_byte() {
    let text = "hello\n世界\n😀!";
    for byte in 0..text.len() {
        if !text.is_char_boundary(byte) {
            continue;
        }
        let (line, col) = perl_position_tracking::offset_to_utf16_line_col(text, byte);
        let back = perl_position_tracking::utf16_line_col_to_offset(text, line, col);
        assert_eq!(back, byte, "roundtrip failed at byte {byte}: line={line}, col={col}");
    }
}

// ─── WirePosition / WireRange / WireLocation ────────────────────────────────

#[test]
fn wire_position_new() {
    let wp = WirePosition::new(3, 7);
    assert_eq!(wp.line, 3);
    assert_eq!(wp.character, 7);
}

#[test]
fn wire_position_default() {
    let wp = WirePosition::default();
    assert_eq!(wp.line, 0);
    assert_eq!(wp.character, 0);
}

#[test]
fn wire_position_from_byte_offset_ascii() {
    let src = "abc\ndef";
    let wp = WirePosition::from_byte_offset(src, 4);
    assert_eq!(wp.line, 1);
    assert_eq!(wp.character, 0);
}

#[test]
fn wire_position_to_byte_offset() {
    let src = "abc\ndef";
    let wp = WirePosition::new(1, 2);
    assert_eq!(wp.to_byte_offset(src), 6);
}

#[test]
fn wire_position_roundtrip_emoji() {
    let src = "a😀b\ncd";
    let wp = WirePosition::from_byte_offset(src, 5); // after emoji
    assert_eq!(wp.line, 0);
    assert_eq!(wp.character, 3); // 1 + 2 UTF-16 units
    assert_eq!(wp.to_byte_offset(src), 5);
}

#[test]
fn wire_range_new() {
    let wr = WireRange::new(WirePosition::new(1, 0), WirePosition::new(2, 5));
    assert_eq!(wr.start.line, 1);
    assert_eq!(wr.end.character, 5);
}

#[test]
fn wire_range_empty() {
    let pos = WirePosition::new(3, 7);
    let wr = WireRange::empty(pos);
    assert_eq!(wr.start, wr.end);
}

#[test]
fn wire_range_from_byte_offsets() {
    let src = "abc\ndef\nghi";
    let wr = WireRange::from_byte_offsets(src, 0, 7);
    assert_eq!(wr.start, WirePosition::new(0, 0));
    assert_eq!(wr.end, WirePosition::new(1, 3));
}

#[test]
fn wire_range_whole_document() {
    let src = "abc\ndef";
    let wr = WireRange::whole_document(src);
    assert_eq!(wr.start, WirePosition::new(0, 0));
    // End of document
    let end = WirePosition::from_byte_offset(src, src.len());
    assert_eq!(wr.end, end);
}

#[test]
fn wire_range_default() {
    let wr = WireRange::default();
    assert_eq!(wr.start, WirePosition::default());
    assert_eq!(wr.end, WirePosition::default());
}

#[test]
fn wire_location_new() {
    let loc = WireLocation::new(
        "file:///test.pl".to_string(),
        WireRange::new(WirePosition::new(0, 0), WirePosition::new(0, 5)),
    );
    assert_eq!(loc.uri, "file:///test.pl");
    assert_eq!(loc.range.start.line, 0);
}

// ─── PositionMapper ──────────────────────────────────────────────────────────

#[test]
fn mapper_byte_to_lsp_pos_ascii() {
    let m = PositionMapper::new("abc\ndef\nghi");
    assert_eq!(m.byte_to_lsp_pos(0), WirePosition::new(0, 0));
    assert_eq!(m.byte_to_lsp_pos(3), WirePosition::new(0, 3));
    assert_eq!(m.byte_to_lsp_pos(4), WirePosition::new(1, 0));
    assert_eq!(m.byte_to_lsp_pos(8), WirePosition::new(2, 0));
}

#[test]
fn mapper_lsp_pos_to_byte_ascii() {
    let m = PositionMapper::new("abc\ndef\nghi");
    assert_eq!(m.lsp_pos_to_byte(WirePosition::new(0, 0)), Some(0));
    assert_eq!(m.lsp_pos_to_byte(WirePosition::new(1, 0)), Some(4));
    assert_eq!(m.lsp_pos_to_byte(WirePosition::new(2, 2)), Some(10));
}

#[test]
fn mapper_lsp_pos_to_byte_invalid_line() {
    let m = PositionMapper::new("hello");
    assert_eq!(m.lsp_pos_to_byte(WirePosition::new(99, 0)), None);
}

#[test]
fn mapper_utf16_emoji_roundtrip() {
    // 😀 is 4 UTF-8 bytes, 2 UTF-16 code units
    let m = PositionMapper::new("a😀b");

    // byte 1 (before emoji) → (0, 1)
    assert_eq!(m.byte_to_lsp_pos(1), WirePosition::new(0, 1));
    // byte 5 (after emoji) → (0, 3) [1 + 2 UTF-16]
    assert_eq!(m.byte_to_lsp_pos(5), WirePosition::new(0, 3));

    // Reverse
    assert_eq!(m.lsp_pos_to_byte(WirePosition::new(0, 1)), Some(1));
    assert_eq!(m.lsp_pos_to_byte(WirePosition::new(0, 3)), Some(5));
}

#[test]
fn mapper_utf16_surrogate_pair_chars() {
    // 𝄞 (U+1D11E, MUSICAL SYMBOL G CLEF) — 4 UTF-8 bytes, 2 UTF-16 code units
    let m = PositionMapper::new("x𝄞y");
    assert_eq!(m.byte_to_lsp_pos(1), WirePosition::new(0, 1)); // before 𝄞
    assert_eq!(m.byte_to_lsp_pos(5), WirePosition::new(0, 3)); // after 𝄞 (1+2)
    assert_eq!(m.byte_to_lsp_pos(6), WirePosition::new(0, 4)); // after y
}

#[test]
fn mapper_crlf_line_ending_detection() {
    let m = PositionMapper::new("a\r\nb\r\nc");
    assert_eq!(m.line_ending(), LineEnding::CrLf);
    assert_eq!(m.byte_to_lsp_pos(3), WirePosition::new(1, 0));
    assert_eq!(m.byte_to_lsp_pos(6), WirePosition::new(2, 0));
}

#[test]
fn mapper_lf_line_ending_detection() {
    let m = PositionMapper::new("a\nb");
    assert_eq!(m.line_ending(), LineEnding::Lf);
}

#[test]
fn mapper_cr_only_line_ending_detection() {
    let m = PositionMapper::new("a\rb");
    assert_eq!(m.line_ending(), LineEnding::Cr);
}

#[test]
fn mapper_mixed_line_ending_detection() {
    let m = PositionMapper::new("a\r\nb\nc\rd");
    assert_eq!(m.line_ending(), LineEnding::Mixed);
}

#[test]
fn mapper_no_newlines_defaults_to_lf() {
    let m = PositionMapper::new("hello");
    assert_eq!(m.line_ending(), LineEnding::Lf);
}

#[test]
fn mapper_empty_text() {
    let m = PositionMapper::new("");
    assert!(m.is_empty());
    assert_eq!(m.len_bytes(), 0);
    assert_eq!(m.byte_to_lsp_pos(0), WirePosition::new(0, 0));
}

#[test]
fn mapper_text_and_slice() {
    let m = PositionMapper::new("hello world");
    assert_eq!(m.text(), "hello world");
    assert_eq!(m.slice(0, 5), "hello");
    assert_eq!(m.slice(6, 11), "world");
}

#[test]
fn mapper_len_bytes_and_lines() {
    let m = PositionMapper::new("ab\ncd\nef");
    assert_eq!(m.len_bytes(), 8);
    assert_eq!(m.len_lines(), 3);
}

#[test]
fn mapper_apply_edit_replace() {
    let mut m = PositionMapper::new("hello world");
    m.apply_edit(6, 11, "Rust");
    assert_eq!(m.text(), "hello Rust");
}

#[test]
fn mapper_apply_edit_insert() {
    let mut m = PositionMapper::new("ac");
    m.apply_edit(1, 1, "b");
    assert_eq!(m.text(), "abc");
}

#[test]
fn mapper_apply_edit_delete() {
    let mut m = PositionMapper::new("abcd");
    m.apply_edit(1, 3, "");
    assert_eq!(m.text(), "ad");
}

#[test]
fn mapper_apply_edit_clamps_beyond_end() {
    let mut m = PositionMapper::new("hi");
    m.apply_edit(0, 999, "bye");
    assert_eq!(m.text(), "bye");
}

#[test]
fn mapper_update() {
    let mut m = PositionMapper::new("old");
    m.update("new text\nhere");
    assert_eq!(m.text(), "new text\nhere");
    assert_eq!(m.len_lines(), 2);
}

#[test]
fn mapper_lsp_pos_to_char() {
    let m = PositionMapper::new("abc\ndef");
    let ch = must_some(m.lsp_pos_to_char(WirePosition::new(1, 1)));
    // Line 1 starts at char 4 ('d'), char index 5 = 'e'
    assert_eq!(ch, 5);
}

#[test]
fn mapper_char_to_lsp_pos() {
    let m = PositionMapper::new("abc\ndef");
    // char 5 is 'e' on line 1, col 1
    assert_eq!(m.char_to_lsp_pos(5), WirePosition::new(1, 1));
}

#[test]
fn mapper_byte_to_lsp_pos_clamps_beyond_end() {
    let m = PositionMapper::new("abc");
    // Beyond end should clamp
    let pos = m.byte_to_lsp_pos(999);
    assert_eq!(pos, WirePosition::new(0, 3));
}

// ─── mapper utilities ────────────────────────────────────────────────────────

#[test]
fn json_to_position_valid() {
    let json = serde_json::json!({"line": 5, "character": 10});
    let pos = must_some(perl_position_tracking::json_to_position(&json));
    assert_eq!(pos.line, 5);
    assert_eq!(pos.character, 10);
}

#[test]
fn json_to_position_missing_field() {
    let json = serde_json::json!({"line": 5});
    assert!(perl_position_tracking::json_to_position(&json).is_none());
}

#[test]
fn position_to_json_roundtrip() {
    let pos = WirePosition::new(3, 7);
    let json = perl_position_tracking::position_to_json(pos);
    let back = must_some(perl_position_tracking::json_to_position(&json));
    assert_eq!(back, pos);
}

#[test]
fn apply_edit_utf8_basic() {
    let mut text = "hello world".to_string();
    perl_position_tracking::apply_edit_utf8(&mut text, 5, 11, " Rust");
    assert_eq!(text, "hello Rust");
}

#[test]
fn apply_edit_utf8_insert() {
    let mut text = "ac".to_string();
    perl_position_tracking::apply_edit_utf8(&mut text, 1, 1, "b");
    assert_eq!(text, "abc");
}

#[test]
fn apply_edit_utf8_delete() {
    let mut text = "abcdef".to_string();
    perl_position_tracking::apply_edit_utf8(&mut text, 2, 4, "");
    assert_eq!(text, "abef");
}

#[test]
fn apply_edit_utf8_non_char_boundary_is_noop() {
    // 😀 is 4 bytes: attempting to edit mid-character should be a no-op
    let mut text = "a😀b".to_string();
    let original = text.clone();
    perl_position_tracking::apply_edit_utf8(&mut text, 2, 3, "x");
    assert_eq!(text, original);
}

#[test]
fn newline_count_no_newlines() {
    assert_eq!(perl_position_tracking::newline_count("hello"), 0);
}

#[test]
fn newline_count_multiple() {
    assert_eq!(perl_position_tracking::newline_count("a\nb\nc\n"), 3);
}

#[test]
fn newline_count_empty() {
    assert_eq!(perl_position_tracking::newline_count(""), 0);
}

#[test]
fn last_line_column_utf8_no_newline() {
    assert_eq!(perl_position_tracking::last_line_column_utf8("hello"), 5);
}

#[test]
fn last_line_column_utf8_trailing_newline() {
    assert_eq!(perl_position_tracking::last_line_column_utf8("abc\n"), 0);
}

#[test]
fn last_line_column_utf8_multi_line() {
    assert_eq!(perl_position_tracking::last_line_column_utf8("abc\ndef"), 3);
}

#[test]
fn last_line_column_utf8_empty() {
    assert_eq!(perl_position_tracking::last_line_column_utf8(""), 0);
}

// ─── Unicode edge cases ─────────────────────────────────────────────────────

#[test]
fn unicode_combining_characters() {
    // é can be represented as e + combining acute (2 chars, 3 bytes)
    // but as a precomposed character it's 1 char, 2 bytes (U+00E9)
    let src = "e\u{0301}x"; // e + combining acute + x
    let cache = LineStartsCache::new(src);
    // e is 1 byte, combining acute is 2 bytes, x is 1 byte → 4 bytes total
    assert_eq!(src.len(), 4);
    // UTF-16: e=1 unit, combining=1 unit, x=1 unit → total 3
    assert_eq!(cache.offset_to_position(src, 4), (0, 3));
}

#[test]
fn unicode_cjk_characters() {
    // CJK characters are in BMP: 3 UTF-8 bytes, 1 UTF-16 code unit each
    let src = "日本語";
    let cache = LineStartsCache::new(src);
    assert_eq!(cache.offset_to_position(src, 0), (0, 0));
    assert_eq!(cache.offset_to_position(src, 3), (0, 1));
    assert_eq!(cache.offset_to_position(src, 6), (0, 2));
    assert_eq!(cache.offset_to_position(src, 9), (0, 3));
}

#[test]
fn unicode_mixed_bmp_and_supplementary() {
    // Mix of BMP and supplementary plane characters
    let src = "a世😀b";
    let m = PositionMapper::new(src);

    // a: byte 0, UTF-16 col 0
    assert_eq!(m.byte_to_lsp_pos(0), WirePosition::new(0, 0));
    // 世: byte 1, UTF-16 col 1
    assert_eq!(m.byte_to_lsp_pos(1), WirePosition::new(0, 1));
    // 😀: byte 4, UTF-16 col 2
    assert_eq!(m.byte_to_lsp_pos(4), WirePosition::new(0, 2));
    // b: byte 8, UTF-16 col 4 (2 + 2 for emoji)
    assert_eq!(m.byte_to_lsp_pos(8), WirePosition::new(0, 4));
}

#[test]
fn unicode_empty_lines() {
    let src = "\n\n\n";
    let cache = LineStartsCache::new(src);
    assert_eq!(cache.offset_to_position(src, 0), (0, 0));
    assert_eq!(cache.offset_to_position(src, 1), (1, 0));
    assert_eq!(cache.offset_to_position(src, 2), (2, 0));
    assert_eq!(cache.offset_to_position(src, 3), (3, 0));
}

#[test]
fn mapper_byte_in_middle_of_multibyte_char() {
    // Ensure mid-character byte offsets are handled gracefully
    let m = PositionMapper::new("😀");
    // Byte 0 is start of the 4-byte emoji
    assert_eq!(m.byte_to_lsp_pos(0), WirePosition::new(0, 0));
    // Byte 4 is past the emoji
    assert_eq!(m.byte_to_lsp_pos(4), WirePosition::new(0, 2));
}

// ─── Serde roundtrip ─────────────────────────────────────────────────────────

#[test]
fn wire_position_serde_roundtrip() -> Result<(), serde_json::Error> {
    let wp = WirePosition::new(5, 10);
    let json = serde_json::to_string(&wp)?;
    let back: WirePosition = serde_json::from_str(&json)?;
    assert_eq!(wp, back);
    Ok(())
}

#[test]
fn wire_range_serde_roundtrip() -> Result<(), serde_json::Error> {
    let wr = WireRange::new(WirePosition::new(1, 0), WirePosition::new(3, 5));
    let json = serde_json::to_string(&wr)?;
    let back: WireRange = serde_json::from_str(&json)?;
    assert_eq!(wr, back);
    Ok(())
}

#[test]
fn wire_location_serde_roundtrip() -> Result<(), serde_json::Error> {
    let loc = WireLocation::new(
        "file:///test.pl".to_string(),
        WireRange::new(WirePosition::new(0, 0), WirePosition::new(0, 5)),
    );
    let json = serde_json::to_string(&loc)?;
    let back: WireLocation = serde_json::from_str(&json)?;
    assert_eq!(loc, back);
    Ok(())
}

#[test]
fn byte_span_serde_roundtrip() -> Result<(), serde_json::Error> {
    let span = ByteSpan::new(3, 7);
    let json = serde_json::to_string(&span)?;
    let back: ByteSpan = serde_json::from_str(&json)?;
    assert_eq!(span, back);
    Ok(())
}

#[test]
fn position_serde_roundtrip() -> Result<(), serde_json::Error> {
    let pos = perl_position_tracking::Position::new(10, 3, 7);
    let json = serde_json::to_string(&pos)?;
    let back: perl_position_tracking::Position = serde_json::from_str(&json)?;
    assert_eq!(pos, back);
    Ok(())
}

#[test]
fn range_serde_roundtrip() -> Result<(), serde_json::Error> {
    let r = perl_position_tracking::Range::new(
        perl_position_tracking::Position::new(0, 1, 1),
        perl_position_tracking::Position::new(10, 2, 5),
    );
    let json = serde_json::to_string(&r)?;
    let back: perl_position_tracking::Range = serde_json::from_str(&json)?;
    assert_eq!(r, back);
    Ok(())
}

// ─── Rope-backed methods ─────────────────────────────────────────────────────

#[test]
fn line_starts_cache_rope_basic() {
    let text = "abc\ndef\nghi";
    let rope = ropey::Rope::from_str(text);
    let cache = LineStartsCache::new_rope(&rope);

    assert_eq!(cache.offset_to_position_rope(&rope, 0), (0, 0));
    assert_eq!(cache.offset_to_position_rope(&rope, 4), (1, 0));
    assert_eq!(cache.offset_to_position_rope(&rope, 8), (2, 0));
}

#[test]
fn line_starts_cache_rope_utf16() {
    let text = "a😀b\ncd";
    let rope = ropey::Rope::from_str(text);
    let cache = LineStartsCache::new_rope(&rope);

    // After 'a': col 1
    assert_eq!(cache.offset_to_position_rope(&rope, 1), (0, 1));
    // After emoji: col 3 (1 + 2 UTF-16 units)
    assert_eq!(cache.offset_to_position_rope(&rope, 5), (0, 3));
    // 'c' on line 1
    assert_eq!(cache.offset_to_position_rope(&rope, 7), (1, 0));
}

#[test]
fn line_starts_cache_rope_position_to_offset_roundtrip() {
    let text = "hello\n世界\n!";
    let rope = ropey::Rope::from_str(text);
    let cache = LineStartsCache::new_rope(&rope);

    for byte in 0..text.len() {
        if !text.is_char_boundary(byte) {
            continue;
        }
        let (line, col) = cache.offset_to_position_rope(&rope, byte);
        let back = cache.position_to_offset_rope(&rope, line, col);
        assert_eq!(back, byte, "rope roundtrip failed at byte {byte}: line={line}, col={col}");
    }
}

#[test]
fn line_starts_cache_rope_beyond_end_clamps() {
    let text = "abc";
    let rope = ropey::Rope::from_str(text);
    let cache = LineStartsCache::new_rope(&rope);
    let (line, col) = cache.offset_to_position_rope(&rope, 999);
    assert_eq!(line, 0);
    assert_eq!(col, 3);
}

#[test]
fn line_starts_cache_rope_past_last_line() {
    let text = "abc\ndef";
    let rope = ropey::Rope::from_str(text);
    let cache = LineStartsCache::new_rope(&rope);
    let off = cache.position_to_offset_rope(&rope, 99, 0);
    assert_eq!(off, rope.len_bytes());
}

// ─── Additional ByteSpan edge cases ─────────────────────────────────────────

#[test]
fn byte_span_empty_does_not_contain_any_offset() {
    let span = ByteSpan::empty(5);
    assert!(!span.contains(5));
    assert!(!span.contains(4));
    assert!(!span.contains(6));
}

#[test]
fn byte_span_empty_inside_nonempty_overlaps() {
    // overlaps uses strict less-than on half-open intervals:
    // empty(5) = [5,5), nonempty = [3,8)
    // 5 < 8 && 3 < 5 → true (the point lies within the interval)
    let empty = ByteSpan::empty(5);
    let non_empty = ByteSpan::new(3, 8);
    assert!(empty.overlaps(non_empty));
    assert!(non_empty.overlaps(empty));
}

#[test]
fn byte_span_empty_at_boundary_does_not_overlap() {
    // empty(5) = [5,5), span = [5,10)
    // 5 < 10 && 5 < 5 → false (second condition fails)
    let empty = ByteSpan::empty(5);
    let span = ByteSpan::new(5, 10);
    assert!(!empty.overlaps(span));
    assert!(!span.overlaps(empty));
}

#[test]
fn byte_span_two_empty_spans_do_not_overlap() {
    let a = ByteSpan::empty(5);
    let b = ByteSpan::empty(5);
    assert!(!a.overlaps(b));
}

#[test]
fn byte_span_intersection_identical_spans() {
    let span = ByteSpan::new(3, 10);
    assert_eq!(span.intersection(span), Some(ByteSpan::new(3, 10)));
}

#[test]
fn byte_span_intersection_one_contains_other() {
    let outer = ByteSpan::new(0, 20);
    let inner = ByteSpan::new(5, 10);
    assert_eq!(outer.intersection(inner), Some(inner));
    assert_eq!(inner.intersection(outer), Some(inner));
}

#[test]
fn byte_span_union_identical() {
    let span = ByteSpan::new(3, 7);
    assert_eq!(span.union(span), span);
}

#[test]
fn byte_span_union_nested() {
    let outer = ByteSpan::new(0, 20);
    let inner = ByteSpan::new(5, 10);
    assert_eq!(outer.union(inner), outer);
    assert_eq!(inner.union(outer), outer);
}

#[test]
fn byte_span_whole_empty_string() {
    let span = ByteSpan::whole("");
    assert_eq!(span.start, 0);
    assert_eq!(span.end, 0);
    assert!(span.is_empty());
}

#[test]
fn byte_span_try_slice_empty_source() {
    let span = ByteSpan::new(0, 0);
    assert_eq!(span.try_slice(""), Some(""));
}

#[test]
fn byte_span_try_slice_out_of_bounds_start() {
    let span = ByteSpan::new(5, 10);
    assert_eq!(span.try_slice("hi"), None);
}

#[test]
fn byte_span_copy_semantics() {
    let a = ByteSpan::new(1, 5);
    let b = a; // Copy
    assert_eq!(a, b);
    assert_eq!(a.start, b.start);
}

#[test]
fn byte_span_hash_equality() {
    use std::collections::HashSet;
    let mut set = HashSet::new();
    set.insert(ByteSpan::new(1, 5));
    set.insert(ByteSpan::new(1, 5));
    assert_eq!(set.len(), 1);
}

#[test]
fn byte_span_hash_different() {
    use std::collections::HashSet;
    let mut set = HashSet::new();
    set.insert(ByteSpan::new(1, 5));
    set.insert(ByteSpan::new(1, 6));
    assert_eq!(set.len(), 2);
}

#[test]
fn byte_span_contains_span_empty_contained_in_nonempty() {
    let outer = ByteSpan::new(0, 10);
    let inner_empty = ByteSpan::empty(5);
    assert!(outer.contains_span(inner_empty));
}

#[test]
fn byte_span_contains_span_disjoint() {
    let a = ByteSpan::new(0, 5);
    let b = ByteSpan::new(6, 10);
    assert!(!a.contains_span(b));
    assert!(!b.contains_span(a));
}

#[test]
fn byte_span_overlaps_self() {
    let span = ByteSpan::new(3, 7);
    assert!(span.overlaps(span));
}

#[test]
fn byte_span_slice_full_source() {
    let src = "complete text";
    let span = ByteSpan::whole(src);
    assert_eq!(span.slice(src), src);
}

#[test]
fn byte_span_slice_single_char() {
    let src = "abcdef";
    let span = ByteSpan::new(2, 3);
    assert_eq!(span.slice(src), "c");
}

#[test]
fn byte_span_display_empty() {
    assert_eq!(format!("{}", ByteSpan::empty(0)), "0..0");
}

#[test]
fn byte_span_from_range_empty() {
    let span: ByteSpan = (5..5).into();
    assert!(span.is_empty());
    assert_eq!(span.start, 5);
}

// ─── Additional Position edge cases ─────────────────────────────────────────

#[test]
fn position_advance_empty_string() {
    let mut pos = perl_position_tracking::Position::start();
    pos.advance("");
    assert_eq!(pos.byte, 0);
    assert_eq!(pos.line, 1);
    assert_eq!(pos.column, 1);
}

#[test]
fn position_advance_only_newlines() {
    let mut pos = perl_position_tracking::Position::start();
    pos.advance("\n\n\n");
    assert_eq!(pos.byte, 3);
    assert_eq!(pos.line, 4);
    assert_eq!(pos.column, 1);
}

#[test]
fn position_advance_char_multibyte() {
    let mut pos = perl_position_tracking::Position::start();
    pos.advance_char('😀');
    assert_eq!(pos.byte, 4);
    assert_eq!(pos.column, 2);
    assert_eq!(pos.line, 1);
}

#[test]
fn position_advance_char_cjk() {
    let mut pos = perl_position_tracking::Position::start();
    pos.advance_char('日');
    assert_eq!(pos.byte, 3);
    assert_eq!(pos.column, 2);
}

#[test]
fn position_default_is_all_zeros() {
    let pos = perl_position_tracking::Position::default();
    assert_eq!(pos.byte, 0);
    assert_eq!(pos.line, 0);
    assert_eq!(pos.column, 0);
}

#[test]
fn position_new_arbitrary_values() {
    let pos = perl_position_tracking::Position::new(42, 7, 13);
    assert_eq!(pos.byte, 42);
    assert_eq!(pos.line, 7);
    assert_eq!(pos.column, 13);
}

#[test]
fn position_advance_cr_lf_sequence() {
    let mut pos = perl_position_tracking::Position::start();
    pos.advance("a\r\nb");
    // \r is treated as non-newline, \n advances line
    assert_eq!(pos.byte, 4);
    assert_eq!(pos.line, 2);
}

#[test]
fn position_advance_tab_character() {
    let mut pos = perl_position_tracking::Position::start();
    pos.advance("\t");
    assert_eq!(pos.byte, 1);
    assert_eq!(pos.column, 2); // tab is a non-newline char
}

// ─── Additional Range edge cases ────────────────────────────────────────────

#[test]
fn range_len_nonempty() {
    let r = perl_position_tracking::Range::new(
        perl_position_tracking::Position::new(10, 2, 1),
        perl_position_tracking::Position::new(25, 3, 6),
    );
    assert_eq!(r.len(), 15);
    assert!(!r.is_empty());
}

#[test]
fn range_extend_subset_no_change() {
    let mut r = perl_position_tracking::Range::new(
        perl_position_tracking::Position::new(0, 1, 1),
        perl_position_tracking::Position::new(20, 3, 1),
    );
    let subset = perl_position_tracking::Range::new(
        perl_position_tracking::Position::new(5, 1, 6),
        perl_position_tracking::Position::new(15, 2, 5),
    );
    r.extend(&subset);
    assert_eq!(r.start.byte, 0);
    assert_eq!(r.end.byte, 20);
}

#[test]
fn range_extend_only_start_earlier() {
    let mut r = perl_position_tracking::Range::new(
        perl_position_tracking::Position::new(5, 1, 6),
        perl_position_tracking::Position::new(20, 3, 1),
    );
    let other = perl_position_tracking::Range::new(
        perl_position_tracking::Position::new(0, 1, 1),
        perl_position_tracking::Position::new(10, 2, 1),
    );
    r.extend(&other);
    assert_eq!(r.start.byte, 0);
    assert_eq!(r.end.byte, 20);
}

#[test]
fn range_extend_only_end_later() {
    let mut r = perl_position_tracking::Range::new(
        perl_position_tracking::Position::new(0, 1, 1),
        perl_position_tracking::Position::new(10, 2, 1),
    );
    let other = perl_position_tracking::Range::new(
        perl_position_tracking::Position::new(5, 1, 6),
        perl_position_tracking::Position::new(30, 4, 1),
    );
    r.extend(&other);
    assert_eq!(r.start.byte, 0);
    assert_eq!(r.end.byte, 30);
}

#[test]
fn range_span_to_identical() {
    let r = perl_position_tracking::Range::new(
        perl_position_tracking::Position::new(5, 1, 6),
        perl_position_tracking::Position::new(10, 1, 11),
    );
    let s = r.span_to(&r);
    assert_eq!(s.start.byte, 5);
    assert_eq!(s.end.byte, 10);
}

#[test]
fn range_contains_byte_at_start_boundary() {
    let r = perl_position_tracking::Range::new(
        perl_position_tracking::Position::new(10, 2, 1),
        perl_position_tracking::Position::new(20, 3, 1),
    );
    assert!(r.contains_byte(10)); // inclusive
    assert!(!r.contains_byte(20)); // exclusive
}

#[test]
fn range_overlaps_identical() {
    let r = perl_position_tracking::Range::new(
        perl_position_tracking::Position::new(0, 1, 1),
        perl_position_tracking::Position::new(10, 1, 11),
    );
    assert!(r.overlaps(&r));
}

// ─── Additional LineStartsCache edge cases ──────────────────────────────────

#[test]
fn line_starts_cache_empty_string() {
    let src = "";
    let cache = LineStartsCache::new(src);
    let (line, col) = cache.offset_to_position(src, 0);
    assert_eq!(line, 0);
    assert_eq!(col, 0);
}

#[test]
fn line_starts_cache_only_newlines() {
    let src = "\n\n";
    let cache = LineStartsCache::new(src);
    assert_eq!(cache.offset_to_position(src, 0), (0, 0));
    assert_eq!(cache.offset_to_position(src, 1), (1, 0));
    assert_eq!(cache.offset_to_position(src, 2), (2, 0));
}

#[test]
fn line_starts_cache_trailing_newline() {
    let src = "abc\n";
    let cache = LineStartsCache::new(src);
    assert_eq!(cache.offset_to_position(src, 3), (0, 3));
    assert_eq!(cache.offset_to_position(src, 4), (1, 0));
}

#[test]
fn line_starts_cache_position_to_offset_large_column() {
    let src = "abc\ndef";
    let cache = LineStartsCache::new(src);
    // Column past end of line should clamp to end of line
    let off = cache.position_to_offset(src, 0, 999);
    assert!(off <= src.len());
}

#[test]
fn line_starts_cache_mixed_crlf_and_lf() {
    let src = "a\r\nb\nc";
    let cache = LineStartsCache::new(src);
    assert_eq!(cache.offset_to_position(src, 0), (0, 0)); // 'a'
    assert_eq!(cache.offset_to_position(src, 3), (1, 0)); // 'b' (after \r\n)
    assert_eq!(cache.offset_to_position(src, 5), (2, 0)); // 'c' (after \n)
}

#[test]
fn line_starts_cache_consecutive_crlf() {
    let src = "\r\n\r\n";
    let cache = LineStartsCache::new(src);
    assert_eq!(cache.offset_to_position(src, 0), (0, 0));
    assert_eq!(cache.offset_to_position(src, 2), (1, 0));
    assert_eq!(cache.offset_to_position(src, 4), (2, 0));
}

// ─── Additional LineIndex edge cases ────────────────────────────────────────

#[test]
fn line_index_empty_string() {
    let idx = LineIndex::new(String::new());
    assert_eq!(idx.offset_to_position(0), (0, 0));
}

#[test]
fn line_index_single_newline() {
    let idx = LineIndex::new("\n".to_string());
    assert_eq!(idx.offset_to_position(0), (0, 0));
    assert_eq!(idx.offset_to_position(1), (1, 0));
}

#[test]
fn line_index_position_to_offset_end_of_line() {
    let idx = LineIndex::new("abc\ndef".to_string());
    // Position (0, 3) is at the \n char itself
    assert_eq!(idx.position_to_offset(0, 3), Some(3));
}

#[test]
fn line_index_range_single_line() {
    let idx = LineIndex::new("hello".to_string());
    let (start, end) = idx.range(0, 5);
    assert_eq!(start, (0, 0));
    assert_eq!(end, (0, 5));
}

#[test]
fn line_index_position_to_offset_at_utf16_boundary() {
    // CJK: 3 UTF-8 bytes, 1 UTF-16 code unit
    let idx = LineIndex::new("日本".to_string());
    assert_eq!(idx.position_to_offset(0, 0), Some(0));
    assert_eq!(idx.position_to_offset(0, 1), Some(3));
    assert_eq!(idx.position_to_offset(0, 2), Some(6));
}

// ─── Additional convert function edge cases ─────────────────────────────────

#[test]
fn offset_to_utf16_line_col_empty_text() {
    let (line, col) = perl_position_tracking::offset_to_utf16_line_col("", 0);
    assert_eq!(line, 0);
    assert_eq!(col, 0);
}

#[test]
fn utf16_line_col_to_offset_empty_text() {
    assert_eq!(perl_position_tracking::utf16_line_col_to_offset("", 0, 0), 0);
}

#[test]
fn offset_to_utf16_line_col_at_newline_char() {
    let text = "abc\ndef";
    // Offset 3 is at the '\n' character itself
    let (line, col) = perl_position_tracking::offset_to_utf16_line_col(text, 3);
    assert_eq!(line, 0);
    assert_eq!(col, 3);
}

#[test]
fn offset_to_utf16_line_col_crlf() {
    let text = "ab\r\ncd";
    assert_eq!(perl_position_tracking::offset_to_utf16_line_col(text, 0), (0, 0));
    assert_eq!(perl_position_tracking::offset_to_utf16_line_col(text, 4), (1, 0));
    assert_eq!(perl_position_tracking::offset_to_utf16_line_col(text, 5), (1, 1));
}

#[test]
fn utf16_line_col_to_offset_crlf() {
    let text = "ab\r\ncd";
    assert_eq!(perl_position_tracking::utf16_line_col_to_offset(text, 0, 0), 0);
    assert_eq!(perl_position_tracking::utf16_line_col_to_offset(text, 1, 0), 4);
    assert_eq!(perl_position_tracking::utf16_line_col_to_offset(text, 1, 1), 5);
}

#[test]
fn convert_roundtrip_cjk_text() {
    let text = "日本語\n中文\n한국어";
    for byte in 0..text.len() {
        if !text.is_char_boundary(byte) {
            continue;
        }
        let (line, col) = perl_position_tracking::offset_to_utf16_line_col(text, byte);
        let back = perl_position_tracking::utf16_line_col_to_offset(text, line, col);
        assert_eq!(back, byte, "CJK roundtrip failed at byte {byte}");
    }
}

#[test]
fn convert_roundtrip_multiple_emoji() {
    let text = "😀😂🎉";
    for byte in 0..text.len() {
        if !text.is_char_boundary(byte) {
            continue;
        }
        let (line, col) = perl_position_tracking::offset_to_utf16_line_col(text, byte);
        let back = perl_position_tracking::utf16_line_col_to_offset(text, line, col);
        assert_eq!(back, byte, "emoji roundtrip failed at byte {byte}");
    }
}

// ─── Additional WireType edge cases ─────────────────────────────────────────

#[test]
fn wire_position_from_byte_offset_empty_string() {
    let wp = WirePosition::from_byte_offset("", 0);
    assert_eq!(wp.line, 0);
    assert_eq!(wp.character, 0);
}

#[test]
fn wire_position_to_byte_offset_at_newline() {
    let src = "abc\ndef";
    let wp = WirePosition::new(0, 3); // right before newline
    assert_eq!(wp.to_byte_offset(src), 3);
}

#[test]
fn wire_range_from_byte_offsets_emoji() {
    let src = "a😀b";
    let wr = WireRange::from_byte_offsets(src, 0, 6); // 'a' + emoji + 'b'
    assert_eq!(wr.start, WirePosition::new(0, 0));
    assert_eq!(wr.end, WirePosition::new(0, 4)); // 1 + 2 + 1 UTF-16
}

#[test]
fn wire_range_whole_document_empty() {
    let wr = WireRange::whole_document("");
    assert_eq!(wr.start, WirePosition::new(0, 0));
    assert_eq!(wr.end, WirePosition::new(0, 0));
}

#[test]
fn wire_range_whole_document_multiline() {
    let src = "abc\ndef\nghi";
    let wr = WireRange::whole_document(src);
    assert_eq!(wr.start, WirePosition::new(0, 0));
    assert_eq!(wr.end, WirePosition::new(2, 3));
}

#[test]
fn wire_location_different_uris() {
    let loc1 =
        WireLocation::new("file:///a.pl".to_string(), WireRange::empty(WirePosition::new(0, 0)));
    let loc2 =
        WireLocation::new("file:///b.pl".to_string(), WireRange::empty(WirePosition::new(0, 0)));
    assert_ne!(loc1, loc2);
}

// ─── Additional PositionMapper edge cases ───────────────────────────────────

#[test]
fn mapper_roundtrip_every_char_boundary() {
    let text = "hello\n世界\n😀!";
    let m = PositionMapper::new(text);
    for byte in 0..text.len() {
        if !text.is_char_boundary(byte) {
            continue;
        }
        let pos = m.byte_to_lsp_pos(byte);
        let back = m.lsp_pos_to_byte(pos);
        assert_eq!(back, Some(byte), "mapper roundtrip failed at byte {byte}: pos={pos:?}");
    }
}

#[test]
fn mapper_multiple_sequential_edits() {
    let mut m = PositionMapper::new("aaabbbccc");
    m.apply_edit(3, 6, "BBB");
    assert_eq!(m.text(), "aaaBBBccc");
    m.apply_edit(0, 3, "AAA");
    assert_eq!(m.text(), "AAABBBccc");
    m.apply_edit(6, 9, "CCC");
    assert_eq!(m.text(), "AAABBBCCC");
}

#[test]
fn mapper_edit_insert_at_start() {
    let mut m = PositionMapper::new("world");
    m.apply_edit(0, 0, "hello ");
    assert_eq!(m.text(), "hello world");
}

#[test]
fn mapper_edit_insert_at_end() {
    let mut m = PositionMapper::new("hello");
    m.apply_edit(5, 5, " world");
    assert_eq!(m.text(), "hello world");
}

#[test]
fn mapper_edit_delete_all() {
    let mut m = PositionMapper::new("hello world");
    m.apply_edit(0, 11, "");
    assert_eq!(m.text(), "");
    assert!(m.is_empty());
}

#[test]
fn mapper_update_to_empty_and_back() {
    let mut m = PositionMapper::new("hello");
    m.update("");
    assert!(m.is_empty());
    assert_eq!(m.len_bytes(), 0);
    m.update("new content");
    assert!(!m.is_empty());
    assert_eq!(m.text(), "new content");
}

#[test]
fn mapper_len_lines_trailing_newline() {
    let m = PositionMapper::new("abc\n");
    // ropey considers trailing newline as creating a new (empty) line
    assert_eq!(m.len_lines(), 2);
}

#[test]
fn mapper_slice_with_multibyte() {
    let m = PositionMapper::new("a😀b");
    assert_eq!(m.slice(0, 1), "a");
    assert_eq!(m.slice(1, 5), "😀");
    assert_eq!(m.slice(5, 6), "b");
}

#[test]
fn mapper_slice_clamps_beyond_end() {
    let m = PositionMapper::new("abc");
    assert_eq!(m.slice(0, 999), "abc");
}

#[test]
fn mapper_lsp_pos_to_byte_past_line_end() {
    let m = PositionMapper::new("abc\ndef");
    // Character past end of line
    let byte = m.lsp_pos_to_byte(WirePosition::new(0, 100));
    // Should return some value (clamped to line boundary)
    assert!(byte.is_some());
}

#[test]
fn mapper_byte_to_lsp_pos_at_newline() {
    let m = PositionMapper::new("abc\ndef");
    let pos = m.byte_to_lsp_pos(3); // at the '\n'
    assert_eq!(pos.line, 0);
    assert_eq!(pos.character, 3);
}

// ─── Additional mapper utility edge cases ───────────────────────────────────

#[test]
fn json_to_position_string_values_returns_none() {
    let json = serde_json::json!({"line": "five", "character": "ten"});
    assert!(perl_position_tracking::json_to_position(&json).is_none());
}

#[test]
fn json_to_position_empty_object_returns_none() {
    let json = serde_json::json!({});
    assert!(perl_position_tracking::json_to_position(&json).is_none());
}

#[test]
fn json_to_position_null_returns_none() {
    let json = serde_json::Value::Null;
    assert!(perl_position_tracking::json_to_position(&json).is_none());
}

#[test]
fn position_to_json_zero_position() {
    let pos = WirePosition::new(0, 0);
    let json = perl_position_tracking::position_to_json(pos);
    assert_eq!(json["line"], 0);
    assert_eq!(json["character"], 0);
}

#[test]
fn apply_edit_utf8_at_start() {
    let mut text = "hello".to_string();
    perl_position_tracking::apply_edit_utf8(&mut text, 0, 0, "oh ");
    assert_eq!(text, "oh hello");
}

#[test]
fn apply_edit_utf8_at_end() {
    let mut text = "hello".to_string();
    perl_position_tracking::apply_edit_utf8(&mut text, 5, 5, " world");
    assert_eq!(text, "hello world");
}

#[test]
fn apply_edit_utf8_replace_entire_content() {
    let mut text = "old".to_string();
    perl_position_tracking::apply_edit_utf8(&mut text, 0, 3, "new");
    assert_eq!(text, "new");
}

#[test]
fn newline_count_only_newlines() {
    assert_eq!(perl_position_tracking::newline_count("\n\n\n"), 3);
}

#[test]
fn newline_count_cr_not_counted() {
    // \r is not a \n, so should not be counted
    assert_eq!(perl_position_tracking::newline_count("a\rb\rc"), 0);
}

#[test]
fn newline_count_crlf_counts_lf() {
    assert_eq!(perl_position_tracking::newline_count("a\r\nb\r\n"), 2);
}

#[test]
fn last_line_column_utf8_single_newline() {
    assert_eq!(perl_position_tracking::last_line_column_utf8("\n"), 0);
}

#[test]
fn last_line_column_utf8_multibyte_last_line() {
    // 日 is 3 UTF-8 bytes
    assert_eq!(perl_position_tracking::last_line_column_utf8("abc\n日本"), 6);
}

// ─── Additional Rope-backed edge cases ──────────────────────────────────────

#[test]
fn line_starts_cache_rope_empty_string() {
    let rope = ropey::Rope::from_str("");
    let cache = LineStartsCache::new_rope(&rope);
    assert_eq!(cache.offset_to_position_rope(&rope, 0), (0, 0));
}

#[test]
fn line_starts_cache_rope_single_char() {
    let rope = ropey::Rope::from_str("x");
    let cache = LineStartsCache::new_rope(&rope);
    assert_eq!(cache.offset_to_position_rope(&rope, 0), (0, 0));
    assert_eq!(cache.offset_to_position_rope(&rope, 1), (0, 1));
}

#[test]
fn line_starts_cache_rope_cjk_roundtrip() {
    let text = "日本語\n中文";
    let rope = ropey::Rope::from_str(text);
    let cache = LineStartsCache::new_rope(&rope);

    for byte in 0..text.len() {
        if !text.is_char_boundary(byte) {
            continue;
        }
        let (line, col) = cache.offset_to_position_rope(&rope, byte);
        let back = cache.position_to_offset_rope(&rope, line, col);
        assert_eq!(back, byte, "rope CJK roundtrip failed at byte {byte}");
    }
}

#[test]
fn line_starts_cache_rope_position_to_offset_past_last_line() {
    let text = "abc";
    let rope = ropey::Rope::from_str(text);
    let cache = LineStartsCache::new_rope(&rope);
    let off = cache.position_to_offset_rope(&rope, 99, 0);
    assert_eq!(off, rope.len_bytes());
}

// ─── Serde edge cases ───────────────────────────────────────────────────────

#[test]
fn byte_span_serde_empty() -> Result<(), serde_json::Error> {
    let span = ByteSpan::empty(0);
    let json = serde_json::to_string(&span)?;
    let back: ByteSpan = serde_json::from_str(&json)?;
    assert_eq!(span, back);
    assert!(back.is_empty());
    Ok(())
}

#[test]
fn wire_position_serde_large_values() -> Result<(), serde_json::Error> {
    let wp = WirePosition::new(u32::MAX, u32::MAX);
    let json = serde_json::to_string(&wp)?;
    let back: WirePosition = serde_json::from_str(&json)?;
    assert_eq!(wp, back);
    Ok(())
}

#[test]
fn wire_range_serde_empty_range() -> Result<(), serde_json::Error> {
    let wr = WireRange::empty(WirePosition::new(5, 10));
    let json = serde_json::to_string(&wr)?;
    let back: WireRange = serde_json::from_str(&json)?;
    assert_eq!(wr, back);
    assert_eq!(back.start, back.end);
    Ok(())
}
