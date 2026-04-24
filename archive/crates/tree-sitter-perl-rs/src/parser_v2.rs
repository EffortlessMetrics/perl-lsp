//! Parser V2 - Fixed borrowing issues and complete implementation
//!
//! This is a complete recursive descent parser for Perl that properly
//! handles borrowing and implements full expression parsing.

use crate::ast::{Node, NodeKind, SourceLocation};
use crate::error::{ParseError, ParseErrorKind};
use crate::perl_lexer::PerlLexer;
use crate::regex_parser::RegexParser;
use crate::token_compat::{Token, TokenType, from_perl_lexer_token};
use std::sync::Arc;

/// Perl parser with proper memory management
pub struct ParserV2<'a> {
    /// Token stream
    tokens: Vec<Token>,
    /// Current position
    current: usize,
    /// Source code for error reporting
    source: &'a str,
    /// Accumulated errors
    errors: Vec<ParseError>,
}

impl<'a> ParserV2<'a> {
    /// Create a new parser
    pub fn new(source: &'a str) -> Self {
        let mut lexer = PerlLexer::new(source);
        let mut tokens = Vec::new();

        // Tokenize entire input
        while let Some(perl_token) = lexer.next_token() {
            let token = from_perl_lexer_token(&perl_token);
            // Keep all tokens including whitespace for accurate positions
            tokens.push(token);
        }

        Self { tokens, current: 0, source, errors: Vec::new() }
    }

    /// Parse the source into an AST
    pub fn parse(&mut self) -> Result<Node, ParseError> {
        self.skip_whitespace();
        let program = self.parse_program()?;

        if !self.errors.is_empty() {
            // Return first error for now
            return Err(self.errors[0].clone());
        }

        Ok(program)
    }

    /// Get parse errors
    pub fn errors(&self) -> &[ParseError] {
        &self.errors
    }

    // Core parsing methods

    fn parse_program(&mut self) -> Result<Node, ParseError> {
        let start = self.current_pos();
        let mut statements = Vec::new();

        while !self.is_at_end() {
            self.skip_whitespace();

            if self.is_at_end() {
                break;
            }

            match self.parse_statement() {
                Ok(stmt) => statements.push(stmt),
                Err(e) => {
                    self.errors.push(e);
                    self.synchronize();
                }
            }
        }

        let end = self.current_pos();
        Ok(Node::new(NodeKind::Program { statements }, SourceLocation { start, end }))
    }

    fn parse_statement(&mut self) -> Result<Node, ParseError> {
        self.skip_whitespace();

        // Package declaration
        if self.check_keyword("package") {
            return self.parse_package_declaration();
        }

        // Use statement
        if self.check_keyword("use") {
            return self.parse_use_statement();
        }

        // Subroutine
        if self.check_keyword("sub") {
            return self.parse_subroutine();
        }

        // Control flow
        if self.check_keyword("if") {
            return self.parse_if_statement();
        }

        if self.check_keyword("unless") {
            return self.parse_unless_statement();
        }

        if self.check_keyword("while") {
            return self.parse_while_statement();
        }

        if self.check_keyword("until") {
            return self.parse_until_statement();
        }

        if self.check_keyword("for") || self.check_keyword("foreach") {
            return self.parse_for_statement();
        }

        // Variable declaration
        if self.check_keyword("my")
            || self.check_keyword("our")
            || self.check_keyword("local")
            || self.check_keyword("state")
        {
            return self.parse_variable_declaration();
        }

        // Return statement
        if self.check_keyword("return") {
            return self.parse_return_statement();
        }

        // Last, next, redo
        if self.check_keyword("last") || self.check_keyword("next") || self.check_keyword("redo") {
            return self.parse_loop_control();
        }

        // Default: expression statement
        let expr = self.parse_expression()?;

        // Optional semicolon
        self.skip_whitespace();
        if self.check(&TokenType::Semicolon) {
            self.advance();
        }

        Ok(expr)
    }

    // Package and module parsing

    fn parse_package_declaration(&mut self) -> Result<Node, ParseError> {
        let start = self.current_pos();
        self.advance(); // consume 'package'
        self.skip_whitespace();

        let name = self.expect_package_name()?;
        self.skip_whitespace();

        let version = if self.check_number() { Some(self.advance().text.clone()) } else { None };

        self.skip_whitespace();

        let block = if self.check(&TokenType::LeftBrace) {
            Some(Box::new(self.parse_block()?))
        } else {
            self.consume(&TokenType::Semicolon)?;
            None
        };

        let end = self.current_pos();
        Ok(Node::new(
            NodeKind::PackageDeclaration { name, version, block },
            SourceLocation { start, end },
        ))
    }

    fn parse_use_statement(&mut self) -> Result<Node, ParseError> {
        let start = self.current_pos();
        self.advance(); // consume 'use'
        self.skip_whitespace();

        let module = self.expect_package_name()?;
        self.skip_whitespace();

        let version = if self.check_number() { Some(self.advance().text.clone()) } else { None };

        self.skip_whitespace();

        let imports = if self.check(&TokenType::LeftParen) || self.check_keyword("qw") {
            Some(self.parse_import_list()?)
        } else {
            None
        };

        self.consume(&TokenType::Semicolon)?;

        let end = self.current_pos();
        Ok(Node::new(
            NodeKind::UseStatement { module, version, imports },
            SourceLocation { start, end },
        ))
    }

    // Subroutine parsing

    fn parse_subroutine(&mut self) -> Result<Node, ParseError> {
        let start = self.current_pos();
        self.advance(); // consume 'sub'
        self.skip_whitespace();

        let name = if self.check(&TokenType::Identifier) {
            Some(self.advance().text.clone())
        } else {
            None
        };

        self.skip_whitespace();

        // Prototype
        let prototype =
            if self.check(&TokenType::LeftParen) { Some(self.parse_prototype()?) } else { None };

        self.skip_whitespace();

        // Attributes
        let attributes =
            if self.check(&TokenType::Colon) { self.parse_attributes()? } else { Vec::new() };

        self.skip_whitespace();

        // Body
        let body = Box::new(self.parse_block()?);

        let end = self.current_pos();
        Ok(Node::new(
            NodeKind::Subroutine { name, prototype, attributes, body },
            SourceLocation { start, end },
        ))
    }

    // Control flow parsing

    fn parse_if_statement(&mut self) -> Result<Node, ParseError> {
        let start = self.current_pos();
        self.advance(); // consume 'if'
        self.skip_whitespace();

        self.consume(&TokenType::LeftParen)?;
        let condition = Box::new(self.parse_expression()?);
        self.consume(&TokenType::RightParen)?;

        let then_branch = Box::new(self.parse_block_or_statement()?);

        let mut elsif_branches = Vec::new();
        let mut else_branch = None;

        loop {
            self.skip_whitespace();

            if self.check_keyword("elsif") || self.check_keyword("elseif") {
                self.advance();
                self.skip_whitespace();

                self.consume(&TokenType::LeftParen)?;
                let elsif_cond = Box::new(self.parse_expression()?);
                self.consume(&TokenType::RightParen)?;

                let elsif_body = Box::new(self.parse_block_or_statement()?);
                elsif_branches.push((elsif_cond, elsif_body));
            } else if self.check_keyword("else") {
                self.advance();
                self.skip_whitespace();
                else_branch = Some(Box::new(self.parse_block_or_statement()?));
                break;
            } else {
                break;
            }
        }

        let end = self.current_pos();
        Ok(Node::new(
            NodeKind::IfStatement { condition, then_branch, elsif_branches, else_branch },
            SourceLocation { start, end },
        ))
    }

    fn parse_unless_statement(&mut self) -> Result<Node, ParseError> {
        let start = self.current_pos();
        self.advance(); // consume 'unless'
        self.skip_whitespace();

        self.consume(&TokenType::LeftParen)?;
        let condition = Box::new(self.parse_expression()?);
        self.consume(&TokenType::RightParen)?;

        let then_branch = Box::new(self.parse_block_or_statement()?);

        let else_branch = if self.check_keyword("else") {
            self.advance();
            self.skip_whitespace();
            Some(Box::new(self.parse_block_or_statement()?))
        } else {
            None
        };

        // Convert unless to if with negated condition
        let negated_condition = Box::new(Node::new(
            NodeKind::Unary { op: TokenType::Not, operand: condition },
            SourceLocation { start, end: self.current_pos() },
        ));

        let end = self.current_pos();
        Ok(Node::new(
            NodeKind::IfStatement {
                condition: negated_condition,
                then_branch,
                elsif_branches: Vec::new(),
                else_branch,
            },
            SourceLocation { start, end },
        ))
    }

    fn parse_while_statement(&mut self) -> Result<Node, ParseError> {
        let start = self.current_pos();
        self.advance(); // consume 'while'
        self.skip_whitespace();

        self.consume(&TokenType::LeftParen)?;
        let condition = Box::new(self.parse_expression()?);
        self.consume(&TokenType::RightParen)?;

        let body = Box::new(self.parse_block_or_statement()?);

        let continue_block = if self.check_keyword("continue") {
            self.advance();
            self.skip_whitespace();
            Some(Box::new(self.parse_block()?))
        } else {
            None
        };

        let end = self.current_pos();
        Ok(Node::new(
            NodeKind::WhileStatement { condition, body, continue_block },
            SourceLocation { start, end },
        ))
    }

    fn parse_until_statement(&mut self) -> Result<Node, ParseError> {
        let start = self.current_pos();
        self.advance(); // consume 'until'
        self.skip_whitespace();

        self.consume(&TokenType::LeftParen)?;
        let condition = Box::new(self.parse_expression()?);
        self.consume(&TokenType::RightParen)?;

        let body = Box::new(self.parse_block_or_statement()?);

        // Convert until to while with negated condition
        let negated_condition = Box::new(Node::new(
            NodeKind::Unary { op: TokenType::Not, operand: condition },
            SourceLocation { start, end: self.current_pos() },
        ));

        let end = self.current_pos();
        Ok(Node::new(
            NodeKind::WhileStatement { condition: negated_condition, body, continue_block: None },
            SourceLocation { start, end },
        ))
    }

    fn parse_for_statement(&mut self) -> Result<Node, ParseError> {
        let start = self.current_pos();
        let _is_foreach = self.peek_text() == "foreach";
        self.advance(); // consume 'for' or 'foreach'
        self.skip_whitespace();

        // Check for C-style for loop
        if self.check(&TokenType::LeftParen) {
            let saved_pos = self.current;
            self.advance();

            // Look for semicolons to detect C-style
            let is_c_style = self.scan_for_c_style();
            self.current = saved_pos;

            if is_c_style {
                return self.parse_c_style_for(start);
            }
        }

        // Perl-style foreach
        self.parse_foreach_statement(start)
    }

    fn parse_c_style_for(&mut self, start: usize) -> Result<Node, ParseError> {
        self.consume(&TokenType::LeftParen)?;

        let init = if !self.check(&TokenType::Semicolon) {
            Some(Box::new(self.parse_expression()?))
        } else {
            None
        };
        self.consume(&TokenType::Semicolon)?;

        let condition = if !self.check(&TokenType::Semicolon) {
            Some(Box::new(self.parse_expression()?))
        } else {
            None
        };
        self.consume(&TokenType::Semicolon)?;

        let update = if !self.check(&TokenType::RightParen) {
            Some(Box::new(self.parse_expression()?))
        } else {
            None
        };
        self.consume(&TokenType::RightParen)?;

        let body = Box::new(self.parse_block_or_statement()?);

        let end = self.current_pos();
        Ok(Node::new(
            NodeKind::ForStatement { init, condition, update, body },
            SourceLocation { start, end },
        ))
    }

    fn parse_foreach_statement(&mut self, start: usize) -> Result<Node, ParseError> {
        self.skip_whitespace();

        // Variable (optional - defaults to $_)
        let variable = if self.check_keyword("my") || self.check_keyword("our") {
            Box::new(self.parse_variable_declaration()?)
        } else if self.check_variable() {
            Box::new(self.parse_primary()?)
        } else {
            Box::new(Node::new(
                NodeKind::Variable { name: Arc::from("$_") },
                SourceLocation { start: self.current_pos(), end: self.current_pos() },
            ))
        };

        self.skip_whitespace();
        self.consume(&TokenType::LeftParen)?;
        let list = Box::new(self.parse_expression()?);
        self.consume(&TokenType::RightParen)?;

        let body = Box::new(self.parse_block_or_statement()?);

        let end = self.current_pos();
        Ok(Node::new(
            NodeKind::ForeachStatement { variable, list, body },
            SourceLocation { start, end },
        ))
    }

    // Variable declaration

    fn parse_variable_declaration(&mut self) -> Result<Node, ParseError> {
        let start = self.current_pos();
        let declarator = self.advance().text.clone();
        self.skip_whitespace();

        let mut variables = Vec::new();

        // List of variables in parens
        if self.check(&TokenType::LeftParen) {
            self.advance();
            self.skip_whitespace();

            while !self.check(&TokenType::RightParen) && !self.is_at_end() {
                if self.check_keyword("undef") {
                    self.advance();
                    variables.push(Node::new(
                        NodeKind::Bareword { value: Arc::from("undef") },
                        SourceLocation { start: self.current_pos(), end: self.current_pos() },
                    ));
                } else {
                    variables.push(self.parse_variable_or_field()?);
                }

                self.skip_whitespace();
                if !self.match_token(&TokenType::Comma) {
                    break;
                }
                self.skip_whitespace();
            }

            self.consume(&TokenType::RightParen)?;
        } else {
            // Single variable
            variables.push(self.parse_variable_or_field()?);
        }

        self.skip_whitespace();

        // Optional assignment
        if self.check(&TokenType::Equal) {
            self.advance();
            self.skip_whitespace();

            let value = self.parse_expression()?;

            // Consume optional trailing semicolon
            self.skip_whitespace();
            if self.check(&TokenType::Semicolon) {
                self.advance();
            }

            // If single variable, convert to assignment
            if variables.len() == 1 {
                let var =
                    variables.into_iter().next().ok_or(crate::error::ParseError::ParseFailed)?;
                return Ok(Node::new(
                    NodeKind::Assignment {
                        left: Box::new(var),
                        op: TokenType::Equal,
                        right: Box::new(value),
                    },
                    SourceLocation { start, end: self.current_pos() },
                ));
            } else {
                // List assignment
                let lhs = Node::new(
                    NodeKind::List { elements: variables },
                    SourceLocation { start, end: self.current_pos() },
                );
                return Ok(Node::new(
                    NodeKind::Assignment {
                        left: Box::new(lhs),
                        op: TokenType::Equal,
                        right: Box::new(value),
                    },
                    SourceLocation { start, end: self.current_pos() },
                ));
            }
        }

        // Consume optional trailing semicolon
        self.skip_whitespace();
        if self.check(&TokenType::Semicolon) {
            self.advance();
        }

        let end = self.current_pos();
        Ok(Node::new(
            NodeKind::VariableDeclaration { declarator, variables },
            SourceLocation { start, end },
        ))
    }

    // Expression parsing with precedence

    fn parse_expression(&mut self) -> Result<Node, ParseError> {
        self.parse_ternary()
    }

    fn parse_ternary(&mut self) -> Result<Node, ParseError> {
        let mut expr = self.parse_or()?;

        self.skip_whitespace();
        if self.match_token(&TokenType::Question) {
            self.skip_whitespace();
            let then_expr = Box::new(self.parse_expression()?);
            self.skip_whitespace();
            self.consume(&TokenType::Colon)?;
            self.skip_whitespace();
            let else_expr = Box::new(self.parse_expression()?);
            let start = expr.location.start;
            let end = else_expr.location.end;

            expr = Node::new(
                NodeKind::Ternary { condition: Box::new(expr), then_expr, else_expr },
                SourceLocation { start, end },
            );
        }

        Ok(expr)
    }

    fn parse_or(&mut self) -> Result<Node, ParseError> {
        let mut expr = self.parse_and()?;

        while self.match_any(&[TokenType::OrOr]) || self.match_keyword("or") {
            let _op_start = self.current_pos();
            let op = self.previous().token_type.clone();
            self.skip_whitespace();

            let right = self.parse_and()?;
            let loc = SourceLocation { start: expr.location.start, end: right.location.end };

            expr = Node::new(
                NodeKind::Binary { left: Box::new(expr), op, right: Box::new(right) },
                loc,
            );
        }

        Ok(expr)
    }

    fn parse_and(&mut self) -> Result<Node, ParseError> {
        let mut expr = self.parse_not()?;

        while self.match_any(&[TokenType::AndAnd]) || self.match_keyword("and") {
            let op = self.previous().token_type.clone();
            self.skip_whitespace();

            let right = self.parse_not()?;
            let loc = SourceLocation { start: expr.location.start, end: right.location.end };

            expr = Node::new(
                NodeKind::Binary { left: Box::new(expr), op, right: Box::new(right) },
                loc,
            );
        }

        Ok(expr)
    }

    fn parse_not(&mut self) -> Result<Node, ParseError> {
        self.skip_whitespace();

        if self.match_any(&[TokenType::Not]) || self.match_keyword("not") {
            let start = self.previous().start;
            let op = self.previous().token_type.clone();
            self.skip_whitespace();

            let operand = self.parse_not()?;
            let loc = SourceLocation { start, end: operand.location.end };

            return Ok(Node::new(NodeKind::Unary { op, operand: Box::new(operand) }, loc));
        }

        self.parse_relational()
    }

    fn parse_relational(&mut self) -> Result<Node, ParseError> {
        let mut expr = self.parse_shift()?;

        while self.match_any(&[
            TokenType::Less,
            TokenType::Greater,
            TokenType::LessEqual,
            TokenType::GreaterEqual,
            TokenType::EqualEqual,
            TokenType::NotEqual,
            TokenType::Spaceship,
            TokenType::StringLt,
            TokenType::StringGt,
            TokenType::StringLe,
            TokenType::StringGe,
            TokenType::StringEq,
            TokenType::StringNe,
            TokenType::StringCmp,
        ]) {
            let op = self.previous().token_type.clone();
            self.skip_whitespace();

            let right = self.parse_shift()?;
            let loc = SourceLocation { start: expr.location.start, end: right.location.end };

            expr = Node::new(
                NodeKind::Binary { left: Box::new(expr), op, right: Box::new(right) },
                loc,
            );
        }

        Ok(expr)
    }

    fn parse_shift(&mut self) -> Result<Node, ParseError> {
        let mut expr = self.parse_additive()?;

        while self.match_any(&[TokenType::LeftShift, TokenType::RightShift]) {
            let op = self.previous().token_type.clone();
            self.skip_whitespace();

            let right = self.parse_additive()?;
            let loc = SourceLocation { start: expr.location.start, end: right.location.end };

            expr = Node::new(
                NodeKind::Binary { left: Box::new(expr), op, right: Box::new(right) },
                loc,
            );
        }

        Ok(expr)
    }

    fn parse_additive(&mut self) -> Result<Node, ParseError> {
        let mut expr = self.parse_multiplicative()?;

        while self.match_any(&[TokenType::Plus, TokenType::Minus, TokenType::Dot]) {
            let op = self.previous().token_type.clone();
            self.skip_whitespace();

            let right = self.parse_multiplicative()?;
            let loc = SourceLocation { start: expr.location.start, end: right.location.end };

            expr = Node::new(
                NodeKind::Binary { left: Box::new(expr), op, right: Box::new(right) },
                loc,
            );
        }

        Ok(expr)
    }

    fn parse_multiplicative(&mut self) -> Result<Node, ParseError> {
        let mut expr = self.parse_unary()?;

        while self.match_any(&[
            TokenType::Star,
            TokenType::Slash,
            TokenType::Percent,
            TokenType::StringRepeat,
        ]) {
            let op = self.previous().token_type.clone();
            self.skip_whitespace();

            let right = self.parse_unary()?;
            let loc = SourceLocation { start: expr.location.start, end: right.location.end };

            expr = Node::new(
                NodeKind::Binary { left: Box::new(expr), op, right: Box::new(right) },
                loc,
            );
        }

        Ok(expr)
    }

    fn parse_unary(&mut self) -> Result<Node, ParseError> {
        self.skip_whitespace();

        if self.match_any(&[
            TokenType::Plus,
            TokenType::Minus,
            TokenType::Not,
            TokenType::BitwiseNot,
            TokenType::Backslash,
        ]) {
            let start = self.previous().start;
            let op = self.previous().token_type.clone();
            self.skip_whitespace();

            let operand = self.parse_unary()?;
            let loc = SourceLocation { start, end: operand.location.end };

            return Ok(Node::new(NodeKind::Unary { op, operand: Box::new(operand) }, loc));
        }

        // Prefix increment/decrement
        if self.match_any(&[TokenType::Increment, TokenType::Decrement]) {
            let start = self.previous().start;
            let op = self.previous().token_type.clone();
            self.skip_whitespace();

            let operand = self.parse_postfix()?;
            let loc = SourceLocation { start, end: operand.location.end };

            return Ok(Node::new(NodeKind::PrefixUpdate { op, operand: Box::new(operand) }, loc));
        }

        self.parse_postfix()
    }

    fn parse_postfix(&mut self) -> Result<Node, ParseError> {
        let mut expr = self.parse_primary()?;

        loop {
            self.skip_whitespace();

            if self.match_token(&TokenType::Arrow) {
                self.skip_whitespace();

                if self.check(&TokenType::LeftBracket) {
                    // Array dereference
                    self.advance();
                    let index = self.parse_expression()?;
                    self.consume(&TokenType::RightBracket)?;

                    let start = expr.location.start;
                    expr = Node::new(
                        NodeKind::ArrayAccess { array: Box::new(expr), index: Box::new(index) },
                        SourceLocation { start, end: self.current_pos() },
                    );
                } else if self.check(&TokenType::LeftBrace) {
                    // Hash dereference
                    self.advance();
                    let key = self.parse_expression()?;
                    self.consume(&TokenType::RightBrace)?;

                    let start = expr.location.start;
                    expr = Node::new(
                        NodeKind::HashAccess { hash: Box::new(expr), key: Box::new(key) },
                        SourceLocation { start, end: self.current_pos() },
                    );
                } else if self.check(&TokenType::Identifier) {
                    // Method call
                    let method = self.advance().text.clone();
                    let mut args = Vec::new();

                    if self.check(&TokenType::LeftParen) {
                        self.advance();
                        self.skip_whitespace();

                        while !self.check(&TokenType::RightParen) && !self.is_at_end() {
                            args.push(self.parse_expression()?);
                            self.skip_whitespace();

                            if !self.match_token(&TokenType::Comma) {
                                break;
                            }
                            self.skip_whitespace();
                        }

                        self.consume(&TokenType::RightParen)?;
                    }

                    let start = expr.location.start;
                    expr = Node::new(
                        NodeKind::MethodCall { object: Box::new(expr), method, args },
                        SourceLocation { start, end: self.current_pos() },
                    );
                } else {
                    // Scalar dereference
                    let deref = self.parse_unary()?;
                    let start = expr.location.start;
                    let end = deref.location.end;
                    expr = Node::new(
                        NodeKind::Dereference { expr: Box::new(expr), type_: Box::new(deref) },
                        SourceLocation { start, end },
                    );
                }
            } else if self.check(&TokenType::LeftBracket) {
                // Direct array access
                self.advance();
                let index = self.parse_expression()?;
                self.consume(&TokenType::RightBracket)?;

                let start = expr.location.start;
                expr = Node::new(
                    NodeKind::ArrayAccess { array: Box::new(expr), index: Box::new(index) },
                    SourceLocation { start, end: self.current_pos() },
                );
            } else if self.check(&TokenType::LeftBrace)
                && matches!(expr.kind, NodeKind::Variable { .. })
            {
                // Direct hash access
                self.advance();
                let key = self.parse_expression()?;
                self.consume(&TokenType::RightBrace)?;

                let start = expr.location.start;
                expr = Node::new(
                    NodeKind::HashAccess { hash: Box::new(expr), key: Box::new(key) },
                    SourceLocation { start, end: self.current_pos() },
                );
            } else if self.match_any(&[TokenType::Increment, TokenType::Decrement]) {
                // Postfix increment/decrement
                let op = self.previous().token_type.clone();

                let start = expr.location.start;
                expr = Node::new(
                    NodeKind::PostfixUpdate { op, operand: Box::new(expr) },
                    SourceLocation { start, end: self.current_pos() },
                );
            } else {
                break;
            }
        }

        Ok(expr)
    }

    fn parse_primary(&mut self) -> Result<Node, ParseError> {
        self.skip_whitespace();

        // Numbers
        if self.check(&TokenType::Number) {
            let token = self.advance();
            return Ok(Node::new(
                NodeKind::Number { value: token.text.clone() },
                SourceLocation { start: token.start, end: token.end },
            ));
        }

        // Strings
        if self.check_any(&[
            TokenType::SingleQuotedString,
            TokenType::DoubleQuotedString,
            TokenType::BacktickString,
        ]) {
            let token = self.advance();
            return Ok(Node::new(
                NodeKind::String { value: token.text.clone() },
                SourceLocation { start: token.start, end: token.end },
            ));
        }

        // Variables
        if self.check_variable() {
            return self.parse_variable_or_field();
        }

        // Regex
        if self.check(&TokenType::RegexMatch) {
            return self.parse_regex();
        }

        // Substitution
        if self.check(&TokenType::Substitution) {
            return self.parse_substitution();
        }

        // Bareword or function call
        if self.check(&TokenType::Identifier) {
            let id_token = self.advance().clone();
            let name = id_token.text.clone();

            // Check for function call
            if self.check(&TokenType::LeftParen) {
                self.advance();
                let mut args = Vec::new();

                self.skip_whitespace();
                while !self.check(&TokenType::RightParen) && !self.is_at_end() {
                    args.push(self.parse_expression()?);
                    self.skip_whitespace();

                    if !self.match_token(&TokenType::Comma) {
                        break;
                    }
                    self.skip_whitespace();
                }

                self.consume(&TokenType::RightParen)?;

                return Ok(Node::new(
                    NodeKind::FunctionCall { name, args },
                    SourceLocation { start: id_token.start, end: self.current_pos() },
                ));
            }

            // Bareword
            return Ok(Node::new(
                NodeKind::Bareword { value: name },
                SourceLocation { start: id_token.start, end: id_token.end },
            ));
        }

        // Parenthesized expression
        if self.match_token(&TokenType::LeftParen) {
            let start = self.previous().start;
            self.skip_whitespace();

            // Check for empty list
            if self.check(&TokenType::RightParen) {
                self.advance();
                return Ok(Node::new(
                    NodeKind::List { elements: Vec::new() },
                    SourceLocation { start, end: self.current_pos() },
                ));
            }

            // Parse expression or list
            let first = self.parse_expression()?;
            self.skip_whitespace();

            if self.match_token(&TokenType::Comma) {
                // It's a list
                let mut elements = vec![first];
                self.skip_whitespace();

                while !self.check(&TokenType::RightParen) && !self.is_at_end() {
                    elements.push(self.parse_expression()?);
                    self.skip_whitespace();

                    if !self.match_token(&TokenType::Comma) {
                        break;
                    }
                    self.skip_whitespace();
                }

                self.consume(&TokenType::RightParen)?;

                return Ok(Node::new(
                    NodeKind::List { elements },
                    SourceLocation { start, end: self.current_pos() },
                ));
            }

            self.consume(&TokenType::RightParen)?;
            return Ok(first);
        }

        // Array reference
        if self.match_token(&TokenType::LeftBracket) {
            let start = self.previous().start;
            let mut elements = Vec::new();

            self.skip_whitespace();
            while !self.check(&TokenType::RightBracket) && !self.is_at_end() {
                elements.push(self.parse_expression()?);
                self.skip_whitespace();

                if !self.match_token(&TokenType::Comma) {
                    break;
                }
                self.skip_whitespace();
            }

            self.consume(&TokenType::RightBracket)?;

            return Ok(Node::new(
                NodeKind::ArrayRef { elements },
                SourceLocation { start, end: self.current_pos() },
            ));
        }

        // Hash reference
        if self.match_token(&TokenType::LeftBrace) {
            let start = self.previous().start;
            let mut pairs = Vec::new();

            self.skip_whitespace();
            while !self.check(&TokenType::RightBrace) && !self.is_at_end() {
                let key = self.parse_expression()?;
                self.skip_whitespace();

                // Expect => or ,
                if self.match_token(&TokenType::Arrow) || self.match_token(&TokenType::Comma) {
                    self.skip_whitespace();
                    let value = self.parse_expression()?;
                    pairs.push((key, value));
                } else {
                    return Err(ParseError::new(
                        ParseErrorKind::UnexpectedToken,
                        self.current_pos(),
                        "Expected => or , in hash".to_string(),
                    ));
                }

                self.skip_whitespace();
                if !self.match_token(&TokenType::Comma) {
                    break;
                }
                self.skip_whitespace();
            }

            self.consume(&TokenType::RightBrace)?;

            return Ok(Node::new(
                NodeKind::HashRef { pairs },
                SourceLocation { start, end: self.current_pos() },
            ));
        }

        // Heredoc
        if self.check(&TokenType::HeredocStart) {
            return self.parse_heredoc();
        }

        Err(ParseError::new(
            ParseErrorKind::UnexpectedToken,
            self.current_pos(),
            format!("Expected expression, got {:?}", self.peek()),
        ))
    }

    // Helper parsing methods

    fn parse_variable_or_field(&mut self) -> Result<Node, ParseError> {
        let var_token = self.advance().clone();
        let var_name = var_token.text.clone();

        Ok(Node::new(
            NodeKind::Variable { name: var_name },
            SourceLocation { start: var_token.start, end: var_token.end },
        ))
    }

    fn parse_regex(&mut self) -> Result<Node, ParseError> {
        let token = self.advance();
        let text = token.text.as_ref();

        // Try to parse with RegexParser, but fall back to original behavior on failure
        let (pattern, modifiers) = if text.starts_with('m')
            && text.chars().nth(1).is_some_and(|c| !c.is_ascii_alphanumeric())
        {
            // Try parsing m// regex
            let mut parser = RegexParser::new(text, 1);
            match parser.parse_match_operator() {
                Ok(construct) => (Arc::from(construct.pattern), Arc::from(construct.modifiers)),
                Err(_) => {
                    // Fall back to original behavior - use whole token as pattern
                    (token.text.clone(), Arc::from(""))
                }
            }
        } else {
            // Try parsing bare // regex
            let mut parser = RegexParser::new(text, 0);
            match parser.parse_bare_regex() {
                Ok(construct) => (Arc::from(construct.pattern), Arc::from(construct.modifiers)),
                Err(_) => {
                    // Fall back to original behavior - use whole token as pattern
                    (token.text.clone(), Arc::from(""))
                }
            }
        };

        Ok(Node::new(
            NodeKind::Regex { pattern, replacement: None, modifiers },
            SourceLocation { start: token.start, end: token.end },
        ))
    }

    fn parse_substitution(&mut self) -> Result<Node, ParseError> {
        let token = self.advance();
        let text = token.text.as_ref();

        // Try to parse with RegexParser to extract components
        let mut parser = RegexParser::new(text, 1);
        match parser.parse_substitute_operator() {
            Ok(construct) => {
                // Successfully parsed - return as Substitution node
                Ok(Node::new(
                    NodeKind::Substitution {
                        pattern: Arc::from(construct.pattern),
                        replacement: Arc::from(construct.replacement.unwrap_or_default()),
                        modifiers: Arc::from(construct.modifiers),
                    },
                    SourceLocation { start: token.start, end: token.end },
                ))
            }
            Err(_) => {
                // Fall back to original behavior - treat as string for backward compatibility
                Ok(Node::new(
                    NodeKind::String { value: token.text.clone() },
                    SourceLocation { start: token.start, end: token.end },
                ))
            }
        }
    }

    fn parse_heredoc(&mut self) -> Result<Node, ParseError> {
        self.advance();
        let start_token = self.previous();
        let marker = start_token.text.clone();
        let start_pos = start_token.start;
        let end_pos = start_token.end;

        // Look for heredoc body
        if self.check(&TokenType::HeredocBody) {
            self.advance();
            let body_token = self.previous();
            let content = body_token.text.clone();
            let body_end = body_token.end;

            return Ok(Node::new(
                NodeKind::Heredoc { marker, content },
                SourceLocation { start: start_pos, end: body_end },
            ));
        }

        // No body found - error
        Ok(Node::new(
            NodeKind::Heredoc { marker, content: Arc::from("") },
            SourceLocation { start: start_pos, end: end_pos },
        ))
    }

    fn parse_block(&mut self) -> Result<Node, ParseError> {
        let start = self.current_pos();
        self.consume(&TokenType::LeftBrace)?;
        self.skip_whitespace();

        let mut statements = Vec::new();

        while !self.check(&TokenType::RightBrace) && !self.is_at_end() {
            match self.parse_statement() {
                Ok(stmt) => statements.push(stmt),
                Err(e) => {
                    self.errors.push(e);
                    self.synchronize();
                }
            }
            self.skip_whitespace();
        }

        self.consume(&TokenType::RightBrace)?;

        let end = self.current_pos();
        Ok(Node::new(NodeKind::Block { statements }, SourceLocation { start, end }))
    }

    fn parse_block_or_statement(&mut self) -> Result<Node, ParseError> {
        self.skip_whitespace();
        if self.check(&TokenType::LeftBrace) { self.parse_block() } else { self.parse_statement() }
    }

    fn parse_prototype(&mut self) -> Result<Arc<str>, ParseError> {
        self.consume(&TokenType::LeftParen)?;
        let start = self.current_pos();

        // Scan to matching paren
        let mut depth = 1;
        while depth > 0 && !self.is_at_end() {
            if self.check(&TokenType::LeftParen) {
                depth += 1;
            } else if self.check(&TokenType::RightParen) {
                depth -= 1;
                if depth == 0 {
                    break;
                }
            }
            self.advance();
        }

        let end = self.current_pos();
        self.consume(&TokenType::RightParen)?;

        // Extract prototype text
        Ok(Arc::from(self.source[start..end].trim()))
    }

    fn parse_attributes(&mut self) -> Result<Vec<Arc<str>>, ParseError> {
        let mut attributes = Vec::new();

        while self.match_token(&TokenType::Colon) {
            self.skip_whitespace();

            if self.check(&TokenType::Identifier) {
                let attr = self.advance().text.clone();

                // Skip attribute arguments if present
                if self.check(&TokenType::LeftParen) {
                    self.advance();
                    let mut depth = 1;
                    while depth > 0 && !self.is_at_end() {
                        if self.check(&TokenType::LeftParen) {
                            depth += 1;
                        } else if self.check(&TokenType::RightParen) {
                            depth -= 1;
                        }
                        self.advance();
                    }
                }

                attributes.push(attr);
            }

            self.skip_whitespace();
        }

        Ok(attributes)
    }

    fn parse_import_list(&mut self) -> Result<Vec<Arc<str>>, ParseError> {
        let mut imports = Vec::new();

        if self.check_keyword("qw") {
            self.advance();
            self.skip_whitespace();

            // Find delimiter
            if let Some(token) = self.peek() {
                let delim = match token.text.chars().next() {
                    Some('(') => ')',
                    Some('[') => ']',
                    Some('{') => '}',
                    Some('<') => '>',
                    Some(c) => c,
                    None => return Ok(imports),
                };

                self.advance(); // consume opening delimiter

                // Collect words until closing delimiter
                while !self.is_at_end() {
                    if let Some(token) = self.peek() {
                        if token.text.starts_with(delim) {
                            self.advance();
                            break;
                        }

                        if matches!(token.token_type, TokenType::Identifier) {
                            imports.push(token.text.clone());
                        }
                        self.advance();
                    }
                }
            }
        } else if self.match_token(&TokenType::LeftParen) {
            self.skip_whitespace();

            while !self.check(&TokenType::RightParen) && !self.is_at_end() {
                if self.check(&TokenType::Identifier) || self.check_string() {
                    imports.push(self.advance().text.clone());
                }

                self.skip_whitespace();
                if !self.match_token(&TokenType::Comma) {
                    break;
                }
                self.skip_whitespace();
            }

            self.consume(&TokenType::RightParen)?;
        }

        Ok(imports)
    }

    fn parse_return_statement(&mut self) -> Result<Node, ParseError> {
        let start = self.current_pos();
        self.advance(); // consume 'return'
        self.skip_whitespace();

        let value = if self.check(&TokenType::Semicolon) || self.is_at_end() {
            None
        } else {
            Some(Box::new(self.parse_expression()?))
        };

        // Consume optional trailing semicolon
        self.skip_whitespace();
        if self.check(&TokenType::Semicolon) {
            self.advance();
        }

        let end = self.current_pos();
        Ok(Node::new(NodeKind::Return { value }, SourceLocation { start, end }))
    }

    fn parse_loop_control(&mut self) -> Result<Node, ParseError> {
        let start = self.current_pos();
        let control_type = self.advance().text.clone();
        self.skip_whitespace();

        let label = if self.check(&TokenType::Identifier) {
            Some(self.advance().text.clone())
        } else {
            None
        };

        // Consume optional trailing semicolon
        self.skip_whitespace();
        if self.check(&TokenType::Semicolon) {
            self.advance();
        }

        let end = self.current_pos();
        Ok(Node::new(NodeKind::LoopControl { control_type, label }, SourceLocation { start, end }))
    }

    fn expect_package_name(&mut self) -> Result<Arc<str>, ParseError> {
        let mut parts = Vec::new();

        if !self.check(&TokenType::Identifier) {
            return Err(ParseError::new(
                ParseErrorKind::UnexpectedToken,
                self.current_pos(),
                "Expected package name".to_string(),
            ));
        }

        parts.push(self.advance().text.clone());

        while self.match_token(&TokenType::ColonColon) {
            if self.check(&TokenType::Identifier) {
                parts.push(self.advance().text.clone());
            } else {
                break;
            }
        }

        let name = parts.join("::");
        Ok(Arc::from(name))
    }

    // Token manipulation methods

    fn is_at_end(&self) -> bool {
        self.current >= self.tokens.len()
            || self.peek().map(|t| matches!(t.token_type, TokenType::EOF)).unwrap_or(true)
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.current)
    }

    fn peek_text(&self) -> &str {
        self.peek().map(|t| t.text.as_ref()).unwrap_or("")
    }

    fn previous(&self) -> &Token {
        &self.tokens[self.current - 1]
    }

    fn advance(&mut self) -> &Token {
        if !self.is_at_end() {
            self.current += 1;
        }
        self.previous()
    }

    fn check(&self, token_type: &TokenType) -> bool {
        self.peek()
            .map(|t| std::mem::discriminant(&t.token_type) == std::mem::discriminant(token_type))
            .unwrap_or(false)
    }

    fn check_any(&self, types: &[TokenType]) -> bool {
        types.iter().any(|t| self.check(t))
    }

    fn check_keyword(&self, keyword: &str) -> bool {
        self.peek()
            .map(|t| matches!(&t.token_type, TokenType::Identifier) && t.text.as_ref() == keyword)
            .unwrap_or(false)
    }

    fn check_variable(&self) -> bool {
        self.check_any(&[
            TokenType::ScalarVariable,
            TokenType::ArrayVariable,
            TokenType::HashVariable,
        ])
    }

    fn check_string(&self) -> bool {
        self.check_any(&[TokenType::SingleQuotedString, TokenType::DoubleQuotedString])
    }

    fn check_number(&self) -> bool {
        self.check(&TokenType::Number)
    }

    fn match_token(&mut self, token_type: &TokenType) -> bool {
        if self.check(token_type) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn match_any(&mut self, types: &[TokenType]) -> bool {
        for t in types {
            if self.match_token(t) {
                return true;
            }
        }
        false
    }

    fn match_keyword(&mut self, keyword: &str) -> bool {
        if self.check_keyword(keyword) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn consume(&mut self, token_type: &TokenType) -> Result<(), ParseError> {
        if self.check(token_type) {
            self.advance();
            Ok(())
        } else {
            Err(ParseError::new(
                ParseErrorKind::UnexpectedToken,
                self.current_pos(),
                format!("Expected {:?}, got {:?}", token_type, self.peek()),
            ))
        }
    }

    fn skip_whitespace(&mut self) {
        while self
            .peek()
            .map(|t| matches!(t.token_type, TokenType::Whitespace | TokenType::Comment))
            .unwrap_or(false)
        {
            self.advance();
        }
    }

    fn current_pos(&self) -> usize {
        self.peek()
            .map(|t| t.start)
            .unwrap_or_else(|| self.tokens.last().map(|t| t.end).unwrap_or(0))
    }

    fn synchronize(&mut self) {
        self.advance();

        while !self.is_at_end() {
            if matches!(self.previous().token_type, TokenType::Semicolon) {
                return;
            }

            if self.check_keyword("package")
                || self.check_keyword("sub")
                || self.check_keyword("if")
                || self.check_keyword("while")
                || self.check_keyword("for")
                || self.check_keyword("my")
            {
                return;
            }

            self.advance();
        }
    }

    fn scan_for_c_style(&mut self) -> bool {
        let saved = self.current;
        let mut semicolons = 0;
        let mut depth = 1;

        while depth > 0 && !self.is_at_end() {
            if self.check(&TokenType::LeftParen) {
                depth += 1;
            } else if self.check(&TokenType::RightParen) {
                depth -= 1;
            } else if self.check(&TokenType::Semicolon) {
                semicolons += 1;
            }
            self.advance();
        }

        self.current = saved;
        semicolons >= 2
    }
}

// Note: Additional node kinds are now defined in ast.rs
// Note: Additional token types are now defined in token_compat.rs

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_parsing() {
        use perl_tdd_support::must;
        let source = "my $x = 42;";
        let mut parser = ParserV2::new(source);
        let result = parser.parse();

        assert!(result.is_ok());
        let ast = must(result);
        println!("AST: {:#?}", ast);
        println!("S-expression: {}", ast.to_sexp());
    }

    #[test]
    fn test_complex_expression() {
        use perl_tdd_support::must;
        let source = "$a + $b * $c";
        let mut parser = ParserV2::new(source);
        let result = parser.parse();

        assert!(result.is_ok());
        let ast = must(result);
        println!("S-expression: {}", ast.to_sexp());
    }

    #[test]
    fn test_if_statement() {
        use perl_tdd_support::must;
        let source = "if ($x > 10) { print \"big\"; } else { print \"small\"; }";
        let mut parser = ParserV2::new(source);
        let result = parser.parse();

        assert!(result.is_ok());
        let ast = must(result);
        println!("S-expression: {}", ast.to_sexp());
    }

    #[test]
    fn test_subroutine() {
        use perl_tdd_support::must;
        let source = "sub hello { my $name = shift; print \"Hello, $name!\\n\"; }";
        let mut parser = ParserV2::new(source);
        let result = parser.parse();

        assert!(result.is_ok());
        let ast = must(result);
        println!("S-expression: {}", ast.to_sexp());
    }

    #[test]
    fn test_foreach_loop() {
        use perl_tdd_support::must;
        let source = "foreach my $item (@list) { print $item; }";
        let mut parser = ParserV2::new(source);
        let result = parser.parse();

        assert!(result.is_ok());
        let ast = must(result);
        println!("S-expression: {}", ast.to_sexp());
    }
}
