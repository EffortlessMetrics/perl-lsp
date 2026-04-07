//! Lightweight parser built on top of the Perl lexer
//!
//! This module provides a recursive descent parser that consumes tokens
//! from the Perl lexer and builds an AST compatible with tree-sitter.

use crate::ast::{Node, NodeKind, SourceLocation};
use crate::error::{ParseError, ParseErrorKind};
use crate::perl_lexer::PerlLexer;
use crate::token_compat::{Token, TokenType, from_perl_lexer_token};
use std::collections::VecDeque;
use std::sync::Arc;

/// Parser for Perl code using the robust lexer
pub struct Parser<'a> {
    /// Token stream from lexer
    tokens: VecDeque<Token>,
    /// Current position in token stream
    current: usize,
    /// Source code for error reporting
    #[allow(dead_code)]
    source: &'a str,
    /// Error accumulator
    errors: Vec<ParseError>,
}

impl<'a> Parser<'a> {
    /// Create a new parser for the given source
    pub fn new(source: &'a str) -> Self {
        let mut lexer = PerlLexer::new(source);
        let mut tokens = VecDeque::new();

        // Tokenize entire input
        while let Some(perl_token) = lexer.next_token() {
            let token = from_perl_lexer_token(&perl_token);
            tokens.push_back(token);
        }

        Self { tokens, current: 0, source, errors: Vec::new() }
    }

    /// Parse the source into an AST
    pub fn parse(&mut self) -> Result<Node, ParseError> {
        self.parse_program()
    }

    /// Parse a complete program
    fn parse_program(&mut self) -> Result<Node, ParseError> {
        let start = self.current_location();
        let mut statements = Vec::new();

        while !self.is_at_end() {
            // Skip whitespace and comments
            if self.match_token(&TokenType::Whitespace) || self.match_token(&TokenType::Comment) {
                continue;
            }

            match self.parse_statement() {
                Ok(stmt) => statements.push(stmt),
                Err(e) => {
                    self.errors.push(e);
                    // Try to recover by skipping to next statement
                    self.synchronize();
                }
            }
        }

        let end = self.current_location();
        Ok(Node::new(NodeKind::Program { statements }, SourceLocation { start, end }))
    }

    /// Parse a single statement
    fn parse_statement(&mut self) -> Result<Node, ParseError> {
        // Package declaration
        if self.match_keyword("package") {
            return self.parse_package_declaration();
        }

        // Use statement
        if self.match_keyword("use") {
            return self.parse_use_statement();
        }

        // Subroutine declaration
        if self.match_keyword("sub") {
            return self.parse_subroutine();
        }

        // Control flow
        if self.match_keyword("if") {
            return self.parse_if_statement();
        }

        if self.match_keyword("while") {
            return self.parse_while_statement();
        }

        if self.match_keyword("for") || self.match_keyword("foreach") {
            return self.parse_for_statement();
        }

        // Variable declaration
        if self.match_keyword("my")
            || self.match_keyword("our")
            || self.match_keyword("local")
            || self.match_keyword("state")
        {
            return self.parse_variable_declaration();
        }

        // Default: expression statement
        let expr = self.parse_expression()?;

        // Check for statement terminator
        if !self.match_token(&TokenType::Semicolon) && !self.is_at_end() {
            // Allow implicit semicolon before closing brace
            if !self.check_token(&TokenType::RightBrace) {
                return Err(ParseError::new(
                    ParseErrorKind::UnexpectedToken,
                    self.current_location(),
                    "Expected semicolon after expression".to_string(),
                ));
            }
        }

        Ok(expr)
    }

    /// Parse package declaration
    fn parse_package_declaration(&mut self) -> Result<Node, ParseError> {
        let start = self.previous_location();

        // Package name
        let name = self.expect_identifier("package name")?;

        // Optional version
        let version = if self.check_token(&TokenType::Number) {
            Some(self.advance().text.clone())
        } else {
            None
        };

        // Optional block
        let block = if self.match_token(&TokenType::LeftBrace) {
            Some(Box::new(self.parse_block()?))
        } else {
            self.expect_token(&TokenType::Semicolon)?;
            None
        };

        let end = self.previous_location();
        Ok(Node::new(
            NodeKind::PackageDeclaration { name, version, block },
            SourceLocation { start, end },
        ))
    }

    /// Parse use statement
    fn parse_use_statement(&mut self) -> Result<Node, ParseError> {
        let start = self.previous_location();

        // Module name
        let module = self.expect_identifier("module name")?;

        // Optional version
        let version = if self.check_token(&TokenType::Number) {
            Some(self.advance().text.clone())
        } else {
            None
        };

        // Optional import list
        let imports = if self.match_token(&TokenType::LeftParen) {
            let mut list = Vec::new();

            if !self.check_token(&TokenType::RightParen) {
                loop {
                    list.push(self.expect_identifier("import item")?);

                    if !self.match_token(&TokenType::Comma) {
                        break;
                    }
                }
            }

            self.expect_token(&TokenType::RightParen)?;
            Some(list)
        } else {
            None
        };

        self.expect_token(&TokenType::Semicolon)?;

        let end = self.previous_location();
        Ok(Node::new(
            NodeKind::UseStatement { module, version, imports },
            SourceLocation { start, end },
        ))
    }

    /// Parse subroutine declaration
    fn parse_subroutine(&mut self) -> Result<Node, ParseError> {
        let start = self.previous_location();

        // Subroutine name (optional for anonymous subs)
        let name = if self.check_token(&TokenType::Identifier) {
            Some(self.advance().text.clone())
        } else {
            None
        };

        // Optional prototype
        let prototype = if self.match_token(&TokenType::LeftParen) {
            let proto_start = self.current;
            let mut depth = 1;

            while depth > 0 && !self.is_at_end() {
                if self.match_token(&TokenType::LeftParen) {
                    depth += 1;
                } else if self.match_token(&TokenType::RightParen) {
                    depth -= 1;
                } else {
                    self.advance();
                }
            }

            Some(self.slice_tokens(proto_start, self.current - 1))
        } else {
            None
        };

        // Optional attributes
        let attributes =
            if self.match_token(&TokenType::Colon) { self.parse_attributes()? } else { Vec::new() };

        // Subroutine body
        let body = Box::new(self.parse_block()?);

        let end = self.previous_location();
        Ok(Node::new(
            NodeKind::Subroutine { name, prototype, attributes, body },
            SourceLocation { start, end },
        ))
    }

    /// Parse expression with operator precedence
    fn parse_expression(&mut self) -> Result<Node, ParseError> {
        self.parse_assignment()
    }

    /// Parse assignment expression
    fn parse_assignment(&mut self) -> Result<Node, ParseError> {
        let expr = self.parse_or()?;

        if self.match_any(&[
            TokenType::Equal,
            TokenType::PlusEqual,
            TokenType::MinusEqual,
            TokenType::StarEqual,
            TokenType::SlashEqual,
        ]) {
            let op = self.previous().token_type.clone();
            let right = Box::new(self.parse_assignment()?);
            let left = Box::new(expr);

            let location = self.span_locations(&left, &right);
            return Ok(Node::new(NodeKind::Assignment { left, op, right }, location));
        }

        Ok(expr)
    }

    /// Parse logical OR expression
    fn parse_or(&mut self) -> Result<Node, ParseError> {
        let mut expr = self.parse_and()?;

        while self.match_keyword("or") || self.match_token(&TokenType::OrOr) {
            let op = self.previous().token_type.clone();
            let right = Box::new(self.parse_and()?);
            let left = Box::new(expr);

            let location = self.span_locations(&left, &right);
            expr = Node::new(NodeKind::Binary { left, op, right }, location);
        }

        Ok(expr)
    }

    /// Parse logical AND expression
    fn parse_and(&mut self) -> Result<Node, ParseError> {
        let mut expr = self.parse_equality()?;

        while self.match_keyword("and") || self.match_token(&TokenType::AndAnd) {
            let op = self.previous().token_type.clone();
            let right = Box::new(self.parse_equality()?);
            let left = Box::new(expr);

            let location = self.span_locations(&left, &right);
            expr = Node::new(NodeKind::Binary { left, op, right }, location);
        }

        Ok(expr)
    }

    /// Parse equality expression
    fn parse_equality(&mut self) -> Result<Node, ParseError> {
        let mut expr = self.parse_relational()?;

        while self.match_any(&[
            TokenType::EqualEqual,
            TokenType::NotEqual,
            TokenType::StringEq,
            TokenType::StringNe,
        ]) {
            let op = self.previous().token_type.clone();
            let right = Box::new(self.parse_relational()?);
            let left = Box::new(expr);

            let location = self.span_locations(&left, &right);
            expr = Node::new(NodeKind::Binary { left, op, right }, location);
        }

        Ok(expr)
    }

    /// Parse relational expression
    fn parse_relational(&mut self) -> Result<Node, ParseError> {
        let mut expr = self.parse_additive()?;

        while self.match_any(&[
            TokenType::Less,
            TokenType::Greater,
            TokenType::LessEqual,
            TokenType::GreaterEqual,
            TokenType::StringLt,
            TokenType::StringGt,
            TokenType::StringLe,
            TokenType::StringGe,
        ]) {
            let op = self.previous().token_type.clone();
            let right = Box::new(self.parse_additive()?);
            let left = Box::new(expr);

            let location = self.span_locations(&left, &right);
            expr = Node::new(NodeKind::Binary { left, op, right }, location);
        }

        Ok(expr)
    }

    // ... Additional parsing methods would continue here ...

    // Helper methods

    /// Check if we've reached the end of tokens
    fn is_at_end(&self) -> bool {
        self.current >= self.tokens.len()
    }

    /// Get current token without advancing
    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.current)
    }

    /// Get previous token
    fn previous(&self) -> &Token {
        &self.tokens[self.current - 1]
    }

    /// Advance to next token
    fn advance(&mut self) -> &Token {
        if !self.is_at_end() {
            self.current += 1;
        }
        self.previous()
    }

    /// Check if current token matches type
    fn check_token(&self, token_type: &TokenType) -> bool {
        if let Some(token) = self.peek() {
            std::mem::discriminant(&token.token_type) == std::mem::discriminant(token_type)
        } else {
            false
        }
    }

    /// Consume token if it matches type
    fn match_token(&mut self, token_type: &TokenType) -> bool {
        if self.check_token(token_type) {
            self.advance();
            true
        } else {
            false
        }
    }

    /// Match any of the given token types
    fn match_any(&mut self, types: &[TokenType]) -> bool {
        for t in types {
            if self.match_token(t) {
                return true;
            }
        }
        false
    }

    /// Check if current token is keyword
    fn check_keyword(&self, keyword: &str) -> bool {
        if let Some(token) = self.peek() {
            matches!(&token.token_type, TokenType::Identifier) && token.text.as_ref() == keyword
        } else {
            false
        }
    }

    /// Consume keyword token
    fn match_keyword(&mut self, keyword: &str) -> bool {
        if self.check_keyword(keyword) {
            self.advance();
            true
        } else {
            false
        }
    }

    /// Expect specific token type
    fn expect_token(&mut self, token_type: &TokenType) -> Result<&Token, ParseError> {
        if self.check_token(token_type) {
            Ok(self.advance())
        } else {
            Err(ParseError::new(
                ParseErrorKind::UnexpectedToken,
                self.current_location(),
                format!("Expected {:?}", token_type),
            ))
        }
    }

    /// Expect identifier token
    fn expect_identifier(&mut self, context: &str) -> Result<Arc<str>, ParseError> {
        if let Some(token) = self.peek() {
            if matches!(&token.token_type, TokenType::Identifier) {
                Ok(self.advance().text.clone())
            } else {
                Err(ParseError::new(
                    ParseErrorKind::UnexpectedToken,
                    self.current_location(),
                    format!("Expected {} identifier", context),
                ))
            }
        } else {
            Err(ParseError::new(
                ParseErrorKind::UnexpectedEndOfInput,
                self.current_location(),
                format!("Expected {} identifier", context),
            ))
        }
    }

    /// Get current source location
    fn current_location(&self) -> usize {
        if let Some(token) = self.peek() {
            token.start
        } else if let Some(last) = self.tokens.back() {
            last.end
        } else {
            0
        }
    }

    /// Get previous token location
    fn previous_location(&self) -> usize {
        if self.current > 0 { self.tokens[self.current - 1].start } else { 0 }
    }

    /// Create location spanning two nodes
    fn span_locations(&self, start_node: &Node, end_node: &Node) -> SourceLocation {
        SourceLocation { start: start_node.location.start, end: end_node.location.end }
    }

    /// Get slice of tokens as string
    fn slice_tokens(&self, start: usize, end: usize) -> Arc<str> {
        if start >= end || end > self.tokens.len() {
            return Arc::from("");
        }

        let mut text = String::new();
        for i in start..end {
            if let Some(token) = self.tokens.get(i) {
                text.push_str(token.text.as_ref());
            }
        }
        Arc::from(text)
    }

    /// Synchronize after parse error
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

    /// Parse additive expression (+ -)
    fn parse_additive(&mut self) -> Result<Node, ParseError> {
        let mut expr = self.parse_multiplicative()?;

        while self.match_any(&[TokenType::Plus, TokenType::Minus]) {
            let op = self.previous().token_type.clone();
            let right = Box::new(self.parse_multiplicative()?);
            let left = Box::new(expr);

            let location = self.span_locations(&left, &right);
            expr = Node::new(NodeKind::Binary { left, op, right }, location);
        }

        Ok(expr)
    }

    /// Parse multiplicative expression (* / %)
    fn parse_multiplicative(&mut self) -> Result<Node, ParseError> {
        let mut expr = self.parse_unary()?;

        while self.match_any(&[TokenType::Star, TokenType::Slash, TokenType::Percent]) {
            let op = self.previous().token_type.clone();
            let right = Box::new(self.parse_unary()?);
            let left = Box::new(expr);

            let location = self.span_locations(&left, &right);
            expr = Node::new(NodeKind::Binary { left, op, right }, location);
        }

        Ok(expr)
    }

    /// Parse unary expression
    fn parse_unary(&mut self) -> Result<Node, ParseError> {
        if self.match_any(&[TokenType::Not, TokenType::Minus, TokenType::Plus]) {
            let op = self.previous().token_type.clone();
            let operand = Box::new(self.parse_unary()?);
            let start = self.previous_location();

            let end = operand.location.end;
            return Ok(Node::new(NodeKind::Unary { op, operand }, SourceLocation { start, end }));
        }

        self.parse_postfix()
    }

    /// Parse postfix expressions (array/hash access, method calls)
    fn parse_postfix(&mut self) -> Result<Node, ParseError> {
        let mut expr = self.parse_primary()?;

        loop {
            if self.match_token(&TokenType::Arrow) {
                // Method call or dereference
                if self.match_token(&TokenType::Identifier) {
                    let method = self.previous().text.clone();

                    if self.match_token(&TokenType::LeftParen) {
                        let args = self.parse_argument_list()?;
                        self.expect_token(&TokenType::RightParen)?;

                        let start_loc = expr.location.start;
                        let end_loc = self.previous_location();
                        expr = Node::new(
                            NodeKind::MethodCall { object: Box::new(expr), method, args },
                            SourceLocation { start: start_loc, end: end_loc },
                        );
                    } else {
                        // Property access
                        let start_loc = expr.location.start;
                        expr = Node::new(
                            NodeKind::MethodCall {
                                object: Box::new(expr),
                                method,
                                args: Vec::new(),
                            },
                            SourceLocation { start: start_loc, end: self.previous_location() },
                        );
                    }
                } else if self.match_token(&TokenType::LeftBracket) {
                    // Array dereference
                    let index = Box::new(self.parse_expression()?);
                    self.expect_token(&TokenType::RightBracket)?;

                    let start_loc = expr.location.start;
                    let end_loc = self.previous_location();
                    expr = Node::new(
                        NodeKind::ArrayAccess { array: Box::new(expr), index },
                        SourceLocation { start: start_loc, end: end_loc },
                    );
                } else if self.match_token(&TokenType::LeftBrace) {
                    // Hash dereference
                    let key = Box::new(self.parse_expression()?);
                    self.expect_token(&TokenType::RightBrace)?;

                    let start_loc = expr.location.start;
                    let end_loc = self.previous_location();
                    expr = Node::new(
                        NodeKind::HashAccess { hash: Box::new(expr), key },
                        SourceLocation { start: start_loc, end: end_loc },
                    );
                }
            } else if self.match_token(&TokenType::LeftBracket) {
                // Array subscript
                let index = Box::new(self.parse_expression()?);
                self.expect_token(&TokenType::RightBracket)?;

                let start_loc = expr.location.start;
                expr = Node::new(
                    NodeKind::ArrayAccess { array: Box::new(expr), index },
                    SourceLocation { start: start_loc, end: self.previous_location() },
                );
            } else if self.match_token(&TokenType::LeftBrace)
                && matches!(&expr.kind, NodeKind::Variable { .. })
            {
                // Hash subscript
                let key = Box::new(self.parse_expression()?);
                self.expect_token(&TokenType::RightBrace)?;

                let start_loc = expr.location.start;
                expr = Node::new(
                    NodeKind::HashAccess { hash: Box::new(expr), key },
                    SourceLocation { start: start_loc, end: self.previous_location() },
                );
            } else {
                break;
            }
        }

        Ok(expr)
    }

    /// Parse block statement
    fn parse_block(&mut self) -> Result<Node, ParseError> {
        let start = self.current_location();
        let mut statements = Vec::new();

        while !self.check_token(&TokenType::RightBrace) && !self.is_at_end() {
            if self.match_token(&TokenType::Whitespace) || self.match_token(&TokenType::Comment) {
                continue;
            }

            match self.parse_statement() {
                Ok(stmt) => statements.push(stmt),
                Err(e) => {
                    self.errors.push(e);
                    self.synchronize();
                }
            }
        }

        self.expect_token(&TokenType::RightBrace)?;
        let end = self.previous_location();

        Ok(Node::new(NodeKind::Block { statements }, SourceLocation { start, end }))
    }

    /// Parse subroutine attributes
    fn parse_attributes(&mut self) -> Result<Vec<Arc<str>>, ParseError> {
        let mut attributes = Vec::new();

        while self.match_token(&TokenType::Identifier) {
            attributes.push(self.previous().text.clone());

            if self.match_token(&TokenType::LeftParen) {
                // Skip attribute arguments
                let mut depth = 1;
                while depth > 0 && !self.is_at_end() {
                    if self.match_token(&TokenType::LeftParen) {
                        depth += 1;
                    } else if self.match_token(&TokenType::RightParen) {
                        depth -= 1;
                    } else {
                        self.advance();
                    }
                }
            }

            if !self.match_token(&TokenType::Colon) {
                break;
            }
        }

        Ok(attributes)
    }

    /// Parse if statement
    fn parse_if_statement(&mut self) -> Result<Node, ParseError> {
        let start = self.previous_location();

        // Condition
        self.expect_token(&TokenType::LeftParen)?;
        let condition = Box::new(self.parse_expression()?);
        self.expect_token(&TokenType::RightParen)?;

        // Then branch
        let then_branch = Box::new(self.parse_block_or_statement()?);

        // Elsif branches
        let mut elsif_branches = Vec::new();
        while self.match_keyword("elsif") || self.match_keyword("elseif") {
            self.expect_token(&TokenType::LeftParen)?;
            let elsif_cond = Box::new(self.parse_expression()?);
            self.expect_token(&TokenType::RightParen)?;
            let elsif_block = Box::new(self.parse_block_or_statement()?);
            elsif_branches.push((elsif_cond, elsif_block));
        }

        // Else branch
        let else_branch = if self.match_keyword("else") {
            Some(Box::new(self.parse_block_or_statement()?))
        } else {
            None
        };

        let end = self.previous_location();
        Ok(Node::new(
            NodeKind::IfStatement { condition, then_branch, elsif_branches, else_branch },
            SourceLocation { start, end },
        ))
    }

    /// Parse while statement
    fn parse_while_statement(&mut self) -> Result<Node, ParseError> {
        let start = self.previous_location();

        // Condition
        self.expect_token(&TokenType::LeftParen)?;
        let condition = Box::new(self.parse_expression()?);
        self.expect_token(&TokenType::RightParen)?;

        // Body
        let body = Box::new(self.parse_block_or_statement()?);

        // Optional continue block
        let continue_block =
            if self.match_keyword("continue") { Some(Box::new(self.parse_block()?)) } else { None };

        let end = self.previous_location();
        Ok(Node::new(
            NodeKind::WhileStatement { condition, body, continue_block },
            SourceLocation { start, end },
        ))
    }

    /// Parse for/foreach statement
    fn parse_for_statement(&mut self) -> Result<Node, ParseError> {
        let start = self.previous_location();

        // Check for C-style for loop
        if self.match_token(&TokenType::LeftParen) {
            // Look ahead to determine loop type
            let saved_pos = self.current;
            let is_c_style = self.scan_for_c_style_loop();
            self.current = saved_pos;

            if is_c_style {
                // C-style: for (init; condition; update)
                let init = if !self.check_token(&TokenType::Semicolon) {
                    Some(Box::new(self.parse_expression()?))
                } else {
                    None
                };
                self.expect_token(&TokenType::Semicolon)?;

                let condition = if !self.check_token(&TokenType::Semicolon) {
                    Some(Box::new(self.parse_expression()?))
                } else {
                    None
                };
                self.expect_token(&TokenType::Semicolon)?;

                let update = if !self.check_token(&TokenType::RightParen) {
                    Some(Box::new(self.parse_expression()?))
                } else {
                    None
                };
                self.expect_token(&TokenType::RightParen)?;

                let body = Box::new(self.parse_block_or_statement()?);

                return Ok(Node::new(
                    NodeKind::ForStatement { init, condition, update, body },
                    SourceLocation { start, end: self.previous_location() },
                ));
            }
        }

        // Foreach style
        let variable = if self.match_keyword("my") || self.match_keyword("our") {
            self.parse_variable_declaration()?
        } else if self.check_token(&TokenType::ScalarVariable) {
            self.parse_primary()?
        } else {
            // Default to $_
            let loc = self.current_location();
            Node::new(
                NodeKind::Variable { name: Arc::from("$_") },
                SourceLocation { start: loc, end: loc + 2 },
            )
        };

        self.expect_token(&TokenType::LeftParen)?;
        let list = Box::new(self.parse_expression()?);
        self.expect_token(&TokenType::RightParen)?;

        let body = Box::new(self.parse_block_or_statement()?);

        let end = self.previous_location();
        Ok(Node::new(
            NodeKind::ForeachStatement { variable: Box::new(variable), list, body },
            SourceLocation { start, end },
        ))
    }

    /// Parse variable declaration
    fn parse_variable_declaration(&mut self) -> Result<Node, ParseError> {
        let start = self.previous_location();
        let declarator = self.previous().text.clone();

        let mut variables = Vec::new();

        // Parse variable or list of variables
        if self.match_token(&TokenType::LeftParen) {
            // List of variables
            if !self.check_token(&TokenType::RightParen) {
                loop {
                    variables.push(self.parse_variable_or_undef()?);

                    if !self.match_token(&TokenType::Comma) {
                        break;
                    }
                }
            }
            self.expect_token(&TokenType::RightParen)?;
        } else {
            // Single variable
            variables.push(self.parse_variable_or_undef()?);
        }

        let end = self.previous_location();
        Ok(Node::new(
            NodeKind::VariableDeclaration { declarator, variables },
            SourceLocation { start, end },
        ))
    }

    /// Parse block or single statement
    fn parse_block_or_statement(&mut self) -> Result<Node, ParseError> {
        if self.check_token(&TokenType::LeftBrace) {
            self.advance();
            self.parse_block()
        } else {
            self.parse_statement()
        }
    }

    /// Parse variable or undef
    fn parse_variable_or_undef(&mut self) -> Result<Node, ParseError> {
        if self.match_keyword("undef") {
            let start = self.previous_location();
            let end = start + 5; // "undef" is 5 characters
            return Ok(Node::new(
                NodeKind::Bareword { value: Arc::from("undef") },
                SourceLocation { start, end },
            ));
        }

        if self.check_token(&TokenType::ScalarVariable)
            || self.check_token(&TokenType::ArrayVariable)
            || self.check_token(&TokenType::HashVariable)
        {
            self.parse_primary()
        } else {
            Err(ParseError::new(
                ParseErrorKind::UnexpectedToken,
                self.current_location(),
                "Expected variable".to_string(),
            ))
        }
    }

    /// Scan ahead to check for C-style for loop
    fn scan_for_c_style_loop(&mut self) -> bool {
        let mut semicolon_count = 0;
        let mut paren_depth = 1;

        while paren_depth > 0 && !self.is_at_end() {
            if self.match_token(&TokenType::LeftParen) {
                paren_depth += 1;
            } else if self.match_token(&TokenType::RightParen) {
                paren_depth -= 1;
            } else if self.match_token(&TokenType::Semicolon) {
                semicolon_count += 1;
            } else {
                self.advance();
            }
        }

        semicolon_count >= 2
    }

    fn parse_primary(&mut self) -> Result<Node, ParseError> {
        let start = self.current_location();

        // Numbers
        if self.match_token(&TokenType::Number) {
            let value = self.previous().text.clone();
            let end = self.previous_location();
            return Ok(Node::new(NodeKind::Number { value }, SourceLocation { start, end }));
        }

        // Strings
        if self.match_any(&[
            TokenType::SingleQuotedString,
            TokenType::DoubleQuotedString,
            TokenType::BacktickString,
        ]) {
            let value = self.previous().text.clone();
            let end = self.previous_location();
            return Ok(Node::new(NodeKind::String { value }, SourceLocation { start, end }));
        }

        // Variables
        if self.match_any(&[
            TokenType::ScalarVariable,
            TokenType::ArrayVariable,
            TokenType::HashVariable,
        ]) {
            let name = self.previous().text.clone();
            let end = self.previous_location();
            return Ok(Node::new(NodeKind::Variable { name }, SourceLocation { start, end }));
        }

        // Identifiers (barewords, function calls)
        if self.match_token(&TokenType::Identifier) {
            let name = self.previous().text.clone();

            // Check for function call
            if self.match_token(&TokenType::LeftParen) {
                let args = self.parse_argument_list()?;
                self.expect_token(&TokenType::RightParen)?;

                let end = self.previous_location();
                return Ok(Node::new(
                    NodeKind::FunctionCall { name, args },
                    SourceLocation { start, end },
                ));
            }

            let end = self.previous_location();
            return Ok(Node::new(
                NodeKind::Bareword { value: name },
                SourceLocation { start, end },
            ));
        }

        // Parenthesized expression
        if self.match_token(&TokenType::LeftParen) {
            let expr = self.parse_expression()?;
            self.expect_token(&TokenType::RightParen)?;
            return Ok(expr);
        }

        Err(ParseError::new(
            ParseErrorKind::UnexpectedToken,
            self.current_location(),
            "Expected expression".to_string(),
        ))
    }

    fn parse_argument_list(&mut self) -> Result<Vec<Node>, ParseError> {
        let mut args = Vec::new();

        if !self.check_token(&TokenType::RightParen) {
            loop {
                args.push(self.parse_expression()?);

                if !self.match_token(&TokenType::Comma) {
                    break;
                }
            }
        }

        Ok(args)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_assignment() {
        let source = "my $x = 42;";
        let mut parser = Parser::new(source);
        let result = parser.parse();
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_function_call() {
        let source = "print(\"Hello, world!\");";
        let mut parser = Parser::new(source);
        let result = parser.parse();
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_package_declaration() {
        let source = "package MyModule 1.0;";
        let mut parser = Parser::new(source);
        let result = parser.parse();
        assert!(result.is_ok());
    }
}
