//! Recovery-aware parser implementation
//!
//! This module provides a parser that can recover from syntax errors
//! and continue parsing, producing partial ASTs with error nodes.

use crate::{
    ast_v2::{Node, NodeKind},
    error_recovery::{
        ErrorRecovery, ParseError, ParserErrorRecovery, StatementRecovery, SyncPoint,
    },
    parser_context::ParserContext,
    position::Range,
};
use perl_lexer::TokenType;

/// A parser with error recovery capabilities
pub struct RecoveryParser {
    context: ParserContext,
}

impl RecoveryParser {
    /// Create a new recovery parser
    pub fn new(source: String) -> Self {
        RecoveryParser { context: ParserContext::new(source) }
    }

    /// Parse with error recovery, returning AST and errors
    pub fn parse(mut self) -> (Node, Vec<ParseError>) {
        let root = self.parse_program();
        let errors = self.context.take_errors();
        (root, errors)
    }

    /// Parse a program with recovery
    fn parse_program(&mut self) -> Node {
        let start_pos = self.context.current_position();
        let mut statements = Vec::new();

        while !self.context.is_eof() {
            statements.push(self.parse_statement_with_recovery());

            // Try to consume optional semicolon
            self.context.consume(&TokenType::Semicolon);
        }

        let end_pos = self.context.current_position();
        Node::new(
            self.context.id_generator.next_id(),
            NodeKind::Program { statements },
            Range::new(start_pos, end_pos),
        )
    }
}

impl ParserErrorRecovery for RecoveryParser {
    fn parse_with_recovery(&mut self) -> (Node, Vec<ParseError>) {
        let root = self.parse_program();
        let errors = self.context.take_errors();
        (root, errors)
    }

    fn try_parse<F>(&mut self, parse_fn: F) -> Node
    where
        F: FnOnce(&mut Self) -> Option<Node>,
    {
        // Save current position
        let saved_pos = self.context.current_index();

        // Try to parse
        match parse_fn(self) {
            Some(node) => node,
            None => {
                // Restore position and create error node
                self.context.set_index(saved_pos);

                let error = ParseError::new(
                    "Failed to parse construct".to_string(),
                    self.context.current_position_range(),
                );

                self.context.recover_with_node(error)
            }
        }
    }

    fn parse_list_with_recovery<F>(
        &mut self,
        parse_element: F,
        separator: TokenType,
        terminator: TokenType,
    ) -> Vec<Node>
    where
        F: Fn(&mut Self) -> Node,
    {
        let mut elements = Vec::new();

        // Check for empty list
        if self.context.check(&terminator) {
            return elements;
        }

        loop {
            // Parse element with recovery
            elements.push(parse_element(self));

            // Check for separator or terminator
            if self.context.check(&terminator) {
                break;
            }

            if self.context.check(&separator) {
                self.context.advance();

                // Check for trailing separator
                if self.context.check(&terminator) {
                    break;
                }
            } else {
                // Missing separator - recover
                let error = ParseError::new(
                    format!("Expected {:?} or {:?}", separator, terminator),
                    self.context.current_position_range(),
                )
                .with_expected(vec![format!("{:?}", separator), format!("{:?}", terminator)]);

                self.context.add_error(error);

                // Try to continue
                if !self.context.is_sync_point(SyncPoint::Semicolon)
                    && !self.context.is_sync_point(SyncPoint::CloseBrace)
                {
                    // Skip one token and continue
                    self.context.advance();
                } else {
                    break;
                }
            }
        }

        elements
    }
}

impl StatementRecovery for RecoveryParser {
    fn parse_statement_with_recovery(&mut self) -> Node {
        // Try to parse a statement
        match self.try_parse_statement() {
            Ok(node) => node,
            Err(error) => {
                // Create error node and recover
                self.context.recover_with_node(error)
            }
        }
    }

    fn parse_expression_with_recovery(&mut self) -> Node {
        // Try to parse an expression
        match self.try_parse_expression() {
            Ok(node) => node,
            Err(error) => {
                // Create error node with partial expression
                self.context.recover_with_node(error)
            }
        }
    }

    fn parse_block_with_recovery(&mut self) -> Node {
        let start_pos = self.context.current_position();

        // Expect opening brace
        if let Err(error) = self.context.expect(TokenType::LeftBrace) {
            self.context.add_error(error);
            // Try to continue anyway
        }

        let mut statements = Vec::new();

        // Parse statements until closing brace or EOF
        while !self.context.check(&TokenType::RightBrace) && !self.context.is_eof() {
            statements.push(self.parse_statement_with_recovery());

            // Optional semicolon
            self.context.consume(&TokenType::Semicolon);
        }

        // Expect closing brace
        if let Err(error) = self.context.expect(TokenType::RightBrace) {
            self.context.add_error(error);
            // Continue anyway - we've parsed the content
        }

        let end_pos = self.context.current_position();
        Node::new(
            self.context.id_generator.next_id(),
            NodeKind::Block { statements },
            Range::new(start_pos, end_pos),
        )
    }
}

// Core parsing methods (simplified for demonstration)
impl RecoveryParser {
    /// Try to parse a statement
    fn try_parse_statement(&mut self) -> Result<Node, ParseError> {
        match self.context.current_token() {
            Some(token) => match &token.token.token_type {
                TokenType::Keyword(kw) => match kw.as_ref() {
                    "my" | "our" | "local" | "state" => self.parse_variable_declaration(),
                    "if" => self.parse_if_statement(),
                    "while" => self.parse_while_statement(),
                    "sub" => self.parse_subroutine(),
                    _ => self.try_parse_expression_statement(),
                },
                _ => self.try_parse_expression_statement(),
            },
            None => Err(ParseError::new(
                "Unexpected end of file".to_string(),
                self.context.current_position_range(),
            )),
        }
    }

    /// Parse variable declaration
    fn parse_variable_declaration(&mut self) -> Result<Node, ParseError> {
        let start_pos = self.context.current_position();

        // Consume declarator (my, our, etc.)
        let declarator = match self.context.current_token() {
            Some(token) => match &token.token.token_type {
                TokenType::Keyword(kw) => kw.to_string(),
                _ => {
                    return Err(ParseError::new(
                        "Expected declarator keyword".to_string(),
                        token.range(),
                    ));
                }
            },
            None => {
                return Err(ParseError::new(
                    "Expected declarator keyword".to_string(),
                    self.context.current_position_range(),
                ));
            }
        };
        self.context.advance();

        // Parse variable
        let variable = self.parse_variable()?;

        // Optional initializer
        let initializer = if self.context.consume(&TokenType::Operator("=".into())) {
            Some(Box::new(self.parse_expression_with_recovery()))
        } else {
            None
        };

        let end_pos = self.context.current_position();
        Ok(Node::new(
            self.context.id_generator.next_id(),
            NodeKind::VariableDeclaration {
                declarator,
                variable: Box::new(variable),
                attributes: Vec::new(),
                initializer,
            },
            Range::new(start_pos, end_pos),
        ))
    }

    /// Parse a variable
    fn parse_variable(&mut self) -> Result<Node, ParseError> {
        match self.context.current_token() {
            Some(token) => match &token.token.token_type {
                TokenType::Identifier(name) => {
                    // Check if it looks like a variable (starts with sigil)
                    if let Some(sigil) = name.chars().next() {
                        if matches!(sigil, '$' | '@' | '%' | '*' | '&') {
                            let var_name = name[1..].to_string();
                            let range = token.range();
                            self.context.advance();
                            let node = Node::new(
                                self.context.id_generator.next_id(),
                                NodeKind::Variable { sigil: sigil.to_string(), name: var_name },
                                range,
                            );
                            return Ok(node);
                        }
                    }
                    Err(ParseError::new("Expected variable".to_string(), token.range())
                        .with_expected(vec!["variable".to_string()]))
                }
                _ => Err(ParseError::new("Expected variable".to_string(), token.range())
                    .with_expected(vec!["variable".to_string()])),
            },
            None => Err(ParseError::new(
                "Expected variable".to_string(),
                self.context.current_position_range(),
            )
            .with_expected(vec!["variable".to_string()])),
        }
    }

    /// Try to parse expression
    fn try_parse_expression(&mut self) -> Result<Node, ParseError> {
        // Simplified expression parsing
        match self.context.current_token() {
            Some(token) => match &token.token.token_type {
                TokenType::Number(n) => {
                    let value = n.to_string();
                    let range = token.range();
                    self.context.advance();
                    let node = Node::new(
                        self.context.id_generator.next_id(),
                        NodeKind::Number { value },
                        range,
                    );
                    Ok(node)
                }
                TokenType::StringLiteral => {
                    let value = token.token.text.to_string();
                    let range = token.range();
                    self.context.advance();
                    let node = Node::new(
                        self.context.id_generator.next_id(),
                        NodeKind::String { value, interpolated: false },
                        range,
                    );
                    Ok(node)
                }
                TokenType::Identifier(_) => self.parse_variable(),
                _ => Err(ParseError::new("Expected expression".to_string(), token.range())),
            },
            None => Err(ParseError::new(
                "Expected expression".to_string(),
                self.context.current_position_range(),
            )),
        }
    }

    /// Parse expression statement
    fn try_parse_expression_statement(&mut self) -> Result<Node, ParseError> {
        self.try_parse_expression()
    }

    /// Parse if statement (simplified)
    fn parse_if_statement(&mut self) -> Result<Node, ParseError> {
        let start_pos = self.context.current_position();

        // Consume 'if'
        self.context.expect(TokenType::Keyword("if".into()))?;

        // Parse condition
        let condition = Box::new(self.parse_expression_with_recovery());

        // Parse then block
        let then_branch = Box::new(self.parse_block_with_recovery());

        let end_pos = self.context.current_position();
        Ok(Node::new(
            self.context.id_generator.next_id(),
            NodeKind::If { condition, then_branch, elsif_branches: Vec::new(), else_branch: None },
            Range::new(start_pos, end_pos),
        ))
    }

    /// Parse while statement (simplified)
    fn parse_while_statement(&mut self) -> Result<Node, ParseError> {
        let start_pos = self.context.current_position();

        // Consume 'while'
        self.context.expect(TokenType::Keyword("while".into()))?;

        // For now, create a simple identifier node
        let end_pos = self.context.current_position();
        Ok(Node::new(
            self.context.id_generator.next_id(),
            NodeKind::Identifier { name: "while_stmt".to_string() },
            Range::new(start_pos, end_pos),
        ))
    }

    /// Parse subroutine (simplified)
    fn parse_subroutine(&mut self) -> Result<Node, ParseError> {
        let start_pos = self.context.current_position();

        // Consume 'sub'
        self.context.expect(TokenType::Keyword("sub".into()))?;

        // For now, create a simple identifier node
        let end_pos = self.context.current_position();
        Ok(Node::new(
            self.context.id_generator.next_id(),
            NodeKind::Identifier { name: "sub_decl".to_string() },
            Range::new(start_pos, end_pos),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_with_errors() {
        let source = "my $x = ; my $y = 42".to_string();
        let parser = RecoveryParser::new(source);
        let (ast, errors) = parser.parse();

        // Should have parsed both declarations despite error
        match &ast.kind {
            NodeKind::Program { statements } => {
                assert_eq!(statements.len(), 2);
            }
            _ => unreachable!("Expected program node"),
        }

        // Should have recorded at least one error
        assert!(!errors.is_empty());
    }

    #[test]
    fn test_missing_semicolon_recovery() {
        let source = "my $x = 42 my $y = 99".to_string();
        let parser = RecoveryParser::new(source);
        let (ast, _errors) = parser.parse();

        // Should parse both statements
        match &ast.kind {
            NodeKind::Program { statements } => {
                assert_eq!(statements.len(), 2);
            }
            _ => unreachable!("Expected program node"),
        }
    }

    #[test]
    fn test_unclosed_block_recovery() {
        let source = "if $x { my $y = 42".to_string();
        let parser = RecoveryParser::new(source);
        let (ast, errors) = parser.parse();

        // Should create if statement with block
        match &ast.kind {
            NodeKind::Program { statements } => {
                assert_eq!(statements.len(), 1);
                match &statements[0].kind {
                    NodeKind::If { then_branch, .. } => match &then_branch.kind {
                        NodeKind::Block { statements } => {
                            assert_eq!(statements.len(), 1);
                        }
                        _ => unreachable!("Expected block"),
                    },
                    _ => unreachable!("Expected if statement"),
                }
            }
            _ => unreachable!("Expected program node"),
        }

        // Should have error about missing brace
        assert!(!errors.is_empty());
    }
}
