//! The main Perl parser implementation
//!
//! This module implements a recursive descent parser with operator precedence
//! handling that consumes tokens from perl-lexer and produces an AST.

use crate::{
    ast::{Node, NodeKind, SourceLocation},
    error::{ParseError, ParseResult},
    token_stream::{Token, TokenKind, TokenStream},
};

/// The main parser struct
pub struct Parser<'a> {
    tokens: TokenStream<'a>,
    recursion_depth: usize,
    last_end_position: usize,
    in_for_loop_init: bool,
}

const MAX_RECURSION_DEPTH: usize = 1000;

impl<'a> Parser<'a> {
    /// Create a new parser for the given input
    pub fn new(input: &'a str) -> Self {
        Parser {
            tokens: TokenStream::new(input),
            recursion_depth: 0,
            last_end_position: 0,
            in_for_loop_init: false,
        }
    }
    
    /// Parse the input and return the AST
    pub fn parse(&mut self) -> ParseResult<Node> {
        self.parse_program()
    }
    
    /// Check recursion depth
    fn check_recursion(&mut self) -> ParseResult<()> {
        self.recursion_depth += 1;
        if self.recursion_depth > MAX_RECURSION_DEPTH {
            return Err(ParseError::RecursionLimit);
        }
        Ok(())
    }
    
    fn exit_recursion(&mut self) {
        self.recursion_depth = self.recursion_depth.saturating_sub(1);
    }
    
    /// Parse a complete program
    fn parse_program(&mut self) -> ParseResult<Node> {
        let start = self.current_position();
        let mut statements = Vec::new();
        
        while !self.tokens.is_eof() {
            statements.push(self.parse_statement()?);
        }
        
        let end = self.previous_position();
        Ok(Node::new(
            NodeKind::Program { statements },
            SourceLocation { start, end }
        ))
    }
    
    /// Parse a single statement
    fn parse_statement(&mut self) -> ParseResult<Node> {
        self.check_recursion()?;
        let result = self.parse_statement_inner();
        self.exit_recursion();
        result
    }
    
    fn parse_statement_inner(&mut self) -> ParseResult<Node> {
        let token = self.tokens.peek()?;
        
        match token.kind {
            // Variable declarations
            TokenKind::My | TokenKind::Our | TokenKind::Local | TokenKind::State => {
                self.parse_variable_declaration()
            }
            
            // Control flow
            TokenKind::If => self.parse_if_statement(),
            TokenKind::Unless => self.parse_unless_statement(),
            TokenKind::While => self.parse_while_statement(),
            TokenKind::Until => self.parse_until_statement(),
            TokenKind::For => self.parse_for_statement(),
            TokenKind::Foreach => self.parse_foreach_statement(),
            
            // Subroutines
            TokenKind::Sub => self.parse_subroutine(),
            
            // Package management
            TokenKind::Package => self.parse_package(),
            TokenKind::Use => self.parse_use(),
            TokenKind::No => self.parse_no(),
            
            // Return statement
            TokenKind::Return => self.parse_return(),
            
            // Block
            TokenKind::LeftBrace => self.parse_block(),
            
            // Expression statement
            _ => {
                let expr = self.parse_expression()?;
                
                // Consume optional semicolon
                if self.peek_kind() == Some(TokenKind::Semicolon) {
                    self.consume_token()?;
                }
                
                Ok(expr)
            }
        }
    }
    
    /// Parse variable declaration (my, our, local, state)
    fn parse_variable_declaration(&mut self) -> ParseResult<Node> {
        let start = self.current_position();
        let declarator_token = self.consume_token()?;
        let declarator = declarator_token.text.clone();
        
        let variable = self.parse_variable()?;
        
        let initializer = if self.peek_kind() == Some(TokenKind::Assign) {
            self.tokens.next()?; // consume =
            Some(Box::new(self.parse_expression()?))
        } else {
            None
        };
        
        // Consume semicolon if present (but not in for loop context)
        if self.peek_kind() == Some(TokenKind::Semicolon) && !self.in_for_loop_init {
            self.consume_token()?;
        }
        
        let end = self.previous_position();
        Ok(Node::new(
            NodeKind::VariableDeclaration {
                declarator,
                variable: Box::new(variable),
                initializer,
            },
            SourceLocation { start, end }
        ))
    }
    
    /// Parse a variable ($foo, @bar, %baz)
    fn parse_variable(&mut self) -> ParseResult<Node> {
        let token = self.consume_token()?;
        
        // The lexer returns variables as identifiers like "$x", "@array", etc.
        // We need to split the sigil from the name
        let text = &token.text;
        let (sigil, name) = if let Some(rest) = text.strip_prefix('$') {
            ("$".to_string(), rest.to_string())
        } else if let Some(rest) = text.strip_prefix('@') {
            ("@".to_string(), rest.to_string())
        } else if let Some(rest) = text.strip_prefix('%') {
            ("%".to_string(), rest.to_string())
        } else if let Some(rest) = text.strip_prefix('&') {
            ("&".to_string(), rest.to_string())
        } else if text.starts_with('*') && text.len() > 1 {
            let rest = &text[1..];
            ("*".to_string(), rest.to_string())
        } else {
            return Err(ParseError::syntax(
                &format!("Expected variable, found '{}'", text),
                token.start
            ));
        };
        
        Ok(Node::new(
            NodeKind::Variable { sigil, name },
            SourceLocation { start: token.start, end: token.end }
        ))
    }
    
    /// Parse a variable when we have a sigil token first
    fn parse_variable_from_sigil(&mut self) -> ParseResult<Node> {
        let sigil_token = self.consume_token()?;
        let sigil = sigil_token.text.clone();
        let start = sigil_token.start;
        
        // Check if next token is an identifier for the variable name
        let name = if self.peek_kind() == Some(TokenKind::Identifier) {
            let name_token = self.tokens.next()?;
            name_token.text.clone()
        } else {
            // Handle special variables like $$, $@, $!, $?, etc.
            match self.peek_kind() {
                Some(TokenKind::ScalarSigil) => {
                    // $$ - process ID
                    self.tokens.next()?;
                    "$".to_string()
                }
                Some(TokenKind::ArraySigil) => {
                    // $@ - eval error
                    self.tokens.next()?;
                    "@".to_string()
                }
                Some(TokenKind::Not) => {
                    // $! - system error
                    self.tokens.next()?;
                    "!".to_string()
                }
                Some(TokenKind::Unknown) => {
                    // Could be $?, $^, or other special
                    let token = self.tokens.peek()?;
                    match token.text.as_str() {
                        "?" => {
                            self.tokens.next()?;
                            "?".to_string()
                        }
                        "^" => {
                            // Handle $^X variables
                            self.tokens.next()?;
                            if self.peek_kind() == Some(TokenKind::Identifier) {
                                let var_token = self.tokens.next()?;
                                format!("^{}", var_token.text)
                            } else {
                                "^".to_string()
                            }
                        }
                        _ => {
                            return Err(ParseError::syntax(
                                &format!("Unexpected character after sigil: {}", token.text),
                                token.start
                            ));
                        }
                    }
                }
                Some(TokenKind::Number) => {
                    // $0, $1, $2, etc. - numbered capture groups
                    let num_token = self.tokens.next()?;
                    num_token.text.clone()
                }
                _ => {
                    // Empty variable name (just the sigil)
                    String::new()
                }
            }
        };
        
        let end = self.previous_position();
        Ok(Node::new(
            NodeKind::Variable { sigil, name },
            SourceLocation { start, end }
        ))
    }
    
    /// Parse if statement
    fn parse_if_statement(&mut self) -> ParseResult<Node> {
        let start = self.current_position();
        self.tokens.next()?; // consume 'if'
        
        self.expect(TokenKind::LeftParen)?;
        let condition = self.parse_expression()?;
        self.expect(TokenKind::RightParen)?;
        
        let then_branch = self.parse_block()?;
        
        let mut elsif_branches = Vec::new();
        let mut else_branch = None;
        
        // Handle elsif chains
        while self.peek_kind() == Some(TokenKind::Elsif) {
            self.tokens.next()?; // consume 'elsif'
            self.expect(TokenKind::LeftParen)?;
            let elsif_cond = self.parse_expression()?;
            self.expect(TokenKind::RightParen)?;
            let elsif_block = self.parse_block()?;
            elsif_branches.push((Box::new(elsif_cond), Box::new(elsif_block)));
        }
        
        // Handle else
        if self.peek_kind() == Some(TokenKind::Else) {
            self.tokens.next()?; // consume 'else'
            else_branch = Some(Box::new(self.parse_block()?));
        }
        
        let end = self.previous_position();
        Ok(Node::new(
            NodeKind::If {
                condition: Box::new(condition),
                then_branch: Box::new(then_branch),
                elsif_branches,
                else_branch,
            },
            SourceLocation { start, end }
        ))
    }
    
    /// Parse unless statement (syntactic sugar for if not)
    fn parse_unless_statement(&mut self) -> ParseResult<Node> {
        let start = self.current_position();
        self.tokens.next()?; // consume 'unless'
        
        self.expect(TokenKind::LeftParen)?;
        let condition = self.parse_expression()?;
        self.expect(TokenKind::RightParen)?;
        
        // Negate the condition
        let negated_condition = Node::new(
            NodeKind::Unary {
                op: "!".to_string(),
                operand: Box::new(condition),
            },
            SourceLocation { start, end: self.previous_position() }
        );
        
        let then_branch = self.parse_block()?;
        let end = self.previous_position();
        
        Ok(Node::new(
            NodeKind::If {
                condition: Box::new(negated_condition),
                then_branch: Box::new(then_branch),
                elsif_branches: vec![],
                else_branch: None,
            },
            SourceLocation { start, end }
        ))
    }
    
    /// Parse while loop
    fn parse_while_statement(&mut self) -> ParseResult<Node> {
        let start = self.current_position();
        self.tokens.next()?; // consume 'while'
        
        self.expect(TokenKind::LeftParen)?;
        let condition = self.parse_expression()?;
        self.expect(TokenKind::RightParen)?;
        
        let body = self.parse_block()?;
        
        // TODO: Handle continue block
        let continue_block = None;
        
        let end = self.previous_position();
        Ok(Node::new(
            NodeKind::While {
                condition: Box::new(condition),
                body: Box::new(body),
                continue_block,
            },
            SourceLocation { start, end }
        ))
    }
    
    /// Parse until loop (while not)
    fn parse_until_statement(&mut self) -> ParseResult<Node> {
        let start = self.current_position();
        self.tokens.next()?; // consume 'until'
        
        self.expect(TokenKind::LeftParen)?;
        let condition = self.parse_expression()?;
        self.expect(TokenKind::RightParen)?;
        
        // Negate the condition
        let negated_condition = Node::new(
            NodeKind::Unary {
                op: "!".to_string(),
                operand: Box::new(condition),
            },
            SourceLocation { start, end: self.previous_position() }
        );
        
        let body = self.parse_block()?;
        let end = self.previous_position();
        
        Ok(Node::new(
            NodeKind::While {
                condition: Box::new(negated_condition),
                body: Box::new(body),
                continue_block: None,
            },
            SourceLocation { start, end }
        ))
    }
    
    /// Parse for loop
    fn parse_for_statement(&mut self) -> ParseResult<Node> {
        let start = self.current_position();
        self.tokens.next()?; // consume 'for'
        
        // Check if it's a foreach-style for loop
        if self.peek_kind() == Some(TokenKind::My) || 
           self.is_variable_start() {
            return self.parse_foreach_style_for();
        }
        
        self.expect(TokenKind::LeftParen)?;
        
        // Parse init
        let init = if self.peek_kind() == Some(TokenKind::Semicolon) {
            None
        } else if self.peek_kind() == Some(TokenKind::My) {
            // Handle variable declaration in for loop init
            self.in_for_loop_init = true;
            let decl = self.parse_variable_declaration()?;
            self.in_for_loop_init = false;
            // Variable declarations in for loops don't have trailing semicolons
            Some(Box::new(decl))
        } else {
            Some(Box::new(self.parse_expression()?))
        };
        self.expect(TokenKind::Semicolon)?;
        
        // Parse condition
        let condition = if self.peek_kind() == Some(TokenKind::Semicolon) {
            None
        } else {
            Some(Box::new(self.parse_expression()?))
        };
        self.expect(TokenKind::Semicolon)?;
        
        // Parse update
        let update = if self.peek_kind() == Some(TokenKind::RightParen) {
            None
        } else {
            Some(Box::new(self.parse_expression()?))
        };
        
        self.expect(TokenKind::RightParen)?;
        let body = self.parse_block()?;
        
        let end = self.previous_position();
        Ok(Node::new(
            NodeKind::For {
                init,
                condition,
                update,
                body: Box::new(body),
            },
            SourceLocation { start, end }
        ))
    }
    
    /// Parse foreach loop
    fn parse_foreach_statement(&mut self) -> ParseResult<Node> {
        let start = self.current_position();
        self.tokens.next()?; // consume 'foreach'
        
        let variable = if self.peek_kind() == Some(TokenKind::My) {
            self.parse_variable_declaration()?
        } else {
            self.parse_variable()?
        };
        
        self.expect(TokenKind::LeftParen)?;
        let list = self.parse_expression()?;
        self.expect(TokenKind::RightParen)?;
        
        let body = self.parse_block()?;
        
        let end = self.previous_position();
        Ok(Node::new(
            NodeKind::Foreach {
                variable: Box::new(variable),
                list: Box::new(list),
                body: Box::new(body),
            },
            SourceLocation { start, end }
        ))
    }
    
    /// Parse foreach-style for loop
    fn parse_foreach_style_for(&mut self) -> ParseResult<Node> {
        let variable = if self.peek_kind() == Some(TokenKind::My) {
            self.parse_variable_declaration()?
        } else {
            self.parse_variable()?
        };
        
        self.expect(TokenKind::LeftParen)?;
        let list = self.parse_expression()?;
        self.expect(TokenKind::RightParen)?;
        
        let body = self.parse_block()?;
        
        let start = variable.location.start;
        let end = self.previous_position();
        
        Ok(Node::new(
            NodeKind::Foreach {
                variable: Box::new(variable),
                list: Box::new(list),
                body: Box::new(body),
            },
            SourceLocation { start, end }
        ))
    }
    
    /// Parse subroutine definition
    fn parse_subroutine(&mut self) -> ParseResult<Node> {
        let start = self.current_position();
        self.tokens.next()?; // consume 'sub'
        
        let name = if self.peek_kind() == Some(TokenKind::Identifier) {
            Some(self.tokens.next()?.text.clone())
        } else {
            None
        };
        
        // TODO: Parse parameters
        let params = Vec::new();
        
        let body = self.parse_block()?;
        
        let end = self.previous_position();
        Ok(Node::new(
            NodeKind::Subroutine {
                name,
                params,
                body: Box::new(body),
            },
            SourceLocation { start, end }
        ))
    }
    
    /// Parse package declaration
    fn parse_package(&mut self) -> ParseResult<Node> {
        let start = self.current_position();
        self.tokens.next()?; // consume 'package'
        
        // Parse package name (can include ::)
        let mut name = self.expect(TokenKind::Identifier)?.text.clone();
        
        // Handle :: in package names
        while self.peek_kind() == Some(TokenKind::DoubleColon) {
            self.tokens.next()?; // consume ::
            name.push_str("::");
            name.push_str(&self.expect(TokenKind::Identifier)?.text);
        }
        
        let block = if self.peek_kind() == Some(TokenKind::LeftBrace) {
            Some(Box::new(self.parse_block()?))
        } else {
            self.expect(TokenKind::Semicolon)?;
            None
        };
        
        let end = self.previous_position();
        Ok(Node::new(
            NodeKind::Package { name, block },
            SourceLocation { start, end }
        ))
    }
    
    /// Parse use statement
    fn parse_use(&mut self) -> ParseResult<Node> {
        let start = self.current_position();
        self.tokens.next()?; // consume 'use'
        
        // Parse module name (can include ::)
        let mut module = self.expect(TokenKind::Identifier)?.text.clone();
        
        // Handle :: in module names
        while self.peek_kind() == Some(TokenKind::DoubleColon) {
            self.tokens.next()?; // consume ::
            module.push_str("::");
            module.push_str(&self.expect(TokenKind::Identifier)?.text);
        }
        
        // Parse optional version number
        if self.peek_kind() == Some(TokenKind::Number) {
            module.push(' ');
            module.push_str(&self.tokens.next()?.text);
        }
        
        // Parse optional import list
        let mut args = Vec::new();
        if self.peek_kind() == Some(TokenKind::LeftParen) {
            self.tokens.next()?; // consume (
            
            // Parse import list
            while self.peek_kind() != Some(TokenKind::RightParen) {
                if self.peek_kind() == Some(TokenKind::String) {
                    args.push(self.tokens.next()?.text.clone());
                } else if self.peek_kind() == Some(TokenKind::Identifier) {
                    args.push(self.tokens.next()?.text.clone());
                } else {
                    return Err(ParseError::syntax(
                        "Expected string or identifier in import list",
                        self.current_position()
                    ));
                }
                
                if self.peek_kind() == Some(TokenKind::Comma) {
                    self.tokens.next()?; // consume comma
                } else if self.peek_kind() != Some(TokenKind::RightParen) {
                    return Err(ParseError::syntax(
                        "Expected comma or closing parenthesis",
                        self.current_position()
                    ));
                }
            }
            
            self.expect(TokenKind::RightParen)?;
        }
        
        self.expect(TokenKind::Semicolon)?;
        
        let end = self.previous_position();
        Ok(Node::new(
            NodeKind::Use { module, args },
            SourceLocation { start, end }
        ))
    }
    
    /// Parse no statement (similar to use but disables pragmas/modules)
    fn parse_no(&mut self) -> ParseResult<Node> {
        let start = self.current_position();
        self.tokens.next()?; // consume 'no'
        
        // Parse module name (can include ::)
        let mut module = self.expect(TokenKind::Identifier)?.text.clone();
        
        // Handle :: in module names
        while self.peek_kind() == Some(TokenKind::DoubleColon) {
            self.tokens.next()?; // consume ::
            module.push_str("::");
            module.push_str(&self.expect(TokenKind::Identifier)?.text);
        }
        
        // Parse optional version number
        if self.peek_kind() == Some(TokenKind::Number) {
            module.push(' ');
            module.push_str(&self.tokens.next()?.text);
        }
        
        // Parse optional arguments list
        let mut args = Vec::new();
        if self.peek_kind() == Some(TokenKind::LeftParen) {
            self.tokens.next()?; // consume (
            
            // Parse argument list
            while self.peek_kind() != Some(TokenKind::RightParen) {
                if self.peek_kind() == Some(TokenKind::String) {
                    args.push(self.tokens.next()?.text.clone());
                } else if self.peek_kind() == Some(TokenKind::Identifier) {
                    args.push(self.tokens.next()?.text.clone());
                } else {
                    return Err(ParseError::syntax(
                        "Expected string or identifier in argument list",
                        self.current_position()
                    ));
                }
                
                if self.peek_kind() == Some(TokenKind::Comma) {
                    self.tokens.next()?; // consume comma
                } else if self.peek_kind() != Some(TokenKind::RightParen) {
                    return Err(ParseError::syntax(
                        "Expected comma or closing parenthesis",
                        self.current_position()
                    ));
                }
            }
            
            self.expect(TokenKind::RightParen)?;
        }
        
        self.expect(TokenKind::Semicolon)?;
        
        let end = self.previous_position();
        Ok(Node::new(
            NodeKind::No { module, args },
            SourceLocation { start, end }
        ))
    }
    
    /// Parse return statement
    fn parse_return(&mut self) -> ParseResult<Node> {
        let start = self.current_position();
        self.tokens.next()?; // consume 'return'
        
        let value = if self.peek_kind() == Some(TokenKind::Semicolon) {
            None
        } else {
            Some(Box::new(self.parse_expression()?))
        };
        
        if self.peek_kind() == Some(TokenKind::Semicolon) {
            self.tokens.next()?;
        }
        
        let end = self.previous_position();
        Ok(Node::new(
            NodeKind::Return { value },
            SourceLocation { start, end }
        ))
    }
    
    /// Parse a block statement
    fn parse_block(&mut self) -> ParseResult<Node> {
        let start = self.current_position();
        self.expect(TokenKind::LeftBrace)?;
        
        let mut statements = Vec::new();
        
        while self.peek_kind() != Some(TokenKind::RightBrace) && !self.tokens.is_eof() {
            statements.push(self.parse_statement()?);
        }
        
        self.expect(TokenKind::RightBrace)?;
        let end = self.previous_position();
        
        Ok(Node::new(
            NodeKind::Block { statements },
            SourceLocation { start, end }
        ))
    }
    
    /// Parse an expression
    fn parse_expression(&mut self) -> ParseResult<Node> {
        self.check_recursion()?;
        let result = self.parse_comma();
        self.exit_recursion();
        result
    }
    
    /// Parse comma operator (lowest precedence)
    fn parse_comma(&mut self) -> ParseResult<Node> {
        let mut expr = self.parse_assignment()?;
        
        // In scalar context, comma creates a list
        // For now, we'll just parse it as sequential expressions
        if self.peek_kind() == Some(TokenKind::Comma) {
            let mut expressions = vec![expr];
            
            while self.peek_kind() == Some(TokenKind::Comma) {
                self.tokens.next()?; // consume comma
                expressions.push(self.parse_assignment()?);
            }
            
            // Return as array literal for now
            let start = expressions[0].location.start;
            let end = expressions.last().unwrap().location.end;
            
            expr = Node::new(
                NodeKind::ArrayLiteral { elements: expressions },
                SourceLocation { start, end }
            );
        }
        
        Ok(expr)
    }
    
    /// Parse assignment expression
    fn parse_assignment(&mut self) -> ParseResult<Node> {
        let mut expr = self.parse_or()?;
        
        if self.peek_kind() == Some(TokenKind::Assign) {
            self.tokens.next()?; // consume =
            let rhs = self.parse_assignment()?;
            let start = expr.location.start;
            let end = rhs.location.end;
            
            expr = Node::new(
                NodeKind::Assignment {
                    lhs: Box::new(expr),
                    rhs: Box::new(rhs),
                },
                SourceLocation { start, end }
            );
        }
        
        Ok(expr)
    }
    
    /// Parse logical OR expression
    fn parse_or(&mut self) -> ParseResult<Node> {
        let mut expr = self.parse_and()?;
        
        while self.peek_kind() == Some(TokenKind::Or) {
            let op_token = self.tokens.next()?;
            let right = self.parse_and()?;
            let start = expr.location.start;
            let end = right.location.end;
            
            expr = Node::new(
                NodeKind::Binary {
                    op: op_token.text.clone(),
                    left: Box::new(expr),
                    right: Box::new(right),
                },
                SourceLocation { start, end }
            );
        }
        
        Ok(expr)
    }
    
    /// Parse logical AND expression
    fn parse_and(&mut self) -> ParseResult<Node> {
        let mut expr = self.parse_equality()?;
        
        while self.peek_kind() == Some(TokenKind::And) {
            let op_token = self.tokens.next()?;
            let right = self.parse_equality()?;
            let start = expr.location.start;
            let end = right.location.end;
            
            expr = Node::new(
                NodeKind::Binary {
                    op: op_token.text.clone(),
                    left: Box::new(expr),
                    right: Box::new(right),
                },
                SourceLocation { start, end }
            );
        }
        
        Ok(expr)
    }
    
    /// Parse equality expression
    fn parse_equality(&mut self) -> ParseResult<Node> {
        let mut expr = self.parse_relational()?;
        
        while let Some(kind) = self.peek_kind() {
            match kind {
                TokenKind::Equal | TokenKind::NotEqual | TokenKind::Match | TokenKind::NotMatch => {
                    let op_token = self.tokens.next()?;
                    let right = self.parse_relational()?;
                    let start = expr.location.start;
                    let end = right.location.end;
                    
                    expr = Node::new(
                        NodeKind::Binary {
                            op: op_token.text.clone(),
                            left: Box::new(expr),
                            right: Box::new(right),
                        },
                        SourceLocation { start, end }
                    );
                }
                _ => break,
            }
        }
        
        Ok(expr)
    }
    
    /// Parse relational expression
    fn parse_relational(&mut self) -> ParseResult<Node> {
        let mut expr = self.parse_additive()?;
        
        while let Some(kind) = self.peek_kind() {
            match kind {
                TokenKind::Less | TokenKind::Greater | 
                TokenKind::LessEqual | TokenKind::GreaterEqual => {
                    let op_token = self.tokens.next()?;
                    let right = self.parse_additive()?;
                    let start = expr.location.start;
                    let end = right.location.end;
                    
                    expr = Node::new(
                        NodeKind::Binary {
                            op: op_token.text.clone(),
                            left: Box::new(expr),
                            right: Box::new(right),
                        },
                        SourceLocation { start, end }
                    );
                }
                _ => break,
            }
        }
        
        Ok(expr)
    }
    
    /// Parse additive expression
    fn parse_additive(&mut self) -> ParseResult<Node> {
        let mut expr = self.parse_multiplicative()?;
        
        while let Some(kind) = self.peek_kind() {
            match kind {
                TokenKind::Plus | TokenKind::Minus => {
                    let op_token = self.tokens.next()?;
                    let right = self.parse_multiplicative()?;
                    let start = expr.location.start;
                    let end = right.location.end;
                    
                    expr = Node::new(
                        NodeKind::Binary {
                            op: op_token.text.clone(),
                            left: Box::new(expr),
                            right: Box::new(right),
                        },
                        SourceLocation { start, end }
                    );
                }
                _ => break,
            }
        }
        
        Ok(expr)
    }
    
    /// Parse multiplicative expression
    fn parse_multiplicative(&mut self) -> ParseResult<Node> {
        let mut expr = self.parse_unary()?;
        
        while let Some(kind) = self.peek_kind() {
            match kind {
                TokenKind::Star | TokenKind::Slash | TokenKind::Percent => {
                    let op_token = self.tokens.next()?;
                    let right = self.parse_unary()?;
                    let start = expr.location.start;
                    let end = right.location.end;
                    
                    expr = Node::new(
                        NodeKind::Binary {
                            op: op_token.text.clone(),
                            left: Box::new(expr),
                            right: Box::new(right),
                        },
                        SourceLocation { start, end }
                    );
                }
                _ => break,
            }
        }
        
        Ok(expr)
    }
    
    /// Parse unary expression
    fn parse_unary(&mut self) -> ParseResult<Node> {
        if let Some(kind) = self.peek_kind() {
            match kind {
                TokenKind::Minus | TokenKind::Not => {
                    let op_token = self.tokens.next()?;
                    let start = op_token.start;
                    let operand = self.parse_unary()?;
                    let end = operand.location.end;
                    
                    return Ok(Node::new(
                        NodeKind::Unary {
                            op: op_token.text.clone(),
                            operand: Box::new(operand),
                        },
                        SourceLocation { start, end }
                    ));
                }
                _ => {}
            }
        }
        
        self.parse_postfix()
    }
    
    /// Parse postfix expression
    fn parse_postfix(&mut self) -> ParseResult<Node> {
        let mut expr = self.parse_primary()?;
        
        loop {
            match self.peek_kind() {
                Some(TokenKind::Increment) | Some(TokenKind::Decrement) => {
                    let op_token = self.consume_token()?;
                    let start = expr.location.start;
                    let end = op_token.end;
                    
                    expr = Node::new(
                        NodeKind::Unary {
                            op: op_token.text.clone(),
                            operand: Box::new(expr),
                        },
                        SourceLocation { start, end }
                    );
                }
                
                Some(TokenKind::Arrow) => {
                    self.tokens.next()?; // consume ->
                    
                    // Method call
                    if self.peek_kind() == Some(TokenKind::Identifier) {
                        let method = self.tokens.next()?.text.clone();
                        
                        let args = if self.peek_kind() == Some(TokenKind::LeftParen) {
                            self.parse_args()?
                        } else {
                            Vec::new()
                        };
                        
                        let start = expr.location.start;
                        let end = self.previous_position();
                        
                        expr = Node::new(
                            NodeKind::MethodCall {
                                object: Box::new(expr),
                                method,
                                args,
                            },
                            SourceLocation { start, end }
                        );
                    }
                }
                
                Some(TokenKind::LeftBracket) => {
                    // Array indexing
                    self.tokens.next()?;
                    let index = self.parse_expression()?;
                    self.expect(TokenKind::RightBracket)?;
                    
                    let start = expr.location.start;
                    let end = self.previous_position();
                    
                    // Represent as binary subscript operation
                    expr = Node::new(
                        NodeKind::Binary {
                            op: "[]".to_string(),
                            left: Box::new(expr),
                            right: Box::new(index),
                        },
                        SourceLocation { start, end }
                    );
                }
                
                Some(TokenKind::LeftBrace) => {
                    // Hash element access
                    self.tokens.next()?; // consume {
                    let key = self.parse_expression()?;
                    self.expect(TokenKind::RightBrace)?;
                    
                    let start = expr.location.start;
                    let end = self.previous_position();
                    
                    // Represent as binary subscript operation
                    expr = Node::new(
                        NodeKind::Binary {
                            op: "{}".to_string(),
                            left: Box::new(expr),
                            right: Box::new(key),
                        },
                        SourceLocation { start, end }
                    );
                }
                
                Some(TokenKind::LeftParen) if matches!(&expr.kind, NodeKind::Identifier { .. }) => {
                    // Function call
                    if let NodeKind::Identifier { name } = &expr.kind {
                        let name = name.clone();
                        let args = self.parse_args()?;
                        let start = expr.location.start;
                        let end = self.previous_position();
                        
                        expr = Node::new(
                            NodeKind::FunctionCall { name, args },
                            SourceLocation { start, end }
                        );
                    }
                }
                
                _ => break,
            }
        }
        
        Ok(expr)
    }
    
    /// Parse primary expression
    fn parse_primary(&mut self) -> ParseResult<Node> {
        let token_kind = self.tokens.peek()?.kind;
        
        match token_kind {
            TokenKind::Number => {
                let token = self.tokens.next()?;
                Ok(Node::new(
                    NodeKind::Number { value: token.text.clone() },
                    SourceLocation { start: token.start, end: token.end }
                ))
            }
            
            TokenKind::String => {
                let token = self.tokens.next()?;
                // Check if it's a double-quoted string (interpolated)
                let interpolated = token.text.starts_with('"');
                Ok(Node::new(
                    NodeKind::String { 
                        value: token.text.clone(),
                        interpolated,
                    },
                    SourceLocation { start: token.start, end: token.end }
                ))
            }
            
            TokenKind::Regex => {
                let token = self.tokens.next()?;
                Ok(Node::new(
                    NodeKind::Regex { 
                        pattern: token.text.clone(),
                        modifiers: String::new(), // TODO: Parse modifiers
                    },
                    SourceLocation { start: token.start, end: token.end }
                ))
            }
            
            TokenKind::Identifier => {
                // Check if it's a variable (starts with sigil)
                let token = self.tokens.peek()?;
                if token.text.starts_with('$') || token.text.starts_with('@') ||
                   token.text.starts_with('%') || token.text.starts_with('&') {
                    self.parse_variable()
                } else if token.text.starts_with('*') && token.text.len() > 1 {
                    // Only treat * as a glob sigil if followed by identifier
                    self.parse_variable()
                } else {
                    // Regular identifier
                    let token = self.tokens.next()?;
                    Ok(Node::new(
                        NodeKind::Identifier { name: token.text.clone() },
                        SourceLocation { start: token.start, end: token.end }
                    ))
                }
            }
            
            // Handle sigil tokens (for when lexer sends them separately)
            TokenKind::ScalarSigil | TokenKind::ArraySigil | TokenKind::HashSigil | TokenKind::SubSigil | TokenKind::GlobSigil => {
                self.parse_variable_from_sigil()
            }
            
            TokenKind::LeftParen => {
                let start_token = self.tokens.next()?; // consume (
                let start = start_token.start;
                
                // Check for empty list
                if self.peek_kind() == Some(TokenKind::RightParen) {
                    let end_token = self.tokens.next()?;
                    return Ok(Node::new(
                        NodeKind::ArrayLiteral { elements: vec![] },
                        SourceLocation { start, end: end_token.end }
                    ));
                }
                
                // Parse comma-separated list
                let first = self.parse_expression()?;
                
                if self.peek_kind() == Some(TokenKind::Comma) {
                    // It's a list
                    let mut elements = vec![first];
                    
                    while self.peek_kind() == Some(TokenKind::Comma) {
                        self.tokens.next()?; // consume comma
                        if self.peek_kind() == Some(TokenKind::RightParen) {
                            break;
                        }
                        elements.push(self.parse_expression()?);
                    }
                    
                    self.expect(TokenKind::RightParen)?;
                    let end = self.previous_position();
                    
                    Ok(Node::new(
                        NodeKind::ArrayLiteral { elements },
                        SourceLocation { start, end }
                    ))
                } else {
                    // It's a parenthesized expression
                    self.expect(TokenKind::RightParen)?;
                    Ok(first)
                }
            }
            
            TokenKind::LeftBracket => {
                // Array literal
                let start_token = self.tokens.next()?; // consume [
                let start = start_token.start;
                
                let mut elements = Vec::new();
                
                while self.peek_kind() != Some(TokenKind::RightBracket) && !self.tokens.is_eof() {
                    elements.push(self.parse_expression()?);
                    
                    if self.peek_kind() == Some(TokenKind::Comma) {
                        self.tokens.next()?;
                    } else {
                        break;
                    }
                }
                
                self.expect(TokenKind::RightBracket)?;
                let end = self.previous_position();
                
                Ok(Node::new(
                    NodeKind::ArrayLiteral { elements },
                    SourceLocation { start, end }
                ))
            }
            
            TokenKind::LeftBrace => {
                // Could be hash literal or block
                // Try to parse as hash literal first
                self.parse_hash_or_block()
            }
            
            _ => {
                // Get position before consuming
                let pos = self.current_position();
                Err(ParseError::unexpected(
                    "expression",
                    &format!("{:?}", token_kind),
                    pos
                ))
            }
        }
    }
    
    /// Parse function arguments
    fn parse_args(&mut self) -> ParseResult<Vec<Node>> {
        self.expect(TokenKind::LeftParen)?;
        let mut args = Vec::new();
        
        while self.peek_kind() != Some(TokenKind::RightParen) && !self.tokens.is_eof() {
            args.push(self.parse_expression()?);
            
            if self.peek_kind() == Some(TokenKind::Comma) {
                self.tokens.next()?;
            } else {
                break;
            }
        }
        
        self.expect(TokenKind::RightParen)?;
        Ok(args)
    }
    
    // Helper methods
    
    /// Peek at the next token's kind
    fn peek_kind(&mut self) -> Option<TokenKind> {
        self.tokens.peek().ok().map(|t| t.kind)
    }
    
    /// Peek at the next token without consuming it
    fn peek_token(&mut self) -> ParseResult<&Token> {
        self.tokens.peek()
    }
    
    /// Check if the next token starts a variable
    fn is_variable_start(&mut self) -> bool {
        matches!(
            self.peek_kind(),
            Some(TokenKind::ScalarSigil) | 
            Some(TokenKind::ArraySigil) | 
            Some(TokenKind::HashSigil)
        )
    }
    
    /// Expect a specific token kind
    fn expect(&mut self, kind: TokenKind) -> ParseResult<Token> {
        let token = self.tokens.next()?;
        if token.kind != kind {
            return Err(ParseError::unexpected(
                &format!("{:?}", kind),
                &format!("{:?}", token.kind),
                token.start
            ));
        }
        self.last_end_position = token.end;
        Ok(token)
    }
    
    /// Get current position
    fn current_position(&mut self) -> usize {
        self.tokens.peek().map(|t| t.start).unwrap_or(0)
    }
    
    /// Get previous position
    fn previous_position(&self) -> usize {
        self.last_end_position
    }
    
    /// Consume next token and track position
    fn consume_token(&mut self) -> ParseResult<Token> {
        let token = self.tokens.next()?;
        self.last_end_position = token.end;
        Ok(token)
    }
    
    /// Parse hash literal or block
    fn parse_hash_or_block(&mut self) -> ParseResult<Node> {
        let start_token = self.tokens.next()?; // consume {
        let start = start_token.start;
        
        // Peek ahead to determine if it's a hash or block
        // Empty {} is always a hash ref in expression context
        if self.peek_kind() == Some(TokenKind::RightBrace) {
            self.tokens.next()?; // consume }
            let end = self.previous_position();
            return Ok(Node::new(
                NodeKind::HashLiteral { pairs: Vec::new() },
                SourceLocation { start, end }
            ));
        }
        
        // For now, just parse as block
        // TODO: Implement proper hash vs block disambiguation
        let mut statements = Vec::new();
        
        while self.peek_kind() != Some(TokenKind::RightBrace) && !self.tokens.is_eof() {
            statements.push(self.parse_statement()?);
        }
        
        self.expect(TokenKind::RightBrace)?;
        let end = self.previous_position();
        
        Ok(Node::new(
            NodeKind::Block { statements },
            SourceLocation { start, end }
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_simple_variable() {
        let mut parser = Parser::new("my $x = 42;");
        let result = parser.parse();
        assert!(result.is_ok());
        
        let ast = result.unwrap();
        println!("AST: {}", ast.to_sexp());
    }
    
    #[test]
    fn test_if_statement() {
        let mut parser = Parser::new("if ($x > 10) { print $x; }");
        let result = parser.parse();
        assert!(result.is_ok());
        
        let ast = result.unwrap();
        println!("AST: {}", ast.to_sexp());
    }
    
    #[test]
    fn test_function_definition() {
        let mut parser = Parser::new("sub greet { print \"Hello\"; }");
        let result = parser.parse();
        assert!(result.is_ok());
        
        let ast = result.unwrap();
        println!("AST: {}", ast.to_sexp());
    }
}