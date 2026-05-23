use crate::PerlLexer;

impl PerlLexer<'_> {
    /// General-purpose balanced-segment consumer (no quote-boundary recovery).
    ///
    /// For use inside double-quoted string interpolation where the outer `"` must
    /// act as a recovery boundary, use [`consume_balanced_segment_in_string`] instead.
    #[allow(dead_code)] // Recovery helper retained for future interpolation callers.
    #[inline]
    pub(crate) fn consume_balanced_segment(&mut self, open: char, close: char) -> Option<usize> {
        if self.current_char() != Some(open) {
            return None;
        }

        let mut depth = 1usize;
        self.advance();
        while let Some(ch) = self.current_char() {
            match ch {
                '\\' => {
                    self.advance();
                    if self.current_char().is_some() {
                        self.advance();
                    }
                }
                c if c == open => {
                    depth += 1;
                    self.advance();
                }
                c if c == close => {
                    self.advance();
                    depth -= 1;
                    if depth == 0 {
                        return Some(self.position);
                    }
                }
                _ => self.advance(),
            }
        }

        None
    }

    #[inline]
    pub(crate) fn consume_balanced_segment_in_string(
        &mut self,
        open: char,
        close: char,
        terminator: char,
    ) -> Option<usize> {
        if self.current_char() != Some(open) {
            return None;
        }

        let mut depth = 1usize;
        self.advance();
        while let Some(ch) = self.current_char() {
            match ch {
                '\\' => {
                    self.advance();
                    if self.current_char().is_some() {
                        self.advance();
                    }
                }
                c if c == terminator => {
                    // Local recovery for interpolation tails in quoted strings:
                    // stop at the closing quote so the outer string parser can
                    // still terminate this token cleanly.
                    return None;
                }
                c if c == open => {
                    depth += 1;
                    self.advance();
                }
                c if c == close => {
                    self.advance();
                    depth -= 1;
                    if depth == 0 {
                        return Some(self.position);
                    }
                }
                _ => self.advance(),
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn consume_balanced_segment_handles_nested_segments_and_escapes() {
        let mut lexer = PerlLexer::new("(a(\\)b)c)");

        let end = lexer.consume_balanced_segment('(', ')');

        assert_eq!(end, Some(9));
        assert_eq!(lexer.position, 9);
    }

    #[test]
    fn consume_balanced_segment_returns_none_for_unbalanced_segment() {
        let mut lexer = PerlLexer::new("(a(b)");

        let end = lexer.consume_balanced_segment('(', ')');

        assert_eq!(end, None);
    }

    #[test]
    fn consume_balanced_segment_in_string_stops_at_terminator_for_recovery() {
        let mut lexer = PerlLexer::new("(${foo\"tail");

        let end = lexer.consume_balanced_segment_in_string('(', ')', '"');

        assert_eq!(end, None);
        assert_eq!(lexer.current_char(), Some('"'));
    }
}
