//! Regex and quote-like operator parsing
//!
//! Handles m//, s///, qr//, tr///, and other quote-like operators

use crate::simple_token::Token;

/// Quote-like operators in Perl
#[derive(Debug, Clone, PartialEq)]
pub enum QuoteOperator {
    Match,         // m//
    Substitute,    // s///
    QuoteRegex,    // qr//
    Transliterate, // tr/// or y///
    Quote,         // q//
    QuoteDouble,   // qq//
    QuoteWords,    // qw//
    QuoteExec,     // qx//
}

/// Result of parsing a quote-like construct
#[derive(Debug, Clone, PartialEq)]
pub struct QuoteConstruct {
    pub operator: QuoteOperator,
    pub delimiter: char,
    pub pattern: String,
    pub replacement: Option<String>, // For s/// and tr///
    pub modifiers: String,
}

/// Parser for regex and quote-like constructs
pub struct RegexParser<'source> {
    input: &'source str,
    position: usize,
}

impl<'source> RegexParser<'source> {
    pub fn new(input: &'source str, start_position: usize) -> Self {
        Self { input, position: start_position }
    }

    /// Parse a bare regex starting with /
    pub fn parse_bare_regex(&mut self) -> Result<QuoteConstruct, String> {
        if !self.current_char_is('/') {
            return Err("Expected / to start regex".to_string());
        }

        self.advance(); // Skip initial /
        let pattern = self.parse_until_delimiter('/')?;
        let modifiers = self.parse_modifiers();

        Ok(QuoteConstruct {
            operator: QuoteOperator::Match,
            delimiter: '/',
            pattern,
            replacement: None,
            modifiers,
        })
    }

    /// Parse a quote-like operator (m//, s///, etc.)
    pub fn parse_quote_operator(&mut self, _op: Token) -> Result<QuoteConstruct, String> {
        // This function currently always returns an error - stub implementation
        match _op {
            Token::BinMatch => {
                // This is =~, not a quote operator
                Err("BinMatch is not a quote operator".to_string())
            }
            _ => Err(format!("Unexpected token for quote operator: {:?}", _op)),
        }
    }

    /// Parse m// operator
    pub fn parse_match_operator(&mut self) -> Result<QuoteConstruct, String> {
        // Skip optional whitespace after 'm'
        self.skip_whitespace();

        let delimiter = if self.current_char_is('/') {
            self.advance(); // consume '/'
            '/'
        } else {
            // parse_delimiter() consumes the opening delimiter
            self.parse_delimiter()?
        };

        let pattern = self.parse_until_delimiter(delimiter)?;
        let modifiers = self.parse_modifiers();

        Ok(QuoteConstruct {
            operator: QuoteOperator::Match,
            delimiter,
            pattern,
            replacement: None,
            modifiers,
        })
    }

    /// Parse s/// operator
    pub fn parse_substitute_operator(&mut self) -> Result<QuoteConstruct, String> {
        // Skip optional whitespace after 's'
        self.skip_whitespace();

        let delimiter = if self.current_char_is('/') {
            self.advance(); // consume '/'
            '/'
        } else {
            // parse_delimiter() consumes the opening delimiter
            self.parse_delimiter()?
        };

        let pattern = self.parse_until_delimiter(delimiter)?;
        let replacement = self.parse_until_delimiter(delimiter)?;
        let modifiers = self.parse_modifiers();

        Ok(QuoteConstruct {
            operator: QuoteOperator::Substitute,
            delimiter,
            pattern,
            replacement: Some(replacement),
            modifiers,
        })
    }

    /// Parse content until the given delimiter, handling escapes
    fn parse_until_delimiter(&mut self, delimiter: char) -> Result<String, String> {
        let mut result = String::new();
        let mut escaped = false;

        while self.position < self.input.len() {
            let ch = self.current_char();

            if escaped {
                result.push(ch);
                escaped = false;
            } else if ch == '\\' {
                result.push(ch);
                escaped = true;
            } else if ch == delimiter {
                self.advance(); // Consume delimiter
                return Ok(result);
            } else {
                result.push(ch);
            }

            self.advance();
        }

        Err(format!("Unterminated pattern, expected {}", delimiter))
    }

    /// Parse regex modifiers
    fn parse_modifiers(&mut self) -> String {
        let mut modifiers = String::new();

        while self.position < self.input.len() {
            let ch = self.current_char();
            if matches!(
                ch,
                'i' | 'm'
                    | 's'
                    | 'x'
                    | 'g'
                    | 'o'
                    | 'c'
                    | 'e'
                    | 'r'
                    | 'a'
                    | 'd'
                    | 'l'
                    | 'u'
                    | 'p'
                    | 'n'
            ) {
                modifiers.push(ch);
                self.advance();
            } else {
                break;
            }
        }

        modifiers
    }

    /// Parse delimiter for quote-like operators
    fn parse_delimiter(&mut self) -> Result<char, String> {
        if self.position >= self.input.len() {
            return Err("Expected delimiter".to_string());
        }

        let ch = self.current_char();

        // Check for paired delimiters
        let delimiter = match ch {
            '(' => ')',
            '[' => ']',
            '{' => '}',
            '<' => '>',
            _ => ch, // Use same character for both open and close
        };

        self.advance();
        Ok(delimiter)
    }

    /// Skip whitespace
    fn skip_whitespace(&mut self) {
        while self.position < self.input.len() && self.current_char().is_whitespace() {
            self.advance();
        }
    }

    /// Get current character
    fn current_char(&self) -> char {
        self.input.chars().nth(self.position).unwrap_or('\0')
    }

    /// Check if current character matches
    fn current_char_is(&self, ch: char) -> bool {
        self.current_char() == ch
    }

    /// Advance position by one character
    fn advance(&mut self) {
        if self.position < self.input.len() {
            self.position += self.current_char().len_utf8();
        }
    }

    /// Get current position
    pub fn position(&self) -> usize {
        self.position
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_bare_regex() {
        use perl_tdd_support::must;
        let input = "/test/i";
        let mut parser = RegexParser::new(input, 0);

        let result = must(parser.parse_bare_regex());
        assert_eq!(result.pattern, "test");
        assert_eq!(result.modifiers, "i");
        assert_eq!(result.delimiter, '/');
    }

    #[test]
    fn test_parse_regex_with_escapes() {
        use perl_tdd_support::must;
        let input = r"/test\/path/";
        let mut parser = RegexParser::new(input, 0);

        let result = must(parser.parse_bare_regex());
        assert_eq!(result.pattern, r"test\/path");
    }

    #[test]
    fn test_parse_match_operator() {
        use perl_tdd_support::must;
        let input = "m/pattern/gi";
        let mut parser = RegexParser::new(input, 1); // Start after 'm'

        let result = must(parser.parse_match_operator());
        assert_eq!(result.pattern, "pattern");
        assert_eq!(result.modifiers, "gi");
    }

    #[test]
    fn test_parse_substitute_operator() {
        use perl_tdd_support::must;
        let input = "s/old/new/g";
        let mut parser = RegexParser::new(input, 1); // Start after 's'

        let result = must(parser.parse_substitute_operator());
        assert_eq!(result.pattern, "old");
        assert_eq!(result.replacement, Some("new".to_string()));
        assert_eq!(result.modifiers, "g");
    }

    #[test]
    fn test_parse_with_alternate_delimiters() {
        use perl_tdd_support::must;
        let input = "m{test}i";
        let mut parser = RegexParser::new(input, 1); // Start after 'm'

        let result = must(parser.parse_match_operator());
        assert_eq!(result.pattern, "test");
        assert_eq!(result.delimiter, '}');
        assert_eq!(result.modifiers, "i");
    }
}
