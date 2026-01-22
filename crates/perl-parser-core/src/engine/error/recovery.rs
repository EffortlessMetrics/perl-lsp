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

use crate::{
    ast_v2::{Node, NodeKind},
    error::{BudgetTracker, ParseBudget},
    parser_context::ParserContext,
    position::Range,
};
use perl_lexer::TokenType;

/// Error information with recovery context for comprehensive Perl parsing error handling.
///
/// This structure encapsulates all information needed for intelligent error recovery
/// in the Perl parser, enabling continued parsing after syntax errors and providing
/// detailed diagnostic information for IDE integration.
///
/// # Examples
///
/// ```
/// use perl_parser::error_recovery::ParseError;
/// use perl_parser::position::{Position, Range};
///
/// let range = Range::new(Position::new(0, 1, 1), Position::new(5, 1, 6));
/// let error = ParseError::new("Syntax error".to_string(), range)
///     .with_expected(vec!["identifier".to_string()])
///     .with_found("number".to_string())
///     .with_hint("Did you mean to use a variable?".to_string());
/// ```
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

impl ErrorRecovery for ParserContext {
    fn create_error_node(
        &mut self,
        message: String,
        expected: Vec<String>,
        partial: Option<Node>,
    ) -> Node {
        let range = if let Some(token) = self.current_token() {
            token.range()
        } else {
            // End of file
            let pos = self.current_position();
            Range::new(pos, pos)
        };

        Node::new(
            self.id_generator.next_id(),
            NodeKind::Error { message, expected, partial: partial.map(Box::new) },
            range,
        )
    }

    fn synchronize(&mut self, sync_points: &[SyncPoint]) -> bool {
        let skipped = self.skip_until(sync_points);
        skipped > 0
    }

    fn recover_with_node(&mut self, error: ParseError) -> Node {
        // Add error to diagnostics
        self.add_error(error.clone());

        // Create error node
        let error_node = self.create_error_node(error.message, error.expected, None);

        // Try to synchronize
        let sync_points = vec![SyncPoint::Semicolon, SyncPoint::CloseBrace, SyncPoint::Keyword];
        self.synchronize(&sync_points);

        error_node
    }

    fn skip_until(&mut self, sync_points: &[SyncPoint]) -> usize {
        // Copy budget out (ParseBudget is Copy)
        let budget = *self.budget();

        // Move tracker out to avoid &mut self + &mut field aliasing
        let mut tracker = std::mem::take(self.budget_tracker_mut());
        let before = tracker.tokens_skipped;

        let _result = self.skip_until_with_budget(sync_points, &budget, &mut tracker);

        let after = tracker.tokens_skipped;

        // Restore the tracker
        *self.budget_tracker_mut() = tracker;

        // Return how many tokens we skipped in THIS call, not the total
        after.saturating_sub(before)
    }

    fn skip_until_with_budget(
        &mut self,
        sync_points: &[SyncPoint],
        budget: &ParseBudget,
        tracker: &mut BudgetTracker,
    ) -> RecoveryResult {
        // Check if already at a sync point BEFORE consuming anything.
        // This prevents infinite loops at call boundaries where recovery
        // is called repeatedly without making progress.
        if sync_points.iter().any(|sp| self.is_sync_point(*sp)) {
            return RecoveryResult::AtSyncPoint;
        }

        // Check if at EOF before attempting recovery
        if self.current_token().is_none() {
            return RecoveryResult::ReachedEof;
        }

        // Begin recovery attempt - checks budget BEFORE recording
        if !tracker.begin_recovery(budget) {
            return RecoveryResult::BudgetExhausted;
        }

        let mut skipped_this_call: usize = 0;

        while let Some(_token) = self.current_token() {
            // Check budget before skipping another token.
            // Use can_skip_more which considers skipped_this_call + 1 as "additional"
            if !tracker.can_skip_more(budget, skipped_this_call.saturating_add(1)) {
                tracker.record_skip(skipped_this_call);
                return RecoveryResult::BudgetExhausted;
            }

            // PROGRESS INVARIANT: Consume at least one token per iteration
            self.advance();
            skipped_this_call += 1;

            // Check if we've reached a sync point AFTER consuming
            if sync_points.iter().any(|sp| self.is_sync_point(*sp)) {
                tracker.record_skip(skipped_this_call);
                return RecoveryResult::Recovered(skipped_this_call);
            }
        }

        // Reached EOF
        tracker.record_skip(skipped_this_call);
        RecoveryResult::ReachedEof
    }

    fn is_sync_point(&self, sync_point: SyncPoint) -> bool {
        match self.current_token() {
            Some(token) => match sync_point {
                SyncPoint::Semicolon => matches!(&token.token.token_type, TokenType::Semicolon),
                SyncPoint::CloseBrace => matches!(&token.token.token_type, TokenType::RightBrace),
                SyncPoint::Keyword => matches!(
                    &token.token.token_type,
                    TokenType::Keyword(kw) if matches!(
                        kw.as_ref(),
                        "my" | "our" | "local" | "state" | "sub" | "if" | "unless" |
                        "while" | "until" | "for" | "foreach" | "return" | "last" |
                        "next" | "redo" | "goto" | "die" | "eval" | "do"
                    )
                ),
                SyncPoint::Eof => false,
            },
            None => sync_point == SyncPoint::Eof,
        }
    }
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

/// Common recovery patterns
pub mod recovery_patterns {
    use super::*;

    /// Try to recover a missing semicolon
    pub fn recover_missing_semicolon(ctx: &mut ParserContext) -> Option<ParseError> {
        // Check if we're at a natural statement boundary
        if ctx.is_sync_point(SyncPoint::Keyword)
            || ctx.is_sync_point(SyncPoint::CloseBrace)
            || ctx.current_token().is_none()
        {
            Some(
                ParseError::new("Missing semicolon".to_string(), ctx.current_position_range())
                    .with_expected(vec![";".to_string()])
                    .with_hint("Add a semicolon to end the statement".to_string()),
            )
        } else {
            None
        }
    }

    /// Try to recover from unmatched delimiter
    pub fn recover_unmatched_delimiter(
        ctx: &mut ParserContext,
        expected: &str,
        found: Option<&TokenType>,
    ) -> ParseError {
        let message = match found {
            Some(token) => format!("Expected '{}', found '{:?}'", expected, token),
            None => format!("Expected '{}', found end of file", expected),
        };

        ParseError::new(message, ctx.current_position_range())
            .with_expected(vec![expected.to_string()])
            .with_found(found.map(|t| format!("{:?}", t)).unwrap_or_else(|| "EOF".to_string()))
            .with_hint(format!("Add '{}' to match the opening delimiter", expected))
    }

    /// Create a placeholder for missing expressions
    pub fn create_missing_expression(ctx: &mut ParserContext) -> Node {
        Node::new(
            ctx.id_generator.next_id(),
            NodeKind::MissingExpression,
            ctx.current_position_range(),
        )
    }

    /// Create a placeholder for missing statements
    pub fn create_missing_statement(ctx: &mut ParserContext) -> Node {
        Node::new(
            ctx.id_generator.next_id(),
            NodeKind::MissingStatement,
            ctx.current_position_range(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::position::Position;

    #[test]
    fn test_error_creation() {
        let range = Range::new(Position::new(0, 1, 1), Position::new(5, 1, 6));

        let error = ParseError::new("Syntax error".to_string(), range)
            .with_expected(vec!["identifier".to_string()])
            .with_found("number".to_string())
            .with_hint("Did you mean to use a variable?".to_string());

        assert_eq!(error.message, "Syntax error");
        assert_eq!(error.expected, vec!["identifier"]);
        assert_eq!(error.found, "number");
        assert_eq!(error.recovery_hint, Some("Did you mean to use a variable?".to_string()));
    }

    #[test]
    fn test_begin_recovery_respects_budget() {
        let budget = ParseBudget { max_recoveries: 2, ..Default::default() };
        let mut tracker = BudgetTracker::new();

        // First two should succeed
        assert!(tracker.begin_recovery(&budget));
        assert_eq!(tracker.recoveries_attempted, 1);
        assert!(tracker.begin_recovery(&budget));
        assert_eq!(tracker.recoveries_attempted, 2);

        // Third should fail - budget exhausted
        assert!(!tracker.begin_recovery(&budget));
        assert_eq!(tracker.recoveries_attempted, 2); // Unchanged
    }

    #[test]
    fn test_can_skip_more_considers_pending_skips() {
        let budget = ParseBudget { max_tokens_skipped: 5, ..Default::default() };
        let mut tracker = BudgetTracker::new();

        // Skip 3 tokens
        tracker.record_skip(3);
        assert_eq!(tracker.tokens_skipped, 3);

        // Can skip 2 more (total would be 5)
        assert!(tracker.can_skip_more(&budget, 2));

        // Cannot skip 3 more (total would be 6)
        assert!(!tracker.can_skip_more(&budget, 3));
    }

    #[test]
    fn test_skip_until_at_sync_point_returns_immediately() {
        // When already at a sync point, skip_until should return AtSyncPoint
        // without consuming anything to prevent infinite loops
        let mut ctx = ParserContext::new("my $x;".to_string());
        let budget = ParseBudget::default();
        let mut tracker = BudgetTracker::new();

        // First, advance past "my" and "$x" to reach ";"
        ctx.advance(); // Skip 'my'
        ctx.advance(); // Skip '$x'

        // Now at semicolon - should return AtSyncPoint
        let result = ctx.skip_until_with_budget(&[SyncPoint::Semicolon], &budget, &mut tracker);

        assert_eq!(result, RecoveryResult::AtSyncPoint);
        assert_eq!(tracker.tokens_skipped, 0);
        assert_eq!(tracker.recoveries_attempted, 0);
    }

    #[test]
    fn test_skip_until_respects_token_budget() {
        let mut ctx = ParserContext::new("a b c d e f g h i j".to_string());
        let budget = ParseBudget { max_tokens_skipped: 3, ..Default::default() };
        let mut tracker = BudgetTracker::new();

        // Try to skip until semicolon (which doesn't exist)
        // Should stop after 3 tokens
        let result = ctx.skip_until_with_budget(&[SyncPoint::Semicolon], &budget, &mut tracker);

        assert_eq!(result, RecoveryResult::BudgetExhausted);
        assert_eq!(tracker.tokens_skipped, 3);
    }

    #[test]
    fn test_skip_until_respects_recovery_budget() {
        let mut ctx = ParserContext::new("a b c d e f".to_string());
        let budget = ParseBudget { max_recoveries: 1, ..Default::default() };
        let mut tracker = BudgetTracker::new();

        // First recovery should work - skip until semicolon (which doesn't exist)
        let result = ctx.skip_until_with_budget(&[SyncPoint::Semicolon], &budget, &mut tracker);
        assert!(matches!(result, RecoveryResult::ReachedEof));
        assert_eq!(tracker.recoveries_attempted, 1);

        // Reset context to beginning
        ctx.set_index(0);

        // Second recovery should fail - budget exhausted
        let result = ctx.skip_until_with_budget(&[SyncPoint::Semicolon], &budget, &mut tracker);
        assert_eq!(result, RecoveryResult::BudgetExhausted);
        assert_eq!(tracker.recoveries_attempted, 1); // Unchanged
    }

    #[test]
    fn test_skip_until_uses_context_budget() {
        // Verify that skip_until (without _with_budget) uses the context's budget
        let mut ctx = ParserContext::with_budget(
            "a b c d e f g h i j".to_string(),
            ParseBudget { max_tokens_skipped: 3, ..Default::default() },
        );

        // skip_until should use the context's budget
        let skipped = ctx.skip_until(&[SyncPoint::Semicolon]);

        // Should have skipped 3 tokens before hitting budget
        assert_eq!(skipped, 3);
        assert_eq!(ctx.budget_tracker().tokens_skipped, 3);
    }

    #[test]
    fn test_recovery_makes_progress_on_pathological_input() {
        // Test that recovery doesn't spin on malformed input
        let input = "= = = = = =";
        let mut ctx = ParserContext::with_budget(
            input.to_string(),
            ParseBudget {
                max_errors: 5,
                max_tokens_skipped: 50,
                max_recoveries: 10,
                ..Default::default()
            },
        );

        let budget = *ctx.budget();
        let mut tracker = std::mem::take(ctx.budget_tracker_mut());

        // Recovery should either find a sync point or exhaust budget
        let mut iterations = 0;
        let max_iterations = 100;

        loop {
            let result = ctx.skip_until_with_budget(&[SyncPoint::Semicolon], &budget, &mut tracker);

            iterations += 1;

            match result {
                RecoveryResult::Recovered(_) => {
                    // Found sync point, try next recovery
                    ctx.advance(); // Consume the sync point
                }
                RecoveryResult::AtSyncPoint => {
                    // At sync point, consume and continue
                    ctx.advance();
                }
                RecoveryResult::BudgetExhausted | RecoveryResult::ReachedEof => {
                    break;
                }
            }

            assert!(
                iterations < max_iterations,
                "Recovery appears to be spinning (iteration {})",
                iterations
            );
        }

        // Should have made progress before terminating
        assert!(tracker.tokens_skipped > 0 || iterations > 0);
    }
}
