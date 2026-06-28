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

// ============================================================================
// balanced_segment_conformance — inline tests for consume_balanced_segment
//
// This test block is one of TWO that together form the conformance matrix for
// balanced-segment consumption across the workspace (#1323).
//
// The same input set (is_balanced contract) is tested in:
//   - crates/perl-parser-core/src/engine/parser/expressions/primary.rs
//     (consume_balanced_in_interpolated_string)
//
// Normalized contract:
//   - is_balanced: does the segment have a matching close before the boundary?
//   - end_offset: byte offset one-past the closing delimiter (lexer only)
//
// Adapter: consume_balanced_segment(open, close) -> Option<usize>
//   Some(pos) => (true,  Some(pos))
//   None      => (false, None)
//
// DIVERGENCE SUMMARY (no semantic divergence found for the shared input set):
//   - Structural: lexer uses a char-level terminator for string-boundary
//     recovery; parser-core uses a quote_end byte index. Both return
//     "unbalanced" when the boundary is hit.
//   - Structural: backslash-at-EOF — lexer checks current_char().is_some()
//     before the second advance; parser-core uses i.saturating_add(2). Both
//     correctly return unbalanced for a trailing backslash.
//   - Verdict: AGREE on all inputs in the shared matrix. Safe to centralize
//     once a follow-up refactor PR is scoped.
// ============================================================================
#[cfg(test)]
mod balanced_segment_conformance {
    use super::*;

    /// Normalize `consume_balanced_segment` to `(is_balanced, end_offset)`.
    fn normalize(input: &str, open: char, close: char) -> (bool, Option<usize>) {
        let mut lexer = PerlLexer::new(input);
        let result = lexer.consume_balanced_segment(open, close);
        (result.is_some(), result)
    }

    /// Normalize `consume_balanced_segment_in_string` to `(is_balanced, end_offset)`.
    /// The `terminator` parameter gates early exit (string-boundary recovery).
    fn normalize_in_string(
        input: &str,
        open: char,
        close: char,
        terminator: char,
    ) -> (bool, Option<usize>) {
        let mut lexer = PerlLexer::new(input);
        let result = lexer.consume_balanced_segment_in_string(open, close, terminator);
        (result.is_some(), result)
    }

    // -----------------------------------------------------------------------
    // Simple balanced cases — both impls must agree: is_balanced = true
    // -----------------------------------------------------------------------

    #[test]
    fn simple_parens_balanced() {
        // "(a b c)" — one level, no escapes
        let (balanced, end) = normalize("(a b c)", '(', ')');
        assert!(balanced, "lexer: '(a b c)' should be balanced");
        assert_eq!(end, Some(7), "lexer: end offset after '(a b c)'");
    }

    #[test]
    fn simple_braces_balanced() {
        // "{x}" — curly braces
        let (balanced, end) = normalize("{x}", '{', '}');
        assert!(balanced, "lexer: '{{x}}' should be balanced");
        assert_eq!(end, Some(3), "lexer: end offset after '{{x}}'");
    }

    #[test]
    fn simple_brackets_balanced() {
        // "[1]" — square brackets
        let (balanced, end) = normalize("[1]", '[', ']');
        assert!(balanced, "lexer: '[1]' should be balanced");
        assert_eq!(end, Some(3), "lexer: end offset after '[1]'");
    }

    // -----------------------------------------------------------------------
    // Nested balanced cases
    // -----------------------------------------------------------------------

    #[test]
    fn nested_parens_balanced() {
        // "(a (b) c)" — depth 2 then back to 1 then 0
        let (balanced, end) = normalize("(a (b) c)", '(', ')');
        assert!(balanced, "lexer: '(a (b) c)' should be balanced");
        assert_eq!(end, Some(9), "lexer: end offset after '(a (b) c)'");
    }

    #[test]
    fn nested_braces_balanced() {
        // "{ {x} {y} }" — two inner braces (length 11, closing '}' at index 10)
        let (balanced, end) = normalize("{ {x} {y} }", '{', '}');
        assert!(balanced, "lexer: '{{ {{x}} {{y}} }}' should be balanced");
        assert_eq!(end, Some(11), "lexer: end offset after '{{ {{x}} {{y}} }}'");
    }

    // -----------------------------------------------------------------------
    // Escaped delimiter cases — escape prevents the delimiter from closing
    // -----------------------------------------------------------------------

    #[test]
    fn escaped_close_in_middle_balanced() {
        // "(a \) b)" — \) is escaped, real close is at the end
        let (balanced, end) = normalize("(a \\) b)", '(', ')');
        assert!(balanced, "lexer: escaped close '\\\\)' does not close; ')' at end closes");
        assert_eq!(end, Some(8), "lexer: end offset after '(a \\\\) b)'");
    }

    #[test]
    fn escaped_open_in_middle_balanced() {
        // "(a \( b)" — \( is escaped so depth does NOT increase
        let (balanced, end) = normalize("(a \\( b)", '(', ')');
        assert!(balanced, "lexer: escaped open '\\\\(' does not nest; one close suffices");
        assert_eq!(end, Some(8), "lexer: end offset after '(a \\\\( b)'");
    }

    // -----------------------------------------------------------------------
    // Backslash at/near end — trailing backslash; result is unbalanced
    //
    // DIVERGENCE (structural, not semantic):
    //   Lexer: advance past '\\', then checks current_char().is_some() before
    //          the second advance. At EOF the second advance is skipped; loop
    //          exits naturally.
    //   Parser-core: i.saturating_add(2) from the backslash position. If that
    //                overshoots quote_end, the loop exits on the boundary check.
    //   Both correctly return unbalanced (None / false).
    // -----------------------------------------------------------------------

    #[test]
    fn backslash_at_eof_unbalanced() {
        // "(a \" — backslash is the last byte; nothing to escape; never closes
        let (balanced, end) = normalize("(a \\", '(', ')');
        assert!(!balanced, "lexer: trailing backslash at EOF → unbalanced");
        assert_eq!(end, None, "lexer: no end offset for trailing-backslash input");
    }

    #[test]
    fn escaped_close_only_unbalanced() {
        // "(\)" — the ')' is escaped, so the segment never receives a real close
        let (balanced, end) = normalize("(\\)", '(', ')');
        assert!(!balanced, "lexer: '(\\\\)' has only an escaped close → unbalanced");
        assert_eq!(end, None, "lexer: no end offset for '(\\\\)'");
    }

    // -----------------------------------------------------------------------
    // Unbalanced / never-closed cases
    // -----------------------------------------------------------------------

    #[test]
    fn unbalanced_open_only_no_close() {
        // "(a b c" — no closing paren anywhere
        let (balanced, end) = normalize("(a b c", '(', ')');
        assert!(!balanced, "lexer: '(a b c' (no close) → unbalanced");
        assert_eq!(end, None, "lexer: no end offset for '(a b c'");
    }

    // -----------------------------------------------------------------------
    // Empty balanced pair
    // -----------------------------------------------------------------------

    #[test]
    fn empty_pair_balanced() {
        // "()" — open immediately followed by close; depth goes 1→0
        let (balanced, end) = normalize("()", '(', ')');
        assert!(balanced, "lexer: '()' should be balanced");
        assert_eq!(end, Some(2), "lexer: end offset after '()'");
    }

    // -----------------------------------------------------------------------
    // String-boundary variant (_in_string): terminator stops the scan
    //
    // DIVERGENCE (structural, not semantic):
    //   Lexer _in_string: uses a terminator *char*; returns None on first hit.
    //   Parser-core: uses a quote_end *byte index* as exclusive upper bound.
    //   Both return "not balanced" when the relevant boundary is reached.
    //   Correct Perl behavior: unmatched delimiter within a double-quoted
    //   string is an error; the outer string parser handles recovery.
    // -----------------------------------------------------------------------

    #[test]
    fn in_string_stops_at_terminator_unbalanced() {
        // "(foo\"bar" — terminator '"' hit before any close paren
        let (balanced, end) = normalize_in_string("(foo\"bar", '(', ')', '"');
        assert!(!balanced, "lexer in_string: '\"' terminator hit → unbalanced");
        assert_eq!(end, None, "lexer in_string: no end offset when stopped by terminator");
    }

    #[test]
    fn in_string_balanced_before_terminator() {
        // "(foo)\"tail" — ')' closes before '"' terminator
        let (balanced, end) = normalize_in_string("(foo)\"tail", '(', ')', '"');
        assert!(balanced, "lexer in_string: '(foo)' closes before '\"' → balanced");
        assert_eq!(end, Some(5), "lexer in_string: end offset after '(foo)'");
    }

    #[test]
    fn in_string_nested_balanced_before_terminator() {
        // "(a(b)c)\"rest" — nested and fully balanced before terminator
        let (balanced, end) = normalize_in_string("(a(b)c)\"rest", '(', ')', '"');
        assert!(balanced, "lexer in_string: nested '(a(b)c)' balanced before '\"'");
        assert_eq!(end, Some(7), "lexer in_string: end offset after '(a(b)c)'");
    }

    #[test]
    fn in_string_escaped_terminator_keeps_scanning() {
        // "(a\\\"b)" — the backslash escapes the quote; scan continues to ')'
        let (balanced, end) = normalize_in_string("(a\\\"b)", '(', ')', '"');
        assert!(balanced, "lexer in_string: escaped '\\\"' is not a terminator; ')' closes");
        assert_eq!(end, Some(6), "lexer in_string: end offset after '(a\\\\\"b)'");
    }
}
