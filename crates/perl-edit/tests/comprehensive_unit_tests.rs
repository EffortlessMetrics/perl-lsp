//! Comprehensive unit tests for `perl-edit` crate.
//!
//! Covers: Edit construction, byte/line shifts, position application,
//! range application, EditSet ordering, cumulative shifts, and edge cases.

use perl_edit::{Edit, EditSet};
use perl_position_tracking::{Position, Range};
use perl_tdd_support::must_some;

// ---------------------------------------------------------------------------
// Helper: build an Edit from compact tuples
// ---------------------------------------------------------------------------

fn pos(byte: usize, line: u32, col: u32) -> Position {
    Position::new(byte, line, col)
}

fn edit(
    start_byte: usize,
    old_end_byte: usize,
    new_end_byte: usize,
    start: Position,
    old_end: Position,
    new_end: Position,
) -> Edit {
    Edit::new(start_byte, old_end_byte, new_end_byte, start, old_end, new_end)
}

// ===========================================================================
// Edit – construction & field access
// ===========================================================================

#[test]
fn edit_new_stores_all_fields() -> Result<(), Box<dyn std::error::Error>> {
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
fn edit_clone_and_eq() -> Result<(), Box<dyn std::error::Error>> {
    let e = edit(1, 2, 3, pos(1, 1, 2), pos(2, 1, 3), pos(3, 1, 4));
    let cloned = e.clone();
    assert_eq!(e, cloned);
    Ok(())
}

#[test]
fn edit_debug_is_nonempty() -> Result<(), Box<dyn std::error::Error>> {
    let e = edit(0, 1, 2, pos(0, 1, 1), pos(1, 1, 2), pos(2, 1, 3));
    let debug = format!("{e:?}");
    assert!(!debug.is_empty());
    Ok(())
}

// ===========================================================================
// Edit – byte_shift / line_shift
// ===========================================================================

#[test]
fn byte_shift_positive_when_insertion_larger() -> Result<(), Box<dyn std::error::Error>> {
    // Replace 5 bytes with 8 → +3
    let e = edit(10, 15, 18, pos(10, 2, 1), pos(15, 2, 6), pos(18, 2, 9));
    assert_eq!(e.byte_shift(), 3);
    Ok(())
}

#[test]
fn byte_shift_negative_when_deletion() -> Result<(), Box<dyn std::error::Error>> {
    // Replace 10 bytes with 3 → -7
    let e = edit(5, 15, 8, pos(5, 1, 6), pos(15, 1, 16), pos(8, 1, 9));
    assert_eq!(e.byte_shift(), -7);
    Ok(())
}

#[test]
fn byte_shift_zero_for_same_size_replacement() -> Result<(), Box<dyn std::error::Error>> {
    let e = edit(0, 5, 5, pos(0, 1, 1), pos(5, 1, 6), pos(5, 1, 6));
    assert_eq!(e.byte_shift(), 0);
    Ok(())
}

#[test]
fn line_shift_positive_when_lines_added() -> Result<(), Box<dyn std::error::Error>> {
    let e = edit(0, 10, 20, pos(0, 1, 1), pos(10, 1, 11), pos(20, 3, 5));
    assert_eq!(e.line_shift(), 2);
    Ok(())
}

#[test]
fn line_shift_negative_when_lines_removed() -> Result<(), Box<dyn std::error::Error>> {
    let e = edit(0, 30, 10, pos(0, 1, 1), pos(30, 5, 1), pos(10, 2, 1));
    assert_eq!(e.line_shift(), -3);
    Ok(())
}

#[test]
fn line_shift_zero_for_same_line() -> Result<(), Box<dyn std::error::Error>> {
    let e = edit(0, 5, 8, pos(0, 1, 1), pos(5, 1, 6), pos(8, 1, 9));
    assert_eq!(e.line_shift(), 0);
    Ok(())
}

// ===========================================================================
// Edit – affects_byte
// ===========================================================================

#[test]
fn affects_byte_before_start_is_false() -> Result<(), Box<dyn std::error::Error>> {
    let e = edit(10, 15, 17, pos(10, 2, 1), pos(15, 2, 6), pos(17, 2, 8));
    assert!(!e.affects_byte(9));
    Ok(())
}

#[test]
fn affects_byte_at_start_is_true() -> Result<(), Box<dyn std::error::Error>> {
    let e = edit(10, 15, 17, pos(10, 2, 1), pos(15, 2, 6), pos(17, 2, 8));
    assert!(e.affects_byte(10));
    Ok(())
}

#[test]
fn affects_byte_after_start_is_true() -> Result<(), Box<dyn std::error::Error>> {
    let e = edit(10, 15, 17, pos(10, 2, 1), pos(15, 2, 6), pos(17, 2, 8));
    assert!(e.affects_byte(100));
    Ok(())
}

// ===========================================================================
// Edit – overlaps_range
// ===========================================================================

#[test]
fn overlaps_range_entirely_before() -> Result<(), Box<dyn std::error::Error>> {
    let e = edit(10, 15, 17, pos(10, 2, 1), pos(15, 2, 6), pos(17, 2, 8));
    let r = Range::new(pos(0, 1, 1), pos(9, 1, 10));
    assert!(!e.overlaps_range(&r));
    Ok(())
}

#[test]
fn overlaps_range_entirely_after() -> Result<(), Box<dyn std::error::Error>> {
    let e = edit(10, 15, 17, pos(10, 2, 1), pos(15, 2, 6), pos(17, 2, 8));
    let r = Range::new(pos(20, 3, 1), pos(25, 3, 6));
    assert!(!e.overlaps_range(&r));
    Ok(())
}

#[test]
fn overlaps_range_touching_at_start_boundary() -> Result<(), Box<dyn std::error::Error>> {
    // range.end == edit.start_byte → no overlap (end is exclusive)
    let e = edit(10, 15, 17, pos(10, 2, 1), pos(15, 2, 6), pos(17, 2, 8));
    let r = Range::new(pos(5, 1, 6), pos(10, 2, 1));
    assert!(!e.overlaps_range(&r));
    Ok(())
}

#[test]
fn overlaps_range_touching_at_end_boundary() -> Result<(), Box<dyn std::error::Error>> {
    // range.start == edit.old_end_byte → no overlap
    let e = edit(10, 15, 17, pos(10, 2, 1), pos(15, 2, 6), pos(17, 2, 8));
    let r = Range::new(pos(15, 2, 6), pos(20, 3, 1));
    assert!(!e.overlaps_range(&r));
    Ok(())
}

#[test]
fn overlaps_range_partial_overlap_start() -> Result<(), Box<dyn std::error::Error>> {
    let e = edit(10, 15, 17, pos(10, 2, 1), pos(15, 2, 6), pos(17, 2, 8));
    let r = Range::new(pos(5, 1, 6), pos(12, 2, 3));
    assert!(e.overlaps_range(&r));
    Ok(())
}

#[test]
fn overlaps_range_fully_contained() -> Result<(), Box<dyn std::error::Error>> {
    let e = edit(10, 20, 25, pos(10, 2, 1), pos(20, 3, 1), pos(25, 3, 6));
    let r = Range::new(pos(12, 2, 3), pos(18, 2, 9));
    assert!(e.overlaps_range(&r));
    Ok(())
}

#[test]
fn overlaps_range_fully_surrounding() -> Result<(), Box<dyn std::error::Error>> {
    let e = edit(10, 15, 17, pos(10, 2, 1), pos(15, 2, 6), pos(17, 2, 8));
    let r = Range::new(pos(5, 1, 6), pos(20, 3, 1));
    assert!(e.overlaps_range(&r));
    Ok(())
}

// ===========================================================================
// Edit – apply_to_position
// ===========================================================================

#[test]
fn apply_position_before_edit_unchanged() -> Result<(), Box<dyn std::error::Error>> {
    let e = edit(10, 15, 17, pos(10, 2, 5), pos(15, 2, 10), pos(17, 2, 12));
    let p = pos(5, 1, 6);
    let result = must_some(e.apply_to_position(p));
    assert_eq!(result, p);
    Ok(())
}

#[test]
fn apply_position_at_start_byte_returns_none() -> Result<(), Box<dyn std::error::Error>> {
    let e = edit(10, 15, 17, pos(10, 2, 5), pos(15, 2, 10), pos(17, 2, 12));
    // byte 10 is >= start_byte (10) and < old_end_byte (15) → within edit
    assert!(e.apply_to_position(pos(10, 2, 5)).is_none());
    Ok(())
}

#[test]
fn apply_position_inside_edit_returns_none() -> Result<(), Box<dyn std::error::Error>> {
    let e = edit(10, 15, 17, pos(10, 2, 5), pos(15, 2, 10), pos(17, 2, 12));
    assert!(e.apply_to_position(pos(12, 2, 7)).is_none());
    Ok(())
}

#[test]
fn apply_position_at_old_end_byte_is_shifted() -> Result<(), Box<dyn std::error::Error>> {
    let e = edit(10, 15, 17, pos(10, 2, 5), pos(15, 2, 10), pos(17, 2, 12));
    let result = must_some(e.apply_to_position(pos(15, 2, 10)));
    // byte: 15 + 2 = 17, line: 2 + 0 = 2, col: 10 + 2 = 12
    assert_eq!(result.byte, 17);
    assert_eq!(result.line, 2);
    assert_eq!(result.column, 12);
    Ok(())
}

#[test]
fn apply_position_after_edit_same_line_adjusts_column() -> Result<(), Box<dyn std::error::Error>> {
    let e = edit(10, 15, 17, pos(10, 2, 5), pos(15, 2, 10), pos(17, 2, 12));
    let p = pos(20, 2, 15);
    let result = must_some(e.apply_to_position(p));
    assert_eq!(result.byte, 22);
    assert_eq!(result.line, 2);
    assert_eq!(result.column, 17); // 15 + (12-10) = 17
    Ok(())
}

#[test]
fn apply_position_after_edit_different_line_keeps_column() -> Result<(), Box<dyn std::error::Error>>
{
    let e = edit(10, 15, 17, pos(10, 2, 5), pos(15, 2, 10), pos(17, 2, 12));
    let p = pos(30, 4, 3);
    let result = must_some(e.apply_to_position(p));
    assert_eq!(result.byte, 32);
    assert_eq!(result.line, 4); // line_shift = 0 → unchanged
    assert_eq!(result.column, 3); // different line → column unchanged
    Ok(())
}

#[test]
fn apply_position_multiline_edit_shifts_lines() -> Result<(), Box<dyn std::error::Error>> {
    // Delete 2 lines worth of content
    let e = edit(10, 30, 20, pos(10, 2, 5), pos(30, 4, 10), pos(20, 2, 15));
    let p = pos(50, 6, 5);
    let result = must_some(e.apply_to_position(p));
    assert_eq!(result.byte, 40); // 50 - 10
    assert_eq!(result.line, 4); // 6 - 2
    assert_eq!(result.column, 5); // different line → unchanged
    Ok(())
}

#[test]
fn apply_position_zero_size_edit_at_cursor() -> Result<(), Box<dyn std::error::Error>> {
    // Pure insertion: old_end == start (zero-length old range)
    let e = edit(10, 10, 15, pos(10, 2, 5), pos(10, 2, 5), pos(15, 2, 10));
    // Position at insertion point: byte >= start_byte && byte >= old_end_byte → shifted
    let result = must_some(e.apply_to_position(pos(10, 2, 5)));
    assert_eq!(result.byte, 15);
    Ok(())
}

// ===========================================================================
// Edit – apply_to_range
// ===========================================================================

#[test]
fn apply_range_before_edit() -> Result<(), Box<dyn std::error::Error>> {
    let e = edit(20, 25, 30, pos(20, 3, 1), pos(25, 3, 6), pos(30, 3, 11));
    let r = Range::new(pos(0, 1, 1), pos(10, 1, 11));
    let result = must_some(e.apply_to_range(&r));
    assert_eq!(result.start, pos(0, 1, 1));
    assert_eq!(result.end, pos(10, 1, 11));
    Ok(())
}

#[test]
fn apply_range_after_edit() -> Result<(), Box<dyn std::error::Error>> {
    let e = edit(10, 15, 20, pos(10, 2, 1), pos(15, 2, 6), pos(20, 2, 11));
    let r = Range::new(pos(30, 2, 21), pos(40, 2, 31));
    let result = must_some(e.apply_to_range(&r));
    assert_eq!(result.start.byte, 35);
    assert_eq!(result.end.byte, 45);
    Ok(())
}

#[test]
fn apply_range_start_inside_edit_returns_none() -> Result<(), Box<dyn std::error::Error>> {
    let e = edit(10, 20, 25, pos(10, 2, 1), pos(20, 3, 1), pos(25, 3, 6));
    let r = Range::new(pos(12, 2, 3), pos(30, 4, 1));
    assert!(e.apply_to_range(&r).is_none());
    Ok(())
}

#[test]
fn apply_range_end_inside_edit_returns_none() -> Result<(), Box<dyn std::error::Error>> {
    let e = edit(10, 20, 25, pos(10, 2, 1), pos(20, 3, 1), pos(25, 3, 6));
    let r = Range::new(pos(5, 1, 6), pos(15, 2, 6));
    assert!(e.apply_to_range(&r).is_none());
    Ok(())
}

#[test]
fn apply_range_both_inside_edit_returns_none() -> Result<(), Box<dyn std::error::Error>> {
    let e = edit(10, 20, 25, pos(10, 2, 1), pos(20, 3, 1), pos(25, 3, 6));
    let r = Range::new(pos(12, 2, 3), pos(18, 2, 9));
    assert!(e.apply_to_range(&r).is_none());
    Ok(())
}

// ===========================================================================
// EditSet – construction & basic operations
// ===========================================================================

#[test]
fn edit_set_new_is_empty() -> Result<(), Box<dyn std::error::Error>> {
    let es = EditSet::new();
    assert!(es.is_empty());
    assert_eq!(es.len(), 0);
    assert!(es.edits().is_empty());
    Ok(())
}

#[test]
fn edit_set_default_is_empty() -> Result<(), Box<dyn std::error::Error>> {
    let es = EditSet::default();
    assert!(es.is_empty());
    assert_eq!(es.len(), 0);
    Ok(())
}

#[test]
fn edit_set_add_increments_len() -> Result<(), Box<dyn std::error::Error>> {
    let mut es = EditSet::new();
    es.add(edit(0, 5, 5, pos(0, 1, 1), pos(5, 1, 6), pos(5, 1, 6)));
    assert_eq!(es.len(), 1);
    assert!(!es.is_empty());
    es.add(edit(10, 15, 15, pos(10, 2, 1), pos(15, 2, 6), pos(15, 2, 6)));
    assert_eq!(es.len(), 2);
    Ok(())
}

#[test]
fn edit_set_maintains_sorted_order() -> Result<(), Box<dyn std::error::Error>> {
    let mut es = EditSet::new();
    // Insert out of order
    es.add(edit(30, 35, 40, pos(30, 3, 1), pos(35, 3, 6), pos(40, 3, 11)));
    es.add(edit(10, 15, 17, pos(10, 2, 1), pos(15, 2, 6), pos(17, 2, 8)));
    es.add(edit(50, 55, 60, pos(50, 5, 1), pos(55, 5, 6), pos(60, 5, 11)));

    let edits = es.edits();
    assert_eq!(edits.len(), 3);
    assert_eq!(edits[0].start_byte, 10);
    assert_eq!(edits[1].start_byte, 30);
    assert_eq!(edits[2].start_byte, 50);
    Ok(())
}

#[test]
fn edit_set_debug_is_nonempty() -> Result<(), Box<dyn std::error::Error>> {
    let es = EditSet::new();
    let debug = format!("{es:?}");
    assert!(!debug.is_empty());
    Ok(())
}

#[test]
fn edit_set_clone_equals_original() -> Result<(), Box<dyn std::error::Error>> {
    let mut es = EditSet::new();
    es.add(edit(0, 5, 8, pos(0, 1, 1), pos(5, 1, 6), pos(8, 1, 9)));
    let cloned = es.clone();
    assert_eq!(cloned.len(), es.len());
    assert_eq!(cloned.edits()[0], es.edits()[0]);
    Ok(())
}

// ===========================================================================
// EditSet – affects_range
// ===========================================================================

#[test]
fn edit_set_affects_range_empty_set() -> Result<(), Box<dyn std::error::Error>> {
    let es = EditSet::new();
    let r = Range::new(pos(0, 1, 1), pos(10, 1, 11));
    assert!(!es.affects_range(&r));
    Ok(())
}

#[test]
fn edit_set_affects_range_no_overlap() -> Result<(), Box<dyn std::error::Error>> {
    let mut es = EditSet::new();
    es.add(edit(20, 25, 30, pos(20, 3, 1), pos(25, 3, 6), pos(30, 3, 11)));
    let r = Range::new(pos(0, 1, 1), pos(10, 1, 11));
    assert!(!es.affects_range(&r));
    Ok(())
}

#[test]
fn edit_set_affects_range_with_overlap() -> Result<(), Box<dyn std::error::Error>> {
    let mut es = EditSet::new();
    es.add(edit(5, 15, 20, pos(5, 1, 6), pos(15, 1, 16), pos(20, 1, 21)));
    let r = Range::new(pos(0, 1, 1), pos(10, 1, 11));
    assert!(es.affects_range(&r));
    Ok(())
}

#[test]
fn edit_set_affects_range_second_edit_overlaps() -> Result<(), Box<dyn std::error::Error>> {
    let mut es = EditSet::new();
    es.add(edit(0, 3, 3, pos(0, 1, 1), pos(3, 1, 4), pos(3, 1, 4)));
    es.add(edit(8, 12, 15, pos(8, 1, 9), pos(12, 1, 13), pos(15, 1, 16)));
    let r = Range::new(pos(9, 1, 10), pos(20, 2, 1));
    assert!(es.affects_range(&r));
    Ok(())
}

// ===========================================================================
// EditSet – byte_shift_at
// ===========================================================================

#[test]
fn byte_shift_at_before_any_edit() -> Result<(), Box<dyn std::error::Error>> {
    let mut es = EditSet::new();
    es.add(edit(10, 15, 20, pos(10, 2, 1), pos(15, 2, 6), pos(20, 2, 11)));
    // No edits finish before byte 5
    assert_eq!(es.byte_shift_at(5), 0);
    Ok(())
}

#[test]
fn byte_shift_at_after_one_edit() -> Result<(), Box<dyn std::error::Error>> {
    let mut es = EditSet::new();
    es.add(edit(10, 15, 20, pos(10, 2, 1), pos(15, 2, 6), pos(20, 2, 11)));
    // After edit: shift = +5
    assert_eq!(es.byte_shift_at(20), 5);
    Ok(())
}

#[test]
fn byte_shift_at_cumulative() -> Result<(), Box<dyn std::error::Error>> {
    let mut es = EditSet::new();
    es.add(edit(10, 15, 17, pos(10, 2, 1), pos(15, 2, 6), pos(17, 2, 8))); // +2
    es.add(edit(30, 35, 40, pos(30, 3, 1), pos(35, 3, 6), pos(40, 3, 11))); // +5
    assert_eq!(es.byte_shift_at(50), 7);
    Ok(())
}

#[test]
fn byte_shift_at_between_edits() -> Result<(), Box<dyn std::error::Error>> {
    let mut es = EditSet::new();
    es.add(edit(10, 15, 17, pos(10, 2, 1), pos(15, 2, 6), pos(17, 2, 8))); // +2
    es.add(edit(30, 35, 40, pos(30, 3, 1), pos(35, 3, 6), pos(40, 3, 11))); // +5
    // Only first edit finishes before byte 20
    assert_eq!(es.byte_shift_at(20), 2);
    Ok(())
}

#[test]
fn byte_shift_at_with_deletion() -> Result<(), Box<dyn std::error::Error>> {
    let mut es = EditSet::new();
    es.add(edit(5, 15, 8, pos(5, 1, 6), pos(15, 1, 16), pos(8, 1, 9))); // -7
    assert_eq!(es.byte_shift_at(20), -7);
    Ok(())
}

#[test]
fn byte_shift_at_empty_set() -> Result<(), Box<dyn std::error::Error>> {
    let es = EditSet::new();
    assert_eq!(es.byte_shift_at(100), 0);
    Ok(())
}

// ===========================================================================
// EditSet – affected_ranges
// ===========================================================================

#[test]
fn affected_ranges_empty_set() -> Result<(), Box<dyn std::error::Error>> {
    let es = EditSet::new();
    assert!(es.affected_ranges().is_empty());
    Ok(())
}

#[test]
fn affected_ranges_returns_old_ranges() -> Result<(), Box<dyn std::error::Error>> {
    let mut es = EditSet::new();
    es.add(edit(10, 15, 20, pos(10, 2, 1), pos(15, 2, 6), pos(20, 2, 11)));
    es.add(edit(30, 35, 40, pos(30, 3, 1), pos(35, 3, 6), pos(40, 3, 11)));

    let ranges = es.affected_ranges();
    assert_eq!(ranges.len(), 2);
    assert_eq!(ranges[0].start, pos(10, 2, 1));
    assert_eq!(ranges[0].end, pos(15, 2, 6));
    assert_eq!(ranges[1].start, pos(30, 3, 1));
    assert_eq!(ranges[1].end, pos(35, 3, 6));
    Ok(())
}

// ===========================================================================
// EditSet – apply_to_position
// ===========================================================================

#[test]
fn edit_set_apply_position_empty_set() -> Result<(), Box<dyn std::error::Error>> {
    let es = EditSet::new();
    let p = pos(10, 2, 1);
    let result = must_some(es.apply_to_position(p));
    assert_eq!(result, p);
    Ok(())
}

#[test]
fn edit_set_apply_position_before_all() -> Result<(), Box<dyn std::error::Error>> {
    let mut es = EditSet::new();
    es.add(edit(20, 25, 30, pos(20, 3, 1), pos(25, 3, 6), pos(30, 3, 11)));
    let result = must_some(es.apply_to_position(pos(5, 1, 6)));
    assert_eq!(result, pos(5, 1, 6));
    Ok(())
}

#[test]
fn edit_set_apply_position_after_all_cumulative() -> Result<(), Box<dyn std::error::Error>> {
    let mut es = EditSet::new();
    // Edit 1: +2 bytes, same line
    es.add(edit(10, 15, 17, pos(10, 2, 5), pos(15, 2, 10), pos(17, 2, 12)));
    // Edit 2: +5 bytes, same line (positions in already-shifted coordinates)
    es.add(edit(32, 37, 42, pos(32, 3, 5), pos(37, 3, 10), pos(42, 3, 15)));

    let result = must_some(es.apply_to_position(pos(50, 4, 5)));
    // After edit 1: byte 52, line 4, col 5 (different line)
    // After edit 2: byte 57, line 5, col 5 (different line)
    assert_eq!(result.byte, 57);
    Ok(())
}

#[test]
fn edit_set_apply_position_inside_first_edit_returns_none() -> Result<(), Box<dyn std::error::Error>>
{
    let mut es = EditSet::new();
    es.add(edit(10, 20, 25, pos(10, 2, 1), pos(20, 3, 1), pos(25, 3, 6)));
    es.add(edit(30, 35, 40, pos(30, 4, 1), pos(35, 4, 6), pos(40, 4, 11)));
    assert!(es.apply_to_position(pos(15, 2, 6)).is_none());
    Ok(())
}

// ===========================================================================
// EditSet – apply_to_range
// ===========================================================================

#[test]
fn edit_set_apply_range_empty_set() -> Result<(), Box<dyn std::error::Error>> {
    let es = EditSet::new();
    let r = Range::new(pos(0, 1, 1), pos(10, 1, 11));
    let result = must_some(es.apply_to_range(r));
    assert_eq!(result.start, pos(0, 1, 1));
    assert_eq!(result.end, pos(10, 1, 11));
    Ok(())
}

#[test]
fn edit_set_apply_range_shifted() -> Result<(), Box<dyn std::error::Error>> {
    let mut es = EditSet::new();
    es.add(edit(5, 10, 15, pos(5, 1, 6), pos(10, 1, 11), pos(15, 1, 16)));
    let r = Range::new(pos(20, 1, 21), pos(30, 1, 31));
    let result = must_some(es.apply_to_range(r));
    assert_eq!(result.start.byte, 25);
    assert_eq!(result.end.byte, 35);
    Ok(())
}

#[test]
fn edit_set_apply_range_invalidated() -> Result<(), Box<dyn std::error::Error>> {
    let mut es = EditSet::new();
    es.add(edit(5, 15, 20, pos(5, 1, 6), pos(15, 1, 16), pos(20, 1, 21)));
    let r = Range::new(pos(8, 1, 9), pos(25, 1, 26));
    assert!(es.apply_to_range(r).is_none());
    Ok(())
}

// ===========================================================================
// Edge cases
// ===========================================================================

#[test]
fn pure_insertion_at_start_of_file() -> Result<(), Box<dyn std::error::Error>> {
    let e = edit(0, 0, 10, pos(0, 1, 1), pos(0, 1, 1), pos(10, 1, 11));
    assert_eq!(e.byte_shift(), 10);
    assert_eq!(e.line_shift(), 0);
    // Position at byte 0 is after the zero-length old range → shifted
    let result = must_some(e.apply_to_position(pos(0, 1, 1)));
    assert_eq!(result.byte, 10);
    Ok(())
}

#[test]
fn pure_deletion() -> Result<(), Box<dyn std::error::Error>> {
    // Delete bytes 5..15, replace with nothing
    let e = edit(5, 15, 5, pos(5, 1, 6), pos(15, 2, 5), pos(5, 1, 6));
    assert_eq!(e.byte_shift(), -10);
    assert_eq!(e.line_shift(), -1);
    // Position inside deleted range
    assert!(e.apply_to_position(pos(10, 1, 11)).is_none());
    // Position after deleted range
    let result = must_some(e.apply_to_position(pos(20, 3, 1)));
    assert_eq!(result.byte, 10);
    assert_eq!(result.line, 2);
    Ok(())
}

#[test]
fn edit_set_three_edits_cumulative_shift() -> Result<(), Box<dyn std::error::Error>> {
    let mut es = EditSet::new();
    es.add(edit(0, 5, 8, pos(0, 1, 1), pos(5, 1, 6), pos(8, 1, 9))); // +3
    es.add(edit(10, 12, 12, pos(10, 2, 1), pos(12, 2, 3), pos(12, 2, 3))); // 0
    es.add(edit(20, 30, 22, pos(20, 3, 1), pos(30, 4, 1), pos(22, 3, 3))); // -8
    assert_eq!(es.byte_shift_at(100), -5); // +3 + 0 + (-8) = -5
    Ok(())
}

#[test]
fn edit_set_single_edit_affected_range() -> Result<(), Box<dyn std::error::Error>> {
    let mut es = EditSet::new();
    es.add(edit(5, 10, 15, pos(5, 1, 6), pos(10, 1, 11), pos(15, 1, 16)));
    let ranges = es.affected_ranges();
    assert_eq!(ranges.len(), 1);
    assert_eq!(ranges[0].start, pos(5, 1, 6));
    assert_eq!(ranges[0].end, pos(10, 1, 11));
    Ok(())
}

#[test]
fn edit_set_add_same_start_byte() -> Result<(), Box<dyn std::error::Error>> {
    let mut es = EditSet::new();
    es.add(edit(10, 12, 14, pos(10, 2, 1), pos(12, 2, 3), pos(14, 2, 5)));
    es.add(edit(10, 11, 13, pos(10, 2, 1), pos(11, 2, 2), pos(13, 2, 4)));
    // Both edits have start_byte = 10 — second should be inserted after first
    let edits = es.edits();
    assert_eq!(edits.len(), 2);
    assert_eq!(edits[0].start_byte, 10);
    assert_eq!(edits[1].start_byte, 10);
    Ok(())
}

#[test]
fn byte_shift_at_exact_boundary() -> Result<(), Box<dyn std::error::Error>> {
    let mut es = EditSet::new();
    // Edit ends at byte 15
    es.add(edit(10, 15, 20, pos(10, 2, 1), pos(15, 2, 6), pos(20, 2, 11)));
    // byte_shift_at(15) should include this edit (old_end_byte <= 15)
    assert_eq!(es.byte_shift_at(15), 5);
    // byte_shift_at(14) should NOT include it (old_end_byte > 14)
    assert_eq!(es.byte_shift_at(14), 0);
    Ok(())
}
