#![warn(missing_docs)]
//! Text helpers for code-action style source edits.

/// Helper wrapper for source text and pre-split lines.
pub struct TextEditHelpers<'a> {
    source: &'a str,
    lines: &'a [String],
}

impl<'a> TextEditHelpers<'a> {
    fn skip_line_ending(&self, pos: usize) -> usize {
        match (self.source.as_bytes().get(pos), self.source.as_bytes().get(pos.saturating_add(1))) {
            (Some(b'\r'), Some(b'\n')) => pos + 2,
            (Some(b'\n' | b'\r'), _) => pos + 1,
            _ => pos,
        }
    }

    fn next_line_bounds(&self, start: usize) -> Option<(usize, usize)> {
        if start >= self.source.len() {
            return None;
        }

        let bytes = self.source.as_bytes();
        let mut end = start;
        while let Some(byte) = bytes.get(end) {
            if *byte == b'\n' || *byte == b'\r' {
                break;
            }
            end += 1;
        }

        Some((start, end))
    }

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
    ///
    /// Only `;` is treated as a statement boundary. Newlines are not statement
    /// boundaries in Perl — a multi-line expression like `some_func(\n    $arg)`
    /// is a single statement, so treating `\n` as a boundary would insert the
    /// extracted declaration inside the argument list.
    ///
    /// After finding the position immediately following a `;`, a single trailing
    /// `\n` is skipped so that the returned position is the first character of
    /// the next statement line, not the newline between statements.  This keeps
    /// the inserted declaration on its own line rather than appended to the end
    /// of the preceding statement.
    #[must_use]
    pub fn find_statement_start(&self, pos: usize) -> usize {
        let after_semi = self
            .source
            .char_indices()
            .take_while(|(idx, _)| *idx < pos)
            .filter(|(_, ch)| *ch == ';')
            .map(|(idx, _)| idx + 1)
            .last()
            .unwrap_or(0);
        // Skip a single newline that immediately follows the semicolon so the
        // insertion point is the first real character of the next line.
        self.skip_line_ending(after_semi)
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
            && let Some((_, line_end)) = self.next_line_bounds(0)
        {
            let next = self.skip_line_ending(line_end);
            if next > line_end {
                return next;
            }
        }
        0
    }

    /// Find where imports should be inserted.
    #[must_use]
    pub fn find_import_insert_position(&self) -> usize {
        let mut pos = self.find_pragma_insert_position();
        let mut cursor = pos;

        while let Some((line_start, line_end)) = self.next_line_bounds(cursor) {
            let line = &self.source[line_start..line_end];
            if line.starts_with("use ") || line.starts_with("require ") {
                pos = self.skip_line_ending(line_end);
            } else if !line.is_empty() && !line.starts_with('#') {
                break;
            }

            let next_cursor = self.skip_line_ending(line_end);
            if next_cursor == cursor {
                break;
            }
            cursor = next_cursor;
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
