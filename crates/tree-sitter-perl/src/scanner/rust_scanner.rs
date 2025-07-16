//! Rust-native scanner implementation for Perl

use super::{PerlScanner, ScannerConfig, ScannerState, TokenType};
use crate::error::{ParseError, ParseResult};
use crate::unicode::UnicodeUtils;

/// Rust-native Perl scanner implementation
pub struct RustScanner {
    _config: ScannerConfig,
    state: ScannerState,
    input: Vec<u8>,
    position: usize,
    lookahead: Option<char>,
}

impl RustScanner {
    /// Create a new Rust scanner with default configuration
    pub fn new() -> Self {
        Self::with_config(ScannerConfig::default())
    }

    /// Create a new Rust scanner with custom configuration
    pub fn with_config(config: ScannerConfig) -> Self {
        Self {
            _config: config,
            state: ScannerState::default(),
            input: Vec::new(),
            position: 0,
            lookahead: None,
        }
    }

    /// Set the input source for scanning
    pub fn set_input(&mut self, input: &[u8]) {
        self.input = input.to_vec();
        self.position = 0;
        self.state.reset();
        self.lookahead = self.next_char();
    }

    /// Get the next character from input
    fn next_char(&mut self) -> Option<char> {
        if self.position >= self.input.len() {
            return None;
        }

        let ch = char::from_u32(self.input[self.position] as u32)?;
        self.position += ch.len_utf8();
        Some(ch)
    }

    /// Peek at the next character without consuming it
    fn _peek_char(&self) -> Option<char> {
        if self.position >= self.input.len() {
            return None;
        }
        char::from_u32(self.input[self.position] as u32)
    }

    /// Advance the scanner by one character
    fn advance(&mut self) {
        if let Some(ch) = self.lookahead {
            self.state.advance(ch);
            self.lookahead = self.next_char();
        }
    }

    /// Skip whitespace characters
    fn skip_whitespace(&mut self) {
        while let Some(ch) = self.lookahead {
            if UnicodeUtils::is_unicode_whitespace(ch) {
                self.advance();
            } else {
                break;
            }
        }
    }

    /// Scan a comment
    fn scan_comment(&mut self) -> ParseResult<TokenType> {
        self.state.in_comment = true;

        // Skip the # character
        self.advance();

        // Consume until newline or EOF
        while let Some(ch) = self.lookahead {
            if ch == '\n' {
                break;
            }
            self.advance();
        }

        self.state.in_comment = false;
        Ok(TokenType::Comment)
    }

    /// Scan a string literal
    fn scan_string(&mut self, delimiter: char) -> ParseResult<TokenType> {
        self.state.in_string = true;
        self.state.string_delimiter = Some(delimiter);

        // Skip the opening delimiter
        self.advance();

        while let Some(ch) = self.lookahead {
            if ch == delimiter {
                self.advance();
                self.state.in_string = false;
                self.state.string_delimiter = None;
                return Ok(if delimiter == '\'' {
                    TokenType::SingleQuotedString
                } else {
                    TokenType::DoubleQuotedString
                });
            } else if ch == '\\' {
                self.advance();
                // Skip the escaped character
                if let Some(_) = self.lookahead {
                    self.advance();
                }
            } else {
                self.advance();
            }
        }

        // Unterminated string
        self.state.in_string = false;
        self.state.string_delimiter = None;
        Err(ParseError::unterminated_string(self.state.position()))
    }

    /// Scan an identifier or keyword
    fn scan_identifier(&mut self) -> ParseResult<TokenType> {
        let mut identifier = String::new();

        // First character must be identifier start
        if let Some(ch) = self.lookahead {
            if UnicodeUtils::is_identifier_start(ch) {
                identifier.push(ch);
                self.advance();
            } else {
                return Err(ParseError::invalid_token(
                    ch.to_string(),
                    self.state.position(),
                ));
            }
        }

        // Continue with identifier continue characters
        while let Some(ch) = self.lookahead {
            if UnicodeUtils::is_identifier_continue(ch) {
                identifier.push(ch);
                self.advance();
            } else {
                break;
            }
        }

        // Check if it's a keyword
        match identifier.as_str() {
            "package" => Ok(TokenType::Package),
            "use" => Ok(TokenType::Use),
            "require" => Ok(TokenType::Require),
            "sub" => Ok(TokenType::Sub),
            "my" => Ok(TokenType::My),
            "our" => Ok(TokenType::Our),
            "local" => Ok(TokenType::Local),
            "return" => Ok(TokenType::Return),
            "if" => Ok(TokenType::If),
            "unless" => Ok(TokenType::Unless),
            "elsif" => Ok(TokenType::Elsif),
            "else" => Ok(TokenType::Else),
            "while" => Ok(TokenType::While),
            "until" => Ok(TokenType::Until),
            "for" => Ok(TokenType::For),
            "foreach" => Ok(TokenType::Foreach),
            "do" => Ok(TokenType::Do),
            "last" => Ok(TokenType::Last),
            "next" => Ok(TokenType::Next),
            "redo" => Ok(TokenType::Redo),
            "goto" => Ok(TokenType::Goto),
            "die" => Ok(TokenType::Die),
            "warn" => Ok(TokenType::Warn),
            "print" => Ok(TokenType::Print),
            "say" => Ok(TokenType::Say),
            "defined" => Ok(TokenType::Defined),
            "undef" => Ok(TokenType::Undef),
            _ => Ok(TokenType::Identifier),
        }
    }

    /// Scan a number literal
    fn scan_number(&mut self) -> ParseResult<TokenType> {
        let mut has_decimal = false;
        let mut has_exponent = false;

        // First digit
        if let Some(ch) = self.lookahead {
            if ch.is_ascii_digit() {
                self.advance();
            } else {
                return Err(ParseError::invalid_token(
                    ch.to_string(),
                    self.state.position(),
                ));
            }
        }

        // Continue scanning digits, decimal points, and exponents
        while let Some(ch) = self.lookahead {
            match ch {
                '0'..='9' => {
                    self.advance();
                }
                '.' if !has_decimal && !has_exponent => {
                    has_decimal = true;
                    self.advance();
                }
                'e' | 'E' if !has_exponent => {
                    has_exponent = true;
                    self.advance();

                    // Optional sign after exponent
                    if let Some(sign) = self.lookahead {
                        if sign == '+' || sign == '-' {
                            self.advance();
                        }
                    }
                }
                _ => break,
            }
        }

        Ok(if has_decimal || has_exponent {
            TokenType::Float
        } else {
            TokenType::Integer
        })
    }
}

impl PerlScanner for RustScanner {
    fn scan(&mut self, _input: &[u8]) -> ParseResult<Option<u16>> {
        // Skip whitespace
        self.skip_whitespace();

        // Check for EOF
        if self.lookahead.is_none() {
            return Ok(None);
        }

        // Scan next token
        let token_type = match self.lookahead {
            Some('#') => self.scan_comment()?,
            Some('\'') | Some('"') => self.scan_string(self.lookahead.unwrap())?,
            Some(ch) if UnicodeUtils::is_identifier_start(ch) => self.scan_identifier()?,
            Some(ch) if ch.is_ascii_digit() => self.scan_number()?,
            Some('+') => {
                self.advance();
                if let Some('=') = self.lookahead {
                    self.advance();
                    TokenType::PlusAssign
                } else if let Some('+') = self.lookahead {
                    self.advance();
                    TokenType::Increment
                } else {
                    TokenType::Plus
                }
            }
            Some('-') => {
                self.advance();
                if let Some('=') = self.lookahead {
                    self.advance();
                    TokenType::MinusAssign
                } else if let Some('-') = self.lookahead {
                    self.advance();
                    TokenType::Decrement
                } else {
                    TokenType::Minus
                }
            }
            Some('*') => {
                self.advance();
                if let Some('*') = self.lookahead {
                    self.advance();
                    if let Some('=') = self.lookahead {
                        self.advance();
                        TokenType::PowerAssign
                    } else {
                        TokenType::Power
                    }
                } else if let Some('=') = self.lookahead {
                    self.advance();
                    TokenType::MultiplyAssign
                } else {
                    TokenType::Multiply
                }
            }
            Some('/') => {
                self.advance();
                if let Some('=') = self.lookahead {
                    self.advance();
                    TokenType::DivideAssign
                } else {
                    TokenType::Divide
                }
            }
            Some('%') => {
                self.advance();
                if let Some('=') = self.lookahead {
                    self.advance();
                    TokenType::ModuloAssign
                } else {
                    TokenType::Modulo
                }
            }
            Some('=') => {
                self.advance();
                if let Some('=') = self.lookahead {
                    self.advance();
                    TokenType::Equal
                } else {
                    TokenType::Assign
                }
            }
            Some('!') => {
                self.advance();
                if let Some('=') = self.lookahead {
                    self.advance();
                    TokenType::NotEqual
                } else {
                    TokenType::LogicalNot
                }
            }
            Some('<') => {
                self.advance();
                if let Some('=') = self.lookahead {
                    self.advance();
                    TokenType::LessEqual
                } else if let Some('<') = self.lookahead {
                    self.advance();
                    TokenType::LeftShift
                } else {
                    TokenType::LessThan
                }
            }
            Some('>') => {
                self.advance();
                if let Some('=') = self.lookahead {
                    self.advance();
                    TokenType::GreaterEqual
                } else if let Some('>') = self.lookahead {
                    self.advance();
                    TokenType::RightShift
                } else {
                    TokenType::GreaterThan
                }
            }
            Some('&') => {
                self.advance();
                if let Some('&') = self.lookahead {
                    self.advance();
                    TokenType::LogicalAnd
                } else {
                    TokenType::BitwiseAnd
                }
            }
            Some('|') => {
                self.advance();
                if let Some('|') = self.lookahead {
                    self.advance();
                    TokenType::LogicalOr
                } else {
                    TokenType::BitwiseOr
                }
            }
            Some('^') => TokenType::BitwiseXor,
            Some('~') => TokenType::BitwiseNot,
            Some('.') => {
                self.advance();
                if let Some('.') = self.lookahead {
                    self.advance();
                    if let Some('.') = self.lookahead {
                        self.advance();
                        TokenType::RangeExclusive
                    } else {
                        TokenType::Range
                    }
                } else {
                    TokenType::Dot
                }
            }
            Some(',') => TokenType::Comma,
            Some(';') => TokenType::Semicolon,
            Some(':') => TokenType::Colon,
            Some('?') => TokenType::Question,
            Some('(') => TokenType::LeftParenthesis,
            Some(')') => TokenType::RightParenthesis,
            Some('[') => TokenType::LeftBracket,
            Some(']') => TokenType::RightBracket,
            Some('{') => TokenType::LeftBrace,
            Some('}') => TokenType::RightBrace,
            Some(ch) => {
                self.advance();
                return Err(ParseError::invalid_token(
                    ch.to_string(),
                    self.state.position(),
                ));
            }
            None => return Ok(None),
        };

        // Convert token type to u16 (placeholder - would need mapping)
        Ok(Some(token_type as u16))
    }

    fn serialize(&self, buffer: &mut Vec<u8>) -> ParseResult<()> {
        // Serialize scanner state
        buffer.extend_from_slice(&self.state.line.to_le_bytes());
        buffer.extend_from_slice(&self.state.column.to_le_bytes());
        buffer.extend_from_slice(&self.state.offset.to_le_bytes());
        buffer.extend_from_slice(&self.position.to_le_bytes());
        Ok(())
    }

    fn deserialize(&mut self, buffer: &[u8]) -> ParseResult<()> {
        if buffer.len() < 32 {
            return Err(ParseError::scanner_error_simple("Invalid buffer size"));
        }

        let mut offset = 0;

        // Deserialize state
        self.state.line = u32::from_le_bytes([
            buffer[offset],
            buffer[offset + 1],
            buffer[offset + 2],
            buffer[offset + 3],
        ]) as usize;
        offset += 4;

        self.state.column = u32::from_le_bytes([
            buffer[offset],
            buffer[offset + 1],
            buffer[offset + 2],
            buffer[offset + 3],
        ]) as usize;
        offset += 4;

        self.state.offset = u32::from_le_bytes([
            buffer[offset],
            buffer[offset + 1],
            buffer[offset + 2],
            buffer[offset + 3],
        ]) as usize;
        offset += 4;

        self.position = u32::from_le_bytes([
            buffer[offset],
            buffer[offset + 1],
            buffer[offset + 2],
            buffer[offset + 3],
        ]) as usize;

        Ok(())
    }

    fn is_eof(&self) -> bool {
        self.lookahead.is_none()
    }

    fn position(&self) -> (usize, usize) {
        self.state.position()
    }
}

impl Default for RustScanner {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scanner_creation() {
        let scanner = RustScanner::new();
        assert_eq!(scanner.state.line, 1);
        assert_eq!(scanner.state.column, 1);
    }

    #[test]
    fn test_identifier_scanning() {
        let mut scanner = RustScanner::new();
        scanner.set_input(b"my");

        // This is a simplified test - actual implementation would need
        // proper token type mapping and lexer integration
        assert!(!scanner.is_eof());
    }
}
