//! Error types for tree-sitter Perl parser

use thiserror::Error;

/// Result type for parsing operations
pub type ParseResult<T> = Result<T, ParseError>;

/// Errors that can occur during parsing
#[derive(Error, Debug, Clone, PartialEq)]
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