//! Enhanced edit structure for incremental parsing with text content
//!
//! This module provides an extended Edit type that includes the new text
//! being inserted, enabling efficient incremental parsing with subtree reuse.

use crate::position::Position;

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
        IncrementalEdit {
            start_byte,
            old_end_byte,
            new_text,
            start_position,
            old_end_position,
        }
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

    /// Sort edits by position (for correct application order)
    pub fn sort(&mut self) {
        self.edits.sort_by_key(|e| e.start_byte);
    }

    /// Sort edits in reverse order (for applying from end to start)
    pub fn sort_reverse(&mut self) {
        self.edits.sort_by_key(|e| std::cmp::Reverse(e.start_byte));
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
        sorted_edits.sort_by_key(|e| std::cmp::Reverse(e.start_byte));

        let mut result = source.to_string();
        for edit in &sorted_edits {
            result.replace_range(edit.start_byte..edit.old_end_byte, &edit.new_text);
        }

        result
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
