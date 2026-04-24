//! Enhanced edit structure for incremental parsing with text content
//!
//! This module provides an extended Edit type that includes the new text
//! being inserted, enabling efficient incremental parsing with subtree reuse.

use perl_parser_core::position::Position;

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

/// Validation failures for incremental edit batches.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IncrementalEditSetValidationError {
    /// An edit had a backward range (`start_byte > old_end_byte`).
    BackwardRange { edit_index: usize, start_byte: usize, old_end_byte: usize },
    /// Two edits overlap after normalization.
    OverlappingEdits { first_edit_index: usize, second_edit_index: usize },
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
        self.edits.sort_by(Self::reverse_application_order);
    }

    /// Check if the edit set is empty
    pub fn is_empty(&self) -> bool {
        self.edits.is_empty()
    }

    /// Get the total byte shift for all edits
    pub fn total_byte_shift(&self) -> isize {
        self.edits.iter().map(|e| e.byte_shift()).sum()
    }

    /// Build a deterministically ordered copy for reverse application (end to start).
    pub fn normalized_for_reverse_application(&self) -> Self {
        let mut edits = self.edits.clone();
        edits.sort_by(Self::reverse_application_order);
        Self { edits }
    }

    /// Normalize and validate a batch of edits.
    ///
    /// The returned set is sorted for reverse application order.
    ///
    /// If `allow_overlapping` is `false`, overlapping edits are rejected.
    /// If `filter_no_ops` is `true`, strictly obvious no-ops are removed.
    pub fn normalize_and_validate(
        &self,
        allow_overlapping: bool,
        filter_no_ops: bool,
    ) -> Result<Self, IncrementalEditSetValidationError> {
        let mut indexed_edits = Vec::with_capacity(self.edits.len());

        for (edit_index, edit) in self.edits.iter().enumerate() {
            if edit.start_byte > edit.old_end_byte {
                return Err(IncrementalEditSetValidationError::BackwardRange {
                    edit_index,
                    start_byte: edit.start_byte,
                    old_end_byte: edit.old_end_byte,
                });
            }

            let should_keep = !(filter_no_ops
                && edit.start_byte == edit.old_end_byte
                && edit.new_text.is_empty());
            if should_keep {
                indexed_edits.push((edit_index, edit.clone()));
            }
        }

        if !allow_overlapping {
            let mut forward_order = indexed_edits.clone();
            forward_order.sort_by(Self::forward_validation_order);

            for pair in forward_order.windows(2) {
                if let [previous, current] = pair
                    && previous.1.old_end_byte > current.1.start_byte
                {
                    return Err(IncrementalEditSetValidationError::OverlappingEdits {
                        first_edit_index: previous.0,
                        second_edit_index: current.0,
                    });
                }
            }
        }

        indexed_edits.sort_by(Self::indexed_reverse_application_order);
        let edits = indexed_edits.into_iter().map(|(_, edit)| edit).collect();
        Ok(Self { edits })
    }

    /// Apply edits to a string
    pub fn apply_to_string(&self, source: &str) -> String {
        if self.edits.is_empty() {
            return source.to_string();
        }

        // Sort edits in reverse order to apply from end to start
        let sorted_edits = self.normalized_for_reverse_application().edits;

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

    fn reverse_application_order(
        left: &IncrementalEdit,
        right: &IncrementalEdit,
    ) -> std::cmp::Ordering {
        right
            .start_byte
            .cmp(&left.start_byte)
            .then_with(|| right.old_end_byte.cmp(&left.old_end_byte))
            .then_with(|| right.new_text.len().cmp(&left.new_text.len()))
            .then_with(|| right.new_text.cmp(&left.new_text))
            .then_with(|| right.start_position.byte.cmp(&left.start_position.byte))
            .then_with(|| right.start_position.line.cmp(&left.start_position.line))
            .then_with(|| right.start_position.column.cmp(&left.start_position.column))
            .then_with(|| right.old_end_position.byte.cmp(&left.old_end_position.byte))
            .then_with(|| right.old_end_position.line.cmp(&left.old_end_position.line))
            .then_with(|| right.old_end_position.column.cmp(&left.old_end_position.column))
    }

    fn indexed_reverse_application_order(
        left: &(usize, IncrementalEdit),
        right: &(usize, IncrementalEdit),
    ) -> std::cmp::Ordering {
        Self::reverse_application_order(&left.1, &right.1).then_with(|| left.0.cmp(&right.0))
    }

    fn forward_validation_order(
        left: &(usize, IncrementalEdit),
        right: &(usize, IncrementalEdit),
    ) -> std::cmp::Ordering {
        left.1
            .start_byte
            .cmp(&right.1.start_byte)
            .then_with(|| left.1.old_end_byte.cmp(&right.1.old_end_byte))
            .then_with(|| left.0.cmp(&right.0))
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
}
