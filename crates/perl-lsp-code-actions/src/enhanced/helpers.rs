//! Helper methods for enhanced code actions

/// Helper methods for enhanced code actions
pub struct Helpers<'a> {
    pub source: &'a str,
    pub lines: &'a Vec<String>,
}

impl<'a> Helpers<'a> {
    /// Create a new helper
    pub fn new(source: &'a str, lines: &'a Vec<String>) -> Self {
        Self { source, lines }
    }

    /// Find statement start
    pub fn find_statement_start(&self, pos: usize) -> usize {
        let mut i = pos.saturating_sub(1);
        while i > 0 {
            if self.source.chars().nth(i) == Some(';') || self.source.chars().nth(i) == Some('\n') {
                return i + 1;
            }
            i = i.saturating_sub(1);
        }
        0
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
        // After shebang if present
        if self.source.starts_with("#!")
            && let Some(pos) = self.source.find('\n')
        {
            return pos + 1;
        }
        0
    }

    /// Find import insertion position
    pub fn find_import_insert_position(&self) -> usize {
        // After existing pragmas
        let mut pos = self.find_pragma_insert_position();

        for line in self.lines.iter() {
            if line.starts_with("use ") || line.starts_with("require ") {
                pos = self.source.find(line).unwrap_or(0) + line.len() + 1;
            } else if !line.is_empty() && !line.starts_with('#') {
                break;
            }
        }

        pos
    }

    /// Get indentation at position
    pub fn get_indent_at(&self, pos: usize) -> String {
        let line_start = self.source[..pos].rfind('\n').map(|p| p + 1).unwrap_or(0);

        let line = &self.source[line_start..];
        let mut indent = String::new();
        for ch in line.chars() {
            if ch == ' ' || ch == '\t' {
                indent.push(ch);
            } else {
                break;
            }
        }
        indent
    }

    /// Truncate expression for display
    pub fn truncate_expr(&self, expr: &str, max_len: usize) -> String {
        if expr.len() <= max_len {
            expr.to_string()
        } else {
            format!("{}...", &expr[..max_len - 3])
        }
    }

    /// Check if content has non-ASCII characters
    pub fn has_non_ascii_content(&self) -> bool {
        self.source.chars().any(|c| c as u32 > 127)
    }
}
