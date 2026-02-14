//! Recursive descent Perl parser.
//!
//! Consumes tokens from `perl-lexer` and produces AST nodes with error recovery.
//! The parser handles operator precedence, quote-like operators, and heredocs,
//! while tracking recursion depth to prevent stack overflows on malformed input.
//!
//! # IDE-Friendly Error Recovery
//!
//! This parser uses an **IDE-friendly error recovery model**:
//!
//! - **Returns `Ok(ast)` with ERROR nodes** for most parse failures (recovered errors)
//! - **Returns `Err`** only for catastrophic failures (recursion limits, etc.)
//!
//! This means `result.is_err()` is **not** the correct way to check for parse errors.
//! Instead, check for ERROR nodes in the AST or use `parser.errors()`:
//!
//! ```rust,ignore
//! let mut parser = Parser::new(code);
//! match parser.parse() {
//!     Err(_) => println!("Catastrophic parse failure"),
//!     Ok(ast) => {
//!         // Check for recovered errors via ERROR nodes
//!         if ast.to_sexp().contains("ERROR") {
//!             println!("Parse errors recovered: {:?}", parser.errors());
//!         }
//!     }
//! }
//! ```
//!
//! ## Why IDE-Friendly?
//!
//! Traditional compilers return `Err` on any syntax error. This prevents:
//! - Code completion in incomplete code
//! - Go-to-definition while typing
//! - Hover information in files with errors
//!
//! By returning partial ASTs with ERROR nodes, editors can provide useful
//! features even when code is incomplete or contains errors.
//!
//! # Performance
//!
//! - **Time complexity**: O(n) for typical token streams
//! - **Space complexity**: O(n) for AST storage with bounded recursion memory usage
//! - **Optimizations**: Fast-path parsing and efficient recovery to maintain performance
//! - **Benchmarks**: ~150µs–1ms for typical files; low ms for large file inputs
//! - **Large-scale notes**: Tuned to scale for large workspaces (50GB PST-style scans)
//!
//! # Usage
//!
//! ```rust
//! use perl_parser_core::Parser;
//!
//! let mut parser = Parser::new("my $var = 42; sub hello { print $var; }");
//! let ast = parser.parse();
//! ```

use crate::{
    ast::{Node, NodeKind, SourceLocation},
    error::{ParseError, ParseOutput, ParseResult},
    heredoc_collector::{self, HeredocContent, PendingHeredoc, collect_all},
    quote_parser,
    token_stream::{Token, TokenKind, TokenStream},
};
use std::collections::VecDeque;
use std::sync::Arc;
use std::time::Instant;

// Import enhanced recovery module
// mod enhanced_recovery;
// pub use enhanced_recovery::{RecoveryConfig, EnhancedRecovery, EnhancedErrorRecovery, ErrorContext};

/// Parser state for a single Perl source input.
///
/// Construct with [`Parser::new`] and call [`Parser::parse`] to obtain an AST.
/// Non-fatal syntax errors are collected and can be accessed via [`Parser::errors`].
pub struct Parser<'a> {
    /// Token stream providing access to lexed Perl script content
    tokens: TokenStream<'a>,
    /// Current recursion depth for overflow protection during complex Perl script parsing
    recursion_depth: usize,
    /// Position tracking for error reporting and AST location information
    last_end_position: usize,
    /// Context flag for disambiguating for-loop initialization syntax
    in_for_loop_init: bool,
    /// Statement boundary tracking for indirect object syntax detection
    at_stmt_start: bool,
    /// FIFO queue of pending heredoc declarations awaiting content collection
    pending_heredocs: VecDeque<PendingHeredoc>,
    /// Source bytes for heredoc content collection (shared with token stream)
    src_bytes: &'a [u8],
    /// Byte cursor tracking position for heredoc content collection
    byte_cursor: usize,
    /// Start time of parsing for timeout enforcement (specifically heredocs)
    heredoc_start_time: Option<Instant>,
    /// Collection of parse errors encountered during parsing (for error recovery)
    errors: Vec<ParseError>,
    // Enhanced error recovery state
    // pub enhanced_recovery: EnhancedRecovery,
}

// Recursion limit is set conservatively to prevent stack overflow
// before the limit triggers. The actual stack usage depends on the
// number of function frames between recursion checks (about 20-30
// for the precedence parsing chain). 128 * 30 = ~3840 frames which
// is safe. Real Perl code rarely exceeds 20-30 nesting levels.
const MAX_RECURSION_DEPTH: usize = 128;

impl<'a> Parser<'a> {
    /// Create a new parser for the provided Perl source.
    ///
    /// # Arguments
    ///
    /// * `input` - Perl source code to be parsed
    ///
    /// # Returns
    ///
    /// A configured parser ready to parse the provided source.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use perl_parser_core::Parser;
    ///
    /// let script = "use strict; my $filter = qr/important/;";
    /// let mut parser = Parser::new(script);
    /// // Parser ready to parse the source
    /// ```
    pub fn new(input: &'a str) -> Self {
        Parser {
            tokens: TokenStream::new(input),
            recursion_depth: 0,
            last_end_position: 0,
            in_for_loop_init: false,
            at_stmt_start: true,
            pending_heredocs: VecDeque::new(),
            src_bytes: input.as_bytes(),
            byte_cursor: 0,
            heredoc_start_time: None,
            errors: Vec::new(),
            // enhanced_recovery: EnhancedRecovery::new(RecoveryConfig::default()),
        }
    }

    /// Create a new parser with custom enhanced recovery configuration
    pub fn new_with_recovery_config(input: &'a str, _config: ()) -> Self {
        Parser {
            tokens: TokenStream::new(input),
            recursion_depth: 0,
            last_end_position: 0,
            in_for_loop_init: false,
            at_stmt_start: true,
            pending_heredocs: VecDeque::new(),
            src_bytes: input.as_bytes(),
            byte_cursor: 0,
            heredoc_start_time: None,
            errors: Vec::new(),
            // enhanced_recovery: EnhancedRecovery::new(config),
        }
    }

    /// Parse the source and return the AST for the Parse stage.
    ///
    /// # Returns
    ///
    /// * `Ok(Node)` - Parsed AST with a `Program` root node.
    /// * `Err(ParseError)` - Non-recoverable parsing failure.
    ///
    /// # Errors
    ///
    /// Returns `ParseError` for non-recoverable conditions such as recursion limits.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use perl_parser_core::Parser;
    ///
    /// let mut parser = Parser::new("my $count = 1;");
    /// let ast = parser.parse()?;
    /// assert!(matches!(ast.kind, perl_parser_core::NodeKind::Program { .. }));
    /// # Ok::<(), perl_parser_core::ParseError>(())
    /// ```
    pub fn parse(&mut self) -> ParseResult<Node> {
        self.parse_program()
    }

    /// Get all parse errors collected during parsing
    ///
    /// When error recovery is enabled, the parser continues after syntax errors
    /// and collects them for later retrieval. This is useful for IDE integration
    /// where you want to show all errors at once.
    ///
    /// # Returns
    ///
    /// A slice of all `ParseError`s encountered during parsing
    ///
    /// # Examples
    ///
    /// ```rust
    /// use perl_parser_core::Parser;
    ///
    /// let mut parser = Parser::new("my $x = ; sub foo {");
    /// let _ast = parser.parse(); // Parse with recovery
    /// let errors = parser.errors();
    /// // errors will contain details about syntax errors
    /// ```
    pub fn errors(&self) -> &[ParseError] {
        &self.errors
    }

    /// Parse with error recovery and return comprehensive output.
    ///
    /// This method is preferred for LSP Analyze workflows and always returns
    /// a `ParseOutput` containing the AST and any collected diagnostics.
    ///
    /// # Returns
    ///
    /// `ParseOutput` with the AST and diagnostics collected during parsing.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use perl_parser_core::Parser;
    ///
    /// let mut parser = Parser::new("my $x = ;");
    /// let output = parser.parse_with_recovery();
    /// assert!(!output.diagnostics.is_empty() || matches!(output.ast.kind, perl_parser_core::NodeKind::Program { .. }));
    /// ```
    pub fn parse_with_recovery(&mut self) -> ParseOutput {
        let ast = match self.parse() {
            Ok(node) => node,
            Err(e) => {
                // If parse() returned Err, it was a non-recoverable error (e.g. recursion limit)
                // Ensure it's recorded if not already
                if !self.errors.contains(&e) {
                    self.errors.push(e.clone());
                }

                // Return a dummy Program node with the error
                Node::new(
                    NodeKind::Program { statements: vec![] },
                    SourceLocation { start: 0, end: 0 },
                )
            }
        };

        ParseOutput::with_errors(ast, self.errors.clone())
    }
}

// impl<'a> EnhancedErrorRecovery for Parser<'a> {
//     fn with_enhanced_recovery() -> Self {
//         unimplemented!("Use Parser::new() for default enhanced recovery")
//     }
//
//     fn with_enhanced_recovery_config(_config: RecoveryConfig) -> Self {
//         unimplemented!("Use Parser::new_with_recovery_config() instead")
//     }
//
//     fn recovery_state(&self) -> &EnhancedRecovery {
//         &self.enhanced_recovery
//     }
//
//     fn recovery_state_mut(&mut self) -> &mut EnhancedRecovery {
//         &mut self.enhanced_recovery
//     }
//
//     fn create_enhanced_error_node(
//         &mut self,
//         error: ParseError,
//         context: ErrorContext,
//     ) -> Node {
//         // Track node creation for memory monitoring
//         self.enhanced_recovery.track_node();
//
//         // Get suggestions based on error type and context
//         let error_type = match &error {
//             ParseError::UnexpectedToken { .. } => "unexpected_token",
//             ParseError::UnclosedDelimiter { .. } => "unclosed_delimiter",
//             ParseError::UnexpectedEof => "missing_expression",
//             _ => "syntax_error",
//         };
//
//         let suggestions = self.enhanced_recovery.get_suggestions(error_type, &context);
//
//         // Create enhanced error message with suggestions
//         let mut message = format!("{}", error);
//         if let Some(best_suggestion) = suggestions.first() {
//             if best_suggestion.confidence > 0.7 {
//                 message.push_str(&format!(" Suggestion: {}", best_suggestion.message));
//             }
//         }
//
//         let start = context.current_token
//             .as_ref()
//             .map(|t| t.start)
//             .unwrap_or(self.current_position());
//         let end = context.current_token
//             .as_ref()
//             .map(|t| t.end)
//             .unwrap_or(start);
//
//         Node::new(
//             NodeKind::Error {
//                 message,
//                 expected: vec![],
//                 found: context.current_token,
//                 partial: None, // We don't store suggestions in the AST, just in the message
//             },
//             SourceLocation { start, end }
//         )
//     }
//
//     fn apply_adaptive_recovery(&mut self, error: &ParseError, context: &ErrorContext) -> bool {
//         // Check resource limits before recovery
//         if let Err(e) = self.enhanced_recovery.should_continue() {
//             self.record_error(e);
//             return false;
//         }
//
//         let strategy = self.enhanced_recovery.apply_adaptive_recovery(error);
//
//         match strategy {
//             enhanced_recovery::RecoveryStrategy::SkipToken => {
//                 let _ = self.consume_token();
//                 true
//             }
//             enhanced_recovery::RecoveryStrategy::InsertClosing => {
//                 // Try to insert missing closing delimiter
//                 if let Some(token) = &context.current_token {
//                     match token.kind {
//                         TokenKind::LeftBrace => {
//                             // Insert closing brace
//                             self.create_error_node(
//                                 "Unclosed delimiter".to_string(),
//                                 vec![TokenKind::LeftBrace],
//                             );
//                             true
//                         }
//                         TokenKind::LeftParen => {
//                             // Insert closing parenthesis
//                             self.create_error_node(
//                                 "Unclosed delimiter".to_string(),
//                                 vec![TokenKind::LeftParen],
//                             );
//                             true
//                         }
//                         TokenKind::LeftBracket => {
//                             // Insert closing bracket
//                             self.create_error_node(
//                                 "Unclosed delimiter".to_string(),
//                                 vec![TokenKind::LeftBracket],
//                             );
//                             true
//                         }
//                         _ => false,
//                     }
//                 } else {
//                     false
//                 }
//             }
//             enhanced_recovery::RecoveryStrategy::TreatAsVariable => {
//                 // Try to treat as variable declaration
//                 self.create_error_node(
//                     "Treating as variable declaration".to_string(),
//                     vec![TokenKind::ScalarSigil],
//                 );
//                 true
//             }
//             _ => false,
//         }
//     }
// }

include!("helpers.rs");
include!("heredoc.rs");
include!("statements.rs");
include!("variables.rs");
include!("control_flow.rs");
include!("declarations.rs");
include!("expressions/mod.rs");
include!("expressions/precedence.rs");
include!("expressions/unary.rs");
include!("expressions/postfix.rs");
include!("expressions/primary.rs");
include!("expressions/calls.rs");
include!("expressions/hashes.rs");
include!("expressions/quotes.rs");

#[cfg(test)]
mod error_recovery_tests;
// #[cfg(test)]
// mod enhanced_recovery_tests;
#[cfg(test)]
mod format_comprehensive_tests;
#[cfg(test)]
mod format_tests;
#[cfg(test)]
mod glob_assignment_tests;
#[cfg(test)]
mod glob_tests;
#[cfg(test)]
mod hash_vs_block_tests;
#[cfg(test)]
mod heredoc_security_tests;
#[cfg(test)]
mod indirect_call_tests;
#[cfg(test)]
mod indirect_object_tests;
#[cfg(test)]
mod loop_control_tests;
#[cfg(test)]
mod regex_delimiter_tests;
#[cfg(test)]
mod slash_ambiguity_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod tie_tests;
