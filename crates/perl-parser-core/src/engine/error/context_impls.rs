use crate::engine::parser_context::ParserContext;
use perl_ast::v2::{Node, NodeKind};
use perl_error::BudgetTracker;
use perl_error::ParseBudget;
use perl_error::recovery::{ErrorRecovery, ParseError, RecoveryResult, SyncPoint};
use perl_lexer::TokenType;
use perl_position_tracking::Range;

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
