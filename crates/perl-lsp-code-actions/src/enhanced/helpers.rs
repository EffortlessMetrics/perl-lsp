//! Helper methods for enhanced code actions

use perl_source_editing::{
    find_import_insert_position, find_pragma_insert_position, find_statement_start, get_indent_at,
    has_non_ascii_content, truncate_expr,
};

/// Helper methods for enhanced code actions
pub struct Helpers<'a> {
    pub source: &'a str,
    pub lines: &'a [String],
}

impl<'a> Helpers<'a> {
    /// Create a new helper
    pub fn new(source: &'a str, lines: &'a [String]) -> Self {
        Self { source, lines }
    }

    /// Find statement start
    pub fn find_statement_start(&self, pos: usize) -> usize {
        find_statement_start(self.source, pos)
    }

    /// Find subroutine insertion position
    pub fn find_subroutine_insert_position(&self, current_pos: usize) -> usize {
        // Find the current subroutine
        let mut pos = current_pos;
        while pos > 0 {
            if self.source[pos.saturating_sub(4)..pos].starts_with("sub ") {
                // Found a sub, insert before it
                return pos.saturating_sub(4);
            }
            pos = pos.saturating_sub(1);
        }

        // No sub found, insert at end
        self.source.len()
    }

    /// Find pragma insertion position
    pub fn find_pragma_insert_position(&self) -> usize {
        find_pragma_insert_position(self.source)
    }

    /// Find import insertion position
    pub fn find_import_insert_position(&self) -> usize {
        find_import_insert_position(self.source, self.lines)
    }

    /// Get indentation at position
    pub fn get_indent_at(&self, pos: usize) -> String {
        get_indent_at(self.source, pos)
    }

    /// Truncate expression for display
    pub fn truncate_expr(&self, expr: &str, max_len: usize) -> String {
        truncate_expr(expr, max_len)
    }

    /// Check if content has non-ASCII characters
    pub fn has_non_ascii_content(&self) -> bool {
        has_non_ascii_content(self.source)
    }
}
