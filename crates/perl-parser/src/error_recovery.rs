//! Error recovery for the Perl parser
//!
//! This module implements error recovery strategies to continue parsing
//! even when syntax errors are encountered. This is essential for IDE
//! scenarios where code is often incomplete or temporarily invalid.

use crate::{
    ast_v2::{Node, NodeKind},
    position::Range,
    parser_context::ParserContext,
};
use perl_lexer::TokenType;

/// Error information with recovery context
#[derive(Debug, Clone)]
pub struct ParseError {
    pub message: String,
    pub range: Range,
    pub expected: Vec<String>,
    pub found: String,
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
    
    /// Skip tokens until a sync point
    fn skip_until(&mut self, sync_points: &[SyncPoint]) -> usize;
    
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
            Range::new(pos.clone(), pos)
        };
        
        Node::new(
            self.id_generator.next_id(),
            NodeKind::Error {
                message,
                expected,
                partial: partial.map(Box::new),
            },
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
        let error_node = self.create_error_node(
            error.message,
            error.expected,
            None,
        );
        
        // Try to synchronize
        let sync_points = vec![
            SyncPoint::Semicolon,
            SyncPoint::CloseBrace,
            SyncPoint::Keyword,
        ];
        self.synchronize(&sync_points);
        
        error_node
    }
    
    fn skip_until(&mut self, sync_points: &[SyncPoint]) -> usize {
        let mut skipped = 0;
        
        while let Some(_token) = self.current_token() {
            // Check if we've reached a sync point
            for sync_point in sync_points {
                if self.is_sync_point(*sync_point) {
                    return skipped;
                }
            }
            
            // Skip the current token
            self.advance();
            skipped += 1;
            
            // Prevent infinite loops
            if skipped > 100 {
                break;
            }
        }
        
        skipped
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
        if ctx.is_sync_point(SyncPoint::Keyword) || 
           ctx.is_sync_point(SyncPoint::CloseBrace) ||
           ctx.current_token().is_none() {
            Some(ParseError::new(
                "Missing semicolon".to_string(),
                ctx.current_position_range(),
            ).with_expected(vec![";".to_string()])
             .with_hint("Add a semicolon to end the statement".to_string()))
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
        let range = Range::new(
            Position::new(0, 1, 1),
            Position::new(5, 1, 6),
        );
        
        let error = ParseError::new("Syntax error".to_string(), range.clone())
            .with_expected(vec!["identifier".to_string()])
            .with_found("number".to_string())
            .with_hint("Did you mean to use a variable?".to_string());
        
        assert_eq!(error.message, "Syntax error");
        assert_eq!(error.expected, vec!["identifier"]);
        assert_eq!(error.found, "number");
        assert_eq!(error.recovery_hint, Some("Did you mean to use a variable?".to_string()));
    }
}