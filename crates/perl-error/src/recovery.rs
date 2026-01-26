//! Error recovery for the Perl parser
//!
//! This module implements error recovery strategies to continue parsing
//! even when syntax errors are encountered. This is essential for IDE
//! scenarios where code is often incomplete or temporarily invalid.
//!
//! # Progress Invariant
//!
//! All recovery operations guarantee forward progress: every recovery attempt
//! must consume at least one token or exit. This prevents infinite loops when
//! the parser cannot make sense of the input.
//!
//! # Budget Awareness
//!
//! Recovery operations respect the `ParseBudget` limits to prevent runaway
//! parsing on adversarial input. When budget is exhausted, recovery returns
//! immediately with an appropriate error node.

use perl_ast::v2::Node;
use crate::{BudgetTracker, ParseBudget};
use perl_position_tracking::Range;
use perl_lexer::TokenType;

/// Error information with recovery context for comprehensive Perl parsing error handling.
///
/// This structure encapsulates all information needed for intelligent error recovery
/// in the Perl parser, enabling continued parsing after syntax errors and providing
/// detailed diagnostic information for IDE integration.
#[derive(Debug, Clone)]
pub struct ParseError {
    /// Human-readable error message describing the parsing issue
    pub message: String,
    /// Source code range where the error occurred
    pub range: Range,
    /// List of token types that were expected at this position
    pub expected: Vec<String>,
    /// The token that was actually found instead of expected
    pub found: String,
    /// Optional hint for error recovery or fixing the issue
    pub recovery_hint: Option<String>,
}

impl ParseError {
    /// Create a new parse error
    pub fn new(message: String, range: Range) -> Self {
        ParseError {
            message,
            range,
            expected: Vec::new(),
            found: String::new(),
            recovery_hint: None,
        }
    }

    /// Add expected tokens
    pub fn with_expected(mut self, expected: Vec<String>) -> Self {
        self.expected = expected;
        self
    }

    /// Add found token
    pub fn with_found(mut self, found: String) -> Self {
        self.found = found;
        self
    }

    /// Add recovery hint
    pub fn with_hint(mut self, hint: String) -> Self {
        self.recovery_hint = Some(hint);
        self
    }
}

/// Synchronization tokens for error recovery
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SyncPoint {
    /// Semicolon - statement boundary
    Semicolon,
    /// Closing brace - block boundary
    CloseBrace,
    /// Keywords that start statements
    Keyword,
    /// End of file
    Eof,
}

/// Result of a recovery operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryResult {
    /// Recovery succeeded, consumed the given number of tokens.
    Recovered(usize),
    /// Already at a sync point when recovery was called.
    /// The caller must decide whether to consume the sync token.
    /// This prevents infinite loops at call boundaries.
    AtSyncPoint,
    /// Recovery failed due to budget exhaustion.
    BudgetExhausted,
    /// Recovery reached EOF without finding sync point.
    ReachedEof,
}

/// Error recovery strategies
pub trait ErrorRecovery {
    /// Create an error node and recover
    fn create_error_node(
        &mut self,
        message: String,
        expected: Vec<String>,
        partial: Option<Node>,
    ) -> Node;

    /// Synchronize to a recovery point
    fn synchronize(&mut self, sync_points: &[SyncPoint]) -> bool;

    /// Try to recover from an error
    fn recover_with_node(&mut self, error: ParseError) -> Node;

    /// Skip tokens until a sync point.
    ///
    /// # Progress Invariant
    ///
    /// This method guarantees forward progress: it will consume at least one
    /// token on each call (unless already at EOF or a sync point), preventing
    /// infinite recovery loops.
    fn skip_until(&mut self, sync_points: &[SyncPoint]) -> usize;

    /// Budget-aware skip that respects limits.
    ///
    /// # Progress Invariant
    ///
    /// Consumes at least one token per call (unless at sync point, EOF, or budget exhausted).
    fn skip_until_with_budget(
        &mut self,
        sync_points: &[SyncPoint],
        budget: &ParseBudget,
        tracker: &mut BudgetTracker,
    ) -> RecoveryResult;

    /// Check if current token is a sync point
    fn is_sync_point(&self, sync_point: SyncPoint) -> bool;
}

/// Parser extensions for error recovery
pub trait ParserErrorRecovery {
    /// Parse with error recovery enabled
    fn parse_with_recovery(&mut self) -> (Node, Vec<ParseError>);

    /// Try to parse, returning an error node on failure
    fn try_parse<F>(&mut self, parse_fn: F) -> Node
    where
        F: FnOnce(&mut Self) -> Option<Node>;

    /// Parse a list with recovery on each element
    fn parse_list_with_recovery<F>(
        &mut self,
        parse_element: F,
        separator: TokenType,
        terminator: TokenType,
    ) -> Vec<Node>
    where
        F: Fn(&mut Self) -> Node;
}

/// Recovery-aware statement parsing
pub trait StatementRecovery {
    /// Parse statement with recovery
    fn parse_statement_with_recovery(&mut self) -> Node;

    /// Parse expression with recovery
    fn parse_expression_with_recovery(&mut self) -> Node;

    /// Parse block with recovery
    fn parse_block_with_recovery(&mut self) -> Node;
}
