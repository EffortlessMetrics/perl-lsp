use crate::parser_context::ParserContext;
use perl_ast::v2::{Node, NodeKind};
use perl_error::recovery::{ErrorRecovery, ParseError, RecoveryResult, SyncPoint};
use perl_error::{BudgetTracker, ParseBudget};
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
        self.add_error(error.clone());
        let error_node = self.create_error_node(error.message, error.expected, None);
        let sync_points = vec![SyncPoint::Semicolon, SyncPoint::CloseBrace, SyncPoint::Keyword];
        self.synchronize(&sync_points);
        error_node
    }

    fn skip_until(&mut self, sync_points: &[SyncPoint]) -> usize {
        let budget = *self.budget();
        let mut tracker = std::mem::take(self.budget_tracker_mut());
        let before = tracker.tokens_skipped;
        let _result = self.skip_until_with_budget(sync_points, &budget, &mut tracker);
        let after = tracker.tokens_skipped;
        *self.budget_tracker_mut() = tracker;
        after.saturating_sub(before)
    }

    fn skip_until_with_budget(
        &mut self,
        sync_points: &[SyncPoint],
        budget: &ParseBudget,
        tracker: &mut BudgetTracker,
    ) -> RecoveryResult {
        if sync_points.iter().any(|sp| self.is_sync_point(*sp)) {
            return RecoveryResult::AtSyncPoint;
        }

        if self.current_token().is_none() {
            return RecoveryResult::ReachedEof;
        }

        if !tracker.begin_recovery(budget) {
            return RecoveryResult::BudgetExhausted;
        }

        let mut skipped_this_call: usize = 0;

        while let Some(_token) = self.current_token() {
            if !tracker.can_skip_more(budget, skipped_this_call.saturating_add(1)) {
                tracker.record_skip(skipped_this_call);
                return RecoveryResult::BudgetExhausted;
            }

            self.advance();
            skipped_this_call += 1;

            if sync_points.iter().any(|sp| self.is_sync_point(*sp)) {
                tracker.record_skip(skipped_this_call);
                return RecoveryResult::Recovered(skipped_this_call);
            }
        }

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
