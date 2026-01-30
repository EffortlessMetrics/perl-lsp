//! Error recovery mechanisms for robust parsing
//!
//! This module provides error recovery strategies to continue parsing
//! even when encountering malformed or incomplete Perl code.

use crate::error::ParseError;
use crate::pure_rust_parser::{AstNode, PerlParser, Rule};
use pest::Parser;
use std::sync::Arc;

/// Recovery strategies for different error types
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RecoveryStrategy {
    /// Skip to next statement boundary (semicolon, closing brace)
    SkipToStatementEnd,
    /// Skip to next line
    SkipLine,
    /// Skip current block
    SkipBlock,
    /// Try to parse as expression
    ParseAsExpression,
    /// Create error node and continue
    CreateErrorNode,
}

/// Context for error recovery
#[derive(Debug, Clone)]
pub struct RecoveryContext {
    pub line: usize,
    pub column: usize,
    pub expected: Vec<String>,
    pub found: String,
    pub partial_ast: Option<AstNode>,
}

/// Error recovery parser that can handle malformed input
pub struct ErrorRecoveryParser {
    /// Maximum number of recovery attempts before giving up
    max_recovery_attempts: usize,
    /// Strategies to try in order
    recovery_strategies: Vec<RecoveryStrategy>,
    /// Whether to collect error nodes
    collect_errors: bool,
    /// Collected error nodes
    errors: Vec<ErrorNode>,
}

#[derive(Debug, Clone)]
pub struct ErrorNode {
    pub message: String,
    pub line: usize,
    pub column: usize,
    pub span: (usize, usize),
    pub partial_content: String,
    pub recovery_used: RecoveryStrategy,
}

impl Default for ErrorRecoveryParser {
    fn default() -> Self {
        Self {
            max_recovery_attempts: 5,
            recovery_strategies: vec![
                RecoveryStrategy::CreateErrorNode,
                RecoveryStrategy::ParseAsExpression,
                RecoveryStrategy::SkipToStatementEnd,
                RecoveryStrategy::SkipLine,
                RecoveryStrategy::SkipBlock,
            ],
            collect_errors: true,
            errors: Vec::new(),
        }
    }
}

impl ErrorRecoveryParser {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_strategies(mut self, strategies: Vec<RecoveryStrategy>) -> Self {
        self.recovery_strategies = strategies;
        self
    }

    pub fn with_max_attempts(mut self, max: usize) -> Self {
        self.max_recovery_attempts = max;
        self
    }

    /// Parse with error recovery
    pub fn parse(&mut self, input: &str) -> Result<AstNode, ParseError> {
        self.errors.clear();

        // First, try normal parsing
        match self.try_full_parse(input) {
            Ok(ast) => Ok(ast),
            Err(_) => {
                // Fall back to recovery parsing
                self.parse_with_recovery(input)
            }
        }
    }

    /// Get collected errors
    pub fn errors(&self) -> &[ErrorNode] {
        &self.errors
    }

    fn try_full_parse(&self, input: &str) -> Result<AstNode, ParseError> {
        let pairs = PerlParser::parse(Rule::program, input).map_err(|_| ParseError::ParseFailed)?;

        let mut parser = crate::pure_rust_parser::PureRustPerlParser::new();
        for pair in pairs {
            if let Ok(Some(node)) = parser.build_node(pair) {
                return Ok(node);
            }
        }

        Err(ParseError::ParseFailed)
    }

    fn parse_with_recovery(&mut self, input: &str) -> Result<AstNode, ParseError> {
        let mut statements = Vec::new();
        let mut position = 0;
        let mut line = 1;
        let mut recovery_attempts = 0;

        while position < input.len() && recovery_attempts < self.max_recovery_attempts {
            // Try to parse from current position
            let remaining = &input[position..];

            match self.try_parse_statement(remaining) {
                Ok((stmt, consumed)) => {
                    statements.push(stmt);
                    position += consumed;
                    recovery_attempts = 0; // Reset on success
                }
                Err(_) => {
                    // Try recovery strategies
                    if let Some((recovery_pos, error_node)) =
                        self.recover_from_error(input, position, line)
                    {
                        if self.collect_errors {
                            self.errors.push(error_node);
                        }

                        // Create error AST node
                        let error_content = input[position..recovery_pos].to_string();
                        statements.push(AstNode::ErrorNode {
                            message: Arc::from("Parse error"),
                            content: Arc::from(error_content.as_str()),
                        });

                        position = recovery_pos;
                        recovery_attempts += 1;
                    } else {
                        // No recovery possible, give up
                        break;
                    }
                }
            }

            // Update line count
            line += input[position..].chars().take_while(|&c| c == '\n').count();
        }

        if statements.is_empty() && recovery_attempts >= self.max_recovery_attempts {
            Err(ParseError::ParseFailed)
        } else {
            Ok(AstNode::Program(statements))
        }
    }

    fn try_parse_statement(&self, input: &str) -> Result<(AstNode, usize), ParseError> {
        // Try to parse various statement types
        let statement_rules = [
            Rule::statement,
            Rule::expression_statement,
            Rule::declaration_statement,
            Rule::block_statement,
        ];

        for rule in &statement_rules {
            if let Ok(pairs) = PerlParser::parse(*rule, input) {
                let mut parser = crate::pure_rust_parser::PureRustPerlParser::new();
                for pair in pairs {
                    let consumed = pair.as_span().end();
                    if let Ok(Some(node)) = parser.build_node(pair) {
                        return Ok((node, consumed));
                    }
                }
            }
        }

        Err(ParseError::ParseFailed)
    }

    fn recover_from_error(
        &self,
        input: &str,
        position: usize,
        line: usize,
    ) -> Option<(usize, ErrorNode)> {
        for strategy in &self.recovery_strategies {
            if let Some(recovery_pos) = self.apply_recovery_strategy(input, position, *strategy) {
                let error_node = ErrorNode {
                    message: "Syntax error".to_string(),
                    line,
                    column: self.calculate_column(input, position),
                    span: (position, recovery_pos),
                    partial_content: input[position..recovery_pos.min(position + 50)].to_string(),
                    recovery_used: *strategy,
                };

                return Some((recovery_pos, error_node));
            }
        }

        None
    }

    fn apply_recovery_strategy(
        &self,
        input: &str,
        position: usize,
        strategy: RecoveryStrategy,
    ) -> Option<usize> {
        let remaining = &input[position..];

        match strategy {
            RecoveryStrategy::SkipToStatementEnd => {
                // Find next semicolon or closing brace
                for (i, ch) in remaining.char_indices() {
                    if ch == ';' || ch == '}' {
                        return Some(position + i + 1);
                    }
                }
                None
            }

            RecoveryStrategy::SkipLine => {
                // Find next newline
                if let Some(newline_pos) = remaining.find('\n') {
                    Some(position + newline_pos + 1)
                } else {
                    Some(input.len())
                }
            }

            RecoveryStrategy::SkipBlock => {
                // Skip to matching closing brace
                let mut brace_count = 0;
                let mut in_string = false;
                let mut escape = false;

                for (i, ch) in remaining.char_indices() {
                    if escape {
                        escape = false;
                        continue;
                    }

                    match ch {
                        '\\' => escape = true,
                        '"' => in_string = !in_string,
                        '{' if !in_string => brace_count += 1,
                        '}' if !in_string => {
                            if brace_count == 0 {
                                return Some(position + i + 1);
                            }
                            brace_count -= 1;
                        }
                        _ => {}
                    }
                }
                None
            }

            RecoveryStrategy::ParseAsExpression => {
                // Try to parse as a simple expression
                if let Ok(pairs) = PerlParser::parse(Rule::expression, remaining)
                    && let Some(pair) = pairs.into_iter().next()
                {
                    return Some(position + pair.as_span().end());
                }
                None
            }

            RecoveryStrategy::CreateErrorNode => {
                // Just skip one token/word
                let mut chars = remaining.chars();
                let mut consumed = 0;

                // Skip whitespace
                while let Some(ch) = chars.next() {
                    consumed += ch.len_utf8();
                    if !ch.is_whitespace() {
                        // Skip non-whitespace
                        for ch in chars.by_ref() {
                            consumed += ch.len_utf8();
                            if ch.is_whitespace() {
                                break;
                            }
                        }
                        break;
                    }
                }

                if consumed > 0 { Some(position + consumed) } else { None }
            }
        }
    }

    fn calculate_column(&self, input: &str, position: usize) -> usize {
        let line_start = input[..position].rfind('\n').map(|p| p + 1).unwrap_or(0);
        position - line_start + 1
    }
}

// Add ErrorNode variant to AstNode enum
// This would need to be added to the actual AstNode definition:
// ErrorNode {
//     message: Arc<str>,
//     content: Arc<str>,
// }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_recovery_skip_to_semicolon() {
        let mut parser = ErrorRecoveryParser::new();
        let input = "my $x = ; print 'hello';";

        let result = parser.parse(input);
        assert!(result.is_ok());
        assert!(!parser.errors().is_empty());
    }

    #[test]
    fn test_recovery_skip_line() {
        let mut parser =
            ErrorRecoveryParser::new().with_strategies(vec![RecoveryStrategy::SkipLine]);

        let input = "invalid perl code here\nmy $x = 42;";
        let result = parser.parse(input);

        assert!(result.is_ok());
        assert_eq!(parser.errors().len(), 1);
    }

    #[test]
    fn test_recovery_skip_block() {
        let mut parser = ErrorRecoveryParser::new();
        let input = "if ($x { invalid } print 'after';";

        let result = parser.parse(input);
        assert!(result.is_ok());
    }

    #[test]
    fn test_recovery_strategies() {
        let parser = ErrorRecoveryParser::new();

        // Test skip to statement end
        let pos = parser.apply_recovery_strategy(
            "error here; next;",
            0,
            RecoveryStrategy::SkipToStatementEnd,
        );
        assert_eq!(pos, Some(11)); // Position after semicolon

        // Test skip line
        let pos = parser.apply_recovery_strategy("error\nnext line", 0, RecoveryStrategy::SkipLine);
        assert_eq!(pos, Some(6)); // Position after newline
    }
}
