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

/// Budget limits for parser operations to prevent runaway parsing.
///
/// These limits ensure the parser terminates in bounded time even when
/// processing malformed or adversarial input. Each budget parameter has
/// a sensible default that works for most real-world Perl code.
///
/// # Usage
///
/// ```
/// use perl_parser::ParseBudget;
///
/// // Use defaults for normal parsing
/// let budget = ParseBudget::default();
///
/// // Stricter limits for untrusted input
/// let strict = ParseBudget {
///     max_errors: 10,
///     max_depth: 64,
///     max_tokens_skipped: 100,
///     max_recoveries: 50,
/// };
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParseBudget {
    /// Maximum number of errors to collect before giving up.
    /// After this limit, parsing stops to avoid flooding diagnostics.
    /// Default: 100
    pub max_errors: usize,

    /// Maximum nesting depth for recursive constructs (blocks, expressions).
    /// Prevents stack overflow on deeply nested input.
    /// Default: 256
    pub max_depth: usize,

    /// Maximum tokens to skip during a single recovery attempt.
    /// Prevents infinite loops when recovery can't find a sync point.
    /// Default: 1000
    pub max_tokens_skipped: usize,

    /// Maximum number of recovery attempts per parse.
    /// Bounds total recovery work to prevent pathological cases.
    /// Default: 500
    pub max_recoveries: usize,
}

impl Default for ParseBudget {
    fn default() -> Self {
        Self {
            max_errors: 100,
            max_depth: 256,
            max_tokens_skipped: 1000,
            max_recoveries: 500,
        }
    }
}

impl ParseBudget {
    /// Create a budget suitable for IDE/LSP usage with generous limits.
    pub fn for_ide() -> Self {
        Self::default()
    }

    /// Create a strict budget for parsing untrusted input.
    pub fn strict() -> Self {
        Self {
            max_errors: 10,
            max_depth: 64,
            max_tokens_skipped: 100,
            max_recoveries: 50,
        }
    }

    /// Create an unlimited budget (use with caution).
    pub fn unlimited() -> Self {
        Self {
            max_errors: usize::MAX,
            max_depth: usize::MAX,
            max_tokens_skipped: usize::MAX,
            max_recoveries: usize::MAX,
        }
    }
}

/// Tracks budget consumption during parsing.
///
/// This struct monitors how much of the parse budget has been used
/// and provides methods to check and consume budget atomically.
#[derive(Debug, Clone, Default)]
pub struct BudgetTracker {
    /// Number of errors emitted so far.
    pub errors_emitted: usize,
    /// Current nesting depth.
    pub current_depth: usize,
    /// Maximum depth reached during parse.
    pub max_depth_reached: usize,
    /// Total tokens skipped across all recovery attempts.
    pub tokens_skipped: usize,
    /// Number of recovery attempts made.
    pub recoveries_attempted: usize,
}

impl BudgetTracker {
    /// Create a new budget tracker.
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if error budget is exhausted.
    pub fn errors_exhausted(&self, budget: &ParseBudget) -> bool {
        self.errors_emitted >= budget.max_errors
    }

    /// Check if depth budget would be exceeded by going one level deeper.
    pub fn depth_would_exceed(&self, budget: &ParseBudget) -> bool {
        self.current_depth >= budget.max_depth
    }

    /// Check if skip budget would be exceeded by skipping `count` more tokens.
    pub fn skip_would_exceed(&self, budget: &ParseBudget, count: usize) -> bool {
        self.tokens_skipped.saturating_add(count) > budget.max_tokens_skipped
    }

    /// Check if recovery budget is exhausted.
    pub fn recoveries_exhausted(&self, budget: &ParseBudget) -> bool {
        self.recoveries_attempted >= budget.max_recoveries
    }

    /// Record an error emission.
    pub fn record_error(&mut self) {
        self.errors_emitted = self.errors_emitted.saturating_add(1);
    }

    /// Enter a deeper nesting level.
    pub fn enter_depth(&mut self) {
        self.current_depth = self.current_depth.saturating_add(1);
        if self.current_depth > self.max_depth_reached {
            self.max_depth_reached = self.current_depth;
        }
    }

    /// Exit a nesting level.
    pub fn exit_depth(&mut self) {
        self.current_depth = self.current_depth.saturating_sub(1);
    }

    /// Record tokens skipped during recovery.
    pub fn record_skip(&mut self, count: usize) {
        self.tokens_skipped = self.tokens_skipped.saturating_add(count);
    }

    /// Record a recovery attempt.
    pub fn record_recovery(&mut self) {
        self.recoveries_attempted = self.recoveries_attempted.saturating_add(1);
    }
}

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

pub mod classifier;
pub mod recovery;
pub mod recovery_parser;

use crate::ast::Node;

/// Structured output from parsing, combining AST with all diagnostics.
///
/// This type replaces the simple `Result<Node, ParseError>` pattern to enable
/// error recovery. Even when errors occur, parsing continues and produces a
/// partial AST alongside collected diagnostics.
///
/// # Usage
///
/// ```no_run
/// use perl_parser::{Parser, ParseOutput};
///
/// let mut parser = Parser::new("my $x = ;");
/// let output = parser.parse_with_recovery();
///
/// // AST is always available (may contain error nodes)
/// println!("Statements: {:?}", output.ast);
///
/// // Diagnostics are collected separately
/// for error in &output.diagnostics {
///     println!("Error: {}", error);
/// }
///
/// // Budget tracking shows resource usage
/// println!("Errors: {}", output.budget_usage.errors_emitted);
/// ```
#[derive(Debug, Clone)]
pub struct ParseOutput {
    /// The parsed AST. Always present, but may contain error nodes
    /// if parsing encountered recoverable errors.
    pub ast: Node,

    /// All diagnostics (errors and warnings) collected during parsing.
    /// These are ordered by source position.
    pub diagnostics: Vec<ParseError>,

    /// Budget consumption during this parse.
    /// Useful for diagnosing pathological inputs.
    pub budget_usage: BudgetTracker,

    /// Whether parsing completed normally or was terminated early
    /// due to budget exhaustion.
    pub terminated_early: bool,
}

impl ParseOutput {
    /// Create a successful parse output with no errors.
    pub fn success(ast: Node) -> Self {
        Self {
            ast,
            diagnostics: Vec::new(),
            budget_usage: BudgetTracker::new(),
            terminated_early: false,
        }
    }

    /// Create a parse output with errors.
    pub fn with_errors(ast: Node, diagnostics: Vec<ParseError>) -> Self {
        let mut budget_usage = BudgetTracker::new();
        budget_usage.errors_emitted = diagnostics.len();
        Self {
            ast,
            diagnostics,
            budget_usage,
            terminated_early: false,
        }
    }

    /// Check if parse completed without any errors.
    pub fn is_ok(&self) -> bool {
        self.diagnostics.is_empty()
    }

    /// Check if parse had errors.
    pub fn has_errors(&self) -> bool {
        !self.diagnostics.is_empty()
    }

    /// Get the error count.
    pub fn error_count(&self) -> usize {
        self.diagnostics.len()
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_budget_defaults() {
        let budget = ParseBudget::default();
        assert_eq!(budget.max_errors, 100);
        assert_eq!(budget.max_depth, 256);
        assert_eq!(budget.max_tokens_skipped, 1000);
        assert_eq!(budget.max_recoveries, 500);
    }

    #[test]
    fn test_parse_budget_strict() {
        let budget = ParseBudget::strict();
        assert_eq!(budget.max_errors, 10);
        assert_eq!(budget.max_depth, 64);
        assert_eq!(budget.max_tokens_skipped, 100);
        assert_eq!(budget.max_recoveries, 50);
    }

    #[test]
    fn test_budget_tracker_errors() {
        let budget = ParseBudget { max_errors: 3, ..Default::default() };
        let mut tracker = BudgetTracker::new();

        assert!(!tracker.errors_exhausted(&budget));

        tracker.record_error();
        tracker.record_error();
        assert!(!tracker.errors_exhausted(&budget));

        tracker.record_error();
        assert!(tracker.errors_exhausted(&budget));
    }

    #[test]
    fn test_budget_tracker_depth() {
        let budget = ParseBudget { max_depth: 2, ..Default::default() };
        let mut tracker = BudgetTracker::new();

        assert!(!tracker.depth_would_exceed(&budget));

        tracker.enter_depth();
        assert!(!tracker.depth_would_exceed(&budget));

        tracker.enter_depth();
        assert!(tracker.depth_would_exceed(&budget));

        tracker.exit_depth();
        assert!(!tracker.depth_would_exceed(&budget));
    }

    #[test]
    fn test_budget_tracker_skip() {
        let budget = ParseBudget { max_tokens_skipped: 5, ..Default::default() };
        let mut tracker = BudgetTracker::new();

        assert!(!tracker.skip_would_exceed(&budget, 3));
        tracker.record_skip(3);

        assert!(!tracker.skip_would_exceed(&budget, 2));
        assert!(tracker.skip_would_exceed(&budget, 3));
    }

    #[test]
    fn test_budget_tracker_recoveries() {
        let budget = ParseBudget { max_recoveries: 2, ..Default::default() };
        let mut tracker = BudgetTracker::new();

        assert!(!tracker.recoveries_exhausted(&budget));

        tracker.record_recovery();
        assert!(!tracker.recoveries_exhausted(&budget));

        tracker.record_recovery();
        assert!(tracker.recoveries_exhausted(&budget));
    }

    #[test]
    fn test_parse_output_success() {
        use crate::ast::{Node, NodeKind, SourceLocation};

        let ast = Node::new(
            NodeKind::Program { statements: vec![] },
            SourceLocation { start: 0, end: 0 },
        );
        let output = ParseOutput::success(ast);

        assert!(output.is_ok());
        assert!(!output.has_errors());
        assert_eq!(output.error_count(), 0);
        assert!(!output.terminated_early);
    }

    #[test]
    fn test_parse_output_with_errors() {
        use crate::ast::{Node, NodeKind, SourceLocation};

        let ast = Node::new(
            NodeKind::Program { statements: vec![] },
            SourceLocation { start: 0, end: 0 },
        );
        let errors = vec![
            ParseError::syntax("error 1", 0),
            ParseError::syntax("error 2", 5),
        ];
        let output = ParseOutput::with_errors(ast, errors);

        assert!(!output.is_ok());
        assert!(output.has_errors());
        assert_eq!(output.error_count(), 2);
    }
}
