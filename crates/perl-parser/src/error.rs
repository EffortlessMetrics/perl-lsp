//! Error types for the Perl parser within the Perl parsing workflow pipeline
//!
//! This module defines comprehensive error handling for Perl parsing operations that occur
//! throughout the Perl parsing workflow workflow: Parse → Index → Navigate → Complete → Analyze.
//!
//! # Error Recovery Strategy
//!
//! When parsing errors occur during Perl parsing:
//! 1. **Parse stage**: Parsing failures indicate corrupted or malformed Perl source
//! 2. **Analyze stage**: Syntax errors suggest script inconsistencies requiring fallback processing
//! 3. **Navigate stage**: Parse failures can break thread analysis - graceful degradation applies
//! 4. **Complete stage**: Errors impact output generation but preserve original content
//! 5. **Analyze stage**: Parse failures affect search indexing but maintain basic metadata
//!
//! # Performance Context
//!
//! Error handling is optimized for large Perl codebase processing scenarios with minimal memory overhead
//! and fast recovery paths to maintain enterprise-scale performance targets.
//!
//! # Usage Examples
//!
//! ## Basic Error Handling
//!
//! ```
//! use perl_parser::{Parser, ParseError, ParseResult};
//!
//! fn parse_with_error_handling(code: &str) -> ParseResult<()> {
//!     let mut parser = Parser::new(code);
//!     match parser.parse() {
//!         Ok(ast) => {
//!             println!("Parsing successful");
//!             Ok(())
//!         }
//!         Err(ParseError::UnexpectedEof) => {
//!             eprintln!("Incomplete code: unexpected end of input");
//!             Err(ParseError::UnexpectedEof)
//!         }
//!         Err(ParseError::UnexpectedToken { found, expected, location }) => {
//!             eprintln!("Syntax error at position {}: found '{}', expected '{}'",
//!                      location, found, expected);
//!             Err(ParseError::UnexpectedToken { found, expected, location })
//!         }
//!         Err(e) => {
//!             eprintln!("Parse error: {}", e);
//!             Err(e)
//!         }
//!     }
//! }
//! ```
//!
//! ## Error Recovery in LSP Context
//!
//! ```no_run
//! use perl_parser::{Parser, ParseError, error_recovery::ErrorRecovery};
//!
//! fn parse_with_recovery(code: &str) -> Vec<String> {
//!     let mut parser = Parser::new(code);
//!     let mut errors = Vec::new();
//!
//!     match parser.parse() {
//!         Ok(_) => println!("Parse successful"),
//!         Err(err) => {
//!             // Log error for diagnostics
//!             errors.push(format!("Parse error: {}", err));
//!
//!             // Attempt error recovery for LSP
//!             match err {
//!                 ParseError::UnexpectedToken { .. } => {
//!                     // Continue parsing from next statement
//!                     println!("Attempting recovery...");
//!                 }
//!                 ParseError::RecursionLimit => {
//!                     // Use iterative parsing approach
//!                     println!("Switching to iterative parsing...");
//!                 }
//!                 _ => {
//!                     // Use fallback parsing strategy
//!                     println!("Using fallback parsing...");
//!                 }
//!             }
//!         }
//!     }
//!     errors
//! }
//! ```
//!
//! ## Comprehensive Error Context
//!
//! ```
//! use perl_parser::ParseError;
//!
//! fn create_detailed_error() -> ParseError {
//!     ParseError::UnexpectedToken {
//!         found: "number".to_string(),
//!         expected: "identifier".to_string(),
//!         location: 10, // byte position 10
//!     }
//! }
//!
//! fn handle_error_with_context(error: &ParseError) {
//!     match error {
//!         ParseError::UnexpectedToken { found, expected, location } => {
//!             println!("Syntax error at byte position {}: found '{}', expected '{}'",
//!                     location, found, expected);
//!         }
//!         ParseError::UnexpectedEof => {
//!             println!("Incomplete input: unexpected end of file");
//!         }
//!         _ => {
//!             println!("Parse error: {}", error);
//!         }
//!     }
//! }
//! ```

use thiserror::Error;

/// Result type for parser operations in the Perl parsing workflow pipeline
///
/// This type encapsulates success/failure outcomes throughout the Parse → Index →
/// Navigate → Complete → Analyze workflow, enabling consistent error propagation and recovery
/// strategies across all pipeline stages.
pub type ParseResult<T> = Result<T, ParseError>;

#[derive(Error, Debug, Clone, PartialEq)]
/// Comprehensive error types that can occur during Perl parsing within Perl parsing workflow workflows
///
/// These errors are designed to provide detailed context about parsing failures that occur during
/// Perl code analysis, script processing, and metadata extraction. Each error variant includes
/// location information to enable precise recovery strategies in large Perl file processing scenarios.
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
/// Error handling is optimized for processing 50GB+ Perl files with thousands of Perl scripts
/// and embedded Perl content, ensuring memory-efficient error propagation and logging.
pub enum ParseError {
    /// Parser encountered unexpected end of input during Perl code analysis
    ///
    /// This occurs when processing truncated Perl scripts or incomplete Perl source during
    /// the Parse stage. Recovery strategy: attempt partial parsing and preserve available content.
    #[error("Unexpected end of input")]
    UnexpectedEof,

    /// Parser found an unexpected token during Perl parsing workflow
    ///
    /// Common during Analyze stage when Perl scripts contain syntax variations or encoding issues.
    /// Recovery strategy: skip problematic tokens and attempt continued parsing with relaxed rules.
    #[error("Unexpected token: expected {expected}, found {found} at {location}")]
    UnexpectedToken {
        /// Token type that was expected during Perl script parsing
        expected: String,
        /// Actual token found in Perl script content
        found: String,
        /// Byte position where unexpected token was encountered
        location: usize,
    },

    /// General syntax error occurred during Perl code parsing
    ///
    /// This encompasses malformed Perl constructs found in Perl scripts during Navigate stage analysis.
    /// Recovery strategy: isolate syntax error scope and continue processing surrounding content.
    #[error("Invalid syntax at position {location}: {message}")]
    SyntaxError {
        /// Descriptive error message explaining the syntax issue
        message: String,
        /// Byte position where syntax error occurred in Perl script
        location: usize,
    },

    /// Lexical analysis failure during Perl script tokenization
    ///
    /// Indicates character encoding issues or binary content mixed with text during Parse stage.
    /// Recovery strategy: apply encoding detection and re-attempt tokenization with binary fallbacks.
    #[error("Lexer error: {message}")]
    LexerError {
        /// Detailed lexer error message describing tokenization failure
        message: String,
    },

    /// Parser recursion depth exceeded during complex Perl script analysis
    ///
    /// Occurs with deeply nested structures in Perl code during Complete stage processing.
    /// Recovery strategy: flatten recursive structures and process iteratively to maintain performance.
    #[error("Maximum recursion depth exceeded")]
    RecursionLimit,

    /// Invalid numeric literal found in Perl script content
    ///
    /// Common when processing malformed configuration values during Analyze stage analysis.
    /// Recovery strategy: substitute default values and log for manual review.
    #[error("Invalid number literal: {literal}")]
    InvalidNumber {
        /// The malformed numeric literal found in Perl script content
        literal: String,
    },

    /// Malformed string literal in Perl parsing workflow
    ///
    /// Indicates quote mismatches or encoding issues in Perl script strings during parsing.
    /// Recovery strategy: attempt string repair and normalization before re-parsing.
    #[error("Invalid string literal")]
    InvalidString,

    /// Unclosed delimiter detected during Perl code parsing
    ///
    /// Commonly found in truncated or corrupted Perl script content during Parse stage.
    /// Recovery strategy: auto-close delimiters and continue parsing with synthetic boundaries.
    #[error("Unclosed delimiter: {delimiter}")]
    UnclosedDelimiter {
        /// The delimiter character that was left unclosed
        delimiter: char,
    },

    /// Invalid regular expression syntax in Perl parsing workflow
    ///
    /// Occurs when parsing regex patterns in data filters during Navigate stage analysis.
    /// Recovery strategy: fallback to literal string matching and preserve original pattern.
    #[error("Invalid regex: {message}")]
    InvalidRegex {
        /// Specific error message describing regex syntax issue
        message: String,
    },
}

impl ParseError {
    /// Create a new syntax error for Perl parsing workflow failures
    ///
    /// # Arguments
    ///
    /// * `message` - Descriptive error message with context about the syntax issue
    /// * `location` - Character position within the Perl code where error occurred
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
    /// let error = ParseError::syntax("Missing semicolon in Perl script", 42);
    /// assert!(matches!(error, ParseError::SyntaxError { .. }));
    /// ```
    pub fn syntax(message: impl Into<String>, location: usize) -> Self {
        ParseError::SyntaxError { message: message.into(), location }
    }

    /// Create a new unexpected token error during Perl script parsing
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
    /// This is commonly used during the Analyze stage when Perl scripts contain
    /// syntax variations that require token-level recovery strategies.
    pub fn unexpected(
        expected: impl Into<String>,
        found: impl Into<String>,
        location: usize,
    ) -> Self {
        ParseError::UnexpectedToken { expected: expected.into(), found: found.into(), location }
    }
}
