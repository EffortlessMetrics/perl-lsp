//! Error types for the Perl lexer

use thiserror::Error;

/// Result type for lexer operations
pub type Result<T> = std::result::Result<T, LexerError>;

/// Errors that can occur during lexing
#[derive(Debug, Clone, Error)]
pub enum LexerError {
    /// Unterminated string literal
    #[error("Unterminated string literal starting at position {position}")]
    UnterminatedString {
        /// Byte offset where the string literal started
        position: usize,
    },

    /// Unterminated regex
    #[error("Unterminated regex starting at position {position}")]
    UnterminatedRegex {
        /// Byte offset where the regex started
        position: usize,
    },

    /// Invalid escape sequence
    #[error("Invalid escape sequence '\\{char}' at position {position}")]
    InvalidEscape {
        /// The character following the backslash
        char: char,
        /// Byte offset of the backslash
        position: usize,
    },

    /// Invalid numeric literal
    #[error("Invalid numeric literal at position {position}: {reason}")]
    InvalidNumber {
        /// Byte offset of the number token
        position: usize,
        /// Human-readable description of why the number is invalid
        reason: String,
    },

    /// Unexpected character
    #[error("Unexpected character '{char}' at position {position}")]
    UnexpectedChar {
        /// The character that was not expected
        char: char,
        /// Byte offset of the unexpected character
        position: usize,
    },

    /// Invalid UTF-8
    #[error("Invalid UTF-8 at position {position}")]
    InvalidUtf8 {
        /// Byte offset of the invalid byte sequence
        position: usize,
    },

    /// Heredoc error
    #[error("Heredoc error at position {position}: {reason}")]
    HeredocError {
        /// Byte offset of the heredoc marker
        position: usize,
        /// Human-readable description of the heredoc problem
        reason: String,
    },

    /// Generic error
    #[error("{0}")]
    Other(String),
}

impl LexerError {
    /// Get the position where the error occurred
    pub fn position(&self) -> Option<usize> {
        match self {
            LexerError::UnterminatedString { position }
            | LexerError::UnterminatedRegex { position }
            | LexerError::InvalidEscape { position, .. }
            | LexerError::InvalidNumber { position, .. }
            | LexerError::UnexpectedChar { position, .. }
            | LexerError::InvalidUtf8 { position }
            | LexerError::HeredocError { position, .. } => Some(*position),
            LexerError::Other(_) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::LexerError;

    // --- variants with positions ---

    #[test]
    fn lexer_error_unterminated_string_returns_position() -> Result<(), Box<dyn std::error::Error>>
    {
        let err = LexerError::UnterminatedString { position: 7 };
        assert_eq!(err.position(), Some(7));
        Ok(())
    }

    #[test]
    fn lexer_error_unterminated_regex_returns_position() -> Result<(), Box<dyn std::error::Error>> {
        let err = LexerError::UnterminatedRegex { position: 42 };
        assert_eq!(err.position(), Some(42));
        Ok(())
    }

    #[test]
    fn lexer_error_invalid_escape_returns_position() -> Result<(), Box<dyn std::error::Error>> {
        let err = LexerError::InvalidEscape { char: 'z', position: 15 };
        assert_eq!(err.position(), Some(15));
        Ok(())
    }

    #[test]
    fn lexer_error_invalid_number_returns_position() -> Result<(), Box<dyn std::error::Error>> {
        let err = LexerError::InvalidNumber { position: 3, reason: "bad digit".into() };
        assert_eq!(err.position(), Some(3));
        Ok(())
    }

    #[test]
    fn lexer_error_unexpected_char_returns_position() -> Result<(), Box<dyn std::error::Error>> {
        let err = LexerError::UnexpectedChar { char: '@', position: 99 };
        assert_eq!(err.position(), Some(99));
        Ok(())
    }

    #[test]
    fn lexer_error_invalid_utf8_returns_position() -> Result<(), Box<dyn std::error::Error>> {
        let err = LexerError::InvalidUtf8 { position: 0 };
        assert_eq!(err.position(), Some(0));
        Ok(())
    }

    #[test]
    fn lexer_error_heredoc_error_returns_position() -> Result<(), Box<dyn std::error::Error>> {
        let err = LexerError::HeredocError { position: 256, reason: "no terminator".into() };
        assert_eq!(err.position(), Some(256));
        Ok(())
    }

    // --- variant without a position ---

    #[test]
    fn lexer_error_other_returns_none() -> Result<(), Box<dyn std::error::Error>> {
        let err = LexerError::Other("something went wrong".into());
        assert_eq!(err.position(), None);
        Ok(())
    }

    // --- edge values ---

    #[test]
    fn lexer_error_position_zero_is_returned_correctly() -> Result<(), Box<dyn std::error::Error>> {
        let err = LexerError::UnterminatedString { position: 0 };
        assert_eq!(err.position(), Some(0));
        Ok(())
    }

    #[test]
    fn lexer_error_position_usize_max_is_returned_correctly()
    -> Result<(), Box<dyn std::error::Error>> {
        let err = LexerError::UnterminatedString { position: usize::MAX };
        assert_eq!(err.position(), Some(usize::MAX));
        Ok(())
    }
}
