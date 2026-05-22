//! Coverage gap tests for perl-incremental-parsing.
//!
//! Covers previously unexercised surface:
//! - `ReuseType` enum variants: Debug, PartialEq, Clone
//! - `IncrementalEditBatchError` variants: BackwardRange, OverlappingEdits
//! - `IncrementalEditSet::normalize_and_validate` - backward-range and overlap paths
//! - `IncrementalEditSet::normalize_for_source` - valid, backward-range, and out-of-bounds paths
//! - `IncrementalEditSet::sort_reverse_deterministic` - deterministic sort order
//! - `IncrementalState::clone` - Clone impl is exercised
//! - `lsp_change_to_edit` - full-document and ranged-change branches

use perl_incremental_parsing::incremental::IncrementalState;
use perl_incremental_parsing::incremental::incremental_advanced_reuse::ReuseType;
use perl_incremental_parsing::incremental::incremental_edit::{
    IncrementalEdit, IncrementalEditBatchError, IncrementalEditSet,
};
use perl_incremental_parsing::incremental::incremental_integration::lsp_change_to_edit;
use ropey::Rope;
use serde_json::json;

// ============================================================================
// ReuseType: Debug, PartialEq, Clone
// ============================================================================

#[test]
fn reuse_type_debug_format() -> Result<(), Box<dyn std::error::Error>> {
    let d = format!("{:?}", ReuseType::Direct);
    let p = format!("{:?}", ReuseType::PositionShift);
    let c = format!("{:?}", ReuseType::ContentUpdate);
    let s = format!("{:?}", ReuseType::StructuralEquivalent);
    assert!(d.contains("Direct"), "Debug for Direct: {d}");
    assert!(p.contains("PositionShift"), "Debug for PositionShift: {p}");
    assert!(c.contains("ContentUpdate"), "Debug for ContentUpdate: {c}");
    assert!(s.contains("StructuralEquivalent"), "Debug for StructuralEquivalent: {s}");
    Ok(())
}

#[test]
fn reuse_type_partial_eq_same_variant() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(ReuseType::Direct, ReuseType::Direct);
    assert_eq!(ReuseType::PositionShift, ReuseType::PositionShift);
    assert_eq!(ReuseType::ContentUpdate, ReuseType::ContentUpdate);
    assert_eq!(ReuseType::StructuralEquivalent, ReuseType::StructuralEquivalent);
    Ok(())
}

#[test]
fn reuse_type_partial_eq_different_variants() -> Result<(), Box<dyn std::error::Error>> {
    assert_ne!(ReuseType::Direct, ReuseType::PositionShift);
    assert_ne!(ReuseType::ContentUpdate, ReuseType::StructuralEquivalent);
    assert_ne!(ReuseType::Direct, ReuseType::ContentUpdate);
    Ok(())
}

#[test]
fn reuse_type_clone() -> Result<(), Box<dyn std::error::Error>> {
    let original = ReuseType::PositionShift;
    let cloned = original.clone();
    assert_eq!(original, cloned);

    let original2 = ReuseType::StructuralEquivalent;
    let cloned2 = original2.clone();
    assert_eq!(original2, cloned2);
    Ok(())
}

// ============================================================================
// IncrementalEditBatchError: Debug, PartialEq, Clone, variants
// ============================================================================

#[test]
fn incremental_edit_batch_error_backward_range_debug() -> Result<(), Box<dyn std::error::Error>> {
    let err =
        IncrementalEditBatchError::BackwardRange { index: 0, start_byte: 10, old_end_byte: 5 };
    let dbg = format!("{:?}", err);
    assert!(dbg.contains("BackwardRange"), "Debug: {dbg}");
    assert!(dbg.contains("10"), "start_byte in debug: {dbg}");
    assert!(dbg.contains("5"), "old_end_byte in debug: {dbg}");
    Ok(())
}

#[test]
fn incremental_edit_batch_error_overlapping_edits_debug() -> Result<(), Box<dyn std::error::Error>>
{
    let err = IncrementalEditBatchError::OverlappingEdits { left_index: 1, right_index: 2 };
    let dbg = format!("{:?}", err);
    assert!(dbg.contains("OverlappingEdits"), "Debug: {dbg}");
    Ok(())
}

#[test]
fn incremental_edit_batch_error_partial_eq() -> Result<(), Box<dyn std::error::Error>> {
    let a = IncrementalEditBatchError::BackwardRange { index: 0, start_byte: 10, old_end_byte: 5 };
    let b = IncrementalEditBatchError::BackwardRange { index: 0, start_byte: 10, old_end_byte: 5 };
    let c = IncrementalEditBatchError::OverlappingEdits { left_index: 0, right_index: 1 };
    assert_eq!(a, b);
    assert_ne!(a, c);
    Ok(())
}

#[test]
fn incremental_edit_batch_error_clone() -> Result<(), Box<dyn std::error::Error>> {
    let err = IncrementalEditBatchError::OverlappingEdits { left_index: 3, right_index: 7 };
    let cloned = err.clone();
    assert_eq!(err, cloned);
    Ok(())
}

// ============================================================================
// normalize_and_validate: backward-range path
// ============================================================================

#[test]
fn normalize_and_validate_backward_range_is_error() -> Result<(), Box<dyn std::error::Error>> {
    let mut set = IncrementalEditSet::new();
    // start_byte > old_end_byte is a backward range
    set.add(IncrementalEdit::new(10, 5, "x".to_string()));
    let result = set.normalize_and_validate(false, false);
    assert!(result.is_err(), "expected Err for backward range");
    match result {
        Err(IncrementalEditBatchError::BackwardRange { index, start_byte, old_end_byte }) => {
            assert_eq!(index, 0);
            assert_eq!(start_byte, 10);
            assert_eq!(old_end_byte, 5);
        }
        other => return Err(format!("unexpected result: {other:?}").into()),
    }
    Ok(())
}

#[test]
fn normalize_and_validate_overlapping_edits_is_error() -> Result<(), Box<dyn std::error::Error>> {
    let mut set = IncrementalEditSet::new();
    // Two overlapping edits: [0,10) and [5,15)
    set.add(IncrementalEdit::new(0, 10, "a".to_string()));
    set.add(IncrementalEdit::new(5, 15, "b".to_string()));
    let result = set.normalize_and_validate(false, false);
    assert!(result.is_err(), "expected Err for overlapping edits");
    match result {
        Err(IncrementalEditBatchError::OverlappingEdits { .. }) => {}
        other => return Err(format!("unexpected result: {other:?}").into()),
    }
    Ok(())
}

#[test]
fn normalize_and_validate_valid_non_overlapping_ok() -> Result<(), Box<dyn std::error::Error>> {
    let mut set = IncrementalEditSet::new();
    // Two non-overlapping, non-backward edits
    set.add(IncrementalEdit::new(0, 5, "hello".to_string()));
    set.add(IncrementalEdit::new(10, 15, "world".to_string()));
    let result = set.normalize_and_validate(false, false);
    assert!(result.is_ok(), "expected Ok for valid edits: {result:?}");
    Ok(())
}

#[test]
fn normalize_and_validate_allow_overlaps_flag() -> Result<(), Box<dyn std::error::Error>> {
    let mut set = IncrementalEditSet::new();
    // Two overlapping edits - should pass when allow_overlaps = true
    set.add(IncrementalEdit::new(0, 10, "a".to_string()));
    set.add(IncrementalEdit::new(5, 15, "b".to_string()));
    let result = set.normalize_and_validate(true, false);
    assert!(result.is_ok(), "expected Ok when allow_overlaps=true: {result:?}");
    Ok(())
}

#[test]
fn normalize_and_validate_filter_no_ops_removes_empty_edits()
-> Result<(), Box<dyn std::error::Error>> {
    let mut set = IncrementalEditSet::new();
    // A no-op edit: same start and end, empty new_text
    set.add(IncrementalEdit::new(5, 5, String::new()));
    // A real edit
    set.add(IncrementalEdit::new(10, 15, "new".to_string()));
    let result = set.normalize_and_validate(false, true);
    assert!(result.is_ok(), "expected Ok after filtering no-ops: {result:?}");
    // After filtering, only the real edit remains
    assert_eq!(set.edits.len(), 1, "no-op should have been removed");
    Ok(())
}

// ============================================================================
// normalize_for_source: valid, backward, out-of-bounds
// ============================================================================

#[test]
fn normalize_for_source_valid_edits() -> Result<(), Box<dyn std::error::Error>> {
    let source = "hello world";
    let mut set = IncrementalEditSet::new();
    set.add(IncrementalEdit::new(0, 5, "hi".to_string()));
    let result = set.normalize_for_source(source);
    assert!(result.is_some(), "expected Some for valid edits");
    Ok(())
}

#[test]
fn normalize_for_source_backward_range_returns_none() -> Result<(), Box<dyn std::error::Error>> {
    let source = "hello world";
    let mut set = IncrementalEditSet::new();
    // start_byte > old_end_byte -> not mappable
    set.add(IncrementalEdit::new(5, 3, "x".to_string()));
    let result = set.normalize_for_source(source);
    assert!(result.is_none(), "expected None for backward range");
    Ok(())
}

#[test]
fn normalize_for_source_out_of_bounds_returns_none() -> Result<(), Box<dyn std::error::Error>> {
    let source = "hi";
    let mut set = IncrementalEditSet::new();
    // old_end_byte exceeds source length
    set.add(IncrementalEdit::new(0, 100, "x".to_string()));
    let result = set.normalize_for_source(source);
    assert!(result.is_none(), "expected None for out-of-bounds edit");
    Ok(())
}

#[test]
fn normalize_for_source_overlapping_non_empty_returns_none()
-> Result<(), Box<dyn std::error::Error>> {
    let source = "hello world!";
    let mut set = IncrementalEditSet::new();
    // Two overlapping non-empty edits
    set.add(IncrementalEdit::new(0, 6, "hi".to_string()));
    set.add(IncrementalEdit::new(3, 9, "there".to_string()));
    let result = set.normalize_for_source(source);
    assert!(result.is_none(), "expected None for overlapping edits");
    Ok(())
}

#[test]
fn normalize_for_source_empty_set_returns_some_empty() -> Result<(), Box<dyn std::error::Error>> {
    let source = "hello";
    let set = IncrementalEditSet::new();
    let result = set.normalize_for_source(source);
    assert!(result.is_some(), "empty set should return Some");
    let edits = result.ok_or("empty set should return Some")?;
    assert!(edits.is_empty(), "empty set should normalize to empty vec");
    Ok(())
}

// ============================================================================
// sort_reverse_deterministic
// ============================================================================

#[test]
fn sort_reverse_deterministic_orders_by_byte_desc() -> Result<(), Box<dyn std::error::Error>> {
    let mut set = IncrementalEditSet::new();
    set.add(IncrementalEdit::new(5, 10, "a".to_string()));
    set.add(IncrementalEdit::new(0, 4, "b".to_string()));
    set.add(IncrementalEdit::new(20, 25, "c".to_string()));
    set.sort_reverse_deterministic();
    // After deterministic reverse sort, highest start_byte first
    assert_eq!(set.edits[0].start_byte, 20, "first should be highest byte offset");
    assert_eq!(set.edits[1].start_byte, 5);
    assert_eq!(set.edits[2].start_byte, 0);
    Ok(())
}

#[test]
fn sort_reverse_deterministic_tie_break_by_old_end() -> Result<(), Box<dyn std::error::Error>> {
    let mut set = IncrementalEditSet::new();
    // Same start_byte, different old_end_byte - larger old_end_byte comes first
    set.add(IncrementalEdit::new(5, 8, "a".to_string()));
    set.add(IncrementalEdit::new(5, 12, "b".to_string()));
    set.sort_reverse_deterministic();
    assert_eq!(set.edits[0].old_end_byte, 12, "larger old_end_byte should come first on tie");
    Ok(())
}

// ============================================================================
// IncrementalState::clone
// ============================================================================

#[test]
fn incremental_state_clone_is_independent() -> Result<(), Box<dyn std::error::Error>> {
    let state = IncrementalState::new("my $x = 1;".to_string());
    let mut cloned = state.clone();
    // Modify the clone's source - the original should be unaffected
    cloned.source = "my $y = 2;".to_string();
    assert_eq!(state.source, "my $x = 1;", "original state unchanged after clone mutation");
    assert_eq!(cloned.source, "my $y = 2;");
    Ok(())
}

#[test]
fn incremental_state_clone_preserves_checkpoints() -> Result<(), Box<dyn std::error::Error>> {
    let source = "sub foo { my $x = 1; }";
    let state = IncrementalState::new(source.to_string());
    let cloned = state.clone();
    // Both should have the same checkpoint counts
    assert_eq!(
        state.lex_checkpoints.len(),
        cloned.lex_checkpoints.len(),
        "cloned state has same lex checkpoint count"
    );
    assert_eq!(
        state.parse_checkpoints.len(),
        cloned.parse_checkpoints.len(),
        "cloned state has same parse checkpoint count"
    );
    Ok(())
}

// ============================================================================
// lsp_change_to_edit: ranged change and full-document change
// ============================================================================

#[test]
fn lsp_change_to_edit_full_document_change_returns_none() -> Result<(), Box<dyn std::error::Error>>
{
    let rope = Rope::from_str("hello world");
    // A full-document change has no "range" key
    let change = json!({ "text": "new content" });
    let result = lsp_change_to_edit(&change, &rope);
    assert!(result.is_none(), "full-document change should return None");
    Ok(())
}

#[test]
fn lsp_change_to_edit_ranged_change_returns_some() -> Result<(), Box<dyn std::error::Error>> {
    let rope = Rope::from_str("hello world");
    // Ranged change replacing "hello" with "hi"
    let change = json!({
        "range": {
            "start": { "line": 0, "character": 0 },
            "end": { "line": 0, "character": 5 }
        },
        "text": "hi"
    });
    let result = lsp_change_to_edit(&change, &rope);
    assert!(result.is_some(), "ranged change should return Some");
    let edit = result.ok_or("ranged change should return Some")?;
    assert_eq!(edit.start_byte, 0);
    assert_eq!(edit.new_text, "hi");
    Ok(())
}

#[test]
fn lsp_change_to_edit_ranged_change_mid_line() -> Result<(), Box<dyn std::error::Error>> {
    let rope = Rope::from_str("hello world\n");
    // Replace "world" (chars 6..11) with "there"
    let change = json!({
        "range": {
            "start": { "line": 0, "character": 6 },
            "end": { "line": 0, "character": 11 }
        },
        "text": "there"
    });
    let result = lsp_change_to_edit(&change, &rope);
    assert!(result.is_some(), "mid-line ranged change should return Some");
    let edit = result.ok_or("mid-line ranged change should return Some")?;
    assert_eq!(edit.start_byte, 6, "start byte for 'world'");
    assert_eq!(edit.new_text, "there");
    Ok(())
}

#[test]
fn lsp_change_to_edit_missing_text_field_returns_none() -> Result<(), Box<dyn std::error::Error>> {
    let rope = Rope::from_str("hello world");
    // Range present but "text" field missing
    let change = json!({
        "range": {
            "start": { "line": 0, "character": 0 },
            "end": { "line": 0, "character": 5 }
        }
    });
    let result = lsp_change_to_edit(&change, &rope);
    // text field is null -> as_str() returns None -> None returned
    assert!(result.is_none(), "missing text field should return None");
    Ok(())
}
