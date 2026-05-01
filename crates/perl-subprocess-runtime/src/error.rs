use std::fmt;

/// Error type for subprocess execution failures
#[derive(Debug, Clone)]
pub struct SubprocessError {
    /// Human-readable error message
    pub message: String,
}

impl SubprocessError {
    /// Create a new subprocess error with the given message
    pub fn new(message: impl Into<String>) -> Self {
        Self { message: message.into() }
    }
}

impl fmt::Display for SubprocessError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for SubprocessError {}
