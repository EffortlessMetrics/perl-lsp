//! Error types for the Perl parser within the PSTX email processing pipeline
//!
//! This module defines comprehensive error handling for Perl parsing operations that occur
//! throughout the PSTX email processing workflow: Extract → Normalize → Thread → Render → Index.
//!
//! # Error Recovery Strategy
//!
//! When parsing errors occur during email processing:
//! 1. **Extract stage**: Parsing failures indicate corrupted or malformed PST data
//! 2. **Normalize stage**: Syntax errors suggest script inconsistencies requiring fallback processing
//! 3. **Thread stage**: Parse failures can break thread analysis - graceful degradation applies
//! 4. **Render stage**: Errors impact output generation but preserve original content
//! 5. **Index stage**: Parse failures affect search indexing but maintain basic metadata
//!
//! # Performance Context
//!
//! Error handling is optimized for 50GB PST processing scenarios with minimal memory overhead
//! and fast recovery paths to maintain enterprise-scale performance targets.

use thiserror::Error;

/// Result type for parser operations in the PSTX email processing pipeline
///
/// This type encapsulates success/failure outcomes throughout the Extract → Normalize →
/// Thread → Render → Index workflow, enabling consistent error propagation and recovery
/// strategies across all pipeline stages.
pub type ParseResult<T> = Result<T, ParseError>;

/// Comprehensive error types that can occur during Perl parsing within PSTX email processing workflows
///
/// These errors are designed to provide detailed context about parsing failures that occur during
/// email content analysis, script processing, and metadata extraction. Each error variant includes
/// location information to enable precise recovery strategies in large PST file processing scenarios.
///
/// # Error Recovery Patterns
///
/// - **Syntax Errors**: Attempt fallback parsing or skip problematic content sections
/// - **Lexer Errors**: Re-tokenize with relaxed rules or binary content detection
/// - **Recursion Limits**: Flatten deeply nested structures or process iteratively
/// - **String Handling**: Apply encoding detection and normalization workflows
///
/// # Enterprise Scale Considerations
///
/// Error handling is optimized for processing 50GB+ PST files with thousands of email scripts
/// and embedded Perl content, ensuring memory-efficient error propagation and logging.
#[derive(Error, Debug, Clone, PartialEq)]
pub enum ParseError {
    /// Parser encountered unexpected end of input during email content analysis
    ///
    /// This occurs when processing truncated email scripts or incomplete PST data during
    /// the Extract stage. Recovery strategy: attempt partial parsing and preserve available content.
    #[error("Unexpected end of input")]
    UnexpectedEof,

    /// Parser found an unexpected token during email processing workflow
    ///
    /// Common during Normalize stage when email scripts contain syntax variations or encoding issues.
    /// Recovery strategy: skip problematic tokens and attempt continued parsing with relaxed rules.
    #[error("Unexpected token: expected {expected}, found {found} at {location}")]
    UnexpectedToken {
        /// Token type that was expected during email script parsing
        expected: String,
        /// Actual token found in email script content
        found: String,
        /// Byte position where unexpected token was encountered
        location: usize,
    },

    /// General syntax error occurred during email content parsing
    ///
    /// This encompasses malformed Perl constructs found in email scripts during Thread stage analysis.
    /// Recovery strategy: isolate syntax error scope and continue processing surrounding content.
    #[error("Invalid syntax at position {location}: {message}")]
    SyntaxError {
        /// Descriptive error message explaining the syntax issue
        message: String,
        /// Byte position where syntax error occurred in email script
        location: usize,
    },

    /// Lexical analysis failure during email script tokenization
    ///
    /// Indicates character encoding issues or binary content mixed with text during Extract stage.
    /// Recovery strategy: apply encoding detection and re-attempt tokenization with binary fallbacks.
    #[error("Lexer error: {message}")]
    LexerError {
        /// Detailed lexer error message describing tokenization failure
        message: String,
    },

    /// Parser recursion depth exceeded during complex email script analysis
    ///
    /// Occurs with deeply nested structures in email content during Render stage processing.
    /// Recovery strategy: flatten recursive structures and process iteratively to maintain performance.
    #[error("Maximum recursion depth exceeded")]
    RecursionLimit,

    /// Invalid numeric literal found in email script content
    ///
    /// Common when processing malformed configuration values during Index stage analysis.
    /// Recovery strategy: substitute default values and log for manual review.
    #[error("Invalid number literal: {literal}")]
    InvalidNumber {
        /// The malformed numeric literal found in email script content
        literal: String,
    },

    /// Malformed string literal in email processing workflow
    ///
    /// Indicates quote mismatches or encoding issues in email script strings during parsing.
    /// Recovery strategy: attempt string repair and normalization before re-parsing.
    #[error("Invalid string literal")]
    InvalidString,

    /// Unclosed delimiter detected during email content parsing
    ///
    /// Commonly found in truncated or corrupted email script content during Extract stage.
    /// Recovery strategy: auto-close delimiters and continue parsing with synthetic boundaries.
    #[error("Unclosed delimiter: {delimiter}")]
    UnclosedDelimiter {
        /// The delimiter character that was left unclosed
        delimiter: char,
    },

    /// Invalid regular expression syntax in email processing workflow
    ///
    /// Occurs when parsing regex patterns in email filters during Thread stage analysis.
    /// Recovery strategy: fallback to literal string matching and preserve original pattern.
    #[error("Invalid regex: {message}")]
    InvalidRegex {
        /// Specific error message describing regex syntax issue
        message: String,
    },
}

impl ParseError {
    /// Create a new syntax error for email processing workflow failures
    ///
    /// # Arguments
    ///
    /// * `message` - Descriptive error message with context about the syntax issue
    /// * `location` - Character position within the email content where error occurred
    ///
    /// # Returns
    ///
    /// A [`ParseError::SyntaxError`] variant with embedded location context for recovery strategies
    ///
    /// # Examples
    ///
    /// ```rust
    /// use perl_parser::ParseError;
    ///
    /// let error = ParseError::syntax("Missing semicolon in email script", 42);
    /// assert!(matches!(error, ParseError::SyntaxError { .. }));
    /// ```
    pub fn syntax(message: impl Into<String>, location: usize) -> Self {
        ParseError::SyntaxError { message: message.into(), location }
    }

    /// Create a new unexpected token error during email script parsing
    ///
    /// # Arguments
    ///
    /// * `expected` - Token type that was expected by the parser
    /// * `found` - Actual token type that was encountered
    /// * `location` - Character position where the unexpected token was found
    ///
    /// # Returns
    ///
    /// A [`ParseError::UnexpectedToken`] variant with detailed token mismatch information
    ///
    /// # Examples
    ///
    /// ```rust
    /// use perl_parser::ParseError;
    ///
    /// let error = ParseError::unexpected("semicolon", "comma", 15);
    /// assert!(matches!(error, ParseError::UnexpectedToken { .. }));
    /// ```
    ///
    /// # Email Processing Context
    ///
    /// This is commonly used during the Normalize stage when email scripts contain
    /// syntax variations that require token-level recovery strategies.
    pub fn unexpected(
        expected: impl Into<String>,
        found: impl Into<String>,
        location: usize,
    ) -> Self {
        ParseError::UnexpectedToken { expected: expected.into(), found: found.into(), location }
    }
}
