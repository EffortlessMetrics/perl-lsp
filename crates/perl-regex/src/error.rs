use thiserror::Error;

/// Error type for Perl regex validation failures.
#[derive(Error, Debug, Clone, PartialEq)]
pub enum RegexError {
    /// Syntax error at a specific byte offset in the regex pattern.
    #[error("{message} at offset {offset}")]
    Syntax { message: String, offset: usize },
}

impl RegexError {
    /// Create a new syntax error with a message and byte offset.
    pub fn syntax(message: impl Into<String>, offset: usize) -> Self {
        Self::Syntax { message: message.into(), offset }
    }
}
