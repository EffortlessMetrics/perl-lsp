#![cfg(feature = "incremental")]
use perl_parser::incremental::incremental_edit::{
    IncrementalEdit, IncrementalEditBatchError, IncrementalEditSet,
};

#[test]
fn normalize_unsorted_batch_for_reverse_application_order() -> Result<(), IncrementalEditBatchError>
{
    let mut edits = IncrementalEditSet {
        edits: vec![
            IncrementalEdit::new(4, 6, "XY".to_string()),
            IncrementalEdit::new(10, 10, "!".to_string()),
            IncrementalEdit::new(1, 3, "abc".to_string()),
        ],
    };

    edits.normalize_and_validate(false, false)?;

    let positions: Vec<(usize, usize)> =
        edits.edits.iter().map(|edit| (edit.start_byte, edit.old_end_byte)).collect();
    assert_eq!(positions, vec![(10, 10), (4, 6), (1, 3)]);

    Ok(())
}

#[test]
fn normalize_rejects_overlapping_edits() {
    let mut edits = IncrementalEditSet {
        edits: vec![
            IncrementalEdit::new(2, 6, "alpha".to_string()),
            IncrementalEdit::new(4, 8, "beta".to_string()),
        ],
    };

    let result = edits.normalize_and_validate(false, false);

    assert_eq!(
        result,
        Err(IncrementalEditBatchError::OverlappingEdits { left_index: 0, right_index: 1 })
    );
}

#[test]
fn normalize_rejects_backward_ranges() {
    let mut edits =
        IncrementalEditSet { edits: vec![IncrementalEdit::new(9, 3, "broken".to_string())] };

    let result = edits.normalize_and_validate(false, false);

    assert_eq!(
        result,
        Err(IncrementalEditBatchError::BackwardRange { index: 0, start_byte: 9, old_end_byte: 3 })
    );
}

#[test]
fn normalize_accepts_zero_width_insertions() -> Result<(), IncrementalEditBatchError> {
    let mut edits =
        IncrementalEditSet { edits: vec![IncrementalEdit::new(7, 7, "insert".to_string())] };

    edits.normalize_and_validate(false, false)?;

    assert_eq!(edits.edits.len(), 1);
    assert_eq!(edits.edits[0].start_byte, 7);
    assert_eq!(edits.edits[0].old_end_byte, 7);

    Ok(())
}

#[test]
fn total_byte_shift_stays_correct_after_normalization() -> Result<(), IncrementalEditBatchError> {
    let mut edits = IncrementalEditSet {
        edits: vec![
            IncrementalEdit::new(10, 10, "++".to_string()),
            IncrementalEdit::new(2, 5, "x".to_string()),
        ],
    };

    edits.normalize_and_validate(false, false)?;

    assert_eq!(edits.total_byte_shift(), 0);

    Ok(())
}

#[test]
fn optional_no_op_filter_only_removes_empty_zero_width_edit()
-> Result<(), IncrementalEditBatchError> {
    let mut edits = IncrementalEditSet {
        edits: vec![
            IncrementalEdit::new(3, 3, String::new()),
            IncrementalEdit::new(5, 5, "x".to_string()),
        ],
    };

    edits.normalize_and_validate(false, true)?;

    assert_eq!(edits.edits.len(), 1);
    assert_eq!(edits.edits[0].new_text, "x");

    Ok(())
}

// --- edge cases ---

/// `allow_overlaps = true` must succeed even when edits overlap.
#[test]
fn normalize_allow_overlaps_succeeds_for_overlapping_edits()
-> Result<(), IncrementalEditBatchError> {
    let mut edits = IncrementalEditSet {
        edits: vec![
            IncrementalEdit::new(2, 6, "alpha".to_string()),
            IncrementalEdit::new(4, 8, "beta".to_string()),
        ],
    };

    // Must not error even though the two edits overlap.
    edits.normalize_and_validate(true, false)?;

    // Both edits survive, sorted descending.
    assert_eq!(edits.edits.len(), 2);
    assert_eq!(edits.edits[0].start_byte, 4); // higher offset first
    assert_eq!(edits.edits[1].start_byte, 2);

    Ok(())
}

/// An empty `IncrementalEditSet` must normalize without error in every
/// combination of `allow_overlaps` and `filter_no_ops`.
#[test]
fn normalize_empty_edit_set_is_always_valid() -> Result<(), IncrementalEditBatchError> {
    for &allow_overlaps in &[false, true] {
        for &filter_no_ops in &[false, true] {
            let mut edits = IncrementalEditSet::new();
            edits.normalize_and_validate(allow_overlaps, filter_no_ops)?;
            assert!(edits.edits.is_empty());
        }
    }
    Ok(())
}

/// Adjacent (touching but non-overlapping) edits must NOT be rejected by the
/// overlap check.  `overlaps` uses strict inequalities so [2,4) and [4,6) are
/// disjoint.
#[test]
fn normalize_adjacent_edits_are_not_overlapping() -> Result<(), IncrementalEditBatchError> {
    let mut edits = IncrementalEditSet {
        edits: vec![
            IncrementalEdit::new(2, 4, "AB".to_string()),
            IncrementalEdit::new(4, 6, "CD".to_string()),
        ],
    };

    edits.normalize_and_validate(false, false)?;

    // Sorted descending by start_byte.
    assert_eq!(edits.edits[0].start_byte, 4);
    assert_eq!(edits.edits[1].start_byte, 2);

    Ok(())
}
