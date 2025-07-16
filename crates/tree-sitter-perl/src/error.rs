//! Error types for tree-sitter Perl parser

use thiserror::Error;
use serde::{Serialize, Deserialize};

/// Result type for parsing operations
pub type ParseResult<T> = Result<T, ParseError>;

/// Errors that can occur during parsing
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ParseError {
    /// Parsing failed for an unknown reason
    #[error("Parsing failed")]
    ParseFailed,

    /// Invalid UTF-8 in source code
    #[error("Invalid UTF-8: {0}")]
    InvalidUtf8(String),

    /// Scanner error occurred
    #[error("Scanner error: {0}")]
    ScannerError(String),

    /// Grammar error occurred
    #[error("Grammar error: {0}")]
    GrammarError(String),

    /// Language loading failed
    #[error("Failed to load language")]
    LanguageLoadFailed,

    /// Parser creation failed
    #[error("Failed to create parser")]
    ParserCreationFailed,
}

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

impl ParseError {
    /// Create an unterminated string error
    pub fn unterminated_string(position: (usize, usize)) -> Self {
        ParseError::ScannerError(format!("Unterminated string at line {}, column {}", position.0, position.1))
    }

    /// Create an invalid token error
    pub fn invalid_token(token: String, position: (usize, usize)) -> Self {
        ParseError::ScannerError(format!("Invalid token '{}' at line {}, column {}", token, position.0, position.1))
    }

    /// Create a Unicode error
    pub fn unicode_error(message: &str) -> Self {
        ParseError::ScannerError(format!("Unicode error: {}", message))
    }

    /// Create a simple scanner error
    pub fn scanner_error_simple(message: &str) -> Self {
        ParseError::ScannerError(message.to_string())
    }
} 