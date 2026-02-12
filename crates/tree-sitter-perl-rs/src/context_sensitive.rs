//! Context-sensitive parsing for Perl operators
//!
//! This module handles operators like s///, tr///, and m// that require
//! context-sensitive parsing to distinguish from regular identifiers.

/// Context-sensitive token types
#[derive(Debug, Clone, PartialEq)]
pub enum ContextToken {
    Substitution { pattern: String, replacement: String, flags: String },
    Transliteration { search: String, replace: String, flags: String },
    Match { pattern: String, flags: String },
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

    /// Try to parse quote-like operators (qr, qw, etc.)
    fn try_parse_quote_operator(&mut self) -> Option<ContextToken> {
        // This is a placeholder - would need full implementation
        None
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
            _ => panic!("Failed to parse substitution"),
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
            _ => panic!("Failed to parse match"),
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
            _ => panic!("Failed to parse transliteration"),
        }
    }
}
