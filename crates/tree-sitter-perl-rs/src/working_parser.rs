//! Working parser implementation that correctly integrates with perl-lexer
//!
//! This parser demonstrates the successful integration of the perl-lexer
//! with a recursive descent parser to produce tree-sitter compatible output.

use crate::ast::{Node, NodeKind, SourceLocation};
use crate::perl_lexer::{PerlLexer, TokenType as PLTokenType};
use crate::token_compat::{Token, TokenType, from_perl_lexer_token};
use std::sync::Arc;

pub struct WorkingParser {
    tokens: Vec<Token>,
    current: usize,
}

impl WorkingParser {
    pub fn new(input: &str) -> Self {
        let mut lexer = PerlLexer::new(input);
        let mut tokens = Vec::new();

        while let Some(perl_token) = lexer.next_token() {
            if matches!(perl_token.token_type, PLTokenType::EOF) {
                break;
            }
            let token = from_perl_lexer_token(&perl_token);
            tokens.push(token);
        }

        WorkingParser { tokens, current: 0 }
    }

    pub fn parse(&mut self) -> Node {
        match self.parse_program() {
            Ok(node) => node,
            Err(msg) => Node::new(
                NodeKind::Error { message: Arc::from(msg) },
                SourceLocation { start: 0, end: 0 },
            ),
        }
    }

    fn parse_program(&mut self) -> Result<Node, String> {
        let start = self.current_position();
        let mut statements = Vec::new();

        while !self.is_at_end() {
            match self.parse_statement() {
                Ok(stmt) => statements.push(stmt),
                Err(e) => return Err(e),
            }
        }

        let end = self.previous_position();
        Ok(Node::new(NodeKind::Program { statements }, SourceLocation { start, end }))
    }

    fn parse_statement(&mut self) -> Result<Node, String> {
        // Skip whitespace
        while self.match_token(&TokenType::Whitespace) {
            // Skip
        }

        if self.is_at_end() {
            return Err("Unexpected end of input".to_string());
        }

        // Variable declarations
        if self.match_keyword("my") {
            let decl = self.parse_variable_declaration()?;
            // Consume semicolon if present
            self.match_token(&TokenType::Semicolon);
            return Ok(decl);
        }

        // Control flow
        if self.match_keyword("if") {
            let stmt = self.parse_if_statement()?;
            // No semicolon needed for control structures
            return Ok(stmt);
        }

        // Function definition
        if self.match_keyword("sub") {
            let func = self.parse_function()?;
            // No semicolon needed for function definitions
            return Ok(func);
        }

        // Expression statement
        let expr = self.parse_expression()?;

        // Consume semicolon if present
        self.match_token(&TokenType::Semicolon);

        Ok(expr)
    }

    fn parse_variable_declaration(&mut self) -> Result<Node, String> {
        let start = self.previous_position();

        // Parse variable
        let var = self.parse_primary()?;

        // Check for assignment
        if self.match_token(&TokenType::Equal) {
            let value = self.parse_expression()?;
            let end = self.previous_position();

            return Ok(Node::new(
                NodeKind::Assignment {
                    left: Box::new(var.clone()),
                    op: TokenType::Equal,
                    right: Box::new(value),
                },
                SourceLocation { start, end },
            ));
        }

        let end = self.previous_position();
        Ok(Node::new(
            NodeKind::VariableDeclaration { declarator: Arc::from("my"), variables: vec![var] },
            SourceLocation { start, end },
        ))
    }

    fn parse_if_statement(&mut self) -> Result<Node, String> {
        let start = self.previous_position();

        // Consume opening paren
        self.consume(&TokenType::LeftParen, "Expected '(' after 'if'")?;

        // Parse condition
        let condition = self.parse_expression()?;

        // Consume closing paren
        self.consume(&TokenType::RightParen, "Expected ')' after condition")?;

        // Parse then block
        let then_block = self.parse_block()?;

        // Check for else
        let else_block =
            if self.match_keyword("else") { Some(Box::new(self.parse_block()?)) } else { None };

        let end = self.previous_position();
        Ok(Node::new(
            NodeKind::IfStatement {
                condition: Box::new(condition),
                then_branch: Box::new(then_block),
                elsif_branches: vec![],
                else_branch: else_block,
            },
            SourceLocation { start, end },
        ))
    }

    fn parse_function(&mut self) -> Result<Node, String> {
        let start = self.previous_position();

        // Parse function name
        let name = if self.check_token(&TokenType::Identifier) {
            let token = self.advance();
            Some(token.text.clone())
        } else {
            None
        };

        // Parse body
        let body = self.parse_block()?;

        let end = self.previous_position();
        Ok(Node::new(
            NodeKind::Subroutine {
                name,
                prototype: None,
                attributes: vec![],
                body: Box::new(body),
            },
            SourceLocation { start, end },
        ))
    }

    fn parse_block(&mut self) -> Result<Node, String> {
        self.consume(&TokenType::LeftBrace, "Expected '{'")?;
        let start = self.previous_position();

        let mut statements = Vec::new();

        while !self.check_token(&TokenType::RightBrace) && !self.is_at_end() {
            statements.push(self.parse_statement()?);
        }

        self.consume(&TokenType::RightBrace, "Expected '}'")?;
        let end = self.previous_position();

        Ok(Node::new(NodeKind::Block { statements }, SourceLocation { start, end }))
    }

    fn parse_expression(&mut self) -> Result<Node, String> {
        self.parse_assignment()
    }

    fn parse_assignment(&mut self) -> Result<Node, String> {
        let expr = self.parse_binary()?;

        if self.match_token(&TokenType::Equal) {
            let start = expr.location.start;
            let value = self.parse_assignment()?;
            let end = self.previous_position();

            return Ok(Node::new(
                NodeKind::Assignment {
                    left: Box::new(expr),
                    op: TokenType::Equal,
                    right: Box::new(value),
                },
                SourceLocation { start, end },
            ));
        }

        Ok(expr)
    }

    fn parse_binary(&mut self) -> Result<Node, String> {
        let mut expr = self.parse_unary()?;

        while let Some(op) = self.match_binary_op() {
            let start = expr.location.start;
            let right = self.parse_unary()?;
            let end = self.previous_position();

            expr = Node::new(
                NodeKind::Binary { op, left: Box::new(expr), right: Box::new(right) },
                SourceLocation { start, end },
            );
        }

        Ok(expr)
    }

    fn parse_unary(&mut self) -> Result<Node, String> {
        if self.match_token(&TokenType::Minus) || self.match_token(&TokenType::Not) {
            let op = self.previous().token_type.clone();
            let start = self.previous_position();
            let operand = self.parse_unary()?;
            let end = self.previous_position();

            return Ok(Node::new(
                NodeKind::Unary { op, operand: Box::new(operand) },
                SourceLocation { start, end },
            ));
        }

        self.parse_primary()
    }

    fn parse_primary(&mut self) -> Result<Node, String> {
        // Skip whitespace
        while self.match_token(&TokenType::Whitespace) {
            // Skip
        }

        // Numbers
        if self.check_token(&TokenType::Number) {
            let token = self.advance();
            let value = token.text.clone();
            return Ok(Node::new(
                NodeKind::Number { value },
                SourceLocation { start: token.start, end: token.end },
            ));
        }

        // Strings
        if self.check_token(&TokenType::SingleQuotedString)
            || self.check_token(&TokenType::DoubleQuotedString)
            || self.check_token(&TokenType::BacktickString)
        {
            let token = self.advance();
            let value = token.text.clone();
            return Ok(Node::new(
                NodeKind::String { value },
                SourceLocation { start: token.start, end: token.end },
            ));
        }

        // Variables
        if self.check_token(&TokenType::ScalarVariable)
            || self.check_token(&TokenType::ArrayVariable)
            || self.check_token(&TokenType::HashVariable)
        {
            let token = self.advance();
            let text = token.text.clone();

            // Variable node includes the sigil in the name
            return Ok(Node::new(
                NodeKind::Variable { name: text },
                SourceLocation { start: token.start, end: token.end },
            ));
        }

        // Identifiers (barewords)
        if self.check_token(&TokenType::Identifier) {
            let token = self.advance();
            let value = token.text.clone();
            return Ok(Node::new(
                NodeKind::Bareword { value },
                SourceLocation { start: token.start, end: token.end },
            ));
        }

        // Parenthesized expression
        if self.match_token(&TokenType::LeftParen) {
            let expr = self.parse_expression()?;
            self.consume(&TokenType::RightParen, "Expected ')'")?;
            return Ok(expr);
        }

        Err(format!("Unexpected token: {:?}", self.peek()))
    }

    // Helper methods
    fn match_binary_op(&mut self) -> Option<TokenType> {
        for op in &[
            TokenType::Plus,
            TokenType::Minus,
            TokenType::Star,
            TokenType::Slash,
            TokenType::Equal,
            TokenType::NotEqual,
            TokenType::Less,
            TokenType::Greater,
            TokenType::LessEqual,
            TokenType::GreaterEqual,
            TokenType::AndAnd,
            TokenType::OrOr,
        ] {
            if self.match_token(op) {
                return Some(op.clone());
            }
        }
        None
    }

    fn match_token(&mut self, token_type: &TokenType) -> bool {
        if self.check_token(token_type) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn match_keyword(&mut self, keyword: &str) -> bool {
        if self.is_at_end() {
            return false;
        }

        // Check if current token is an identifier matching the keyword
        match &self.peek().token_type {
            TokenType::Identifier => {
                if self.peek().text.as_ref() == keyword {
                    self.advance();
                    true
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    fn check_token(&self, token_type: &TokenType) -> bool {
        if self.is_at_end() { false } else { &self.peek().token_type == token_type }
    }

    fn advance(&mut self) -> &Token {
        if !self.is_at_end() {
            self.current += 1;
        }
        self.previous()
    }

    fn is_at_end(&self) -> bool {
        self.current >= self.tokens.len()
    }

    fn peek(&self) -> &Token {
        &self.tokens[self.current]
    }

    fn previous(&self) -> &Token {
        &self.tokens[self.current.saturating_sub(1)]
    }

    fn current_position(&self) -> usize {
        if self.is_at_end() {
            self.tokens.last().map(|t| t.end).unwrap_or(0)
        } else {
            self.peek().start
        }
    }

    fn previous_position(&self) -> usize {
        self.previous().end
    }

    fn consume(&mut self, token_type: &TokenType, message: &str) -> Result<&Token, String> {
        if self.check_token(token_type) { Ok(self.advance()) } else { Err(message.to_string()) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_parsing() {
        let input = "my $x = 42;";
        let mut parser = WorkingParser::new(input);
        let ast = parser.parse();

        println!("Input: {}", input);
        println!("AST: {:?}", ast);
        println!("S-expression: {}", ast.to_sexp());

        assert!(!matches!(ast.kind, NodeKind::Error { .. }));
    }

    #[test]
    fn test_if_statement() {
        let input = "if ($x > 10) { print $x; }";
        let mut parser = WorkingParser::new(input);
        let ast = parser.parse();

        println!("S-expression: {}", ast.to_sexp());
        assert!(!matches!(ast.kind, NodeKind::Error { .. }));
    }

    #[test]
    fn test_function() {
        let input = "sub hello { print \"Hello\"; }";
        let mut parser = WorkingParser::new(input);
        let ast = parser.parse();

        println!("S-expression: {}", ast.to_sexp());
        assert!(!matches!(ast.kind, NodeKind::Error { .. }));
    }
}
