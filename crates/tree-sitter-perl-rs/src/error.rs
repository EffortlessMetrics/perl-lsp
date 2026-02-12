//! Error types for tree-sitter Perl parser

use thiserror::Error;

/// Kinds of parse errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseErrorKind {
    UnexpectedToken,
    UnexpectedEndOfInput,
    InvalidSyntax,
    InvalidNumber,
    InvalidString,
    InvalidRegex,
    InvalidVariable,
    MissingToken(String),
    InvalidOperator,
    InvalidIdentifier,
}

/// Error types for tree-sitter Perl parser
#[derive(
    Error,
    Debug,
    Clone,
    PartialEq,
    Eq,
    serde::Serialize,
    serde::Deserialize,
    bincode::Encode,
    bincode::Decode,
)]
pub enum ParseError {
    /// Failed to parse the input
    #[error("Failed to parse input")]
    ParseFailed,

    /// Failed to serialize scanner state
    #[error("Failed to serialize scanner state")]
    SerializationFailed,

    /// Failed to deserialize scanner state
    #[error("Failed to deserialize scanner state")]
    DeserializationFailed,

    /// Invalid token encountered
    #[error("Invalid token: {0}")]
    InvalidToken(String),

    /// Unexpected end of input
    #[error("Unexpected end of input")]
    UnexpectedEof,

    /// Invalid Unicode sequence
    #[error("Invalid Unicode sequence")]
    InvalidUnicode,

    /// Invalid UTF-8 sequence encountered
    #[error("Invalid UTF-8: {0}")]
    InvalidUtf8(String),

    /// Scanner error occurred
    #[error("Scanner error: {0}")]
    ScannerError(String),

    /// Language loading failed
    #[error("Failed to load language")]
    LanguageLoadFailed,
}

/// Result type for parsing operations
pub type ParseResult<T> = Result<T, ParseError>;

/// Errors that can occur during scanning
#[derive(Error, Debug, Clone, PartialEq)]
pub enum ScannerError {
    /// Invalid character encountered
    #[error("Invalid character: {0}")]
    InvalidCharacter(char),

    /// Unterminated string
    #[error("Unterminated string")]
    UnterminatedString,

    /// Unterminated comment
    #[error("Unterminated comment")]
    UnterminatedComment,

    /// Invalid escape sequence
    #[error("Invalid escape sequence: {0}")]
    InvalidEscape(String),

    /// Invalid Unicode sequence
    #[error("Invalid Unicode sequence: {0}")]
    InvalidUnicode(String),

    /// Scanner state error
    #[error("Scanner state error: {0}")]
    StateError(String),
}

/// Errors that can occur during Unicode processing
#[derive(Error, Debug, Clone, PartialEq)]
pub enum UnicodeError {
    /// Invalid Unicode code point
    #[error("Invalid Unicode code point: {0}")]
    InvalidCodePoint(u32),

    /// Invalid UTF-8 sequence
    #[error("Invalid UTF-8 sequence")]
    InvalidUtf8,

    /// Unicode normalization failed
    #[error("Unicode normalization failed: {0}")]
    NormalizationFailed(String),
}

impl From<ScannerError> for ParseError {
    fn from(err: ScannerError) -> Self {
        ParseError::ScannerError(err.to_string())
    }
}

impl From<UnicodeError> for ParseError {
    fn from(err: UnicodeError) -> Self {
        ParseError::ScannerError(err.to_string())
    }
}

impl From<std::str::Utf8Error> for ParseError {
    fn from(err: std::str::Utf8Error) -> Self {
        ParseError::InvalidUtf8(err.to_string())
    }
}

impl From<std::string::FromUtf8Error> for ParseError {
    fn from(err: std::string::FromUtf8Error) -> Self {
        ParseError::InvalidUtf8(err.to_string())
    }
}

impl From<std::io::Error> for ParseError {
    fn from(err: std::io::Error) -> Self {
        ParseError::ScannerError(format!("I/O error: {}", err))
    }
}

impl ParseError {
    /// Create a new parse error
    pub fn new(kind: ParseErrorKind, position: usize, message: String) -> Self {
        let error_msg = match kind {
            ParseErrorKind::UnexpectedToken => {
                format!("Unexpected token at position {}: {}", position, message)
            }
            ParseErrorKind::UnexpectedEndOfInput => {
                format!("Unexpected end of input at position {}: {}", position, message)
            }
            ParseErrorKind::InvalidSyntax => {
                format!("Invalid syntax at position {}: {}", position, message)
            }
            ParseErrorKind::InvalidNumber => {
                format!("Invalid number at position {}: {}", position, message)
            }
            ParseErrorKind::InvalidString => {
                format!("Invalid string at position {}: {}", position, message)
            }
            ParseErrorKind::InvalidRegex => {
                format!("Invalid regex at position {}: {}", position, message)
            }
            ParseErrorKind::InvalidVariable => {
                format!("Invalid variable at position {}: {}", position, message)
            }
            ParseErrorKind::MissingToken(ref token) => {
                format!("Missing {} at position {}: {}", token, position, message)
            }
            ParseErrorKind::InvalidOperator => {
                format!("Invalid operator at position {}: {}", position, message)
            }
            ParseErrorKind::InvalidIdentifier => {
                format!("Invalid identifier at position {}: {}", position, message)
            }
        };
        ParseError::InvalidToken(error_msg)
    }

    /// Create an error for unterminated string literals
    pub fn unterminated_string(position: (usize, usize)) -> Self {
        ParseError::ScannerError(format!(
            "Unterminated string literal at line {}, column {}",
            position.0, position.1
        ))
    }

    /// Create an error for invalid tokens
    pub fn invalid_token(token: String, position: (usize, usize)) -> Self {
        ParseError::InvalidToken(format!(
            "Invalid token '{}' at line {}, column {}",
            token, position.0, position.1
        ))
    }

    /// Create an error for Unicode-related issues
    pub fn unicode_error(_message: &str) -> Self {
        ParseError::InvalidUnicode
    }

    /// Create a simple scanner error
    pub fn scanner_error_simple(message: &str) -> Self {
        ParseError::ScannerError(message.to_string())
    }
}
