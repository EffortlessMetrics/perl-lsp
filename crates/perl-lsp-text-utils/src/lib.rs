#![warn(missing_docs)]
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
        if self.source.as_bytes().get(after_semi) == Some(&b'\n') {
            after_semi + 1
        } else {
            after_semi
        }
    }

    /// Find where to insert an extracted subroutine near `current_pos`.
    #[must_use]
    pub fn find_subroutine_insert_position(&self, current_pos: usize) -> usize {
        let search_end = current_pos.min(self.source.len());
        self.source[..search_end]
            .rfind("sub ")
            .unwrap_or(self.source.len())
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

        self.source[line_start..]
            .chars()
            .take_while(|ch| *ch == ' ' || *ch == '\t')
            .collect()
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
