//! Comprehensive unit tests v2 for perl-edit
//!
//! Covers additional edge cases, interaction patterns, and scenarios
//! beyond the first comprehensive_unit_tests.rs file.

use perl_edit::{Edit, EditSet};
use perl_position_tracking::{Position, Range};
use perl_tdd_support::must_some;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn pos(byte: usize, line: u32, col: u32) -> Position {
    Position::new(byte, line, col)
}

fn edit(sb: usize, oeb: usize, neb: usize, sp: Position, oep: Position, nep: Position) -> Edit {
    Edit::new(sb, oeb, neb, sp, oep, nep)
}

// ===================================================================
// Module 1 – Edit construction & field access
// ===================================================================

#[test]
fn edit_fields_accessible_after_construction() -> Result<(), Box<dyn std::error::Error>> {
    let e = edit(0, 5, 10, pos(0, 1, 1), pos(5, 1, 6), pos(10, 1, 11));
    assert_eq!(e.start_byte, 0);
    assert_eq!(e.old_end_byte, 5);
    assert_eq!(e.new_end_byte, 10);
    assert_eq!(e.start_position, pos(0, 1, 1));
    assert_eq!(e.old_end_position, pos(5, 1, 6));
    assert_eq!(e.new_end_position, pos(10, 1, 11));
    Ok(())
}

#[test]
fn edit_clone_produces_independent_copy() -> Result<(), Box<dyn std::error::Error>> {
    let e1 = edit(0, 5, 10, pos(0, 1, 1), pos(5, 1, 6), pos(10, 1, 11));
    let e2 = e1.clone();
    assert_eq!(e1, e2);
    // Mutating the clone's byte offset via a new instance doesn't affect original
    let e3 = Edit::new(
        e2.start_byte + 1,
        e2.old_end_byte,
        e2.new_end_byte,
        e2.start_position,
        e2.old_end_position,
        e2.new_end_position,
    );
    assert_ne!(e1, e3);
    Ok(())
}

#[test]
fn edit_debug_format_contains_field_names() -> Result<(), Box<dyn std::error::Error>> {
    let e = edit(10, 20, 25, pos(10, 2, 1), pos(20, 2, 11), pos(25, 2, 16));
    let dbg = format!("{e:?}");
    assert!(dbg.contains("start_byte"));
    assert!(dbg.contains("old_end_byte"));
    assert!(dbg.contains("new_end_byte"));
    Ok(())
}

// ===================================================================
// Module 2 – byte_shift edge cases
// ===================================================================

#[test]
fn byte_shift_large_insertion() -> Result<(), Box<dyn std::error::Error>> {
    // Insert 1000 bytes at position 0
    let e = edit(0, 0, 1000, pos(0, 1, 1), pos(0, 1, 1), pos(1000, 1, 1001));
    assert_eq!(e.byte_shift(), 1000);
    Ok(())
}

#[test]
fn byte_shift_large_deletion() -> Result<(), Box<dyn std::error::Error>> {
    let e = edit(0, 500, 0, pos(0, 1, 1), pos(500, 10, 1), pos(0, 1, 1));
    assert_eq!(e.byte_shift(), -500);
    Ok(())
}

#[test]
fn byte_shift_single_char_replacement() -> Result<(), Box<dyn std::error::Error>> {
    // Replace 1 byte with 1 byte → shift 0
    let e = edit(5, 6, 6, pos(5, 1, 6), pos(6, 1, 7), pos(6, 1, 7));
    assert_eq!(e.byte_shift(), 0);
    Ok(())
}

// ===================================================================
// Module 3 – line_shift edge cases
// ===================================================================

#[test]
fn line_shift_large_multiline_insert() -> Result<(), Box<dyn std::error::Error>> {
    // Insert 50 lines
    let e = edit(0, 0, 500, pos(0, 1, 1), pos(0, 1, 1), pos(500, 51, 1));
    assert_eq!(e.line_shift(), 50);
    Ok(())
}

#[test]
fn line_shift_collapse_many_lines() -> Result<(), Box<dyn std::error::Error>> {
    // Remove 100 lines
    let e = edit(0, 2000, 0, pos(0, 1, 1), pos(2000, 101, 1), pos(0, 1, 1));
    assert_eq!(e.line_shift(), -100);
    Ok(())
}

#[test]
fn line_shift_replace_multiline_with_single_line() -> Result<(), Box<dyn std::error::Error>> {
    // 3 lines → 1 line
    let e = edit(10, 40, 20, pos(10, 2, 1), pos(40, 4, 10), pos(20, 2, 11));
    assert_eq!(e.line_shift(), -2);
    Ok(())
}

// ===================================================================
// Module 4 – affects_byte boundary conditions
// ===================================================================

#[test]
fn affects_byte_at_zero() -> Result<(), Box<dyn std::error::Error>> {
    let e = edit(0, 5, 5, pos(0, 1, 1), pos(5, 1, 6), pos(5, 1, 6));
    assert!(e.affects_byte(0));
    Ok(())
}

#[test]
fn affects_byte_far_after() -> Result<(), Box<dyn std::error::Error>> {
    let e = edit(10, 20, 25, pos(10, 2, 1), pos(20, 2, 11), pos(25, 2, 16));
    assert!(e.affects_byte(10000));
    Ok(())
}

#[test]
fn affects_byte_just_before_start() -> Result<(), Box<dyn std::error::Error>> {
    let e = edit(10, 20, 25, pos(10, 2, 1), pos(20, 2, 11), pos(25, 2, 16));
    assert!(!e.affects_byte(9));
    Ok(())
}

// ===================================================================
// Module 5 – overlaps_range edge cases
// ===================================================================

#[test]
fn overlaps_range_zero_length_range_at_edit_start() -> Result<(), Box<dyn std::error::Error>> {
    let e = edit(10, 20, 25, pos(10, 2, 1), pos(20, 2, 11), pos(25, 2, 16));
    let r = Range::new(pos(10, 2, 1), pos(10, 2, 1));
    // Zero-length range: start == end == 10, but start(10) < old_end(20) AND end(10) > start(10)? end==start_byte is not >
    assert!(!e.overlaps_range(&r));
    Ok(())
}

#[test]
fn overlaps_range_zero_length_range_inside_edit() -> Result<(), Box<dyn std::error::Error>> {
    let e = edit(10, 20, 25, pos(10, 2, 1), pos(20, 2, 11), pos(25, 2, 16));
    let r = Range::new(pos(15, 2, 6), pos(15, 2, 6));
    // start(15) < old_end(20) AND end(15) > start(10) → true
    assert!(e.overlaps_range(&r));
    Ok(())
}

#[test]
fn overlaps_range_zero_length_edit_with_overlapping_range() -> Result<(), Box<dyn std::error::Error>>
{
    // Edit at a single point (pure insertion)
    let e = edit(10, 10, 15, pos(10, 2, 1), pos(10, 2, 1), pos(15, 2, 6));
    let r = Range::new(pos(5, 1, 6), pos(15, 2, 6));
    // start(5) < old_end(10) AND end(15) > start(10) → true
    assert!(e.overlaps_range(&r));
    Ok(())
}

#[test]
fn overlaps_range_zero_length_edit_no_overlap() -> Result<(), Box<dyn std::error::Error>> {
    let e = edit(10, 10, 15, pos(10, 2, 1), pos(10, 2, 1), pos(15, 2, 6));
    let r = Range::new(pos(20, 3, 1), pos(25, 3, 6));
    assert!(!e.overlaps_range(&r));
    Ok(())
}

#[test]
fn overlaps_range_single_byte_overlap() -> Result<(), Box<dyn std::error::Error>> {
    let e = edit(10, 20, 25, pos(10, 2, 1), pos(20, 2, 11), pos(25, 2, 16));
    let r = Range::new(pos(19, 2, 10), pos(21, 2, 12));
    assert!(e.overlaps_range(&r));
    Ok(())
}

// ===================================================================
// Module 6 – apply_to_position: column adjustment on same line
// ===================================================================

#[test]
fn apply_position_same_line_column_grows() -> Result<(), Box<dyn std::error::Error>> {
    // Replace "ab" with "abcde" on line 3 → column shift +3
    let e = edit(20, 22, 25, pos(20, 3, 5), pos(22, 3, 7), pos(25, 3, 10));
    let p = pos(30, 3, 15);
    let np = must_some(e.apply_to_position(p));
    assert_eq!(np.byte, 33); // 30 + 3
    assert_eq!(np.line, 3);
    assert_eq!(np.column, 18); // 15 + 3
    Ok(())
}

#[test]
fn apply_position_same_line_column_shrinks() -> Result<(), Box<dyn std::error::Error>> {
    // Replace "abcde" with "ab" on line 3 → column shift -3
    let e = edit(20, 25, 22, pos(20, 3, 5), pos(25, 3, 10), pos(22, 3, 7));
    let p = pos(30, 3, 15);
    let np = must_some(e.apply_to_position(p));
    assert_eq!(np.byte, 27); // 30 - 3
    assert_eq!(np.line, 3);
    assert_eq!(np.column, 12); // 15 - 3
    Ok(())
}

#[test]
fn apply_position_different_line_column_unchanged() -> Result<(), Box<dyn std::error::Error>> {
    // Edit on line 3, position on line 5 → column should not change
    let e = edit(20, 25, 22, pos(20, 3, 5), pos(25, 3, 10), pos(22, 3, 7));
    let p = pos(50, 5, 15);
    let np = must_some(e.apply_to_position(p));
    assert_eq!(np.column, 15);
    Ok(())
}

// ===================================================================
// Module 7 – apply_to_position: boundary positions
// ===================================================================

#[test]
fn apply_position_exactly_at_old_end() -> Result<(), Box<dyn std::error::Error>> {
    let e = edit(10, 20, 25, pos(10, 2, 1), pos(20, 2, 11), pos(25, 2, 16));
    // pos.byte == old_end_byte → "after" branch
    let p = pos(20, 2, 11);
    let np = must_some(e.apply_to_position(p));
    assert_eq!(np.byte, 25);
    Ok(())
}

#[test]
fn apply_position_one_byte_before_old_end() -> Result<(), Box<dyn std::error::Error>> {
    let e = edit(10, 20, 25, pos(10, 2, 1), pos(20, 2, 11), pos(25, 2, 16));
    let p = pos(19, 2, 10);
    // Inside edit → None
    assert!(e.apply_to_position(p).is_none());
    Ok(())
}

#[test]
fn apply_position_one_byte_after_old_end() -> Result<(), Box<dyn std::error::Error>> {
    let e = edit(10, 20, 25, pos(10, 2, 1), pos(20, 2, 11), pos(25, 2, 16));
    let p = pos(21, 2, 12);
    let np = must_some(e.apply_to_position(p));
    assert_eq!(np.byte, 26);
    Ok(())
}

// ===================================================================
// Module 8 – apply_to_range: various configurations
// ===================================================================

#[test]
fn apply_range_spanning_across_edit_returns_none() -> Result<(), Box<dyn std::error::Error>> {
    let e = edit(10, 20, 25, pos(10, 2, 1), pos(20, 2, 11), pos(25, 2, 16));
    // Range straddles the edit: start before, end inside
    let r = Range::new(pos(5, 1, 6), pos(15, 2, 6));
    assert!(e.apply_to_range(&r).is_none());
    Ok(())
}

#[test]
fn apply_range_entirely_before_edit_unchanged() -> Result<(), Box<dyn std::error::Error>> {
    let e = edit(50, 60, 65, pos(50, 5, 1), pos(60, 5, 11), pos(65, 5, 16));
    let r = Range::new(pos(0, 1, 1), pos(10, 1, 11));
    let nr = must_some(e.apply_to_range(&r));
    assert_eq!(nr.start, r.start);
    assert_eq!(nr.end, r.end);
    Ok(())
}

#[test]
fn apply_range_entirely_after_edit_shifted() -> Result<(), Box<dyn std::error::Error>> {
    let e = edit(10, 20, 25, pos(10, 2, 1), pos(20, 2, 11), pos(25, 2, 16));
    let r = Range::new(pos(30, 4, 1), pos(40, 4, 11));
    let nr = must_some(e.apply_to_range(&r));
    assert_eq!(nr.start.byte, 35);
    assert_eq!(nr.end.byte, 45);
    Ok(())
}

#[test]
fn apply_range_zero_length_before_edit() -> Result<(), Box<dyn std::error::Error>> {
    let e = edit(10, 20, 25, pos(10, 2, 1), pos(20, 2, 11), pos(25, 2, 16));
    let r = Range::new(pos(5, 1, 6), pos(5, 1, 6));
    let nr = must_some(e.apply_to_range(&r));
    assert_eq!(nr.start, r.start);
    assert_eq!(nr.end, r.end);
    Ok(())
}

#[test]
fn apply_range_zero_length_after_edit() -> Result<(), Box<dyn std::error::Error>> {
    let e = edit(10, 20, 25, pos(10, 2, 1), pos(20, 2, 11), pos(25, 2, 16));
    let r = Range::new(pos(25, 3, 1), pos(25, 3, 1));
    let nr = must_some(e.apply_to_range(&r));
    assert_eq!(nr.start.byte, 30);
    assert_eq!(nr.end.byte, 30);
    Ok(())
}

// ===================================================================
// Module 9 – EditSet: insertion ordering
// ===================================================================

#[test]
fn edit_set_add_reverse_order_still_sorted() -> Result<(), Box<dyn std::error::Error>> {
    let mut es = EditSet::new();
    es.add(edit(50, 55, 60, pos(50, 5, 1), pos(55, 5, 6), pos(60, 5, 11)));
    es.add(edit(30, 35, 40, pos(30, 3, 1), pos(35, 3, 6), pos(40, 3, 11)));
    es.add(edit(10, 15, 20, pos(10, 2, 1), pos(15, 2, 6), pos(20, 2, 11)));
    let edits = es.edits();
    assert_eq!(edits.len(), 3);
    assert_eq!(edits[0].start_byte, 10);
    assert_eq!(edits[1].start_byte, 30);
    assert_eq!(edits[2].start_byte, 50);
    Ok(())
}

#[test]
fn edit_set_add_interleaved_order() -> Result<(), Box<dyn std::error::Error>> {
    let mut es = EditSet::new();
    es.add(edit(20, 25, 30, pos(20, 2, 1), pos(25, 2, 6), pos(30, 2, 11)));
    es.add(edit(10, 15, 20, pos(10, 1, 11), pos(15, 1, 16), pos(20, 1, 21)));
    es.add(edit(40, 45, 50, pos(40, 4, 1), pos(45, 4, 6), pos(50, 4, 11)));
    es.add(edit(30, 35, 40, pos(30, 3, 1), pos(35, 3, 6), pos(40, 3, 11)));
    let edits = es.edits();
    for i in 1..edits.len() {
        assert!(edits[i - 1].start_byte <= edits[i].start_byte);
    }
    Ok(())
}

// ===================================================================
// Module 10 – EditSet: len / is_empty
// ===================================================================

#[test]
fn edit_set_len_after_multiple_adds() -> Result<(), Box<dyn std::error::Error>> {
    let mut es = EditSet::new();
    assert_eq!(es.len(), 0);
    for i in 0..5 {
        es.add(edit(
            i * 10,
            i * 10 + 5,
            i * 10 + 7,
            pos(i * 10, 1, 1),
            pos(i * 10 + 5, 1, 6),
            pos(i * 10 + 7, 1, 8),
        ));
    }
    assert_eq!(es.len(), 5);
    assert!(!es.is_empty());
    Ok(())
}

#[test]
fn edit_set_is_empty_for_new_set() -> Result<(), Box<dyn std::error::Error>> {
    let es = EditSet::new();
    assert!(es.is_empty());
    assert_eq!(es.len(), 0);
    Ok(())
}

#[test]
fn edit_set_default_is_same_as_new() -> Result<(), Box<dyn std::error::Error>> {
    let a = EditSet::new();
    let b = EditSet::default();
    assert_eq!(a.len(), b.len());
    assert!(a.is_empty());
    assert!(b.is_empty());
    Ok(())
}

// ===================================================================
// Module 11 – EditSet: cumulative shifts with multiple edits
// ===================================================================

#[test]
fn edit_set_apply_position_two_insertions_cumulative() -> Result<(), Box<dyn std::error::Error>> {
    let mut es = EditSet::new();
    // Insert 5 bytes at offset 10
    es.add(edit(10, 10, 15, pos(10, 2, 1), pos(10, 2, 1), pos(15, 2, 6)));
    // Insert 3 bytes at offset 30 (in original coordinates)
    es.add(edit(30, 30, 33, pos(30, 3, 1), pos(30, 3, 1), pos(33, 3, 4)));
    // Position at byte 40 should shift by +5 from first, +3 from second = +8
    let p = pos(40, 4, 1);
    let np = must_some(es.apply_to_position(p));
    assert_eq!(np.byte, 48);
    Ok(())
}

#[test]
fn edit_set_apply_position_insertion_then_deletion() -> Result<(), Box<dyn std::error::Error>> {
    let mut es = EditSet::new();
    // Insert 10 bytes at offset 5
    es.add(edit(5, 5, 15, pos(5, 1, 6), pos(5, 1, 6), pos(15, 1, 16)));
    // Delete 4 bytes at offset 30
    es.add(edit(30, 34, 30, pos(30, 3, 1), pos(34, 3, 5), pos(30, 3, 1)));
    // Position at byte 50 → +10 - 4 = +6
    let p = pos(50, 5, 1);
    let np = must_some(es.apply_to_position(p));
    assert_eq!(np.byte, 56);
    Ok(())
}

// ===================================================================
// Module 12 – EditSet: byte_shift_at
// ===================================================================

#[test]
fn byte_shift_at_position_before_first_edit() -> Result<(), Box<dyn std::error::Error>> {
    let mut es = EditSet::new();
    es.add(edit(10, 15, 20, pos(10, 2, 1), pos(15, 2, 6), pos(20, 2, 11)));
    assert_eq!(es.byte_shift_at(5), 0);
    Ok(())
}

#[test]
fn byte_shift_at_position_at_first_edit_old_end() -> Result<(), Box<dyn std::error::Error>> {
    let mut es = EditSet::new();
    es.add(edit(10, 15, 20, pos(10, 2, 1), pos(15, 2, 6), pos(20, 2, 11)));
    // At byte 15 (old_end_byte), edit is included
    assert_eq!(es.byte_shift_at(15), 5);
    Ok(())
}

#[test]
fn byte_shift_at_between_two_edits() -> Result<(), Box<dyn std::error::Error>> {
    let mut es = EditSet::new();
    es.add(edit(10, 15, 20, pos(10, 2, 1), pos(15, 2, 6), pos(20, 2, 11)));
    es.add(edit(50, 55, 60, pos(50, 5, 1), pos(55, 5, 6), pos(60, 5, 11)));
    // Between edits: only first edit contributes
    assert_eq!(es.byte_shift_at(30), 5);
    Ok(())
}

#[test]
fn byte_shift_at_after_all_edits() -> Result<(), Box<dyn std::error::Error>> {
    let mut es = EditSet::new();
    es.add(edit(10, 15, 20, pos(10, 2, 1), pos(15, 2, 6), pos(20, 2, 11)));
    es.add(edit(50, 55, 52, pos(50, 5, 1), pos(55, 5, 6), pos(52, 5, 3)));
    // After all: +5 + (-3) = +2
    assert_eq!(es.byte_shift_at(100), 2);
    Ok(())
}

// ===================================================================
// Module 13 – EditSet: affected_ranges
// ===================================================================

#[test]
fn affected_ranges_multiple_edits() -> Result<(), Box<dyn std::error::Error>> {
    let mut es = EditSet::new();
    es.add(edit(10, 20, 25, pos(10, 2, 1), pos(20, 2, 11), pos(25, 2, 16)));
    es.add(edit(30, 40, 35, pos(30, 3, 1), pos(40, 3, 11), pos(35, 3, 6)));
    let ranges = es.affected_ranges();
    assert_eq!(ranges.len(), 2);
    assert_eq!(ranges[0].start.byte, 10);
    assert_eq!(ranges[0].end.byte, 20);
    assert_eq!(ranges[1].start.byte, 30);
    assert_eq!(ranges[1].end.byte, 40);
    Ok(())
}

#[test]
fn affected_ranges_pure_insertion_is_zero_length() -> Result<(), Box<dyn std::error::Error>> {
    let mut es = EditSet::new();
    es.add(edit(10, 10, 20, pos(10, 2, 1), pos(10, 2, 1), pos(20, 2, 11)));
    let ranges = es.affected_ranges();
    assert_eq!(ranges.len(), 1);
    assert_eq!(ranges[0].start, ranges[0].end);
    Ok(())
}

// ===================================================================
// Module 14 – EditSet: affects_range
// ===================================================================

#[test]
fn affects_range_with_range_between_two_edits() -> Result<(), Box<dyn std::error::Error>> {
    let mut es = EditSet::new();
    es.add(edit(10, 15, 20, pos(10, 2, 1), pos(15, 2, 6), pos(20, 2, 11)));
    es.add(edit(50, 55, 60, pos(50, 5, 1), pos(55, 5, 6), pos(60, 5, 11)));
    // Range between the two edits
    let r = Range::new(pos(20, 3, 1), pos(40, 4, 11));
    assert!(!es.affects_range(&r));
    Ok(())
}

#[test]
fn affects_range_with_range_overlapping_second_edit() -> Result<(), Box<dyn std::error::Error>> {
    let mut es = EditSet::new();
    es.add(edit(10, 15, 20, pos(10, 2, 1), pos(15, 2, 6), pos(20, 2, 11)));
    es.add(edit(50, 55, 60, pos(50, 5, 1), pos(55, 5, 6), pos(60, 5, 11)));
    // Range overlapping second edit
    let r = Range::new(pos(45, 4, 16), pos(52, 5, 3));
    assert!(es.affects_range(&r));
    Ok(())
}

// ===================================================================
// Module 15 – EditSet: apply_to_range cumulative
// ===================================================================

#[test]
fn edit_set_apply_range_cumulative_two_insertions() -> Result<(), Box<dyn std::error::Error>> {
    let mut es = EditSet::new();
    es.add(edit(5, 5, 10, pos(5, 1, 6), pos(5, 1, 6), pos(10, 1, 11)));
    es.add(edit(20, 20, 23, pos(20, 2, 1), pos(20, 2, 1), pos(23, 2, 4)));
    let r = Range::new(pos(30, 3, 1), pos(40, 3, 11));
    let nr = must_some(es.apply_to_range(r));
    assert_eq!(nr.start.byte, 38); // 30 + 5 + 3
    assert_eq!(nr.end.byte, 48); // 40 + 5 + 3
    Ok(())
}

#[test]
fn edit_set_apply_range_returns_none_when_start_in_edit() -> Result<(), Box<dyn std::error::Error>>
{
    let mut es = EditSet::new();
    es.add(edit(10, 20, 25, pos(10, 2, 1), pos(20, 2, 11), pos(25, 2, 16)));
    let r = Range::new(pos(15, 2, 6), pos(30, 3, 1));
    assert!(es.apply_to_range(r).is_none());
    Ok(())
}

// ===================================================================
// Module 16 – Pure insertion (zero-length old range)
// ===================================================================

#[test]
fn pure_insertion_byte_shift_equals_new_text_length() -> Result<(), Box<dyn std::error::Error>> {
    let e = edit(10, 10, 15, pos(10, 2, 1), pos(10, 2, 1), pos(15, 2, 6));
    assert_eq!(e.byte_shift(), 5);
    assert_eq!(e.line_shift(), 0);
    Ok(())
}

#[test]
fn pure_insertion_multiline() -> Result<(), Box<dyn std::error::Error>> {
    // Insert 2 new lines at offset 10
    let e = edit(10, 10, 30, pos(10, 2, 1), pos(10, 2, 1), pos(30, 4, 1));
    assert_eq!(e.byte_shift(), 20);
    assert_eq!(e.line_shift(), 2);
    Ok(())
}

#[test]
fn pure_insertion_does_not_invalidate_position_at_start() -> Result<(), Box<dyn std::error::Error>>
{
    let e = edit(10, 10, 15, pos(10, 2, 1), pos(10, 2, 1), pos(15, 2, 6));
    // Position at start_byte (10) = old_end_byte (10), so it's "at old_end" → shifted
    let p = pos(10, 2, 1);
    let np = must_some(e.apply_to_position(p));
    assert_eq!(np.byte, 15);
    Ok(())
}

// ===================================================================
// Module 17 – Pure deletion (zero-length new range)
// ===================================================================

#[test]
fn pure_deletion_byte_shift_is_negative() -> Result<(), Box<dyn std::error::Error>> {
    let e = edit(5, 15, 5, pos(5, 1, 6), pos(15, 1, 16), pos(5, 1, 6));
    assert_eq!(e.byte_shift(), -10);
    Ok(())
}

#[test]
fn pure_deletion_multiline() -> Result<(), Box<dyn std::error::Error>> {
    let e = edit(5, 50, 5, pos(5, 1, 6), pos(50, 5, 1), pos(5, 1, 6));
    assert_eq!(e.byte_shift(), -45);
    assert_eq!(e.line_shift(), -4);
    Ok(())
}

#[test]
fn pure_deletion_position_after_deletion_shifted() -> Result<(), Box<dyn std::error::Error>> {
    let e = edit(5, 15, 5, pos(5, 1, 6), pos(15, 1, 16), pos(5, 1, 6));
    let p = pos(20, 1, 21);
    let np = must_some(e.apply_to_position(p));
    assert_eq!(np.byte, 10); // 20 - 10
    Ok(())
}

#[test]
fn pure_deletion_position_inside_deleted_range_returns_none()
-> Result<(), Box<dyn std::error::Error>> {
    let e = edit(5, 15, 5, pos(5, 1, 6), pos(15, 1, 16), pos(5, 1, 6));
    let p = pos(10, 1, 11);
    assert!(e.apply_to_position(p).is_none());
    Ok(())
}

// ===================================================================
// Module 18 – Edit at file start (byte 0)
// ===================================================================

#[test]
fn edit_at_file_start_insertion() -> Result<(), Box<dyn std::error::Error>> {
    let e = edit(0, 0, 10, pos(0, 1, 1), pos(0, 1, 1), pos(10, 1, 11));
    assert_eq!(e.byte_shift(), 10);
    let p = pos(0, 1, 1);
    let np = must_some(e.apply_to_position(p));
    assert_eq!(np.byte, 10);
    Ok(())
}

#[test]
fn edit_at_file_start_replacement() -> Result<(), Box<dyn std::error::Error>> {
    let e = edit(0, 5, 3, pos(0, 1, 1), pos(5, 1, 6), pos(3, 1, 4));
    assert_eq!(e.byte_shift(), -2);
    let p = pos(10, 1, 11);
    let np = must_some(e.apply_to_position(p));
    assert_eq!(np.byte, 8);
    Ok(())
}

// ===================================================================
// Module 19 – Edit equality
// ===================================================================

#[test]
fn edit_equality_same_values() -> Result<(), Box<dyn std::error::Error>> {
    let e1 = edit(0, 5, 10, pos(0, 1, 1), pos(5, 1, 6), pos(10, 1, 11));
    let e2 = edit(0, 5, 10, pos(0, 1, 1), pos(5, 1, 6), pos(10, 1, 11));
    assert_eq!(e1, e2);
    Ok(())
}

#[test]
fn edit_inequality_different_start_byte() -> Result<(), Box<dyn std::error::Error>> {
    let e1 = edit(0, 5, 10, pos(0, 1, 1), pos(5, 1, 6), pos(10, 1, 11));
    let e2 = edit(1, 5, 10, pos(1, 1, 2), pos(5, 1, 6), pos(10, 1, 11));
    assert_ne!(e1, e2);
    Ok(())
}

#[test]
fn edit_inequality_different_new_end_byte() -> Result<(), Box<dyn std::error::Error>> {
    let e1 = edit(0, 5, 10, pos(0, 1, 1), pos(5, 1, 6), pos(10, 1, 11));
    let e2 = edit(0, 5, 11, pos(0, 1, 1), pos(5, 1, 6), pos(11, 1, 12));
    assert_ne!(e1, e2);
    Ok(())
}

// ===================================================================
// Module 20 – EditSet with many edits
// ===================================================================

#[test]
fn edit_set_ten_sequential_insertions() -> Result<(), Box<dyn std::error::Error>> {
    let mut es = EditSet::new();
    for i in 0..10 {
        let base = i * 20;
        es.add(edit(
            base,
            base,
            base + 5,
            pos(base, (i + 1) as u32, 1),
            pos(base, (i + 1) as u32, 1),
            pos(base + 5, (i + 1) as u32, 6),
        ));
    }
    assert_eq!(es.len(), 10);
    // Total byte shift: 10 * 5 = 50
    assert_eq!(es.byte_shift_at(1000), 50);
    Ok(())
}

#[test]
fn edit_set_alternating_insert_delete() -> Result<(), Box<dyn std::error::Error>> {
    let mut es = EditSet::new();
    // Insert 5 bytes at offset 10
    es.add(edit(10, 10, 15, pos(10, 2, 1), pos(10, 2, 1), pos(15, 2, 6)));
    // Delete 5 bytes at offset 50
    es.add(edit(50, 55, 50, pos(50, 5, 1), pos(55, 5, 6), pos(50, 5, 1)));
    // Net shift: +5 -5 = 0
    assert_eq!(es.byte_shift_at(100), 0);
    Ok(())
}

// ===================================================================
// Module 21 – EditSet::edits() accessor
// ===================================================================

#[test]
fn edits_accessor_returns_all_in_sorted_order() -> Result<(), Box<dyn std::error::Error>> {
    let mut es = EditSet::new();
    es.add(edit(30, 35, 40, pos(30, 3, 1), pos(35, 3, 6), pos(40, 3, 11)));
    es.add(edit(10, 15, 20, pos(10, 1, 11), pos(15, 1, 16), pos(20, 1, 21)));
    let edits = es.edits();
    assert_eq!(edits.len(), 2);
    assert_eq!(edits[0].start_byte, 10);
    assert_eq!(edits[1].start_byte, 30);
    Ok(())
}

#[test]
fn edits_accessor_empty_set() -> Result<(), Box<dyn std::error::Error>> {
    let es = EditSet::new();
    assert!(es.edits().is_empty());
    Ok(())
}

// ===================================================================
// Module 22 – Multiline edit position adjustments
// ===================================================================

#[test]
fn multiline_edit_position_on_same_end_line_adjusts_column()
-> Result<(), Box<dyn std::error::Error>> {
    // Edit replaces lines 2-4, ending on line 3 in new text
    let e = edit(10, 50, 30, pos(10, 2, 1), pos(50, 4, 10), pos(30, 3, 5));
    // Position on old line 4 (same as old_end_position.line)
    let p = pos(55, 4, 15);
    let np = must_some(e.apply_to_position(p));
    // Column should adjust: 15 + (5 - 10) = 10
    assert_eq!(np.column, 10);
    assert_eq!(np.line, 3); // 4 + (3 - 4) = 3
    Ok(())
}

#[test]
fn multiline_edit_position_on_later_line_keeps_column() -> Result<(), Box<dyn std::error::Error>> {
    let e = edit(10, 50, 30, pos(10, 2, 1), pos(50, 4, 10), pos(30, 3, 5));
    // Position on line 6 (different from old_end line 4)
    let p = pos(70, 6, 20);
    let np = must_some(e.apply_to_position(p));
    assert_eq!(np.column, 20); // unchanged
    assert_eq!(np.line, 5); // 6 + (3 - 4) = 5
    Ok(())
}

// ===================================================================
// Module 23 – Regression-style: realistic editing scenarios
// ===================================================================

#[test]
fn scenario_rename_variable_same_line() -> Result<(), Box<dyn std::error::Error>> {
    // Rename "$foo" (4 bytes) to "$foobar" (7 bytes) at col 5 on line 3
    let e = edit(20, 24, 27, pos(20, 3, 5), pos(24, 3, 9), pos(27, 3, 12));
    assert_eq!(e.byte_shift(), 3);
    // Cursor right after old variable end
    let cursor = pos(24, 3, 9);
    let nc = must_some(e.apply_to_position(cursor));
    assert_eq!(nc.byte, 27);
    assert_eq!(nc.column, 12);
    Ok(())
}

#[test]
fn scenario_delete_entire_line() -> Result<(), Box<dyn std::error::Error>> {
    // Delete line 3 (bytes 20-40, ends with newline)
    let e = edit(20, 40, 20, pos(20, 3, 1), pos(40, 4, 1), pos(20, 3, 1));
    assert_eq!(e.byte_shift(), -20);
    assert_eq!(e.line_shift(), -1);
    // Position on line 5 → should become line 4
    let p = pos(60, 5, 10);
    let np = must_some(e.apply_to_position(p));
    assert_eq!(np.line, 4);
    assert_eq!(np.byte, 40);
    Ok(())
}

#[test]
fn scenario_insert_new_line_before() -> Result<(), Box<dyn std::error::Error>> {
    // Insert a newline + text before line 3 (at byte 20)
    let e = edit(20, 20, 35, pos(20, 3, 1), pos(20, 3, 1), pos(35, 4, 1));
    assert_eq!(e.line_shift(), 1);
    // Position on line 3 col 5 → line 4 col 5
    let p = pos(25, 3, 6);
    let np = must_some(e.apply_to_position(p));
    assert_eq!(np.line, 4);
    Ok(())
}

// ===================================================================
// Module 24 – EditSet: clone
// ===================================================================

#[test]
fn edit_set_clone_has_same_edits() -> Result<(), Box<dyn std::error::Error>> {
    let mut es = EditSet::new();
    es.add(edit(10, 15, 20, pos(10, 2, 1), pos(15, 2, 6), pos(20, 2, 11)));
    es.add(edit(30, 35, 40, pos(30, 3, 1), pos(35, 3, 6), pos(40, 3, 11)));
    let es2 = es.clone();
    assert_eq!(es.len(), es2.len());
    assert_eq!(es.edits()[0], es2.edits()[0]);
    assert_eq!(es.edits()[1], es2.edits()[1]);
    Ok(())
}

// ===================================================================
// Module 25 – Edge case: maximum values
// ===================================================================

#[test]
fn edit_with_large_byte_offsets() -> Result<(), Box<dyn std::error::Error>> {
    let large = 1_000_000;
    let e = edit(
        large,
        large + 100,
        large + 200,
        pos(large, 10000, 50),
        pos(large + 100, 10000, 150),
        pos(large + 200, 10000, 250),
    );
    assert_eq!(e.byte_shift(), 100);
    let p = pos(large + 200, 10001, 1);
    let np = must_some(e.apply_to_position(p));
    assert_eq!(np.byte, large + 300);
    Ok(())
}

#[test]
fn edit_set_byte_shift_at_zero_with_edit_at_zero() -> Result<(), Box<dyn std::error::Error>> {
    let mut es = EditSet::new();
    es.add(edit(0, 0, 5, pos(0, 1, 1), pos(0, 1, 1), pos(5, 1, 6)));
    // old_end_byte is 0 which is <= 0, so edit is included
    assert_eq!(es.byte_shift_at(0), 5);
    Ok(())
}
