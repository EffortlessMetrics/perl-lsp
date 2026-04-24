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
