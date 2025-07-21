//! Error types for the Perl lexer

use thiserror::Error;

/// Result type for lexer operations
pub type Result<T> = std::result::Result<T, LexerError>;

/// Errors that can occur during lexing
#[derive(Debug, Clone, Error)]
pub enum LexerError {
    /// Unterminated string literal
    #[error("Unterminated string literal starting at position {position}")]
    UnterminatedString { position: usize },
    
    /// Unterminated regex
    #[error("Unterminated regex starting at position {position}")]
    UnterminatedRegex { position: usize },
    
    /// Invalid escape sequence
    #[error("Invalid escape sequence '\\{char}' at position {position}")]
    InvalidEscape { char: char, position: usize },
    
    /// Invalid numeric literal
    #[error("Invalid numeric literal at position {position}: {reason}")]
    InvalidNumber { position: usize, reason: String },
    
    /// Unexpected character
    #[error("Unexpected character '{char}' at position {position}")]
    UnexpectedChar { char: char, position: usize },
    
    /// Invalid UTF-8
    #[error("Invalid UTF-8 at position {position}")]
    InvalidUtf8 { position: usize },
    
    /// Heredoc error
    #[error("Heredoc error at position {position}: {reason}")]
    HeredocError { position: usize, reason: String },
    
    /// Generic error
    #[error("{0}")]
    Other(String),
}

impl LexerError {
    /// Get the position where the error occurred
    pub fn position(&self) -> Option<usize> {
        match self {
            LexerError::UnterminatedString { position } |
            LexerError::UnterminatedRegex { position } |
            LexerError::InvalidEscape { position, .. } |
            LexerError::InvalidNumber { position, .. } |
            LexerError::UnexpectedChar { position, .. } |
            LexerError::InvalidUtf8 { position } |
            LexerError::HeredocError { position, .. } => Some(*position),
            LexerError::Other(_) => None,
        }
    }
}