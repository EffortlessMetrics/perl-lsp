//! Enhanced edit structure for incremental parsing with text content
//!
//! This module provides an extended Edit type that includes the new text
//! being inserted, enabling efficient incremental parsing with subtree reuse.

use perl_parser_core::position::Position;

/// Validation failures for an [`IncrementalEditSet`] batch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IncrementalEditBatchError {
    /// An edit uses a backward range (`start_byte > old_end_byte`).
    ///
    /// `index` is the position of the offending edit in the **original,
    /// unsorted** order at the time `normalize_and_validate` was called.
    BackwardRange { index: usize, start_byte: usize, old_end_byte: usize },
    /// Two normalized edits overlap in the old-source byte space.
    ///
    /// `left_index` and `right_index` are positions in the **post-sort**
    /// order produced by [`IncrementalEditSet::sort_reverse_deterministic`],
    /// not in the original insertion order.
    OverlappingEdits { left_index: usize, right_index: usize },
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
        self.new_text.len() as isize - (self.old_end_byte - self.start_byte) as isize
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

    /// Normalize edit ordering for deterministic reverse-application and validate batch shape.
    ///
    /// Normalization behavior:
    /// - removes obvious no-op edits when `filter_no_ops` is `true`
    /// - sorts edits in deterministic reverse-application order (highest byte offset first)
    ///
    /// Validation behavior:
    /// - rejects backward ranges (`start_byte > old_end_byte`)
    /// - rejects overlaps after normalization unless `allow_overlaps` is `true`
    pub fn normalize_and_validate(
        &mut self,
        allow_overlaps: bool,
        filter_no_ops: bool,
    ) -> Result<(), IncrementalEditBatchError> {
        for (index, edit) in self.edits.iter().enumerate() {
            if edit.start_byte > edit.old_end_byte {
                return Err(IncrementalEditBatchError::BackwardRange {
                    index,
                    start_byte: edit.start_byte,
                    old_end_byte: edit.old_end_byte,
                });
            }
        }

        if filter_no_ops {
            self.edits.retain(|edit| !edit.is_obvious_no_op());
        }

        self.sort_reverse_deterministic();

        if !allow_overlaps {
            for left in 0..self.edits.len() {
                for right in (left + 1)..self.edits.len() {
                    if self.edits[left]
                        .overlaps(self.edits[right].start_byte, self.edits[right].old_end_byte)
                    {
                        return Err(IncrementalEditBatchError::OverlappingEdits {
                            left_index: left,
                            right_index: right,
                        });
                    }
                }
            }
        }

        Ok(())
    }

    /// Sort edits by position (for correct application order)
    pub fn sort(&mut self) {
        self.edits.sort_by_key(|e| e.start_byte);
    }

    /// Sort edits in reverse order (for applying from end to start)
    pub fn sort_reverse(&mut self) {
        self.edits.sort_by_key(|e| std::cmp::Reverse(e.start_byte));
    }

    /// Sort edits in a deterministic reverse-application order.
    ///
    /// This keeps behavior stable even when multiple edits share a start byte.
    pub fn sort_reverse_deterministic(&mut self) {
        self.edits.sort_by(|left, right| {
            right
                .start_byte
                .cmp(&left.start_byte)
                .then_with(|| right.old_end_byte.cmp(&left.old_end_byte))
                .then_with(|| left.new_text.cmp(&right.new_text))
        });
    }

    /// Check if the edit set is empty
    pub fn is_empty(&self) -> bool {
        self.edits.is_empty()
    }

    /// Get the total byte shift for all edits
    pub fn total_byte_shift(&self) -> isize {
        self.edits.iter().map(|e| e.byte_shift()).sum()
    }

    /// Apply edits to a string using deterministic fallback semantics.
    ///
    /// This is intentionally more tolerant than [`Self::normalize_for_source`]:
    /// the incremental fast path rejects overlapping or unmappable batches,
    /// while this fallback applies every edit that still maps safely in reverse
    /// byte order and skips edits that cannot be applied without panicking.
    pub fn apply_to_string(&self, source: &str) -> String {
        if self.edits.is_empty() {
            return source.to_string();
        }

        let mut edits = self.edits.clone();
        edits.sort_by(|left, right| {
            right
                .start_byte
                .cmp(&left.start_byte)
                .then_with(|| right.old_end_byte.cmp(&left.old_end_byte))
                .then_with(|| left.new_text.cmp(&right.new_text))
        });

        let mut result = source.to_string();
        for edit in &edits {
            if !Self::is_edit_mappable(&result, edit) {
                continue;
            }
            result.replace_range(edit.start_byte..edit.old_end_byte, &edit.new_text);
        }

        result
    }

    /// Validate and normalize edits for deterministic reverse-order application.
    ///
    /// Returns `None` when any edit is not safely mappable for the given source
    /// (backwards range, out-of-bounds range, or non-UTF-8 boundaries), or when
    /// non-empty ranges overlap.
    pub fn normalize_for_source(&self, source: &str) -> Option<Vec<IncrementalEdit>> {
        let mut indexed = Vec::with_capacity(self.edits.len());
        for (index, edit) in self.edits.iter().enumerate() {
            if !Self::is_edit_mappable(source, edit) {
                return None;
            }
            indexed.push((index, edit.clone()));
        }

        indexed.sort_by(|(_, left), (_, right)| {
            right
                .start_byte
                .cmp(&left.start_byte)
                .then_with(|| right.old_end_byte.cmp(&left.old_end_byte))
        });

        let mut previous_non_empty: Option<&IncrementalEdit> = None;
        for (_, edit) in &indexed {
            if edit.start_byte == edit.old_end_byte {
                continue;
            }

            if let Some(previous) = previous_non_empty {
                if edit.old_end_byte > previous.start_byte {
                    return None;
                }
            }
            previous_non_empty = Some(edit);
        }

        Some(indexed.into_iter().map(|(_, edit)| edit).collect())
    }

    fn is_edit_mappable(source: &str, edit: &IncrementalEdit) -> bool {
        if edit.start_byte > edit.old_end_byte {
            return false;
        }
        if edit.old_end_byte > source.len() {
            return false;
        }
        source.is_char_boundary(edit.start_byte) && source.is_char_boundary(edit.old_end_byte)
    }
}

impl IncrementalEdit {
    fn is_obvious_no_op(&self) -> bool {
        self.start_byte == self.old_end_byte && self.new_text.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn test_apply_to_string_rejects_invalid_char_boundary_edits() {
        let mut edits = IncrementalEditSet::new();
        edits.add(IncrementalEdit::new(1, 2, "x".to_string()));

        let source = "éa";
        let result = edits.apply_to_string(source);
        assert_eq!(result, source);
    }

    #[test]
    fn test_apply_to_string_applies_overlapping_edits_deterministically() {
        let mut edits = IncrementalEditSet::new();
        edits.add(IncrementalEdit::new(1, 4, "x".to_string()));
        edits.add(IncrementalEdit::new(3, 5, "y".to_string()));

        let source = "abcdef";
        let result = edits.apply_to_string(source);
        assert_eq!(result, "axf");
    }

    #[test]
    fn test_normalize_for_source_rejects_backwards_ranges() {
        let mut edits = IncrementalEditSet::new();
        edits.add(IncrementalEdit::new(4, 2, "x".to_string()));

        assert!(edits.normalize_for_source("abcdef").is_none());
    }

    #[test]
    fn test_normalize_for_source_rejects_overlapping_non_empty_ranges() {
        let mut edits = IncrementalEditSet::new();
        edits.add(IncrementalEdit::new(1, 4, "x".to_string()));
        edits.add(IncrementalEdit::new(3, 5, "y".to_string()));

        assert!(edits.normalize_for_source("abcdef").is_none());
    }

    #[test]
    fn test_normalize_for_source_preserves_zero_width_insertions() {
        let mut edits = IncrementalEditSet::new();
        edits.add(IncrementalEdit::new(2, 2, "X".to_string()));
        edits.add(IncrementalEdit::new(2, 2, "Y".to_string()));

        let normalized = edits.normalize_for_source("abcd");
        assert!(normalized.is_some());
        if let Some(normalized) = normalized {
            assert_eq!(normalized.len(), 2);
            assert_eq!(normalized[0].start_byte, 2);
            assert_eq!(normalized[1].start_byte, 2);
        }
    }
}
