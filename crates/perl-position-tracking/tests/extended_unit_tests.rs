//! Extended unit tests for perl-position-tracking.
//!
//! Covers edge cases, UTF-16 surrogate pairs, multi-byte characters,
//! line ending variants, incremental edits, JSON round-trips, and
//! cross-type conversions.

use perl_position_tracking::{
    ByteSpan, LineEnding, LineIndex, LineStartsCache, Position, PositionMapper, Range,
    SourceLocation, WireLocation, WirePosition, WireRange, apply_edit_utf8, json_to_position,
    last_line_column_utf8, newline_count, offset_to_utf16_line_col, position_to_json,
    utf16_line_col_to_offset,
};
use perl_tdd_support::must_some;

// ─── ByteSpan: construction & basic properties ──────────────────────────────

#[test]
fn bytespan_zero_length_at_origin() {
    let s = ByteSpan::new(0, 0);
    assert!(s.is_empty());
    assert_eq!(s.len(), 0);
}

#[test]
fn bytespan_single_byte() {
    let s = ByteSpan::new(0, 1);
    assert_eq!(s.len(), 1);
    assert!(!s.is_empty());
    assert!(s.contains(0));
    assert!(!s.contains(1));
}

#[test]
fn bytespan_whole_empty_source() {
    let s = ByteSpan::whole("");
    assert!(s.is_empty());
    assert_eq!(s.start, 0);
    assert_eq!(s.end, 0);
}

#[test]
fn bytespan_whole_multibyte_source() {
    let src = "café";
    let s = ByteSpan::whole(src);
    assert_eq!(s.len(), src.len()); // 'é' is 2 bytes
    assert_eq!(s.slice(src), src);
}

#[test]
fn bytespan_display_format() {
    assert_eq!(format!("{}", ByteSpan::new(12, 34)), "12..34");
    assert_eq!(format!("{}", ByteSpan::empty(0)), "0..0");
}

// ─── ByteSpan: containment ──────────────────────────────────────────────────

#[test]
fn bytespan_contains_span_identity() {
    let s = ByteSpan::new(5, 10);
    assert!(s.contains_span(s));
}

#[test]
fn bytespan_contains_span_empty_inside() {
    let outer = ByteSpan::new(0, 10);
    let empty = ByteSpan::empty(5);
    assert!(outer.contains_span(empty));
}

#[test]
fn bytespan_contains_span_at_boundaries() {
    let outer = ByteSpan::new(0, 10);
    assert!(outer.contains_span(ByteSpan::new(0, 10))); // exact match
    assert!(outer.contains_span(ByteSpan::new(0, 1))); // left edge
    assert!(outer.contains_span(ByteSpan::new(9, 10))); // right edge
    assert!(!outer.contains_span(ByteSpan::new(0, 11))); // exceeds right
}

// ─── ByteSpan: overlap & intersection ────────────────────────────────────────

#[test]
fn bytespan_overlaps_symmetric() {
    let a = ByteSpan::new(0, 5);
    let b = ByteSpan::new(3, 8);
    assert!(a.overlaps(b));
    assert!(b.overlaps(a));
}

#[test]
fn bytespan_no_overlap_adjacent() {
    let a = ByteSpan::new(0, 5);
    let b = ByteSpan::new(5, 10);
    assert!(!a.overlaps(b));
    assert!(!b.overlaps(a));
}

#[test]
fn bytespan_empty_spans_no_overlap() {
    let a = ByteSpan::empty(5);
    let b = ByteSpan::empty(5);
    assert!(!a.overlaps(b));
}

#[test]
fn bytespan_intersection_partial() {
    let a = ByteSpan::new(2, 8);
    let b = ByteSpan::new(5, 12);
    let i = must_some(a.intersection(b));
    assert_eq!(i, ByteSpan::new(5, 8));
}

#[test]
fn bytespan_intersection_contained() {
    let outer = ByteSpan::new(0, 20);
    let inner = ByteSpan::new(5, 10);
    let i = must_some(outer.intersection(inner));
    assert_eq!(i, inner);
}

#[test]
fn bytespan_intersection_none_disjoint() {
    let a = ByteSpan::new(0, 3);
    let b = ByteSpan::new(5, 8);
    assert!(a.intersection(b).is_none());
}

#[test]
fn bytespan_intersection_none_adjacent() {
    let a = ByteSpan::new(0, 5);
    let b = ByteSpan::new(5, 10);
    assert!(a.intersection(b).is_none());
}

// ─── ByteSpan: union ─────────────────────────────────────────────────────────

#[test]
fn bytespan_union_disjoint() {
    let a = ByteSpan::new(0, 3);
    let b = ByteSpan::new(7, 10);
    assert_eq!(a.union(b), ByteSpan::new(0, 10));
}

#[test]
fn bytespan_union_overlapping() {
    let a = ByteSpan::new(2, 8);
    let b = ByteSpan::new(5, 12);
    assert_eq!(a.union(b), ByteSpan::new(2, 12));
}

#[test]
fn bytespan_union_with_empty() {
    let a = ByteSpan::new(3, 7);
    let e = ByteSpan::empty(5);
    assert_eq!(a.union(e), ByteSpan::new(3, 7));
}

// ─── ByteSpan: slice & try_slice ─────────────────────────────────────────────

#[test]
fn bytespan_slice_utf8_boundary() {
    let src = "aéb"; // a(1) é(2) b(1) = 4 bytes
    let span = ByteSpan::new(1, 3); // covers 'é'
    assert_eq!(span.slice(src), "é");
}

#[test]
fn bytespan_try_slice_out_of_bounds() {
    let src = "hello";
    let span = ByteSpan::new(0, 100);
    assert!(span.try_slice(src).is_none());
}

#[test]
fn bytespan_try_slice_valid() {
    let src = "hello world";
    let span = ByteSpan::new(6, 11);
    assert_eq!(must_some(span.try_slice(src)), "world");
}

#[test]
fn bytespan_try_slice_empty() {
    let src = "abc";
    let span = ByteSpan::empty(2);
    assert_eq!(must_some(span.try_slice(src)), "");
}

// ─── ByteSpan: conversions ───────────────────────────────────────────────────

#[test]
fn bytespan_from_range() {
    let span: ByteSpan = (3..7usize).into();
    assert_eq!(span.start, 3);
    assert_eq!(span.end, 7);
}

#[test]
fn bytespan_to_range() {
    let span = ByteSpan::new(1, 9);
    let r = span.to_range();
    assert_eq!(r, 1..9);
}

#[test]
fn bytespan_from_tuple() {
    let span: ByteSpan = (10, 20).into();
    assert_eq!(span, ByteSpan::new(10, 20));
}

#[test]
fn bytespan_into_tuple() {
    let t: (usize, usize) = ByteSpan::new(4, 8).into();
    assert_eq!(t, (4, 8));
}

#[test]
fn source_location_alias() {
    let sl: SourceLocation = ByteSpan::new(1, 5);
    assert_eq!(sl.len(), 4);
}

// ─── ByteSpan: default & hash equality ───────────────────────────────────────

#[test]
fn bytespan_default_is_empty_at_zero() {
    let d: ByteSpan = Default::default();
    assert!(d.is_empty());
    assert_eq!(d.start, 0);
}

#[test]
fn bytespan_eq_via_fields() {
    let a = ByteSpan::new(1, 5);
    let b = ByteSpan { start: 1, end: 5 };
    assert_eq!(a, b);
}

// ─── Position (engine type) ──────────────────────────────────────────────────

#[test]
fn position_start_values() {
    let p = Position::start();
    assert_eq!(p.byte, 0);
    assert_eq!(p.line, 1);
    assert_eq!(p.column, 1);
}

#[test]
fn position_advance_empty_string() {
    let mut p = Position::start();
    p.advance("");
    assert_eq!(p, Position::start());
}

#[test]
fn position_advance_single_newline() {
    let mut p = Position::start();
    p.advance("\n");
    assert_eq!(p.byte, 1);
    assert_eq!(p.line, 2);
    assert_eq!(p.column, 1);
}

#[test]
fn position_advance_multiple_newlines() {
    let mut p = Position::start();
    p.advance("\n\n\n");
    assert_eq!(p.line, 4);
    assert_eq!(p.column, 1);
    assert_eq!(p.byte, 3);
}

#[test]
fn position_advance_multibyte_chars() {
    let mut p = Position::start();
    p.advance("日本語"); // 3 chars, 9 bytes
    assert_eq!(p.byte, 9);
    assert_eq!(p.column, 4); // 1-based: started at 1, advanced 3
    assert_eq!(p.line, 1);
}

#[test]
fn position_advance_char_newline() {
    let mut p = Position::new(0, 1, 5);
    p.advance_char('\n');
    assert_eq!(p.line, 2);
    assert_eq!(p.column, 1);
    assert_eq!(p.byte, 1);
}

#[test]
fn position_advance_char_regular() {
    let mut p = Position::new(0, 1, 1);
    p.advance_char('x');
    assert_eq!(p.column, 2);
    assert_eq!(p.byte, 1);
}

#[test]
fn position_advance_char_multibyte() {
    let mut p = Position::new(0, 1, 1);
    p.advance_char('🦀'); // 4 UTF-8 bytes
    assert_eq!(p.byte, 4);
    assert_eq!(p.column, 2);
}

#[test]
fn position_display() {
    let p = Position::new(0, 3, 7);
    assert_eq!(format!("{p}"), "3:7");
}

#[test]
fn position_default_is_zero() {
    let p: Position = Default::default();
    assert_eq!(p.byte, 0);
    assert_eq!(p.line, 0);
    assert_eq!(p.column, 0);
}

// ─── Range (engine type) ─────────────────────────────────────────────────────

#[test]
fn range_empty_at_position() {
    let p = Position::new(5, 2, 3);
    let r = Range::empty(p);
    assert!(r.is_empty());
    assert_eq!(r.len(), 0);
    assert_eq!(r.start, r.end);
}

#[test]
fn range_contains_byte_boundary() {
    let r = Range::new(Position::new(5, 1, 1), Position::new(10, 1, 6));
    assert!(!r.contains_byte(4));
    assert!(r.contains_byte(5));
    assert!(r.contains_byte(9));
    assert!(!r.contains_byte(10)); // exclusive end
}

#[test]
fn range_contains_position() {
    let r = Range::new(Position::new(0, 1, 1), Position::new(20, 3, 5));
    let inside = Position::new(10, 2, 3);
    let outside = Position::new(25, 4, 1);
    assert!(r.contains(inside));
    assert!(!r.contains(outside));
}

#[test]
fn range_overlaps_disjoint() {
    let a = Range::new(Position::new(0, 1, 1), Position::new(5, 1, 6));
    let b = Range::new(Position::new(10, 2, 1), Position::new(15, 2, 6));
    assert!(!a.overlaps(&b));
}

#[test]
fn range_overlaps_touching() {
    let a = Range::new(Position::new(0, 1, 1), Position::new(5, 1, 6));
    let b = Range::new(Position::new(5, 1, 6), Position::new(10, 2, 1));
    assert!(!a.overlaps(&b)); // touching at boundary: no overlap
}

#[test]
fn range_overlaps_partial() {
    let a = Range::new(Position::new(0, 1, 1), Position::new(8, 1, 9));
    let b = Range::new(Position::new(5, 1, 6), Position::new(15, 2, 6));
    assert!(a.overlaps(&b));
    assert!(b.overlaps(&a));
}

#[test]
fn range_len_saturating() {
    // Artificially reversed bytes (start > end)
    let r = Range::new(Position::new(10, 2, 1), Position::new(5, 1, 6));
    assert_eq!(r.len(), 0); // saturating_sub prevents underflow
}

#[test]
fn range_extend_grows_both_ends() {
    let mut r = Range::new(Position::new(5, 1, 6), Position::new(10, 2, 1));
    let wider = Range::new(Position::new(2, 1, 3), Position::new(15, 2, 6));
    r.extend(&wider);
    assert_eq!(r.start.byte, 2);
    assert_eq!(r.end.byte, 15);
}

#[test]
fn range_extend_no_change_when_already_larger() {
    let mut r = Range::new(Position::new(0, 1, 1), Position::new(20, 3, 1));
    let smaller = Range::new(Position::new(5, 1, 6), Position::new(10, 2, 1));
    r.extend(&smaller);
    assert_eq!(r.start.byte, 0);
    assert_eq!(r.end.byte, 20);
}

#[test]
fn range_span_to_picks_outer_bounds() {
    let a = Range::new(Position::new(3, 1, 4), Position::new(8, 1, 9));
    let b = Range::new(Position::new(1, 1, 2), Position::new(6, 1, 7));
    let span = a.span_to(&b);
    assert_eq!(span.start.byte, 1);
    assert_eq!(span.end.byte, 8);
}

#[test]
fn range_display() {
    let r = Range::new(Position::new(0, 1, 1), Position::new(10, 2, 5));
    assert_eq!(format!("{r}"), "1:1-2:5");
}

#[test]
fn range_from_source_location() {
    let sl = SourceLocation::new(3, 9);
    let r: Range = sl.into();
    assert_eq!(r.start.byte, 3);
    assert_eq!(r.end.byte, 9);
}

// ─── WirePosition ────────────────────────────────────────────────────────────

#[test]
fn wire_position_new() {
    let wp = WirePosition::new(5, 10);
    assert_eq!(wp.line, 5);
    assert_eq!(wp.character, 10);
}

#[test]
fn wire_position_default() {
    let wp: WirePosition = Default::default();
    assert_eq!(wp.line, 0);
    assert_eq!(wp.character, 0);
}

#[test]
fn wire_position_from_byte_offset_ascii() {
    let src = "hello\nworld\n";
    let wp = WirePosition::from_byte_offset(src, 6);
    assert_eq!(wp.line, 1);
    assert_eq!(wp.character, 0);
}

#[test]
fn wire_position_from_byte_offset_start() {
    let src = "abc";
    let wp = WirePosition::from_byte_offset(src, 0);
    assert_eq!(wp.line, 0);
    assert_eq!(wp.character, 0);
}

#[test]
fn wire_position_to_byte_offset_roundtrip() {
    let src = "first\nsecond\nthird";
    let wp = WirePosition::new(1, 3);
    let byte = wp.to_byte_offset(src);
    assert_eq!(byte, 9); // 'o' in "second"
    let wp2 = WirePosition::from_byte_offset(src, byte);
    assert_eq!(wp2, wp);
}

#[test]
fn wire_position_utf16_surrogate_pair() {
    // '𝄞' (U+1D11E) is a 4-byte UTF-8 char, 2 UTF-16 code units
    let src = "a𝄞b";
    let wp = WirePosition::from_byte_offset(src, 5); // byte of 'b'
    assert_eq!(wp.line, 0);
    assert_eq!(wp.character, 3); // a(1) + 𝄞(2) = 3 UTF-16 units
}

// ─── WireRange ───────────────────────────────────────────────────────────────

#[test]
fn wire_range_new() {
    let wr = WireRange::new(WirePosition::new(0, 0), WirePosition::new(1, 5));
    assert_eq!(wr.start.line, 0);
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
    let src = "ab\ncd\nef";
    let wr = WireRange::from_byte_offsets(src, 3, 5);
    assert_eq!(wr.start, WirePosition::new(1, 0));
    assert_eq!(wr.end, WirePosition::new(1, 2));
}

#[test]
fn wire_range_whole_document_empty() {
    let wr = WireRange::whole_document("");
    assert_eq!(wr.start, WirePosition::new(0, 0));
    assert_eq!(wr.end, WirePosition::new(0, 0));
}

#[test]
fn wire_range_whole_document_trailing_newline() {
    let src = "line1\nline2\n";
    let wr = WireRange::whole_document(src);
    assert_eq!(wr.start, WirePosition::new(0, 0));
    assert_eq!(wr.end.line, 2);
    assert_eq!(wr.end.character, 0);
}

#[test]
fn wire_range_default() {
    let wr: WireRange = Default::default();
    assert_eq!(wr.start, WirePosition::new(0, 0));
    assert_eq!(wr.end, WirePosition::new(0, 0));
}

// ─── WireLocation ────────────────────────────────────────────────────────────

#[test]
fn wire_location_new() {
    let range = WireRange::new(WirePosition::new(0, 0), WirePosition::new(1, 0));
    let loc = WireLocation::new("file:///test.pl".to_string(), range);
    assert_eq!(loc.uri, "file:///test.pl");
    assert_eq!(loc.range.start.line, 0);
}

// ─── LineStartsCache: construction ───────────────────────────────────────────

#[test]
fn line_starts_cache_empty_text() {
    let cache = LineStartsCache::new("");
    let (line, col) = cache.offset_to_position("", 0);
    assert_eq!(line, 0);
    assert_eq!(col, 0);
}

#[test]
fn line_starts_cache_single_line_no_newline() {
    let src = "hello";
    let cache = LineStartsCache::new(src);
    let (line, col) = cache.offset_to_position(src, 3);
    assert_eq!(line, 0);
    assert_eq!(col, 3);
}

#[test]
fn line_starts_cache_single_newline() {
    let src = "\n";
    let cache = LineStartsCache::new(src);
    let (l0, c0) = cache.offset_to_position(src, 0);
    assert_eq!(l0, 0);
    assert_eq!(c0, 0);
    let (l1, c1) = cache.offset_to_position(src, 1);
    assert_eq!(l1, 1);
    assert_eq!(c1, 0);
}

#[test]
fn line_starts_cache_crlf() {
    let src = "a\r\nb\r\nc";
    let cache = LineStartsCache::new(src);
    let (line, col) = cache.offset_to_position(src, 3); // 'b'
    assert_eq!(line, 1);
    assert_eq!(col, 0);
}

#[test]
fn line_starts_cache_cr_only() {
    let src = "a\rb\rc";
    let cache = LineStartsCache::new(src);
    let (line, col) = cache.offset_to_position(src, 2); // 'b'
    assert_eq!(line, 1);
    assert_eq!(col, 0);
}

// ─── LineStartsCache: offset_to_position ─────────────────────────────────────

#[test]
fn line_starts_cache_utf16_column() {
    let src = "a😀b\nx"; // 😀 is 4 UTF-8 bytes, 2 UTF-16 units
    let cache = LineStartsCache::new(src);
    let (line, col) = cache.offset_to_position(src, 5); // byte of 'b'
    assert_eq!(line, 0);
    assert_eq!(col, 3); // a(1) + 😀(2) = 3 UTF-16 units
}

#[test]
fn line_starts_cache_offset_clamped() {
    let src = "abc";
    let cache = LineStartsCache::new(src);
    // offset beyond text length is clamped
    let (line, col) = cache.offset_to_position(src, 100);
    assert_eq!(line, 0);
    assert_eq!(col, 3);
}

#[test]
fn line_starts_cache_offset_mid_utf8_char_clamps_to_boundary() {
    let src = "a😀b";
    let cache = LineStartsCache::new(src);

    // Byte offset 2 is in the middle of 😀 (starts at 1, ends before 5).
    let (line, col) = cache.offset_to_position(src, 2);
    assert_eq!(line, 0);
    assert_eq!(col, 1);
}

// ─── LineStartsCache: position_to_offset ─────────────────────────────────────

#[test]
fn line_starts_cache_position_to_offset_start() {
    let src = "hello\nworld";
    let cache = LineStartsCache::new(src);
    assert_eq!(cache.position_to_offset(src, 0, 0), 0);
}

#[test]
fn line_starts_cache_position_to_offset_second_line() {
    let src = "hello\nworld";
    let cache = LineStartsCache::new(src);
    assert_eq!(cache.position_to_offset(src, 1, 0), 6);
    assert_eq!(cache.position_to_offset(src, 1, 3), 9);
}

#[test]
fn line_starts_cache_position_to_offset_beyond_lines() {
    let src = "abc";
    let cache = LineStartsCache::new(src);
    let off = cache.position_to_offset(src, 99, 0);
    assert_eq!(off, src.len());
}

#[test]
fn line_starts_cache_position_to_offset_utf16() {
    let src = "a😀b\n"; // a(1)+😀(2)+b(1) UTF-16 units on line 0
    let cache = LineStartsCache::new(src);
    // character 3 should resolve to byte of 'b' = 5
    let off = cache.position_to_offset(src, 0, 3);
    assert_eq!(off, 5);
}

// ─── LineStartsCache: roundtrip ──────────────────────────────────────────────

#[test]
fn line_starts_cache_roundtrip() {
    let src = "first\nsecond\nthird";
    let cache = LineStartsCache::new(src);
    for byte in 0..src.len() {
        if src.is_char_boundary(byte) {
            let (line, col) = cache.offset_to_position(src, byte);
            let back = cache.position_to_offset(src, line, col);
            assert_eq!(back, byte, "roundtrip failed at byte {byte}");
        }
    }
}

// ─── LineStartsCache: rope variants ──────────────────────────────────────────

#[test]
fn line_starts_cache_rope_basic() {
    let src = "abc\ndef\nghi";
    let rope = ropey::Rope::from_str(src);
    let cache = LineStartsCache::new_rope(&rope);
    let (line, col) = cache.offset_to_position_rope(&rope, 4);
    assert_eq!(line, 1);
    assert_eq!(col, 0);
}

#[test]
fn line_starts_cache_rope_position_to_offset() {
    let src = "abc\ndef\nghi";
    let rope = ropey::Rope::from_str(src);
    let cache = LineStartsCache::new_rope(&rope);
    let off = cache.position_to_offset_rope(&rope, 2, 1);
    assert_eq!(off, 9); // 'h' in "ghi" is at byte 9... actually 'h' is at byte 8
    // Verify: "abc\ndef\nghi" offsets: a=0,b=1,c=2,\n=3,d=4,e=5,f=6,\n=7,g=8,h=9,i=10
    // Wait: g=8, h=9, i=10? Let me count: a(0)b(1)c(2)\n(3)d(4)e(5)f(6)\n(7)g(8)h(9)i(10)
    // character 1 on line 2 = 'h' at byte 9. Actually let me just check the value.
    // Actually the offset should be line 2 start (8) + 1 char = 9
}

#[test]
fn line_starts_cache_rope_beyond_end() {
    let src = "abc";
    let rope = ropey::Rope::from_str(src);
    let cache = LineStartsCache::new_rope(&rope);
    let off = cache.position_to_offset_rope(&rope, 99, 0);
    assert_eq!(off, rope.len_bytes());
}

// ─── LineIndex ───────────────────────────────────────────────────────────────

#[test]
fn line_index_empty() {
    let idx = LineIndex::new(String::new());
    let (line, col) = idx.offset_to_position(0);
    assert_eq!(line, 0);
    assert_eq!(col, 0);
}

#[test]
fn line_index_single_line() {
    let idx = LineIndex::new("hello".to_string());
    let (line, col) = idx.offset_to_position(3);
    assert_eq!(line, 0);
    assert_eq!(col, 3);
}

#[test]
fn line_index_multi_line() {
    let idx = LineIndex::new("abc\ndef\nghi".to_string());
    let (line, col) = idx.offset_to_position(4); // 'd'
    assert_eq!(line, 1);
    assert_eq!(col, 0);
    let (line2, col2) = idx.offset_to_position(5); // 'e'
    assert_eq!(line2, 1);
    assert_eq!(col2, 1);
}

#[test]
fn line_index_position_to_offset_valid() {
    let idx = LineIndex::new("abc\ndef\n".to_string());
    let off = must_some(idx.position_to_offset(1, 2));
    assert_eq!(off, 6); // 'f'
}

#[test]
fn line_index_position_to_offset_line_out_of_range() {
    let idx = LineIndex::new("abc".to_string());
    assert!(idx.position_to_offset(5, 0).is_none());
}

#[test]
fn line_index_position_to_offset_start_of_line() {
    let idx = LineIndex::new("abc\ndef\n".to_string());
    let off = must_some(idx.position_to_offset(1, 0));
    assert_eq!(off, 4);
}

#[test]
fn line_index_range() {
    let idx = LineIndex::new("abc\ndef\nghi".to_string());
    let (start, end) = idx.range(4, 9);
    assert_eq!(start, (1, 0)); // 'd'
    assert_eq!(end, (2, 1)); // 'h'
}

#[test]
fn line_index_utf16_column_bmp() {
    // BMP character é (2 UTF-8 bytes, 1 UTF-16 unit)
    let idx = LineIndex::new("café".to_string());
    let (line, col) = idx.offset_to_position(5); // byte after 'é' = 5 bytes: c(1)a(1)f(1)é(2) = 5
    assert_eq!(line, 0);
    assert_eq!(col, 4); // 4 characters = 4 UTF-16 units
}

#[test]
fn line_index_utf16_column_surrogate() {
    // 😀 is 4 UTF-8 bytes, 2 UTF-16 code units
    let idx = LineIndex::new("a😀b".to_string());
    let (line, col) = idx.offset_to_position(5); // byte of 'b'
    assert_eq!(line, 0);
    assert_eq!(col, 3); // a(1) + 😀(2) = 3 UTF-16 units
}

#[test]
fn line_index_offset_to_position_non_boundary_clamps_to_previous_boundary() {
    let idx = LineIndex::new("a😀b".to_string());
    let (line, col) = idx.offset_to_position(2); // in the middle of 😀
    assert_eq!(line, 0);
    assert_eq!(col, 1); // clamps to byte offset 1 (just after 'a')
}

#[test]
fn line_index_offset_to_position_beyond_end_clamps_to_eof() {
    let idx = LineIndex::new("a😀b".to_string());
    let (line, col) = idx.offset_to_position(usize::MAX);
    assert_eq!(line, 0);
    assert_eq!(col, 4); // a(1) + 😀(2) + b(1)
}

// ─── convert: offset_to_utf16_line_col ───────────────────────────────────────

#[test]
fn convert_offset_zero() {
    let (line, col) = offset_to_utf16_line_col("hello", 0);
    assert_eq!(line, 0);
    assert_eq!(col, 0);
}

#[test]
fn convert_offset_end_of_line() {
    let (line, col) = offset_to_utf16_line_col("hello\nworld", 5);
    assert_eq!(line, 0);
    assert_eq!(col, 5);
}

#[test]
fn convert_offset_start_of_second_line() {
    let (line, col) = offset_to_utf16_line_col("hello\nworld", 6);
    assert_eq!(line, 1);
    assert_eq!(col, 0);
}

#[test]
fn convert_offset_beyond_text() {
    let (line, col) = offset_to_utf16_line_col("hi", 100);
    assert_eq!(line, 0);
    assert_eq!(col, 2);
}

#[test]
fn convert_offset_at_text_end_trailing_newline() {
    let text = "hello\n";
    let (line, col) = offset_to_utf16_line_col(text, text.len());
    assert_eq!(line, 1);
    assert_eq!(col, 0);
}

#[test]
fn convert_offset_empty_text() {
    let (line, col) = offset_to_utf16_line_col("", 0);
    assert_eq!(line, 0);
    assert_eq!(col, 0);
}

#[test]
fn convert_offset_utf16_surrogate() {
    let text = "a𝄞b"; // 𝄞 = 4 UTF-8 bytes, 2 UTF-16 units
    let (line, col) = offset_to_utf16_line_col(text, 5); // byte of 'b'
    assert_eq!(line, 0);
    assert_eq!(col, 3); // a(1) + 𝄞(2)
}

// ─── convert: utf16_line_col_to_offset ───────────────────────────────────────

#[test]
fn convert_utf16_to_offset_start() {
    assert_eq!(utf16_line_col_to_offset("hello\nworld", 0, 0), 0);
}

#[test]
fn convert_utf16_to_offset_second_line() {
    assert_eq!(utf16_line_col_to_offset("hello\nworld", 1, 2), 8);
}

#[test]
fn convert_utf16_to_offset_beyond_lines() {
    let text = "hello";
    let off = utf16_line_col_to_offset(text, 99, 0);
    assert_eq!(off, text.len());
}

#[test]
fn convert_utf16_roundtrip_ascii() {
    let text = "abc\ndef\nghi";
    for byte in 0..text.len() {
        if text.is_char_boundary(byte) {
            let (line, col) = offset_to_utf16_line_col(text, byte);
            let back = utf16_line_col_to_offset(text, line, col);
            assert_eq!(back, byte, "roundtrip failed at byte {byte}");
        }
    }
}

#[test]
fn convert_utf16_roundtrip_multibyte() {
    let text = "café\n日本語\n🦀\n";
    // Only check char boundaries
    for (byte, _) in text.char_indices() {
        let (line, col) = offset_to_utf16_line_col(text, byte);
        let back = utf16_line_col_to_offset(text, line, col);
        assert_eq!(back, byte, "roundtrip failed at byte {byte}");
    }
}

// ─── PositionMapper ──────────────────────────────────────────────────────────

#[test]
fn mapper_empty_text() {
    let m = PositionMapper::new("");
    assert!(m.is_empty());
    assert_eq!(m.len_bytes(), 0);
    assert_eq!(m.len_lines(), 1); // rope always has at least 1 line
}

#[test]
fn mapper_byte_to_lsp_pos_start() {
    let m = PositionMapper::new("hello");
    let p = m.byte_to_lsp_pos(0);
    assert_eq!(p.line, 0);
    assert_eq!(p.character, 0);
}

#[test]
fn mapper_byte_to_lsp_pos_middle() {
    let m = PositionMapper::new("hello\nworld");
    let p = m.byte_to_lsp_pos(8);
    assert_eq!(p.line, 1);
    assert_eq!(p.character, 2);
}

#[test]
fn mapper_byte_to_lsp_pos_clamped() {
    let m = PositionMapper::new("abc");
    let p = m.byte_to_lsp_pos(999);
    assert_eq!(p.line, 0);
    assert_eq!(p.character, 3);
}

#[test]
fn mapper_lsp_pos_to_byte_none_for_invalid_line() {
    let m = PositionMapper::new("abc");
    assert!(m.lsp_pos_to_byte(WirePosition::new(5, 0)).is_none());
}

#[test]
fn mapper_lsp_pos_to_byte_valid() {
    let m = PositionMapper::new("hello\nworld");
    let byte = must_some(m.lsp_pos_to_byte(WirePosition::new(1, 3)));
    assert_eq!(byte, 9); // 'l' in "world"
}

#[test]
fn mapper_lsp_pos_to_char_and_back() {
    let m = PositionMapper::new("abc\ndef");
    let char_idx = must_some(m.lsp_pos_to_char(WirePosition::new(1, 1)));
    let pos_back = m.char_to_lsp_pos(char_idx);
    assert_eq!(pos_back, WirePosition::new(1, 1));
}

#[test]
fn mapper_text_roundtrip() {
    let src = "hello\nworld\n";
    let m = PositionMapper::new(src);
    assert_eq!(m.text(), src);
}

#[test]
fn mapper_slice() {
    let m = PositionMapper::new("hello world");
    assert_eq!(m.slice(6, 11), "world");
}

#[test]
fn mapper_slice_clamped() {
    let m = PositionMapper::new("abc");
    assert_eq!(m.slice(0, 999), "abc");
}

#[test]
fn mapper_line_ending_lf() {
    let m = PositionMapper::new("a\nb\n");
    assert_eq!(m.line_ending(), LineEnding::Lf);
}

#[test]
fn mapper_line_ending_crlf() {
    let m = PositionMapper::new("a\r\nb\r\n");
    assert_eq!(m.line_ending(), LineEnding::CrLf);
}

#[test]
fn mapper_line_ending_cr() {
    let m = PositionMapper::new("a\rb\r");
    assert_eq!(m.line_ending(), LineEnding::Cr);
}

#[test]
fn mapper_line_ending_mixed() {
    let m = PositionMapper::new("a\nb\r\nc\r");
    assert_eq!(m.line_ending(), LineEnding::Mixed);
}

#[test]
fn mapper_line_ending_no_newlines() {
    let m = PositionMapper::new("hello");
    assert_eq!(m.line_ending(), LineEnding::Lf); // default
}

#[test]
fn mapper_len_lines() {
    let m = PositionMapper::new("a\nb\nc");
    assert_eq!(m.len_lines(), 3);
}

// ─── PositionMapper: incremental edits ───────────────────────────────────────

#[test]
fn mapper_apply_edit_insert_at_start() {
    let mut m = PositionMapper::new("world");
    m.apply_edit(0, 0, "hello ");
    assert_eq!(m.text(), "hello world");
}

#[test]
fn mapper_apply_edit_delete() {
    let mut m = PositionMapper::new("hello world");
    m.apply_edit(5, 11, "");
    assert_eq!(m.text(), "hello");
}

#[test]
fn mapper_apply_edit_replace() {
    let mut m = PositionMapper::new("hello world");
    m.apply_edit(6, 11, "Perl");
    assert_eq!(m.text(), "hello Perl");
}

#[test]
fn mapper_apply_edit_clamped() {
    let mut m = PositionMapper::new("abc");
    m.apply_edit(0, 999, "xyz");
    assert_eq!(m.text(), "xyz");
}

#[test]
fn mapper_update() {
    let mut m = PositionMapper::new("old");
    m.update("new content");
    assert_eq!(m.text(), "new content");
    assert_eq!(m.len_bytes(), 11);
}

#[test]
fn mapper_utf16_emoji() {
    let text = "print '😀';\n";
    let m = PositionMapper::new(text);
    // '😀' starts at byte 7, is 4 bytes; UTF-16 character 7
    let pos = m.byte_to_lsp_pos(7);
    assert_eq!(pos.line, 0);
    assert_eq!(pos.character, 7);
    // After emoji: byte 11, UTF-16 character 9 (7 + 2 for emoji)
    let pos2 = m.byte_to_lsp_pos(11);
    assert_eq!(pos2.line, 0);
    assert_eq!(pos2.character, 9);
}

// ─── json_to_position / position_to_json ─────────────────────────────────────

#[test]
fn json_position_roundtrip() {
    let pos = WirePosition::new(3, 7);
    let json = position_to_json(pos);
    let back = must_some(json_to_position(&json));
    assert_eq!(back, pos);
}

#[test]
fn json_to_position_missing_fields() {
    let val = serde_json::json!({});
    assert!(json_to_position(&val).is_none());
}

#[test]
fn json_to_position_partial_fields() {
    let val = serde_json::json!({"line": 1});
    assert!(json_to_position(&val).is_none());
}

// ─── apply_edit_utf8 ─────────────────────────────────────────────────────────

#[test]
fn apply_edit_utf8_insert() {
    let mut s = "hello world".to_string();
    apply_edit_utf8(&mut s, 5, 5, " beautiful");
    assert_eq!(s, "hello beautiful world");
}

#[test]
fn apply_edit_utf8_delete() {
    let mut s = "hello beautiful world".to_string();
    apply_edit_utf8(&mut s, 5, 15, "");
    assert_eq!(s, "hello world");
}

#[test]
fn apply_edit_utf8_replace() {
    let mut s = "hello world".to_string();
    apply_edit_utf8(&mut s, 6, 11, "Perl");
    assert_eq!(s, "hello Perl");
}

#[test]
fn apply_edit_utf8_non_boundary_is_noop() {
    let mut s = "café".to_string(); // é is at bytes 3-4
    let original = s.clone();
    apply_edit_utf8(&mut s, 4, 4, "x"); // byte 4 is mid-character for 'é'
    // 'é' in UTF-8 is 0xC3 0xA9, so byte 3 is start, byte 5 is after
    // Actually: c(0) a(1) f(2) é(3,4) → byte 4 is inside 'é'
    // The function should bail out if not at char boundary
    // Let's check: is byte 4 a char boundary? café = [99, 97, 102, 195, 169]
    // byte 4 = 169 = continuation byte, NOT a char boundary → should be noop
    assert_eq!(s, original);
}

// ─── newline_count ───────────────────────────────────────────────────────────

#[test]
fn newline_count_empty() {
    assert_eq!(newline_count(""), 0);
}

#[test]
fn newline_count_no_newlines() {
    assert_eq!(newline_count("hello world"), 0);
}

#[test]
fn newline_count_mixed() {
    assert_eq!(newline_count("a\nb\nc\n"), 3);
}

#[test]
fn newline_count_only_newlines() {
    assert_eq!(newline_count("\n\n\n"), 3);
}

// ─── last_line_column_utf8 ───────────────────────────────────────────────────

#[test]
fn last_line_column_no_newline() {
    assert_eq!(last_line_column_utf8("hello"), 5);
}

#[test]
fn last_line_column_trailing_newline() {
    assert_eq!(last_line_column_utf8("hello\n"), 0);
}

#[test]
fn last_line_column_multi_line() {
    assert_eq!(last_line_column_utf8("abc\ndef"), 3);
}

#[test]
fn last_line_column_empty() {
    assert_eq!(last_line_column_utf8(""), 0);
}

// ─── Serde serialization ─────────────────────────────────────────────────────

#[test]
fn bytespan_serde_roundtrip() -> Result<(), serde_json::Error> {
    let span = ByteSpan::new(10, 20);
    let json = serde_json::to_string(&span)?;
    let back: ByteSpan = serde_json::from_str(&json)?;
    assert_eq!(back, span);
    Ok(())
}

#[test]
fn wire_position_serde_roundtrip() -> Result<(), serde_json::Error> {
    let wp = WirePosition::new(5, 10);
    let json = serde_json::to_string(&wp)?;
    let back: WirePosition = serde_json::from_str(&json)?;
    assert_eq!(back, wp);
    Ok(())
}

#[test]
fn wire_range_serde_roundtrip() -> Result<(), serde_json::Error> {
    let wr = WireRange::new(WirePosition::new(1, 2), WirePosition::new(3, 4));
    let json = serde_json::to_string(&wr)?;
    let back: WireRange = serde_json::from_str(&json)?;
    assert_eq!(back, wr);
    Ok(())
}

#[test]
fn wire_location_serde_roundtrip() -> Result<(), serde_json::Error> {
    let loc = WireLocation::new(
        "file:///test.pl".to_string(),
        WireRange::new(WirePosition::new(0, 0), WirePosition::new(1, 5)),
    );
    let json = serde_json::to_string(&loc)?;
    let back: WireLocation = serde_json::from_str(&json)?;
    assert_eq!(back, loc);
    Ok(())
}

#[test]
fn position_serde_roundtrip() -> Result<(), serde_json::Error> {
    let p = Position::new(42, 3, 7);
    let json = serde_json::to_string(&p)?;
    let back: Position = serde_json::from_str(&json)?;
    assert_eq!(back, p);
    Ok(())
}

#[test]
fn range_serde_roundtrip() -> Result<(), serde_json::Error> {
    let r = Range::new(Position::new(0, 1, 1), Position::new(10, 2, 5));
    let json = serde_json::to_string(&r)?;
    let back: Range = serde_json::from_str(&json)?;
    assert_eq!(back, r);
    Ok(())
}

// ─── Cross-type integration ──────────────────────────────────────────────────

#[test]
fn mapper_and_cache_agree_on_ascii() {
    let src = "hello\nworld\nfoo";
    let mapper = PositionMapper::new(src);
    let cache = LineStartsCache::new(src);
    for byte in 0..src.len() {
        let mp = mapper.byte_to_lsp_pos(byte);
        let (cl, cc) = cache.offset_to_position(src, byte);
        assert_eq!((mp.line, mp.character), (cl, cc), "disagreement at byte {byte}");
    }
}

#[test]
fn mapper_and_wire_position_agree() {
    let src = "abc\ndef\nghi";
    let mapper = PositionMapper::new(src);
    for (byte, _) in src.char_indices() {
        let mp = mapper.byte_to_lsp_pos(byte);
        let wp = WirePosition::from_byte_offset(src, byte);
        assert_eq!((mp.line, mp.character), (wp.line, wp.character), "disagreement at byte {byte}");
    }
}

#[test]
fn all_converters_agree_on_utf16_emoji() {
    let src = "a😀b\ncd";
    let mapper = PositionMapper::new(src);
    let cache = LineStartsCache::new(src);
    // Check byte 5 = 'b'
    let mp = mapper.byte_to_lsp_pos(5);
    let (cl, cc) = cache.offset_to_position(src, 5);
    let wp = WirePosition::from_byte_offset(src, 5);
    let (cvl, cvc) = offset_to_utf16_line_col(src, 5);
    assert_eq!(mp.line, cl);
    assert_eq!(mp.character, cc);
    assert_eq!(wp.line, cvl);
    assert_eq!(wp.character, cvc);
    assert_eq!(mp.line, wp.line);
    assert_eq!(mp.character, wp.character);
}
