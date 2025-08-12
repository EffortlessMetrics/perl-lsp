//! Edit tracking for incremental parsing
//!
//! This module provides types and algorithms for tracking edits to source code
//! and applying them to an existing parse tree.

use crate::position::{Position, Range};

/// Represents an edit to the source text
#[derive(Debug, Clone, PartialEq)]
pub struct Edit {
    /// Start byte offset of the edit
    pub start_byte: usize,
    /// End byte offset of the text being replaced (in old source)
    pub old_end_byte: usize,
    /// End byte offset after the edit (in new source)
    pub new_end_byte: usize,
    /// Start position (line/column)
    pub start_position: Position,
    /// Old end position before edit
    pub old_end_position: Position,
    /// New end position after edit
    pub new_end_position: Position,
}

impl Edit {
    /// Create a new edit
    pub fn new(
        start_byte: usize,
        old_end_byte: usize,
        new_end_byte: usize,
        start_position: Position,
        old_end_position: Position,
        new_end_position: Position,
    ) -> Self {
        Edit {
            start_byte,
            old_end_byte,
            new_end_byte,
            start_position,
            old_end_position,
            new_end_position,
        }
    }

    /// Calculate the byte shift caused by this edit
    pub fn byte_shift(&self) -> isize {
        self.new_end_byte as isize - self.old_end_byte as isize
    }

    /// Calculate the line shift caused by this edit
    pub fn line_shift(&self) -> i32 {
        self.new_end_position.line as i32 - self.old_end_position.line as i32
    }

    /// Check if a byte position is affected by this edit
    pub fn affects_byte(&self, byte: usize) -> bool {
        byte >= self.start_byte
    }

    /// Check if a range overlaps with this edit
    pub fn overlaps_range(&self, range: &Range) -> bool {
        range.start.byte < self.old_end_byte && range.end.byte > self.start_byte
    }

    /// Apply this edit to a position
    pub fn apply_to_position(&self, pos: Position) -> Option<Position> {
        if pos.byte < self.start_byte {
            // Position is before the edit - unchanged
            Some(pos)
        } else if pos.byte >= self.old_end_byte {
            // Position is after the edit - shift it
            Some(Position {
                byte: (pos.byte as isize + self.byte_shift()) as usize,
                line: (pos.line as i32 + self.line_shift()) as u32,
                column: if pos.line == self.old_end_position.line {
                    // Same line as edit end - adjust column
                    let col_shift =
                        self.new_end_position.column as i32 - self.old_end_position.column as i32;
                    (pos.column as i32 + col_shift) as u32
                } else {
                    // Different line - column unchanged
                    pos.column
                },
            })
        } else {
            // Position is within the edit - invalidate
            None
        }
    }

    /// Apply this edit to a range
    pub fn apply_to_range(&self, range: &Range) -> Option<Range> {
        let new_start = self.apply_to_position(range.start)?;
        let new_end = self.apply_to_position(range.end)?;
        Some(Range::new(new_start, new_end))
    }
}

/// Collection of edits that can be applied together
#[derive(Debug, Clone, Default)]
pub struct EditSet {
    pub(crate) edits: Vec<Edit>,
}

impl EditSet {
    /// Create a new empty edit set
    pub fn new() -> Self {
        EditSet { edits: Vec::new() }
    }

    /// Add an edit to the set
    pub fn add(&mut self, edit: Edit) {
        // Keep edits sorted by start position
        let pos = self
            .edits
            .iter()
            .position(|e| e.start_byte > edit.start_byte)
            .unwrap_or(self.edits.len());
        self.edits.insert(pos, edit);
    }

    /// Apply all edits to a position
    pub fn apply_to_position(&self, mut pos: Position) -> Option<Position> {
        for edit in &self.edits {
            pos = edit.apply_to_position(pos)?;
        }
        Some(pos)
    }

    /// Apply all edits to a range
    pub fn apply_to_range(&self, mut range: Range) -> Option<Range> {
        for edit in &self.edits {
            range = edit.apply_to_range(&range)?;
        }
        Some(range)
    }

    /// Check if a range is affected by any edit
    pub fn affects_range(&self, range: &Range) -> bool {
        self.edits.iter().any(|edit| edit.overlaps_range(range))
    }

    /// Get the total byte shift at a given position
    pub fn byte_shift_at(&self, byte: usize) -> isize {
        self.edits
            .iter()
            .filter(|edit| edit.old_end_byte <= byte)
            .map(|edit| edit.byte_shift())
            .sum()
    }

    /// Get all ranges affected by the edits
    pub fn affected_ranges(&self) -> Vec<Range> {
        self.edits
            .iter()
            .map(|edit| Range::new(edit.start_position, edit.old_end_position))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_edit() {
        // Replace "hello" with "goodbye" at position 10
        let edit = Edit::new(
            10,
            15,
            17,
            Position::new(10, 2, 5),
            Position::new(15, 2, 10),
            Position::new(17, 2, 12),
        );

        assert_eq!(edit.byte_shift(), 2);
        assert_eq!(edit.line_shift(), 0);

        // Position before edit - unchanged
        let pos = Position::new(5, 1, 5);
        assert_eq!(edit.apply_to_position(pos), Some(pos));

        // Position after edit - shifted
        let pos = Position::new(20, 2, 15);
        let new_pos = edit.apply_to_position(pos).unwrap();
        assert_eq!(new_pos.byte, 22);
        assert_eq!(new_pos.column, 17);
    }

    #[test]
    fn test_multiline_edit() {
        // Replace multiple lines
        let edit = Edit::new(
            10,
            30,
            20,
            Position::new(10, 2, 5),
            Position::new(30, 4, 10),
            Position::new(20, 2, 15),
        );

        assert_eq!(edit.byte_shift(), -10);
        assert_eq!(edit.line_shift(), -2);

        // Position on later line - shifted
        let pos = Position::new(50, 6, 5);
        let new_pos = edit.apply_to_position(pos).unwrap();
        assert_eq!(new_pos.byte, 40);
        assert_eq!(new_pos.line, 4);
        assert_eq!(new_pos.column, 5);
    }

    #[test]
    fn test_edit_set() {
        let mut edits = EditSet::new();

        // Add two non-overlapping edits
        edits.add(Edit::new(
            10,
            15,
            17,
            Position::new(10, 2, 5),
            Position::new(15, 2, 10),
            Position::new(17, 2, 12),
        ));

        edits.add(Edit::new(
            30,
            35,
            40,
            Position::new(30, 3, 5),
            Position::new(35, 3, 10),
            Position::new(40, 3, 15),
        ));

        // Check cumulative shift
        assert_eq!(edits.byte_shift_at(50), 7); // +2 from first, +5 from second
    }
}
