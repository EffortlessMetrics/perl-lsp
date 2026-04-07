//! Context-sensitive parsing for Perl operators
//!
//! This module handles operators like s///, tr///, and m// that require
//! context-sensitive parsing to distinguish from regular identifiers.
//! It also handles quote-like operators: q, qq, qw, qr, qx.

/// Context-sensitive token types
#[derive(Debug, Clone, PartialEq)]
pub enum ContextToken {
    Substitution {
        pattern: String,
        replacement: String,
        flags: String,
    },
    Transliteration {
        search: String,
        replace: String,
        flags: String,
    },
    Match {
        pattern: String,
        flags: String,
    },
    /// Quote-like operators: q, qq, qw, qr, qx
    ///
    /// - `operator`: the operator keyword ("q", "qq", "qw", "qr", "qx")
    /// - `content`: the delimited content
    /// - `flags`: modifier flags (only non-empty for `qr`, e.g. "imsx")
    QuoteOp {
        operator: String,
        content: String,
        flags: String,
    },
    Identifier(String),
}

/// Context-sensitive lexer for Perl operators
pub struct ContextSensitiveLexer {
    input: String,
    position: usize,
}

impl ContextSensitiveLexer {
    pub fn new(input: String) -> Self {
        Self { input, position: 0 }
    }

    /// Peek at the next character without consuming it
    fn peek(&self) -> Option<char> {
        self.input.chars().nth(self.position)
    }

    /// Peek at the next n characters
    fn peek_str(&self, n: usize) -> &str {
        let end = (self.position + n).min(self.input.len());
        &self.input[self.position..end]
    }

    /// Consume and return the next character
    fn next_char(&mut self) -> Option<char> {
        let ch = self.input.chars().nth(self.position)?;
        self.position += ch.len_utf8();
        Some(ch)
    }

    /// Try to parse a context-sensitive operator
    pub fn try_parse_operator(&mut self) -> Option<ContextToken> {
        match self.peek()? {
            's' => self.try_parse_substitution(),
            't' => self.try_parse_transliteration(),
            'm' => self.try_parse_match(),
            'q' => self.try_parse_quote_operator(),
            _ => None,
        }
    }

    /// Try to parse s/// substitution operator
    fn try_parse_substitution(&mut self) -> Option<ContextToken> {
        let start_pos = self.position;

        // Check for 's' followed by delimiter
        if self.peek_str(1) != "s" {
            return None;
        }
        self.next_char(); // consume 's'

        // Get the delimiter
        let delimiter = match self.peek()? {
            c if !c.is_alphanumeric() && !c.is_whitespace() => c,
            _ => {
                self.position = start_pos;
                return None;
            }
        };
        self.next_char(); // consume delimiter

        // Parse pattern
        let pattern = self.parse_until_delimiter(delimiter, true)?;

        // Parse replacement
        let replacement = self.parse_until_delimiter(delimiter, false)?;

        // Parse flags
        let flags = self.parse_regex_flags();

        Some(ContextToken::Substitution { pattern, replacement, flags })
    }

    /// Try to parse tr/// or y/// transliteration operator
    fn try_parse_transliteration(&mut self) -> Option<ContextToken> {
        let start_pos = self.position;

        // Check for 'tr' or just 't' (for tr///)
        if self.peek_str(2) == "tr" {
            self.position += 2;
        } else if self.peek_str(1) == "y" {
            self.position += 1;
        } else {
            return None;
        }

        // Get the delimiter
        let delimiter = match self.peek()? {
            c if !c.is_alphanumeric() && !c.is_whitespace() => c,
            _ => {
                self.position = start_pos;
                return None;
            }
        };
        self.next_char(); // consume delimiter

        // Parse search list
        let search = self.parse_until_delimiter(delimiter, false)?;

        // Parse replace list
        let replace = self.parse_until_delimiter(delimiter, false)?;

        // Parse flags
        let flags = self.parse_trans_flags();

        Some(ContextToken::Transliteration { search, replace, flags })
    }

    /// Try to parse m// match operator
    fn try_parse_match(&mut self) -> Option<ContextToken> {
        let start_pos = self.position;

        // Check for 'm' followed by delimiter
        if self.peek_str(1) != "m" {
            return None;
        }
        self.next_char(); // consume 'm'

        // Get the delimiter
        let delimiter = match self.peek()? {
            c if !c.is_alphanumeric() && !c.is_whitespace() => c,
            _ => {
                self.position = start_pos;
                return None;
            }
        };
        self.next_char(); // consume delimiter

        // Parse pattern
        let pattern = self.parse_until_delimiter(delimiter, true)?;

        // Parse flags
        let flags = self.parse_regex_flags();

        Some(ContextToken::Match { pattern, flags })
    }

    /// Try to parse quote-like operators: q, qq, qw, qr, qx
    ///
    /// Each operator is followed immediately by an arbitrary delimiter character.
    /// For paired delimiters (`(`, `[`, `{`, `<`) the matching close delimiter
    /// is used; otherwise the same character closes the construct.
    /// Only `qr` accepts modifier flags after the closing delimiter.
    fn try_parse_quote_operator(&mut self) -> Option<ContextToken> {
        let start_pos = self.position;

        // We already know peek() == 'q'; consume it.
        self.next_char();

        // Determine the operator keyword by examining the next character(s).
        let operator: &str = match self.peek()? {
            'r' => {
                self.next_char();
                "qr"
            }
            'w' => {
                self.next_char();
                "qw"
            }
            'x' => {
                self.next_char();
                "qx"
            }
            'q' => {
                self.next_char();
                "qq"
            }
            c if !c.is_alphanumeric() && !c.is_whitespace() => {
                // bare `q` followed directly by delimiter
                "q"
            }
            _ => {
                // Not a recognised quote operator — back up
                self.position = start_pos;
                return None;
            }
        };

        // The next character must be the opening delimiter (non-alnum, non-space).
        let open_delim = match self.peek()? {
            c if !c.is_alphanumeric() && !c.is_whitespace() => c,
            _ => {
                self.position = start_pos;
                return None;
            }
        };
        self.next_char(); // consume opening delimiter

        // For paired delimiters the close is the matching bracket; otherwise
        // the same character closes the construct.
        let close_delim = paired_close(open_delim).unwrap_or(open_delim);
        let is_paired = paired_close(open_delim).is_some();

        // Parse the body, honouring nesting for paired delimiters.
        let content = if is_paired {
            self.parse_paired(open_delim, close_delim)?
        } else {
            // For symmetric delimiters backslash escapes are honoured so that
            // e.g. q/foo\/bar/ works.
            self.parse_until_delimiter(close_delim, true)?
        };

        // Only `qr` takes regex modifier flags.
        let flags = if operator == "qr" { self.parse_regex_flags() } else { String::new() };

        Some(ContextToken::QuoteOp { operator: operator.to_string(), content, flags })
    }

    /// Parse balanced paired delimiters, supporting nesting and backslash escapes.
    ///
    /// Assumes the opening delimiter has already been consumed.  Returns the
    /// inner content (without the outer delimiters) or `None` if the input
    /// ends before the construct is closed.
    fn parse_paired(&mut self, open: char, close: char) -> Option<String> {
        let mut content = String::new();
        let mut depth: usize = 1;
        let mut escaped = false;

        while let Some(ch) = self.peek() {
            if escaped {
                content.push(ch);
                self.next_char();
                escaped = false;
                continue;
            }

            if ch == '\\' {
                escaped = true;
                content.push(ch);
                self.next_char();
                continue;
            }

            if ch == open {
                depth += 1;
                content.push(ch);
                self.next_char();
                continue;
            }

            if ch == close {
                depth -= 1;
                if depth == 0 {
                    self.next_char(); // consume the final close delimiter
                    return Some(content);
                }
                content.push(ch);
                self.next_char();
                continue;
            }

            content.push(ch);
            self.next_char();
        }

        None // unterminated
    }

    /// Parse content until the delimiter is found
    fn parse_until_delimiter(&mut self, delimiter: char, allow_escape: bool) -> Option<String> {
        let mut content = String::new();
        let mut escaped = false;

        while let Some(ch) = self.peek() {
            if !escaped && ch == delimiter {
                self.next_char(); // consume delimiter
                return Some(content);
            }

            escaped = allow_escape && ch == '\\' && !escaped;

            content.push(ch);
            self.next_char();
        }

        None // Unterminated
    }

    /// Parse regex flags (i, m, s, x, etc.)
    fn parse_regex_flags(&mut self) -> String {
        let mut flags = String::new();
        while let Some(ch) = self.peek() {
            if matches!(
                ch,
                'i' | 'm' | 's' | 'x' | 'g' | 'o' | 'a' | 'u' | 'l' | 'n' | 'p' | 'c' | 'e' | 'r'
            ) {
                flags.push(ch);
                self.next_char();
            } else {
                break;
            }
        }
        flags
    }

    /// Parse transliteration flags
    fn parse_trans_flags(&mut self) -> String {
        let mut flags = String::new();
        while let Some(ch) = self.peek() {
            if ch.is_alphabetic() {
                flags.push(ch);
                self.next_char();
            } else {
                break;
            }
        }
        flags
    }
}

/// Get the paired closing delimiter for an opening delimiter
fn paired_close(open: char) -> Option<char> {
    match open {
        '(' => Some(')'),
        '[' => Some(']'),
        '{' => Some('}'),
        '<' => Some('>'),
        _ => None,
    }
}

/// Preprocessor for handling context-sensitive constructs
pub struct ContextSensitivePreprocessor;

impl ContextSensitivePreprocessor {
    /// Preprocess input to handle context-sensitive operators
    pub fn preprocess(input: &str) -> String {
        // This would transform context-sensitive operators into a form
        // that the Pest parser can handle
        // For now, return input unchanged
        input.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_substitution_parsing() {
        let mut lexer = ContextSensitiveLexer::new("s/foo/bar/gi".to_string());
        match lexer.try_parse_operator() {
            Some(ContextToken::Substitution { pattern, replacement, flags }) => {
                assert_eq!(pattern, "foo");
                assert_eq!(replacement, "bar");
                assert_eq!(flags, "gi");
            }
            _ => unreachable!("Failed to parse substitution"),
        }
    }

    #[test]
    fn test_match_parsing() {
        let mut lexer = ContextSensitiveLexer::new("m/pattern/i".to_string());
        match lexer.try_parse_operator() {
            Some(ContextToken::Match { pattern, flags }) => {
                assert_eq!(pattern, "pattern");
                assert_eq!(flags, "i");
            }
            _ => unreachable!("Failed to parse match"),
        }
    }

    #[test]
    fn test_transliteration_parsing() {
        let mut lexer = ContextSensitiveLexer::new("tr/abc/xyz/".to_string());
        match lexer.try_parse_operator() {
            Some(ContextToken::Transliteration { search, replace, flags }) => {
                assert_eq!(search, "abc");
                assert_eq!(replace, "xyz");
                assert_eq!(flags, "");
            }
            _ => unreachable!("Failed to parse transliteration"),
        }
    }

    // ------------------------------------------------------------------
    // Quote-like operator tests (q, qq, qw, qr, qx)
    // ------------------------------------------------------------------

    #[test]
    fn test_qw_slash_delimiter() {
        let mut lexer = ContextSensitiveLexer::new("qw/foo bar baz/".to_string());
        match lexer.try_parse_operator() {
            Some(ContextToken::QuoteOp { operator, content, flags }) => {
                assert_eq!(operator, "qw");
                assert_eq!(content, "foo bar baz");
                assert_eq!(flags, "");
            }
            _ => unreachable!("Failed to parse qw/.../ operator"),
        }
    }

    #[test]
    fn test_qw_parens_delimiter() {
        let mut lexer = ContextSensitiveLexer::new("qw(one two three)".to_string());
        match lexer.try_parse_operator() {
            Some(ContextToken::QuoteOp { operator, content, flags }) => {
                assert_eq!(operator, "qw");
                assert_eq!(content, "one two three");
                assert_eq!(flags, "");
            }
            _ => unreachable!("Failed to parse qw(...) operator"),
        }
    }

    #[test]
    fn test_qr_with_flags() {
        let mut lexer = ContextSensitiveLexer::new("qr/\\d+/imx".to_string());
        match lexer.try_parse_operator() {
            Some(ContextToken::QuoteOp { operator, content, flags }) => {
                assert_eq!(operator, "qr");
                assert_eq!(content, "\\d+");
                assert_eq!(flags, "imx");
            }
            _ => unreachable!("Failed to parse qr/.../imx operator"),
        }
    }

    #[test]
    fn test_qr_no_flags() {
        let mut lexer = ContextSensitiveLexer::new("qr/pattern/".to_string());
        match lexer.try_parse_operator() {
            Some(ContextToken::QuoteOp { operator, content, flags }) => {
                assert_eq!(operator, "qr");
                assert_eq!(content, "pattern");
                assert_eq!(flags, "");
            }
            _ => unreachable!("Failed to parse qr/.../ operator with no flags"),
        }
    }

    #[test]
    fn test_qx_command() {
        let mut lexer = ContextSensitiveLexer::new("qx/ls -la/".to_string());
        match lexer.try_parse_operator() {
            Some(ContextToken::QuoteOp { operator, content, flags }) => {
                assert_eq!(operator, "qx");
                assert_eq!(content, "ls -la");
                assert_eq!(flags, "");
            }
            _ => unreachable!("Failed to parse qx/.../ operator"),
        }
    }

    #[test]
    fn test_q_single_quote() {
        let mut lexer = ContextSensitiveLexer::new("q/hello world/".to_string());
        match lexer.try_parse_operator() {
            Some(ContextToken::QuoteOp { operator, content, flags }) => {
                assert_eq!(operator, "q");
                assert_eq!(content, "hello world");
                assert_eq!(flags, "");
            }
            _ => unreachable!("Failed to parse q/.../ operator"),
        }
    }

    #[test]
    fn test_qq_double_quote() {
        let mut lexer = ContextSensitiveLexer::new("qq/hello $name/".to_string());
        match lexer.try_parse_operator() {
            Some(ContextToken::QuoteOp { operator, content, flags }) => {
                assert_eq!(operator, "qq");
                assert_eq!(content, "hello $name");
                assert_eq!(flags, "");
            }
            _ => unreachable!("Failed to parse qq/.../ operator"),
        }
    }

    #[test]
    fn test_qw_curly_delimiter() {
        let mut lexer = ContextSensitiveLexer::new("qw{alpha beta gamma}".to_string());
        match lexer.try_parse_operator() {
            Some(ContextToken::QuoteOp { operator, content, flags }) => {
                assert_eq!(operator, "qw");
                assert_eq!(content, "alpha beta gamma");
                assert_eq!(flags, "");
            }
            _ => unreachable!("Failed to parse qw{{...}} operator"),
        }
    }

    #[test]
    fn test_qw_angle_delimiter() {
        let mut lexer = ContextSensitiveLexer::new("qw<a b c>".to_string());
        match lexer.try_parse_operator() {
            Some(ContextToken::QuoteOp { operator, content, flags }) => {
                assert_eq!(operator, "qw");
                assert_eq!(content, "a b c");
                assert_eq!(flags, "");
            }
            _ => unreachable!("Failed to parse qw<...> operator"),
        }
    }

    #[test]
    fn test_qw_bracket_delimiter() {
        let mut lexer = ContextSensitiveLexer::new("qw[x y z]".to_string());
        match lexer.try_parse_operator() {
            Some(ContextToken::QuoteOp { operator, content, flags }) => {
                assert_eq!(operator, "qw");
                assert_eq!(content, "x y z");
                assert_eq!(flags, "");
            }
            _ => unreachable!("Failed to parse qw[...] operator"),
        }
    }

    #[test]
    fn test_qr_paired_parens_with_flags() {
        let mut lexer = ContextSensitiveLexer::new("qr(\\w+)ix".to_string());
        match lexer.try_parse_operator() {
            Some(ContextToken::QuoteOp { operator, content, flags }) => {
                assert_eq!(operator, "qr");
                assert_eq!(content, "\\w+");
                assert_eq!(flags, "ix");
            }
            _ => unreachable!("Failed to parse qr(...)ix operator"),
        }
    }

    #[test]
    fn test_q_escape_in_symmetric_delimiter() {
        // q/it\'s/ — backslash-escaped delimiter inside symmetric q
        let mut lexer = ContextSensitiveLexer::new("q/it\\'s a test/".to_string());
        match lexer.try_parse_operator() {
            Some(ContextToken::QuoteOp { operator, content, flags }) => {
                assert_eq!(operator, "q");
                assert_eq!(content, "it\\'s a test");
                assert_eq!(flags, "");
            }
            _ => unreachable!("Failed to parse q/.../ with escaped delimiter"),
        }
    }

    #[test]
    fn test_qw_nested_parens() {
        // qw( ) with nested parens inside — nesting is unusual in qw but
        // the paired-delimiter rule must not lose track.
        let mut lexer = ContextSensitiveLexer::new("qw(a (b) c)".to_string());
        match lexer.try_parse_operator() {
            Some(ContextToken::QuoteOp { operator, content, flags }) => {
                assert_eq!(operator, "qw");
                assert_eq!(content, "a (b) c");
                assert_eq!(flags, "");
            }
            _ => unreachable!("Failed to parse qw(...) with nested parens"),
        }
    }

    #[test]
    fn test_qw_does_not_accept_flags() {
        // qw takes no flags; any trailing letters after the close belong
        // to the surrounding code, so the token's flags must be empty.
        let mut lexer = ContextSensitiveLexer::new("qw/a b/".to_string());
        match lexer.try_parse_operator() {
            Some(ContextToken::QuoteOp { flags, .. }) => {
                assert_eq!(flags, "", "qw must not consume trailing flags");
            }
            _ => unreachable!("Failed to parse qw/.../"),
        }
    }

    #[test]
    fn test_unterminated_qw_returns_none() {
        let mut lexer = ContextSensitiveLexer::new("qw/unterminated".to_string());
        assert!(lexer.try_parse_operator().is_none(), "Unterminated qw should return None");
    }

    #[test]
    fn test_unterminated_qr_parens_returns_none() {
        let mut lexer = ContextSensitiveLexer::new("qr(unclosed".to_string());
        assert!(lexer.try_parse_operator().is_none(), "Unterminated qr(...) should return None");
    }
}
