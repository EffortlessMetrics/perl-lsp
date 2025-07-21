//! Error types for the Perl parser

use thiserror::Error;

/// Result type for parser operations
pub type ParseResult<T> = Result<T, ParseError>;

/// Errors that can occur during parsing
#[derive(Error, Debug, Clone, PartialEq)]
pub enum ParseError {
    #[error("Unexpected end of input")]
    UnexpectedEof,
    
    #[error("Unexpected token: expected {expected}, found {found} at {location}")]
    UnexpectedToken {
        expected: String,
        found: String,
        location: usize,
    },
    
    #[error("Invalid syntax at position {location}: {message}")]
    SyntaxError {
        message: String,
        location: usize,
    },
    
    #[error("Lexer error: {message}")]
    LexerError {
        message: String,
    },
    
    #[error("Maximum recursion depth exceeded")]
    RecursionLimit,
    
    #[error("Invalid number literal: {literal}")]
    InvalidNumber {
        literal: String,
    },
    
    #[error("Invalid string literal")]
    InvalidString,
    
    #[error("Unclosed delimiter: {delimiter}")]
    UnclosedDelimiter {
        delimiter: char,
    },
    
    #[error("Invalid regex: {message}")]
    InvalidRegex {
        message: String,
    },
}

impl ParseError {
    /// Create a new syntax error
    pub fn syntax(message: impl Into<String>, location: usize) -> Self {
        ParseError::SyntaxError {
            message: message.into(),
            location,
        }
    }
    
    /// Create a new unexpected token error
    pub fn unexpected(expected: impl Into<String>, found: impl Into<String>, location: usize) -> Self {
        ParseError::UnexpectedToken {
            expected: expected.into(),
            found: found.into(),
            location,
        }
    }
}