#![cfg(feature = "incremental")]

use perl_parser::incremental_edit::{
    IncrementalEdit, IncrementalEditSet, IncrementalEditSetValidationError,
};

#[test]
fn unsorted_batch_normalizes_to_reverse_application_order()
-> Result<(), IncrementalEditSetValidationError> {
    let mut edits = IncrementalEditSet::new();
    edits.add(IncrementalEdit::new(5, 5, "x".to_string()));
    edits.add(IncrementalEdit::new(1, 1, "y".to_string()));
    edits.add(IncrementalEdit::new(9, 10, "zz".to_string()));

    let normalized = edits.normalize_and_validate(false, false)?;

    let start_bytes: Vec<usize> = normalized.edits.iter().map(|edit| edit.start_byte).collect();
    assert_eq!(start_bytes, vec![9, 5, 1]);
    Ok(())
}

#[test]
fn overlapping_edits_are_rejected_cleanly() {
    let mut edits = IncrementalEditSet::new();
    edits.add(IncrementalEdit::new(3, 8, "abc".to_string()));
    edits.add(IncrementalEdit::new(6, 9, "def".to_string()));

    let result = edits.normalize_and_validate(false, false);

    assert_eq!(
        result.err(),
        Some(IncrementalEditSetValidationError::OverlappingEdits {
            first_edit_index: 0,
            second_edit_index: 1,
        })
    );
}

#[test]
fn backward_range_is_rejected_cleanly() {
    let mut edits = IncrementalEditSet::new();
    edits.add(IncrementalEdit::new(5, 4, "bad".to_string()));

    let result = edits.normalize_and_validate(false, false);

    assert_eq!(
        result.err(),
        Some(IncrementalEditSetValidationError::BackwardRange {
            edit_index: 0,
            start_byte: 5,
            old_end_byte: 4,
        })
    );
}

#[test]
fn zero_width_insertion_is_accepted() -> Result<(), IncrementalEditSetValidationError> {
    let mut edits = IncrementalEditSet::new();
    edits.add(IncrementalEdit::new(4, 4, "insert".to_string()));

    let normalized = edits.normalize_and_validate(false, false)?;

    assert_eq!(normalized.edits.len(), 1);
    assert_eq!(normalized.edits[0].start_byte, 4);
    assert_eq!(normalized.edits[0].old_end_byte, 4);
    Ok(())
}

#[test]
fn total_byte_shift_remains_correct_after_normalization()
-> Result<(), IncrementalEditSetValidationError> {
    let mut edits = IncrementalEditSet::new();
    edits.add(IncrementalEdit::new(10, 10, "abc".to_string())); // +3
    edits.add(IncrementalEdit::new(0, 4, "z".to_string())); // -3
    edits.add(IncrementalEdit::new(7, 9, "".to_string())); // -2

    let normalized = edits.normalize_and_validate(false, false)?;

    assert_eq!(edits.total_byte_shift(), -2);
    assert_eq!(normalized.total_byte_shift(), -2);
    Ok(())
}
