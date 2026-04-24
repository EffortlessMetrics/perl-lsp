//! Enhanced edit structure for incremental parsing with text content
//!
//! This module provides an extended Edit type that includes the new text
//! being inserted, enabling efficient incremental parsing with subtree reuse.

use perl_parser_core::position::Position;
use std::cmp::Reverse;

/// Validation/normalization options for edit batches.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct IncrementalEditNormalizationOptions {
    /// When true, overlap validation is skipped.
    pub allow_overlaps: bool,
    /// When true, obvious no-op edits are removed.
    pub filter_obvious_noops: bool,
}

/// Validation failure for a malformed edit batch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IncrementalEditBatchError {
    /// An edit with `start_byte > old_end_byte`.
    InvalidRange { index: usize, start_byte: usize, old_end_byte: usize },
    /// Two edits overlap in old-source byte space.
    OverlappingEdits {
        first_index: usize,
        second_index: usize,
        first_start_byte: usize,
        first_old_end_byte: usize,
        second_start_byte: usize,
        second_old_end_byte: usize,
    },
}

/// Enhanced edit with text content for incremental parsing
#[derive(Debug, Clone, PartialEq)]
pub struct IncrementalEdit {
    /// Start byte offset of the edit
    pub start_byte: usize,
    /// End byte offset of the text being replaced (in old source)
    pub old_end_byte: usize,
    /// The new text being inserted
    pub new_text: String,
    /// Start position (line/column)
    pub start_position: Position,
    /// Old end position before edit
    pub old_end_position: Position,
}

impl IncrementalEdit {
    /// Create a new incremental edit
    pub fn new(start_byte: usize, old_end_byte: usize, new_text: String) -> Self {
        IncrementalEdit {
            start_byte,
            old_end_byte,
            new_text,
            start_position: Position::new(start_byte, 0, 0),
            old_end_position: Position::new(old_end_byte, 0, 0),
        }
    }

    /// Create with position information
    pub fn with_positions(
        start_byte: usize,
        old_end_byte: usize,
        new_text: String,
        start_position: Position,
        old_end_position: Position,
    ) -> Self {
        IncrementalEdit { start_byte, old_end_byte, new_text, start_position, old_end_position }
    }

    /// Get the new end byte after applying this edit
    pub fn new_end_byte(&self) -> usize {
        self.start_byte + self.new_text.len()
    }

    /// Calculate the byte shift caused by this edit
    pub fn byte_shift(&self) -> isize {
        self.new_text.len() as isize - (self.old_end_byte as isize - self.start_byte as isize)
    }

    /// Check if this edit overlaps with a byte range
    pub fn overlaps(&self, start: usize, end: usize) -> bool {
        self.start_byte < end && self.old_end_byte > start
    }

    /// Check if this edit is entirely before a position
    pub fn is_before(&self, pos: usize) -> bool {
        self.old_end_byte <= pos
    }

    /// Check if this edit is entirely after a position
    pub fn is_after(&self, pos: usize) -> bool {
        self.start_byte >= pos
    }
}

/// Collection of incremental edits
#[derive(Debug, Clone, Default)]
pub struct IncrementalEditSet {
    pub edits: Vec<IncrementalEdit>,
}

impl IncrementalEditSet {
    /// Create a new empty edit set
    pub fn new() -> Self {
        IncrementalEditSet { edits: Vec::new() }
    }

    /// Add an edit to the set
    pub fn add(&mut self, edit: IncrementalEdit) {
        self.edits.push(edit);
    }

    /// Sort edits by position (for correct application order)
    pub fn sort(&mut self) {
        self.edits.sort_by_key(|e| e.start_byte);
    }

    /// Sort edits in reverse order (for applying from end to start)
    pub fn sort_reverse(&mut self) {
        self.edits.sort_by_key(|e| Reverse(e.start_byte));
    }

    /// Return a deterministically normalized copy suitable for reverse application.
    ///
    /// The resulting order is descending by `start_byte`, then descending by
    /// `old_end_byte`, with stable input index as final tie-breaker.
    fn normalized_for_reverse_application(
        &self,
        options: IncrementalEditNormalizationOptions,
    ) -> Vec<(usize, IncrementalEdit)> {
        let mut indexed_edits: Vec<(usize, IncrementalEdit)> = self
            .edits
            .iter()
            .cloned()
            .enumerate()
            .filter(|(_, edit)| {
                !options.filter_obvious_noops
                    || !(edit.start_byte == edit.old_end_byte && edit.new_text.is_empty())
            })
            .collect();

        indexed_edits.sort_by_key(|(index, edit)| {
            (Reverse(edit.start_byte), Reverse(edit.old_end_byte), *index)
        });
        indexed_edits
    }

    /// Normalize edit order and validate batch shape.
    ///
    /// This function does not inspect source content or UTF-8 boundaries.
    pub fn normalize_and_validate(
        &self,
        options: IncrementalEditNormalizationOptions,
    ) -> Result<IncrementalEditSet, IncrementalEditBatchError> {
        let normalized = self.normalized_for_reverse_application(options);

        for (index, edit) in &normalized {
            if edit.start_byte > edit.old_end_byte {
                return Err(IncrementalEditBatchError::InvalidRange {
                    index: *index,
                    start_byte: edit.start_byte,
                    old_end_byte: edit.old_end_byte,
                });
            }
        }

        if !options.allow_overlaps {
            for window in normalized.windows(2) {
                let (left_index, left) = (&window[0].0, &window[0].1);
                let (right_index, right) = (&window[1].0, &window[1].1);
                if right.old_end_byte > left.start_byte {
                    return Err(IncrementalEditBatchError::OverlappingEdits {
                        first_index: *left_index,
                        second_index: *right_index,
                        first_start_byte: left.start_byte,
                        first_old_end_byte: left.old_end_byte,
                        second_start_byte: right.start_byte,
                        second_old_end_byte: right.old_end_byte,
                    });
                }
            }
        }

        Ok(IncrementalEditSet { edits: normalized.into_iter().map(|(_, edit)| edit).collect() })
    }

    /// Check if the edit set is empty
    pub fn is_empty(&self) -> bool {
        self.edits.is_empty()
    }

    /// Get the total byte shift for all edits
    pub fn total_byte_shift(&self) -> isize {
        self.edits.iter().map(|e| e.byte_shift()).sum()
    }

    /// Apply edits to a string
    pub fn apply_to_string(&self, source: &str) -> String {
        if self.edits.is_empty() {
            return source.to_string();
        }

        // Sort edits in reverse order to apply from end to start
        let mut sorted_edits = self.edits.clone();
        sorted_edits.sort_by_key(|e| Reverse(e.start_byte));

        let mut result = source.to_string();
        for edit in &sorted_edits {
            let start = edit.start_byte.min(result.len());
            let end = edit.old_end_byte.min(result.len());

            if start > end {
                continue;
            }

            if !result.is_char_boundary(start) || !result.is_char_boundary(end) {
                continue;
            }

            result.replace_range(start..end, &edit.new_text);
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Error;

    #[test]
    fn test_incremental_edit_basic() {
        let edit = IncrementalEdit::new(5, 10, "hello".to_string());
        assert_eq!(edit.new_end_byte(), 10);
        assert_eq!(edit.byte_shift(), 0);
    }

    #[test]
    fn test_incremental_edit_insertion() {
        let edit = IncrementalEdit::new(5, 5, "inserted".to_string());
        assert_eq!(edit.new_end_byte(), 13);
        assert_eq!(edit.byte_shift(), 8);
    }

    #[test]
    fn test_incremental_edit_deletion() {
        let edit = IncrementalEdit::new(5, 15, "".to_string());
        assert_eq!(edit.new_end_byte(), 5);
        assert_eq!(edit.byte_shift(), -10);
    }

    #[test]
    fn test_incremental_edit_replacement() {
        let edit = IncrementalEdit::new(5, 10, "replaced".to_string());
        assert_eq!(edit.new_end_byte(), 13);
        assert_eq!(edit.byte_shift(), 3);
    }

    #[test]
    fn test_edit_set_apply() {
        let mut edits = IncrementalEditSet::new();
        edits.add(IncrementalEdit::new(0, 5, "Hello".to_string()));
        edits.add(IncrementalEdit::new(6, 11, "Perl".to_string()));

        let source = "hello world";
        let result = edits.apply_to_string(source);
        assert_eq!(result, "Hello Perl");
    }

    #[test]
    fn test_normalize_unsorted_batch_for_reverse_application()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut edits = IncrementalEditSet::new();
        edits.add(IncrementalEdit::new(0, 2, "AA".to_string()));
        edits.add(IncrementalEdit::new(10, 11, "Z".to_string()));
        edits.add(IncrementalEdit::new(4, 4, "++".to_string()));

        let normalized =
            edits.normalize_and_validate(IncrementalEditNormalizationOptions::default()).map_err(
                |err| Error::other(format!("unsorted but valid batch should normalize: {err:?}")),
            )?;

        let starts: Vec<usize> = normalized.edits.iter().map(|edit| edit.start_byte).collect();
        assert_eq!(starts, vec![10, 4, 0]);
        Ok(())
    }

    #[test]
    fn test_normalize_rejects_overlapping_edits() -> Result<(), Box<dyn std::error::Error>> {
        let mut edits = IncrementalEditSet::new();
        edits.add(IncrementalEdit::new(1, 5, "ab".to_string()));
        edits.add(IncrementalEdit::new(3, 7, "cd".to_string()));

        let err = edits
            .normalize_and_validate(IncrementalEditNormalizationOptions::default())
            .err()
            .ok_or_else(|| Error::other("overlapping ranges should fail validation"))?;

        assert!(matches!(err, IncrementalEditBatchError::OverlappingEdits { .. }));
        Ok(())
    }

    #[test]
    fn test_normalize_rejects_backward_range() -> Result<(), Box<dyn std::error::Error>> {
        let mut edits = IncrementalEditSet::new();
        edits.add(IncrementalEdit::new(8, 2, "bad".to_string()));

        let err = edits
            .normalize_and_validate(IncrementalEditNormalizationOptions::default())
            .err()
            .ok_or_else(|| Error::other("backward ranges should fail validation"))?;

        assert!(matches!(err, IncrementalEditBatchError::InvalidRange { .. }));
        Ok(())
    }

    #[test]
    fn test_normalize_accepts_zero_width_insertion() -> Result<(), Box<dyn std::error::Error>> {
        let mut edits = IncrementalEditSet::new();
        edits.add(IncrementalEdit::new(3, 3, "insert".to_string()));

        let normalized =
            edits.normalize_and_validate(IncrementalEditNormalizationOptions::default()).map_err(
                |err| Error::other(format!("zero-width insertion should be valid: {err:?}")),
            )?;

        assert_eq!(normalized.edits.len(), 1);
        assert_eq!(normalized.edits[0].start_byte, 3);
        assert_eq!(normalized.edits[0].old_end_byte, 3);
        Ok(())
    }

    #[test]
    fn test_total_byte_shift_unchanged_after_normalization()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut edits = IncrementalEditSet::new();
        edits.add(IncrementalEdit::new(5, 9, "xy".to_string())); // -2
        edits.add(IncrementalEdit::new(1, 1, "abc".to_string())); // +3
        edits.add(IncrementalEdit::new(12, 14, "Q".to_string())); // -1

        let before = edits.total_byte_shift();
        let normalized = edits
            .normalize_and_validate(IncrementalEditNormalizationOptions::default())
            .map_err(|err| Error::other(format!("valid batch should normalize: {err:?}")))?;
        let after = normalized.total_byte_shift();

        assert_eq!(before, 0);
        assert_eq!(after, 0);
        Ok(())
    }
}
