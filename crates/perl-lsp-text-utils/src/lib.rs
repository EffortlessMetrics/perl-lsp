//! Text helpers for code-action style source edits.

/// Helper wrapper for source text and pre-split lines.
pub struct TextEditHelpers<'a> {
    source: &'a str,
    lines: &'a [String],
}

impl<'a> TextEditHelpers<'a> {
    /// Create a new helper view.
    #[must_use]
    pub fn new(source: &'a str, lines: &'a [String]) -> Self {
        Self { source, lines }
    }

    /// Borrow the source lines backing this helper.
    #[must_use]
    pub fn lines(&self) -> &'a [String] {
        self.lines
    }

    /// Find the start of the statement containing `pos`.
    #[must_use]
    pub fn find_statement_start(&self, pos: usize) -> usize {
        self.source
            .char_indices()
            .take_while(|(idx, _)| *idx < pos)
            .filter(|(_, ch)| *ch == ';' || *ch == '\n')
            .map(|(idx, _)| idx + 1)
            .last()
            .unwrap_or(0)
    }

    /// Find where to insert an extracted subroutine near `current_pos`.
    #[must_use]
    pub fn find_subroutine_insert_position(&self, current_pos: usize) -> usize {
        let search_end = current_pos.min(self.source.len());
        self.source[..search_end].rfind("sub ").unwrap_or(self.source.len())
    }

    /// Find where leading pragmas should be inserted.
    #[must_use]
    pub fn find_pragma_insert_position(&self) -> usize {
        if self.source.starts_with("#!")
            && let Some(pos) = self.source.find('\n')
        {
            return pos + 1;
        }
        0
    }

    /// Find where imports should be inserted.
    #[must_use]
    pub fn find_import_insert_position(&self) -> usize {
        let mut pos = self.find_pragma_insert_position();

        for line in self.lines {
            if line.starts_with("use ") || line.starts_with("require ") {
                pos = self.source.find(line).unwrap_or(0) + line.len() + 1;
            } else if !line.is_empty() && !line.starts_with('#') {
                break;
            }
        }

        pos
    }

    /// Get leading indentation at the line containing `pos`.
    #[must_use]
    pub fn get_indent_at(&self, pos: usize) -> String {
        let safe_pos = pos.min(self.source.len());
        let line_start = self.source[..safe_pos].rfind('\n').map_or(0, |p| p + 1);

        self.source[line_start..].chars().take_while(|ch| *ch == ' ' || *ch == '\t').collect()
    }

    /// Truncate an expression for display.
    #[must_use]
    pub fn truncate_expr(&self, expr: &str, max_len: usize) -> String {
        if expr.chars().count() <= max_len {
            return expr.to_string();
        }

        if max_len <= 3 {
            return "...".to_string();
        }

        format!("{}...", expr.chars().take(max_len - 3).collect::<String>())
    }

    /// Whether the source includes non-ASCII content.
    #[must_use]
    pub fn has_non_ascii_content(&self) -> bool {
        !self.source.is_ascii()
    }
}
