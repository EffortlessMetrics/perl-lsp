#![cfg(feature = "incremental")]

use perl_parser::incremental::incremental_edit::{
    IncrementalEdit, IncrementalEditBatchError, IncrementalEditNormalizationOptions,
    IncrementalEditSet,
};

#[test]
fn normalize_unsorted_batch_for_reverse_application() -> Result<(), Box<dyn std::error::Error>> {
    let mut edits = IncrementalEditSet::new();
    edits.add(IncrementalEdit::new(2, 2, "A".to_string()));
    edits.add(IncrementalEdit::new(8, 10, "xy".to_string()));
    edits.add(IncrementalEdit::new(5, 6, "".to_string()));

    let total_shift_before = edits.total_byte_shift();

    edits.normalize_and_validate(IncrementalEditNormalizationOptions::default())?;

    let ordered_starts: Vec<usize> = edits.edits.iter().map(|edit| edit.start_byte).collect();
    assert_eq!(ordered_starts, vec![8, 5, 2]);
    assert_eq!(edits.total_byte_shift(), total_shift_before);

    Ok(())
}

#[test]
fn normalize_rejects_overlapping_edits() {
    let mut edits = IncrementalEditSet::new();
    edits.add(IncrementalEdit::new(5, 9, "foo".to_string()));
    edits.add(IncrementalEdit::new(7, 12, "bar".to_string()));

    let result = edits.normalize_and_validate(IncrementalEditNormalizationOptions::default());

    assert_eq!(
        result,
        Err(IncrementalEditBatchError::OverlappingEdits {
            first_index: 0,
            first_start_byte: 5,
            first_old_end_byte: 9,
            second_index: 1,
            second_start_byte: 7,
            second_old_end_byte: 12,
        })
    );
}

#[test]
fn normalize_rejects_backward_range() {
    let mut edits = IncrementalEditSet::new();
    edits.add(IncrementalEdit::new(10, 8, "oops".to_string()));

    let result = edits.normalize_and_validate(IncrementalEditNormalizationOptions::default());

    assert_eq!(
        result,
        Err(IncrementalEditBatchError::BackwardRange { index: 0, start_byte: 10, old_end_byte: 8 })
    );
}

#[test]
fn normalize_accepts_zero_width_insertions() -> Result<(), Box<dyn std::error::Error>> {
    let mut edits = IncrementalEditSet::new();
    edits.add(IncrementalEdit::new(3, 3, "A".to_string()));
    edits.add(IncrementalEdit::new(3, 3, "B".to_string()));

    edits.normalize_and_validate(IncrementalEditNormalizationOptions::default())?;

    assert_eq!(edits.edits.len(), 2);
    assert!(edits.edits.iter().all(|edit| edit.start_byte == edit.old_end_byte));

    Ok(())
}

#[test]
fn normalize_can_filter_obvious_noops() -> Result<(), Box<dyn std::error::Error>> {
    let mut edits = IncrementalEditSet::new();
    edits.add(IncrementalEdit::new(0, 0, "".to_string()));
    edits.add(IncrementalEdit::new(4, 4, "X".to_string()));

    let result = edits.normalize_and_validate(IncrementalEditNormalizationOptions {
        allow_overlaps: false,
        filter_obvious_noops: true,
    })?;

    assert_eq!(result.removed_noop_edits, 1);
    assert_eq!(edits.edits.len(), 1);
    assert_eq!(edits.edits[0].start_byte, 4);

    Ok(())
}
