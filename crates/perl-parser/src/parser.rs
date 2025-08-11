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
        let token = self.tokens.peek()?.clone();
        
        // Don't check for labels here - it breaks regular identifier parsing
        // Labels will be handled differently
        
        let mut stmt = match token.kind {
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
            TokenKind::Given => self.parse_given_statement(),
            TokenKind::Default => self.parse_default_statement(),
            TokenKind::Try => self.parse_try(),
            
            // Subroutines and modern OOP
            TokenKind::Sub => self.parse_subroutine(),
            TokenKind::Class => self.parse_class(),
            TokenKind::Method => self.parse_method(),
            
            // Package management
            TokenKind::Package => self.parse_package(),
            TokenKind::Use => self.parse_use(),
            TokenKind::No => self.parse_no(),
            
            // Format declarations
            TokenKind::Format => self.parse_format(),
            
            // Phase blocks
            TokenKind::Begin | TokenKind::End | TokenKind::Check | 
            TokenKind::Init | TokenKind::Unitcheck => self.parse_phase_block(),
            
            // Return statement
            TokenKind::Return => self.parse_return(),
            
            // Block
            TokenKind::LeftBrace => self.parse_block(),
            
            // Expression statement
            _ => {
                // Check if this might be a labeled statement
                if self.is_label_start() {
                    return self.parse_labeled_statement();
                }
                
                // Check for indirect object syntax (print STDOUT "hello")
                if let TokenKind::Identifier = token.kind {
                    if self.is_indirect_call_pattern(&token.text) {
                        return self.parse_indirect_call();
                    }
                }
                
                self.parse_expression_statement()
            }
        }?;
        
        // Check for statement modifiers on ANY statement
        if matches!(self.peek_kind(), 
            Some(TokenKind::If) | Some(TokenKind::Unless) | 
            Some(TokenKind::While) | Some(TokenKind::Until) | 
            Some(TokenKind::For) | Some(TokenKind::Foreach)
        ) {
            stmt = self.parse_statement_modifier(stmt)?;
        }
        
        // Consume optional semicolon
        if self.peek_kind() == Some(TokenKind::Semicolon) {
            self.consume_token()?;
        }
        
        Ok(stmt)
    }
    
    /// Check if this might be an indirect call pattern
    fn is_indirect_call_pattern(&mut self, name: &str) -> bool {
        // Known builtins that commonly use indirect object syntax
        let indirect_builtins = [
            "print", "printf", "say", "open", "close", "pipe", "sysopen",
            "sysread", "syswrite", "truncate", "fcntl", "ioctl", "flock",
            "seek", "tell", "select", "binmode", "exec", "system"
        ];
        
        // Check if it's a known builtin
        if indirect_builtins.contains(&name) {
            // Peek at the next token (not second - we're already at the first)
            if let Ok(next) = self.tokens.peek() {
                match next.kind {
                    // print STDOUT ...
                    TokenKind::Identifier => {
                        // Check if it's uppercase (likely a filehandle)
                        if next.text.chars().next().is_some_and(|c| c.is_uppercase()) {
                            return true;
                        }
                    }
                    // print $fh ...
                    _ if next.text.starts_with('$') => return true,
                    _ => {}
                }
            }
        }
        
        // Check for "new ClassName" pattern
        if name == "new" {
            if let Ok(next) = self.tokens.peek() {
                if let TokenKind::Identifier = next.kind {
                    // Uppercase identifier after "new" suggests constructor
                    if next.text.chars().next().is_some_and(|c| c.is_uppercase()) {
                        return true;
                    }
                }
            }
        }
        
        false
    }
    
    /// Parse indirect object/method call
    fn parse_indirect_call(&mut self) -> ParseResult<Node> {
        let start = self.current_position();
        let method_token = self.consume_token()?; // consume method name
        let method = method_token.text.clone();
        
        // Parse the object/filehandle
        let object = self.parse_primary()?;
        
        // Parse remaining arguments
        let mut args = vec![];
        
        // Continue parsing arguments until we hit a statement terminator
        while !matches!(self.peek_kind(), Some(TokenKind::Semicolon) | Some(TokenKind::Eof) | None) &&
              !self.is_statement_modifier_keyword() {
            args.push(self.parse_expression()?);
            
            // Check if we should continue (comma is optional in indirect syntax)
            if self.peek_kind() == Some(TokenKind::Comma) {
                self.tokens.next()?; // consume comma
            } else if matches!(self.peek_kind(), Some(TokenKind::Semicolon) | Some(TokenKind::Eof) | None) ||
                      self.is_statement_modifier_keyword() {
                break;
            }
        }
        
        let end = self.previous_position();
        
        // Return as an indirect call node (using MethodCall with a flag or separate node)
        Ok(Node::new(
            NodeKind::IndirectCall {
                method,
                object: Box::new(object),
                args,
            },
            SourceLocation { start, end }
        ))
    }
    
    /// Check if current token is a statement modifier keyword
    fn is_statement_modifier_keyword(&mut self) -> bool {
        matches!(self.peek_kind(), 
            Some(TokenKind::If) | Some(TokenKind::Unless) | 
            Some(TokenKind::While) | Some(TokenKind::Until) |
            Some(TokenKind::For) | Some(TokenKind::When)
        )
    }
    
    /// Parse variable declaration (my, our, local, state)
    fn parse_variable_declaration(&mut self) -> ParseResult<Node> {
        let start = self.current_position();
        let declarator_token = self.consume_token()?;
        let declarator = declarator_token.text.clone();
        
        // Check if we have a list declaration like `my ($x, $y)`
        if self.peek_kind() == Some(TokenKind::LeftParen) {
            self.consume_token()?; // consume (
            
            let mut variables = Vec::new();
            
            // Parse comma-separated list of variables with their individual attributes
            while self.peek_kind() != Some(TokenKind::RightParen) && !self.tokens.is_eof() {
                let var = self.parse_variable()?;
                
                // Parse optional attributes for this specific variable
                let mut var_attributes = Vec::new();
                while self.peek_kind() == Some(TokenKind::Colon) {
                    self.tokens.next()?; // consume colon
                    let attr_token = self.expect(TokenKind::Identifier)?;
                    var_attributes.push(attr_token.text.clone());
                }
                
                // Create a node that includes both the variable and its attributes
                let var_with_attrs = if var_attributes.is_empty() {
                    var
                } else {
                    let start = var.location.start;
                    let end = self.previous_position();
                    Node::new(
                        NodeKind::VariableWithAttributes {
                            variable: Box::new(var),
                            attributes: var_attributes,
                        },
                        SourceLocation { start, end }
                    )
                };
                
                variables.push(var_with_attrs);
                
                if self.peek_kind() == Some(TokenKind::Comma) {
                    self.consume_token()?; // consume comma
                } else if self.peek_kind() != Some(TokenKind::RightParen) {
                    return Err(ParseError::syntax(
                        "Expected comma or closing parenthesis in variable list",
                        self.current_position()
                    ));
                }
            }
            
            self.expect(TokenKind::RightParen)?; // consume )
            
            // No longer parse attributes here - they're parsed per variable above
            let attributes = Vec::new();
            
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
                NodeKind::VariableListDeclaration {
                    declarator,
                    variables,
                    attributes,
                    initializer,
                },
                SourceLocation { start, end }
            ))
        } else {
            // Single variable declaration
            let variable = self.parse_variable()?;
            
            // Parse optional attributes
            let mut attributes = Vec::new();
            while self.peek_kind() == Some(TokenKind::Colon) {
                self.tokens.next()?; // consume colon
                let attr_token = self.expect(TokenKind::Identifier)?;
                attributes.push(attr_token.text.clone());
            }
            
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
                    attributes,
                    initializer,
                },
                SourceLocation { start, end }
            ))
        }
    }
    
    /// Parse a variable ($foo, @bar, %baz)
    fn parse_variable(&mut self) -> ParseResult<Node> {
        let token = self.consume_token()?;
        
        // The lexer returns variables as identifiers like "$x", "@array", etc.
        // We need to split the sigil from the name
        let text = &token.text;
        
        // Special handling for @{ and %{ (array/hash dereference)
        if text == "@{" || text == "%{" {
            let sigil = text.chars().next().unwrap().to_string();
            let start = token.start;
            
            // Parse the expression inside the braces
            let expr = self.parse_expression()?;
            
            self.expect(TokenKind::RightBrace)?;
            let end = self.previous_position();
            
            let op = format!("{}{{}}", sigil);
            return Ok(Node::new(
                NodeKind::Unary {
                    op,
                    operand: Box::new(expr),
                },
                SourceLocation { start, end }
            ));
        }
        
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
                format!("Expected variable, found '{}'", text),
                token.start
            ));
        };
        
        // Check if the variable name is followed by :: for package-qualified variables
        let mut full_name = name;
        let mut end = token.end;
        
        // Handle :: in package-qualified variables
        while self.peek_kind() == Some(TokenKind::DoubleColon) {
            self.tokens.next()?; // consume ::
            full_name.push_str("::");
            
            // The next part might be an identifier or another variable
            if self.peek_kind() == Some(TokenKind::Identifier) {
                let name_token = self.tokens.next()?;
                full_name.push_str(&name_token.text);
                end = name_token.end;
            } else {
                // Handle cases like $Foo::$bar
                return Err(ParseError::syntax(
                    "Expected identifier after :: in package-qualified variable",
                    self.current_position()
                ));
            }
        }
        
        Ok(Node::new(
            NodeKind::Variable { sigil, name: full_name },
            SourceLocation { start: token.start, end }
        ))
    }
    
    /// Parse a variable when we have a sigil token first
    fn parse_variable_from_sigil(&mut self) -> ParseResult<Node> {
        let sigil_token = self.consume_token()?;
        let sigil = match sigil_token.kind {
            TokenKind::BitwiseAnd => "&".to_string(), // Handle & as sigil
            _ => sigil_token.text.clone(),
        };
        let start = sigil_token.start;
        
        // Check if next token is an identifier or a keyword that should be treated as identifier
        let next_kind = self.peek_kind();
        let (name, end) = if next_kind == Some(TokenKind::Identifier) || 
                             // Keywords that can be used as subroutine names with & sigil
                             (sigil == "&" && matches!(next_kind, 
                                 Some(TokenKind::Sub) | Some(TokenKind::My) | Some(TokenKind::Our) |
                                 Some(TokenKind::If) | Some(TokenKind::Unless) | Some(TokenKind::While) |
                                 Some(TokenKind::For) | Some(TokenKind::Return) | Some(TokenKind::Do) |
                                 Some(TokenKind::Eval) | Some(TokenKind::Use) | Some(TokenKind::Package)
                             )) {
            let name_token = self.tokens.next()?;
            let mut name = name_token.text.clone();
            let mut end = name_token.end;
            
            // Handle :: in package-qualified variables
            while self.peek_kind() == Some(TokenKind::DoubleColon) {
                self.tokens.next()?; // consume ::
                name.push_str("::");
                
                if self.peek_kind() == Some(TokenKind::Identifier) {
                    let next_token = self.tokens.next()?;
                    name.push_str(&next_token.text);
                    end = next_token.end;
                } else {
                    return Err(ParseError::syntax(
                        "Expected identifier after :: in package-qualified variable",
                        self.current_position()
                    ));
                }
            }
            
            (name, end)
        } else {
            // Handle special variables like $$, $@, $!, $?, etc.
            match self.peek_kind() {
                Some(TokenKind::ScalarSigil) => {
                    // $$ - process ID
                    let token = self.tokens.next()?;
                    ("$".to_string(), token.end)
                }
                Some(TokenKind::ArraySigil) => {
                    // $@ - eval error
                    let token = self.tokens.next()?;
                    ("@".to_string(), token.end)
                }
                Some(TokenKind::Not) => {
                    // $! - system error
                    let token = self.tokens.next()?;
                    ("!".to_string(), token.end)
                }
                Some(TokenKind::Unknown) => {
                    // Could be $?, $^, $#, or other special
                    let token = self.tokens.peek()?;
                    match token.text.as_str() {
                        "?" => {
                            let token = self.tokens.next()?;
                            ("?".to_string(), token.end)
                        }
                        "^" => {
                            // Handle $^X variables
                            let token = self.tokens.next()?;
                            if self.peek_kind() == Some(TokenKind::Identifier) {
                                let var_token = self.tokens.next()?;
                                (format!("^{}", var_token.text), var_token.end)
                            } else {
                                ("^".to_string(), token.end)
                            }
                        }
                        "#" => {
                            // Handle $# (array length) 
                            let token = self.tokens.next()?;
                            if self.peek_kind() == Some(TokenKind::Identifier) {
                                let var_token = self.tokens.next()?;
                                (format!("#{}", var_token.text), var_token.end)
                            } else {
                                // Just $# by itself
                                ("#".to_string(), token.end)
                            }
                        }
                        _ => {
                            return Err(ParseError::syntax(
                                format!("Unexpected character after sigil: {}", token.text),
                                token.start
                            ));
                        }
                    }
                }
                Some(TokenKind::Number) => {
                    // $0, $1, $2, etc. - numbered capture groups
                    let num_token = self.tokens.next()?;
                    (num_token.text.clone(), num_token.end)
                }
                _ => {
                    // Empty variable name (just the sigil)
                    (String::new(), self.previous_position())
                }
            }
        };
        
        // Special handling for @ or % sigil followed by { - array/hash dereference
        if (sigil == "@" || sigil == "%") && self.peek_kind() == Some(TokenKind::LeftBrace) {
            self.tokens.next()?; // consume {
            
            // Parse the expression inside the braces
            let expr = self.parse_expression()?;
            
            self.expect(TokenKind::RightBrace)?;
            let end = self.previous_position();
            
            let op = format!("{}{{}}", sigil);
            return Ok(Node::new(
                NodeKind::Unary {
                    op,
                    operand: Box::new(expr),
                },
                SourceLocation { start, end }
            ));
        }
        
        // Special handling for & sigil - it's a function call
        if sigil == "&" {
            // Check if there are parentheses for arguments
            let args = if self.peek_kind() == Some(TokenKind::LeftParen) {
                self.tokens.next()?; // consume (
                let mut args = vec![];
                
                while self.peek_kind() != Some(TokenKind::RightParen) {
                    args.push(self.parse_expression()?);
                    
                    if self.peek_kind() == Some(TokenKind::Comma) {
                        self.tokens.next()?; // consume comma
                    } else if self.peek_kind() != Some(TokenKind::RightParen) {
                        return Err(ParseError::syntax("Expected comma or right parenthesis", self.current_position()));
                    }
                }
                
                let right_paren = self.expect(TokenKind::RightParen)?;
                let _end = right_paren.end;
                args
            } else {
                vec![]
            };
            
            Ok(Node::new(
                NodeKind::FunctionCall { name, args },
                SourceLocation { start, end }
            ))
        } else {
            Ok(Node::new(
                NodeKind::Variable { sigil, name },
                SourceLocation { start, end }
            ))
        }
    }
    
    /// Parse if statement
    fn parse_if_statement(&mut self) -> ParseResult<Node> {
        let start = self.current_position();
        self.tokens.next()?; // consume 'if'
        
        self.expect(TokenKind::LeftParen)?;
        
        // Check if this is a variable declaration in the condition
        let condition = if matches!(self.peek_kind(), Some(TokenKind::My) | Some(TokenKind::Our) | Some(TokenKind::Local) | Some(TokenKind::State)) {
            self.parse_variable_declaration()?
        } else {
            self.parse_expression()?
        };
        
        self.expect(TokenKind::RightParen)?;
        
        let then_branch = self.parse_block()?;
        
        let mut elsif_branches = Vec::new();
        let mut else_branch = None;
        
        // Handle elsif chains
        while self.peek_kind() == Some(TokenKind::Elsif) {
            self.tokens.next()?; // consume 'elsif'
            self.expect(TokenKind::LeftParen)?;
            
            // Check if this is a variable declaration in the condition
            let elsif_cond = if matches!(self.peek_kind(), Some(TokenKind::My) | Some(TokenKind::Our) | Some(TokenKind::Local) | Some(TokenKind::State)) {
                self.parse_variable_declaration()?
            } else {
                self.parse_expression()?
            };
            
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
        
        // Check if this is a variable declaration in the condition
        let condition = if matches!(self.peek_kind(), Some(TokenKind::My) | Some(TokenKind::Our) | Some(TokenKind::Local) | Some(TokenKind::State)) {
            self.parse_variable_declaration()?
        } else {
            self.parse_expression()?
        };
        
        self.expect(TokenKind::RightParen)?;
        
        let body = self.parse_block()?;
        
        // Handle continue block
        let continue_block = if self.peek_kind() == Some(TokenKind::Continue) {
            self.tokens.next()?; // consume 'continue'
            Some(Box::new(self.parse_block()?))
        } else {
            None
        };
        
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
        
        // Parse init (or check if it's a foreach)
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
            // Parse expression
            let expr = self.parse_expression()?;
            
            // If followed by ), it's a foreach loop  
            if self.peek_kind() == Some(TokenKind::RightParen) {
                self.tokens.next()?; // consume )
                let body = self.parse_block()?;
                
                let end = self.previous_position();
                
                // Create implicit $_ variable
                let implicit_var = Node::new(
                    NodeKind::Variable { 
                        sigil: "$".to_string(),
                        name: "_".to_string()
                    },
                    SourceLocation { start, end: start }
                );
                
                return Ok(Node::new(
                    NodeKind::Foreach {
                        variable: Box::new(implicit_var),
                        list: Box::new(expr),
                        body: Box::new(body),
                    },
                    SourceLocation { start, end }
                ));
            }
            
            Some(Box::new(expr))
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
        
        // Handle continue block
        let continue_block = if self.peek_kind() == Some(TokenKind::Continue) {
            self.tokens.next()?; // consume 'continue'
            Some(Box::new(self.parse_block()?))
        } else {
            None
        };
        
        let end = self.previous_position();
        Ok(Node::new(
            NodeKind::For {
                init,
                condition,
                update,
                body: Box::new(body),
                continue_block,
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
        
        let name = match self.peek_kind() {
            // Regular identifier
            Some(TokenKind::Identifier) => Some(self.tokens.next()?.text.clone()),
            // Keywords that can be used as subroutine names
            Some(TokenKind::Method) | Some(TokenKind::Class) |
            Some(TokenKind::Try) | Some(TokenKind::Catch) | Some(TokenKind::Finally) |
            Some(TokenKind::Given) | Some(TokenKind::When) | Some(TokenKind::Default) |
            Some(TokenKind::Continue) | Some(TokenKind::Format) => {
                Some(self.tokens.next()?.text.clone())
            }
            // No name - anonymous subroutine
            _ => None
        };
        
        // Parse optional attributes first (they come before signature in modern Perl)
        let mut attributes = Vec::new();
        while self.peek_kind() == Some(TokenKind::Colon) {
            self.tokens.next()?; // consume colon
            
            // Parse one or more space-separated attributes after the colon
            loop {
                // Attributes can be identifiers or certain keywords
                let attr_token = match self.peek_kind() {
                    Some(TokenKind::Identifier) | Some(TokenKind::Method) => {
                        self.tokens.next()?
                    }
                    _ => {
                        // If it's not an attribute name, we're done with this attribute list
                        break;
                    }
                };
                
                let mut attr_name = attr_token.text.clone();
                
                // Check if attribute has a value in parentheses (like :prototype($))
                if self.peek_kind() == Some(TokenKind::LeftParen) {
                    self.tokens.next()?; // consume (
                    attr_name.push('(');
                    
                    // Collect tokens until matching )
                    let mut paren_depth = 1;
                    while paren_depth > 0 && !self.tokens.is_eof() {
                        let token = self.tokens.next()?;
                        attr_name.push_str(&token.text);
                        
                        match token.kind {
                            TokenKind::LeftParen => paren_depth += 1,
                            TokenKind::RightParen => {
                                paren_depth -= 1;
                                if paren_depth == 0 {
                                    attr_name.push(')');
                                }
                            }
                            _ => {}
                        }
                    }
                }
                
                attributes.push(attr_name);
                
                // Check if there's another attribute (not preceded by colon)
                match self.peek_kind() {
                    Some(TokenKind::Identifier) | Some(TokenKind::Method) => {
                        // Continue parsing more attributes
                        continue;
                    }
                    _ => break,
                }
            }
        }
        
        // Parse optional prototype or signature after attributes
        let (params, _prototype) = if self.peek_kind() == Some(TokenKind::LeftParen) {
            // Look ahead to determine if this is a prototype or signature
            if self.is_likely_prototype()? {
                // Parse as prototype
                let proto = self.parse_prototype()?;
                // Store prototype as an attribute
                attributes.push(format!("prototype({})", proto));
                (Vec::new(), Some(proto))
            } else {
                (self.parse_signature()?, None)
            }
        } else {
            (Vec::new(), None)
        };
        
        let body = self.parse_block()?;
        
        let end = self.previous_position();
        Ok(Node::new(
            NodeKind::Subroutine {
                name,
                params,
                attributes,
                body: Box::new(body),
            },
            SourceLocation { start, end }
        ))
    }
    
    /// Parse class declaration (Perl 5.38+)
    fn parse_class(&mut self) -> ParseResult<Node> {
        let start = self.current_position();
        self.tokens.next()?; // consume 'class'
        
        let name_token = self.expect(TokenKind::Identifier)?;
        let name = name_token.text.clone();
        
        let body = self.parse_block()?;
        
        let end = self.previous_position();
        Ok(Node::new(
            NodeKind::Class {
                name,
                body: Box::new(body),
            },
            SourceLocation { start, end }
        ))
    }
    
    /// Parse method declaration (Perl 5.38+)
    fn parse_method(&mut self) -> ParseResult<Node> {
        let start = self.current_position();
        self.tokens.next()?; // consume 'method'
        
        let name_token = self.expect(TokenKind::Identifier)?;
        let name = name_token.text.clone();
        
        // Parse optional signature
        let params = if self.peek_kind() == Some(TokenKind::LeftParen) {
            self.parse_signature()?
        } else {
            Vec::new()
        };
        
        let body = self.parse_block()?;
        
        let end = self.previous_position();
        Ok(Node::new(
            NodeKind::Method {
                name,
                params,
                body: Box::new(body),
            },
            SourceLocation { start, end }
        ))
    }
    
    /// Parse format declaration
    fn parse_format(&mut self) -> ParseResult<Node> {
        let start = self.current_position();
        self.tokens.next()?; // consume 'format'
        
        // Parse format name (optional - can be anonymous)
        let name = if self.peek_kind() == Some(TokenKind::Assign) {
            // Anonymous format
            String::new()
        } else {
            // Named format
            let name_token = self.expect(TokenKind::Identifier)?;
            name_token.text.clone()
        };
        
        // Expect =
        self.expect(TokenKind::Assign)?;
        
        // Tell the lexer to enter format body mode
        self.tokens.enter_format_mode();
        
        // Get the format body
        let body_token = self.tokens.next()?;
        let body = if body_token.kind == TokenKind::FormatBody {
            body_token.text
        } else {
            return Err(ParseError::UnexpectedToken {
                expected: "format body".to_string(),
                found: format!("{:?}", body_token.kind),
                location: body_token.start,
            });
        };
        
        let end = self.previous_position();
        Ok(Node::new(
            NodeKind::Format { name, body },
            SourceLocation { start, end }
        ))
    }
    
    /// Parse subroutine signature
    fn parse_signature(&mut self) -> ParseResult<Vec<Node>> {
        self.expect(TokenKind::LeftParen)?; // consume (
        let mut params = Vec::new();
        
        while self.peek_kind() != Some(TokenKind::RightParen) && !self.tokens.is_eof() {
            // Parse parameter
            let param = self.parse_signature_param()?;
            params.push(param);
            
            // Check for comma or end of signature
            if self.peek_kind() == Some(TokenKind::Comma) {
                self.tokens.next()?; // consume comma
            } else if self.peek_kind() == Some(TokenKind::RightParen) {
                break;
            } else {
                return Err(ParseError::syntax(
                    "Expected comma or closing parenthesis in signature",
                    self.current_position()
                ));
            }
        }
        
        self.expect(TokenKind::RightParen)?; // consume )
        Ok(params)
    }
    
    /// Parse a single signature parameter
    fn parse_signature_param(&mut self) -> ParseResult<Node> {
        let start = self.current_position();
        
        // Check for named parameter (:$name)
        let named = if self.peek_kind() == Some(TokenKind::Colon) {
            self.tokens.next()?; // consume :
            true
        } else {
            false
        };
        
        // Check for type constraint (Type $var)
        let type_constraint = if self.peek_kind() == Some(TokenKind::Identifier) {
            // Look ahead to see if this is a type constraint
            let token = self.tokens.peek()?;
            if !token.text.starts_with('$') && !token.text.starts_with('@') && 
               !token.text.starts_with('%') && !token.text.starts_with('&') {
                // It's likely a type constraint
                Some(self.tokens.next()?.text.clone())
            } else {
                None
            }
        } else {
            None
        };
        
        // Parse the variable
        let variable = self.parse_variable()?;
        
        // Check for default value (= expression)
        let default_value = if self.peek_kind() == Some(TokenKind::Assign) {
            self.tokens.next()?; // consume =
            // Parse a primary expression for default value to avoid parsing too far
            Some(Box::new(self.parse_primary()?))
        } else {
            None
        };
        
        let end = if let Some(ref default) = default_value {
            default.location.end
        } else {
            variable.location.end
        };
        
        // Create a parameter node
        // For now, we'll use the Variable node with additional context
        // In a full implementation, we might want a dedicated Parameter node kind
        if named || type_constraint.is_some() || default_value.is_some() {
            // We need to wrap this in a more complex structure
            // For now, let's use a Block node to hold the parameter info
            let mut statements = vec![variable];
            
            // Add type constraint as an identifier if present
            if let Some(tc) = type_constraint {
                let tc_node = Node::new(
                    NodeKind::Identifier { name: tc },
                    SourceLocation { start, end: start }
                );
                statements.insert(0, tc_node);
            }
            
            // Add default value if present
            if let Some(default) = default_value {
                statements.push(*default);
            }
            
            Ok(Node::new(
                NodeKind::Block { statements },
                SourceLocation { start, end }
            ))
        } else {
            // Simple parameter, just return the variable
            Ok(variable)
        }
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
            
            // Check if there's an identifier after ::
            // If not, it's a trailing :: which is valid in Perl
            if self.peek_kind() == Some(TokenKind::Identifier) {
                name.push_str(&self.tokens.next()?.text);
            } else {
                // Trailing :: is valid, just break
                break;
            }
        }
        
        // Check for optional version number or v-string
        let version = if self.peek_kind() == Some(TokenKind::Number) {
            Some(self.tokens.next()?.text.clone())
        } else if let Some(TokenKind::Identifier) = self.peek_kind() {
            // Check if it's a v-string version
            if let Ok(token) = self.tokens.peek() {
                if token.text.starts_with('v') && token.text.len() > 1 {
                    // It's a v-string like v1 or v5
                    let mut version_str = self.tokens.next()?.text.clone();
                    
                    // Collect the rest of the v-string (e.g., .2.3)
                    while let Some(TokenKind::Number) = self.peek_kind() {
                        if let Ok(num_token) = self.tokens.peek() {
                            if num_token.text.starts_with('.') {
                                version_str.push_str(&self.tokens.next()?.text);
                            } else {
                                break;
                            }
                        } else {
                            break;
                        }
                    }
                    
                    Some(version_str)
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };
        
        // If we have a version, append it to the name for now
        // (In a real AST, you'd probably want these as separate fields)
        if let Some(ver) = version {
            name.push(' ');
            name.push_str(&ver);
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
        
        // Parse module name, version, or identifier
        let mut module = if self.peek_kind() == Some(TokenKind::Number) {
            // Numeric version like 5.036
            self.tokens.next()?.text.clone()
        } else {
            let first_token = self.tokens.next()?;
            
            // Check for version strings
            if first_token.kind == TokenKind::Identifier && first_token.text.starts_with('v') && 
               first_token.text.chars().skip(1).all(|c| c.is_numeric()) {
                // Version identifier like v5 or v536
                let mut version = first_token.text.clone();
                
                // Check if followed by dot and more numbers (e.g., v5.36)
                if self.peek_kind() == Some(TokenKind::Unknown) {
                    if let Ok(dot_token) = self.tokens.peek() {
                        if dot_token.text == "." {
                            self.tokens.next()?; // consume dot
                            if self.peek_kind() == Some(TokenKind::Number) {
                                let num = self.tokens.next()?;
                                version.push('.');
                                version.push_str(&num.text);
                            }
                        }
                    }
                }
                version
            } else if first_token.text == "v" && self.peek_kind() == Some(TokenKind::Number) {
                // Version string like v5.36 (tokenized as "v" followed by number)
                let version = self.expect(TokenKind::Number)?;
                format!("v{}", version.text)
            } else if first_token.kind == TokenKind::Identifier {
                first_token.text.clone()
            } else {
                return Err(ParseError::syntax(
                    format!("Expected module name or version, found {:?}", first_token.kind),
                    first_token.start
                ));
            }
        };
        
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
        
        // Handle hash syntax for pragmas like: use constant { FOO => 42, BAR => 43 }
        if self.peek_kind() == Some(TokenKind::LeftBrace) {
            // Just consume the entire hash expression as args
            let mut depth = 0;
            while !self.tokens.is_eof() {
                match self.peek_kind() {
                    Some(TokenKind::LeftBrace) => {
                        depth += 1;
                        args.push(self.tokens.next()?.text.clone());
                    }
                    Some(TokenKind::RightBrace) => {
                        args.push(self.tokens.next()?.text.clone());
                        depth -= 1;
                        if depth == 0 {
                            break;
                        }
                    }
                    _ => {
                        args.push(self.tokens.next()?.text.clone());
                    }
                }
            }
        }
        // Handle bare arguments (no parentheses)
        else if matches!(self.peek_kind(), Some(TokenKind::String) | Some(TokenKind::Identifier)) &&
           !matches!(self.peek_kind(), Some(TokenKind::Semicolon) | Some(TokenKind::Eof) | None) {
            // Parse bare arguments like: use warnings 'void' or use constant FOO => 42
            loop {
                match self.peek_kind() {
                    Some(TokenKind::String) => {
                        args.push(self.tokens.next()?.text.clone());
                    }
                    Some(TokenKind::Identifier) if self.tokens.peek()?.text == "qw" => {
                        // Handle qw()
                        self.tokens.next()?; // consume qw
                        if self.peek_kind() == Some(TokenKind::LeftParen) {
                            self.tokens.next()?; // consume (
                            while self.peek_kind() != Some(TokenKind::RightParen) && !self.tokens.is_eof() {
                                if let Some(TokenKind::Identifier) = self.peek_kind() {
                                    args.push(self.tokens.next()?.text.clone());
                                }
                            }
                            self.expect(TokenKind::RightParen)?;
                        }
                    }
                    Some(TokenKind::Identifier) => {
                        // Check if this might be a constant declaration
                        let ident = self.tokens.next()?;
                        args.push(ident.text.clone());
                        
                        // If we see a fat arrow after an identifier, parse the value
                        if self.peek_kind() == Some(TokenKind::FatArrow) {
                            self.tokens.next()?; // consume =>
                            // Parse the value as a simple expression
                            match self.peek_kind() {
                                Some(TokenKind::Number) | Some(TokenKind::String) => {
                                    args.push(self.tokens.next()?.text.clone());
                                }
                                Some(TokenKind::Identifier) => {
                                    args.push(self.tokens.next()?.text.clone());
                                }
                                _ => {
                                    // For more complex expressions, just consume tokens until semicolon
                                    while !matches!(self.peek_kind(), Some(TokenKind::Semicolon) | Some(TokenKind::Eof) | None) {
                                        args.push(self.tokens.next()?.text.clone());
                                    }
                                }
                            }
                        }
                    }
                    _ => break,
                }
                
                // Check if we should continue parsing arguments
                if matches!(self.peek_kind(), Some(TokenKind::Semicolon) | Some(TokenKind::Eof) | None) {
                    break;
                }
            }
        } else if self.peek_kind() == Some(TokenKind::LeftParen) {
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
        
        // Semicolon is optional at EOF
        if !matches!(self.peek_kind(), Some(TokenKind::Eof) | None) {
            self.expect(TokenKind::Semicolon)?;
        }
        
        let end = self.previous_position();
        Ok(Node::new(
            NodeKind::Use { module, args },
            SourceLocation { start, end }
        ))
    }
    
    /// Parse special block (AUTOLOAD, DESTROY, etc.)
    fn parse_special_block(&mut self) -> ParseResult<Node> {
        let start = self.current_position();
        let name_token = self.consume_token()?;
        let name = name_token.text.clone();
        
        let block = self.parse_block()?;
        let end = block.location.end;
        
        // Treat as a special subroutine
        Ok(Node::new(
            NodeKind::Subroutine {
                name: Some(name),
                params: vec![],
                attributes: vec![],
                body: Box::new(block),
            },
            SourceLocation { start, end }
        ))
    }
    
    /// Parse phase block (BEGIN, END, CHECK, INIT, UNITCHECK)
    fn parse_phase_block(&mut self) -> ParseResult<Node> {
        let start = self.current_position();
        let phase_token = self.consume_token()?;
        let phase = phase_token.text.clone();
        
        // Phase blocks must be followed by a block
        if self.peek_kind() != Some(TokenKind::LeftBrace) {
            return Err(ParseError::syntax(
                format!("{} must be followed by a block", phase),
                self.current_position()
            ));
        }
        
        let block = self.parse_block()?;
        let end = block.location.end;
        
        // Create a special node for phase blocks
        Ok(Node::new(
            NodeKind::PhaseBlock { 
                phase,
                block: Box::new(block),
            },
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
        
        // Handle bare arguments (no parentheses)
        if matches!(self.peek_kind(), Some(TokenKind::String) | Some(TokenKind::Identifier)) &&
           !matches!(self.peek_kind(), Some(TokenKind::Semicolon) | Some(TokenKind::Eof) | None) {
            // Parse bare arguments like: no warnings 'void'
            loop {
                match self.peek_kind() {
                    Some(TokenKind::String) => {
                        args.push(self.tokens.next()?.text.clone());
                    }
                    Some(TokenKind::Identifier) if self.tokens.peek()?.text == "qw" => {
                        // Handle qw()
                        self.tokens.next()?; // consume qw
                        if self.peek_kind() == Some(TokenKind::LeftParen) {
                            self.tokens.next()?; // consume (
                            while self.peek_kind() != Some(TokenKind::RightParen) && !self.tokens.is_eof() {
                                if let Some(TokenKind::Identifier) = self.peek_kind() {
                                    args.push(self.tokens.next()?.text.clone());
                                }
                            }
                            self.expect(TokenKind::RightParen)?;
                        }
                    }
                    Some(TokenKind::Identifier) => {
                        args.push(self.tokens.next()?.text.clone());
                    }
                    _ => break,
                }
                
                // Check if we should continue parsing arguments
                if matches!(self.peek_kind(), Some(TokenKind::Semicolon) | Some(TokenKind::Eof) | None) {
                    break;
                }
            }
        } else if self.peek_kind() == Some(TokenKind::LeftParen) {
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
        
        // Semicolon is optional at EOF
        if !matches!(self.peek_kind(), Some(TokenKind::Eof) | None) {
            self.expect(TokenKind::Semicolon)?;
        }
        
        let end = self.previous_position();
        Ok(Node::new(
            NodeKind::No { module, args },
            SourceLocation { start, end }
        ))
    }
    
    /// Parse format declaration
    /// Parse return statement
    fn parse_return(&mut self) -> ParseResult<Node> {
        let start = self.current_position();
        self.tokens.next()?; // consume 'return'
        
        // Check if we have a value to return - only stop at clear ends or statement modifiers
        let value = if matches!(self.peek_kind(), 
            Some(TokenKind::Semicolon) | Some(TokenKind::RightBrace) | 
            Some(TokenKind::Eof) | None |
            // Statement modifiers - these will be handled at statement level
            Some(TokenKind::If) | Some(TokenKind::Unless) |
            Some(TokenKind::While) | Some(TokenKind::Until) |
            Some(TokenKind::For) | Some(TokenKind::Foreach)
        ) {
            None
        } else {
            // Parse the return value
            Some(Box::new(self.parse_expression()?))
        };
        
        let end = value.as_ref().map(|v| v.location.end).unwrap_or(self.previous_position());
        Ok(Node::new(
            NodeKind::Return { value },
            SourceLocation { start, end }
        ))
    }
    
    /// Parse eval expression/block
    fn parse_eval(&mut self) -> ParseResult<Node> {
        let start = self.consume_token()?.start; // consume 'eval'
        
        // Eval can take either a block or a string expression
        if self.peek_kind() == Some(TokenKind::LeftBrace) {
            // eval { ... }
            let block = self.parse_block()?;
            let end = block.location.end;
            Ok(Node::new(
                NodeKind::Eval { block: Box::new(block) },
                SourceLocation { start, end }
            ))
        } else {
            // eval "string" or eval $expr
            let expr = self.parse_expression()?;
            let end = expr.location.end;
            Ok(Node::new(
                NodeKind::Eval { block: Box::new(expr) },
                SourceLocation { start, end }
            ))
        }
    }
    
    /// Parse try/catch/finally block
    fn parse_try(&mut self) -> ParseResult<Node> {
        let start = self.consume_token()?.start; // consume 'try'
        
        // Parse the try body
        let body = self.parse_block()?;
        
        let mut catch_blocks = Vec::new();
        let mut finally_block = None;
        
        // Parse catch blocks
        while self.peek_kind() == Some(TokenKind::Catch) {
            self.consume_token()?; // consume 'catch'
            
            // Check for optional variable
            let var = if self.peek_kind() == Some(TokenKind::LeftParen) {
                self.consume_token()?; // consume '('
                let var_name = if self.peek_kind() == Some(TokenKind::ScalarSigil) || 
                               self.tokens.peek()?.text.starts_with('$') {
                    let var = self.parse_variable()?;
                    match &var.kind {
                        NodeKind::Variable { sigil, name } => Some(format!("{}{}", sigil, name)),
                        _ => None,
                    }
                } else {
                    None
                };
                self.expect(TokenKind::RightParen)?;
                var_name
            } else {
                None
            };
            
            let block = self.parse_block()?;
            catch_blocks.push((var, block));
        }
        
        // Parse optional finally block
        if self.peek_kind() == Some(TokenKind::Finally) {
            self.consume_token()?; // consume 'finally'
            finally_block = Some(Box::new(self.parse_block()?));
        }
        
        let end = finally_block.as_ref()
            .map(|b| b.location.end)
            .or_else(|| catch_blocks.last().map(|(_, b)| b.location.end))
            .unwrap_or(body.location.end);
        
        Ok(Node::new(
            NodeKind::Try {
                body: Box::new(body),
                catch_blocks: catch_blocks.into_iter()
                    .map(|(v, b)| (v, Box::new(b)))
                    .collect(),
                finally_block,
            },
            SourceLocation { start, end }
        ))
    }
    
    /// Parse do expression/block
    fn parse_do(&mut self) -> ParseResult<Node> {
        let start = self.consume_token()?.start; // consume 'do'
        
        // Do can take either a block or a string (filename)
        if self.peek_kind() == Some(TokenKind::LeftBrace) {
            // do { ... }
            let block = self.parse_block()?;
            let end = block.location.end;
            Ok(Node::new(
                NodeKind::Do { block: Box::new(block) },
                SourceLocation { start, end }
            ))
        } else {
            // do "filename" or do $expr
            let expr = self.parse_expression()?;
            let end = expr.location.end;
            Ok(Node::new(
                NodeKind::Do { block: Box::new(expr) },
                SourceLocation { start, end }
            ))
        }
    }
    
    /// Parse given statement
    fn parse_given_statement(&mut self) -> ParseResult<Node> {
        let start = self.consume_token()?.start; // consume 'given'
        
        // Parse the expression in parentheses
        self.expect(TokenKind::LeftParen)?;
        let expr = self.parse_expression()?;
        self.expect(TokenKind::RightParen)?;
        
        // Parse the body block
        let body = self.parse_given_block()?;
        let end = body.location.end;
        
        Ok(Node::new(
            NodeKind::Given {
                expr: Box::new(expr),
                body: Box::new(body),
            },
            SourceLocation { start, end }
        ))
    }
    
    /// Parse given block (which contains when/default statements)
    fn parse_given_block(&mut self) -> ParseResult<Node> {
        let start = self.current_position();
        self.expect(TokenKind::LeftBrace)?;
        
        let mut statements = Vec::new();
        
        while self.peek_kind() != Some(TokenKind::RightBrace) && !self.tokens.is_eof() {
            match self.peek_kind() {
                Some(TokenKind::When) => {
                    statements.push(self.parse_when_statement()?);
                }
                Some(TokenKind::Default) => {
                    statements.push(self.parse_default_statement()?);
                }
                _ => {
                    return Err(ParseError::syntax(
                        "Expected 'when' or 'default' in given block",
                        self.current_position()
                    ));
                }
            }
        }
        
        self.expect(TokenKind::RightBrace)?;
        let end = self.previous_position();
        
        Ok(Node::new(
            NodeKind::Block { statements },
            SourceLocation { start, end }
        ))
    }
    
    /// Parse when statement
    fn parse_when_statement(&mut self) -> ParseResult<Node> {
        let start = self.consume_token()?.start; // consume 'when'
        
        // Parse the condition in parentheses
        self.expect(TokenKind::LeftParen)?;
        let condition = self.parse_expression()?;
        self.expect(TokenKind::RightParen)?;
        
        // Parse the body block
        let body = self.parse_block()?;
        let end = body.location.end;
        
        Ok(Node::new(
            NodeKind::When {
                condition: Box::new(condition),
                body: Box::new(body),
            },
            SourceLocation { start, end }
        ))
    }
    
    /// Parse default statement
    fn parse_default_statement(&mut self) -> ParseResult<Node> {
        let start = self.consume_token()?.start; // consume 'default'
        
        // Parse the body block
        let body = self.parse_block()?;
        let end = body.location.end;
        
        Ok(Node::new(
            NodeKind::Default {
                body: Box::new(body),
            },
            SourceLocation { start, end }
        ))
    }
    
    
    /// Parse expression statement
    fn parse_expression_statement(&mut self) -> ParseResult<Node> {
        // Check for special blocks like AUTOLOAD and DESTROY
        if let Ok(token) = self.tokens.peek() {
            if matches!(token.text.as_str(), "AUTOLOAD" | "DESTROY" | "CLONE" | "CLONE_SKIP") {
                // Check if next token is a block
                if let Ok(second) = self.tokens.peek_second() {
                    if second.kind == TokenKind::LeftBrace {
                        return self.parse_special_block();
                    }
                }
            }
        }
        
        // First, try to parse the initial part as a simple statement
        let mut expr = self.parse_simple_statement()?;
        
        // Check for word operators (or, and, xor) which have very low precedence
        expr = self.parse_word_or_expr(expr)?;
        
        // Statement modifiers are now handled at the statement level
        
        Ok(expr)
    }
    
    /// Parse simple statement (print, die, next, last, etc. with their arguments)
    fn parse_simple_statement(&mut self) -> ParseResult<Node> {
        // Check if it's a builtin that can take arguments without parens
        if let Ok(token) = self.tokens.peek() {
            match token.text.as_ref() {
            "print" | "say" | "die" | "warn" | "return" | "next" | "last" | "redo" | "open" | "tie" |
            "printf" | "close" | "pipe" | "sysopen" | "sysread" | "syswrite" | "truncate" |
            "fcntl" | "ioctl" | "flock" | "seek" | "tell" | "select" | "binmode" | "exec" | "system" |
            "bless" | "ref" | "defined" | "undef" => {
                let start = token.start;
                let func_name = token.text.clone();
                
                // Check for indirect object syntax before consuming the token
                if self.is_indirect_call_pattern(&func_name) {
                    return self.parse_indirect_call();
                }
                
                // Consume the function name token
                self.consume_token()?;
                
                // Check if there are arguments (not followed by semicolon or modifier)
                match self.peek_kind() {
                    Some(TokenKind::Semicolon) | Some(TokenKind::If) | Some(TokenKind::Unless) |
                    Some(TokenKind::While) | Some(TokenKind::Until) | Some(TokenKind::For) | 
                    Some(TokenKind::Foreach) | Some(TokenKind::RightBrace) | Some(TokenKind::Eof) | None => {
                        // No arguments - return as function call with empty args
                        let end = self.previous_position();
                        Ok(Node::new(
                            NodeKind::FunctionCall { 
                                name: func_name,
                                args: vec![]
                            },
                            SourceLocation { start, end }
                        ))
                    }
                    _ => {
                        // Has arguments - parse them as a comma-separated list
                        let mut args = vec![];
                        
                        // Parse first argument
                        // Special handling for open/pipe/socket/tie which can take my $var as first arg
                        if (func_name == "open" || func_name == "pipe" || func_name == "socket" || func_name == "tie") && 
                           self.peek_kind() == Some(TokenKind::My) {
                            args.push(self.parse_variable_declaration()?);
                        } else {
                            // For builtins, don't parse word operators as part of arguments
                            // Word operators should be handled at statement level
                            args.push(self.parse_assignment()?);
                        }
                        
                        // Parse remaining arguments
                        while self.peek_kind() == Some(TokenKind::Comma) {
                            self.consume_token()?; // consume comma
                            
                            // Check if we hit a statement modifier
                            match self.peek_kind() {
                                Some(TokenKind::If) | Some(TokenKind::Unless) |
                                Some(TokenKind::While) | Some(TokenKind::Until) | 
                                Some(TokenKind::For) | Some(TokenKind::Foreach) => break,
                                _ => args.push(self.parse_assignment()?)
                            }
                        }
                        
                        let end = args.last().map(|a| a.location.end).unwrap_or(start);
                        
                        Ok(Node::new(
                            NodeKind::FunctionCall {
                                name: func_name,
                                args,
                            },
                            SourceLocation { start, end }
                        ))
                    }
                }
            }
                "new" => {
                    // Check for indirect constructor syntax
                    let _start = token.start;
                    let func_name = token.text.clone();
                    
                    if self.is_indirect_call_pattern(&func_name) {
                        return self.parse_indirect_call();
                    }
                    
                    // Otherwise parse as regular expression
                    self.parse_expression()
                }
                _ => {
                    // Regular expression
                    self.parse_expression()
                }
            }
        } else {
            // Regular expression
            self.parse_expression()
        }
    }
    
    /// Parse statement modifier (if, unless, while, until, for)
    fn parse_statement_modifier(&mut self, statement: Node) -> ParseResult<Node> {
        let modifier_token = self.consume_token()?;
        let modifier = modifier_token.text.clone();
        
        // For 'for' and 'foreach', we parse a list expression
        let condition = if matches!(modifier_token.kind, TokenKind::For | TokenKind::Foreach) {
            self.parse_expression()?
        } else {
            // For other modifiers, parse a regular expression
            self.parse_expression()?
        };
        
        let start = statement.location.start;
        let end = condition.location.end;
        
        Ok(Node::new(
            NodeKind::StatementModifier {
                statement: Box::new(statement),
                modifier,
                condition: Box::new(condition),
            },
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
    
    /// Parse comma operator (lowest precedence except for word operators)
    fn parse_comma(&mut self) -> ParseResult<Node> {
        let mut expr = self.parse_assignment()?;
        
        // In scalar context, comma creates a list
        // For now, we'll just parse it as sequential expressions
        // Also handle fat arrow (=>) which acts like comma
        if self.peek_kind() == Some(TokenKind::Comma) || self.peek_kind() == Some(TokenKind::FatArrow) {
            let mut expressions = vec![expr];
            
            // Handle initial fat arrow
            if self.peek_kind() == Some(TokenKind::FatArrow) {
                self.tokens.next()?; // consume =>
                expressions.push(self.parse_assignment()?);
            }
            
            while self.peek_kind() == Some(TokenKind::Comma) || self.peek_kind() == Some(TokenKind::FatArrow) {
                if self.peek_kind() == Some(TokenKind::Comma) {
                    self.tokens.next()?; // consume comma
                }
                
                // Check for end of expression
                match self.peek_kind() {
                    Some(TokenKind::Semicolon) | Some(TokenKind::RightParen) | 
                    Some(TokenKind::RightBrace) | Some(TokenKind::RightBracket) => break,
                    _ => {}
                }
                
                let elem = self.parse_assignment()?;
                
                // Check for fat arrow after element
                if self.peek_kind() == Some(TokenKind::FatArrow) {
                    self.tokens.next()?; // consume =>
                    expressions.push(elem);
                    
                    // Check again for end of expression
                    match self.peek_kind() {
                        Some(TokenKind::Semicolon) | Some(TokenKind::RightParen) | 
                        Some(TokenKind::RightBrace) | Some(TokenKind::RightBracket) => break,
                        _ => expressions.push(self.parse_assignment()?)
                    }
                } else {
                    expressions.push(elem);
                }
            }
            
            // Return as array literal for now
            let start = expressions[0].location.start;
            let end = expressions.last().unwrap().location.end;
            
            expr = Node::new(
                NodeKind::ArrayLiteral { elements: expressions },
                SourceLocation { start, end }
            );
        }
        
        // Now handle word operators (or, xor, and, not) which have the lowest precedence
        expr = self.parse_word_or_expr(expr)?;
        
        Ok(expr)
    }
    
    /// Parse word or expression (or, xor) - takes an existing expr and applies word operators
    fn parse_word_or_expr(&mut self, mut expr: Node) -> ParseResult<Node> {
        // First handle 'and' which has higher precedence than 'or'/'xor'
        expr = self.parse_word_and_expr_with(expr)?;
        
        // Then handle 'or' and 'xor' which have lowest precedence
        while let Some(kind) = self.peek_kind() {
            match kind {
                TokenKind::WordOr | TokenKind::WordXor => {
                    let op_token = self.tokens.next()?;
                    // Parse the right side as a full expression starting with assignment
                    let right = self.parse_assignment()?;
                    // Apply any 'and' operators to the right side
                    let right = self.parse_word_and_expr_with(right)?;
                    
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
    
    /// Parse word and expression with existing left side
    fn parse_word_and_expr_with(&mut self, mut expr: Node) -> ParseResult<Node> {
        while self.peek_kind() == Some(TokenKind::WordAnd) {
            let op_token = self.tokens.next()?;
            // Parse right side as a 'not' expression or assignment
            let right = self.parse_word_not_expr()?;
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
    
    /// Parse word not expression - handles 'not' operator
    fn parse_word_not_expr(&mut self) -> ParseResult<Node> {
        if self.peek_kind() == Some(TokenKind::WordNot) {
            let op_token = self.tokens.next()?;
            let start = op_token.start;
            let operand = self.parse_word_not_expr()?;
            let end = operand.location.end;
            
            return Ok(Node::new(
                NodeKind::Unary {
                    op: op_token.text.clone(),
                    operand: Box::new(operand),
                },
                SourceLocation { start, end }
            ));
        }
        
        // The right side of a word operator should be a full expression
        self.parse_assignment()
    }
    
    /// Parse assignment expression
    fn parse_assignment(&mut self) -> ParseResult<Node> {
        // Check if we have a 'not' operator first
        if self.peek_kind() == Some(TokenKind::WordNot) {
            return self.parse_word_not_expr();
        }
        
        let mut expr = self.parse_ternary()?;
        
        // Check for assignment operators
        if let Some(kind) = self.peek_kind() {
            let op = match kind {
                TokenKind::Assign => Some("="),
                TokenKind::PlusAssign => Some("+="),
                TokenKind::MinusAssign => Some("-="),
                TokenKind::StarAssign => Some("*="),
                TokenKind::SlashAssign => Some("/="),
                TokenKind::PercentAssign => Some("%="),
                TokenKind::DotAssign => Some(".="),
                TokenKind::AndAssign => Some("&="),
                TokenKind::OrAssign => Some("|="),
                TokenKind::XorAssign => Some("^="),
                TokenKind::PowerAssign => Some("**="),
                TokenKind::LeftShiftAssign => Some("<<="),
                TokenKind::RightShiftAssign => Some(">>="),
                TokenKind::LogicalAndAssign => Some("&&="),
                TokenKind::LogicalOrAssign => Some("||="),
                TokenKind::DefinedOrAssign => Some("//="),
                _ => None,
            };
            
            if let Some(op) = op {
                self.tokens.next()?; // consume operator
                // The RHS can be a 'not' expression
                let rhs = if self.peek_kind() == Some(TokenKind::WordNot) {
                    self.parse_word_not_expr()?
                } else {
                    self.parse_assignment()?
                };
                let start = expr.location.start;
                let end = rhs.location.end;
                
                expr = Node::new(
                    NodeKind::Assignment {
                        lhs: Box::new(expr),
                        rhs: Box::new(rhs),
                        op: op.to_string(),
                    },
                    SourceLocation { start, end }
                );
            }
        }
        
        Ok(expr)
    }
    
    /// Parse ternary conditional expression
    fn parse_ternary(&mut self) -> ParseResult<Node> {
        let mut expr = self.parse_or()?;
        
        if self.peek_kind() == Some(TokenKind::Question) {
            self.tokens.next()?; // consume ?
            let then_expr = self.parse_or()?;
            self.expect(TokenKind::Colon)?;
            let else_expr = self.parse_ternary()?;
            
            let start = expr.location.start;
            let end = else_expr.location.end;
            
            expr = Node::new(
                NodeKind::Ternary {
                    condition: Box::new(expr),
                    then_expr: Box::new(then_expr),
                    else_expr: Box::new(else_expr),
                },
                SourceLocation { start, end }
            );
        }
        
        Ok(expr)
    }
    
    /// Parse logical OR expression
    fn parse_or(&mut self) -> ParseResult<Node> {
        let mut expr = self.parse_and()?;
        
        while matches!(self.peek_kind(), Some(TokenKind::Or) | Some(TokenKind::DefinedOr)) {
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
        let mut expr = self.parse_bitwise_or()?;
        
        while self.peek_kind() == Some(TokenKind::And) {
            let op_token = self.tokens.next()?;
            let right = self.parse_bitwise_or()?;
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
    
    /// Parse bitwise OR expression
    fn parse_bitwise_or(&mut self) -> ParseResult<Node> {
        let mut expr = self.parse_bitwise_xor()?;
        
        while self.peek_kind() == Some(TokenKind::BitwiseOr) {
            let op_token = self.tokens.next()?;
            let right = self.parse_bitwise_xor()?;
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
    
    /// Parse bitwise XOR expression
    fn parse_bitwise_xor(&mut self) -> ParseResult<Node> {
        let mut expr = self.parse_bitwise_and()?;
        
        while self.peek_kind() == Some(TokenKind::BitwiseXor) {
            let op_token = self.tokens.next()?;
            let right = self.parse_bitwise_and()?;
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
    
    /// Parse range expression
    fn parse_range(&mut self) -> ParseResult<Node> {
        let mut expr = self.parse_equality()?;
        
        while self.peek_kind() == Some(TokenKind::Range) {
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
    
    /// Parse bitwise AND expression
    fn parse_bitwise_and(&mut self) -> ParseResult<Node> {
        let mut expr = self.parse_range()?;
        
        while self.peek_kind() == Some(TokenKind::BitwiseAnd) {
            let op_token = self.tokens.next()?;
            let right = self.parse_range()?;
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
                TokenKind::Equal | TokenKind::NotEqual | TokenKind::Match | TokenKind::NotMatch | TokenKind::SmartMatch => {
                    let op_token = self.tokens.next()?;
                    let right = self.parse_relational()?;
                    let start = expr.location.start;
                    let end = right.location.end;
                    
                    // Special handling for match operators with substitution/transliteration
                    if matches!(op_token.kind, TokenKind::Match | TokenKind::NotMatch) {
                        // Check if right side is already a substitution or transliteration
                        if let NodeKind::Substitution { pattern, replacement, modifiers, .. } = &right.kind {
                            // Update the expression in the substitution
                            expr = Node::new(
                                NodeKind::Substitution {
                                    expr: Box::new(expr),
                                    pattern: pattern.clone(),
                                    replacement: replacement.clone(),
                                    modifiers: modifiers.clone(),
                                },
                                SourceLocation { start, end }
                            );
                        } else if let NodeKind::Transliteration { search, replace, modifiers, .. } = &right.kind {
                            // Update the expression in the transliteration
                            expr = Node::new(
                                NodeKind::Transliteration {
                                    expr: Box::new(expr),
                                    search: search.clone(),
                                    replace: replace.clone(),
                                    modifiers: modifiers.clone(),
                                },
                                SourceLocation { start, end }
                            );
                        } else if let NodeKind::Regex { pattern, modifiers } = &right.kind {
                            // Create a Match node
                            expr = Node::new(
                                NodeKind::Match {
                                    expr: Box::new(expr),
                                    pattern: pattern.clone(),
                                    modifiers: modifiers.clone(),
                                },
                                SourceLocation { start, end }
                            );
                        } else if let NodeKind::Regex { pattern, .. } = &right.kind {
                            if pattern.starts_with("s/") || pattern.starts_with("s|") || pattern.starts_with("s{") || pattern.starts_with("s[") {
                                // Parse as substitution
                                let parts = parse_substitution_parts(pattern);
                                expr = Node::new(
                                    NodeKind::Substitution {
                                        expr: Box::new(expr),
                                        pattern: parts.0,
                                        replacement: parts.1,
                                        modifiers: parts.2,
                                    },
                                    SourceLocation { start, end }
                                );
                            } else if pattern.starts_with("tr/") || pattern.starts_with("y/") || 
                                     pattern.starts_with("tr{") || pattern.starts_with("y{") ||
                                     pattern.starts_with("tr[") || pattern.starts_with("y[") {
                                // Parse as transliteration
                                let parts = parse_transliteration_parts(pattern);
                                expr = Node::new(
                                    NodeKind::Transliteration {
                                        expr: Box::new(expr),
                                        search: parts.0,
                                        replace: parts.1,
                                        modifiers: parts.2,
                                    },
                                    SourceLocation { start, end }
                                );
                            } else {
                                // Regular match - extract modifiers
                                let (pattern_with_delims, modifiers) = extract_regex_parts(pattern);
                                expr = Node::new(
                                    NodeKind::Match {
                                        expr: Box::new(expr),
                                        pattern: pattern_with_delims,
                                        modifiers,
                                    },
                                    SourceLocation { start, end }
                                );
                            }
                        } else {
                            // Normal binary operation
                            expr = Node::new(
                                NodeKind::Binary {
                                    op: op_token.text.clone(),
                                    left: Box::new(expr),
                                    right: Box::new(right),
                                },
                                SourceLocation { start, end }
                            );
                        }
                    } else {
                        // Normal binary operation for == and !=
                        expr = Node::new(
                            NodeKind::Binary {
                                op: op_token.text.clone(),
                                left: Box::new(expr),
                                right: Box::new(right),
                            },
                            SourceLocation { start, end }
                        );
                    }
                }
                _ => break,
            }
        }
        
        Ok(expr)
    }
    
    /// Parse relational expression
    fn parse_relational(&mut self) -> ParseResult<Node> {
        let mut expr = self.parse_shift()?;
        
        while let Some(kind) = self.peek_kind() {
            match kind {
                TokenKind::Less | TokenKind::Greater | 
                TokenKind::LessEqual | TokenKind::GreaterEqual |
                TokenKind::Spaceship | TokenKind::StringCompare => {
                    let op_token = self.tokens.next()?;
                    let right = self.parse_shift()?;
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
                TokenKind::Identifier => {
                    // Check if it's ISA operator
                    if self.tokens.peek()?.text == "ISA" {
                        let _op_token = self.tokens.next()?;
                        let right = self.parse_shift()?;
                        let start = expr.location.start;
                        let end = right.location.end;
                        
                        expr = Node::new(
                            NodeKind::Binary {
                                op: "ISA".to_string(),
                                left: Box::new(expr),
                                right: Box::new(right),
                            },
                            SourceLocation { start, end }
                        );
                    } else {
                        break;
                    }
                }
                _ => break,
            }
        }
        
        Ok(expr)
    }
    
    /// Parse shift expression
    fn parse_shift(&mut self) -> ParseResult<Node> {
        let mut expr = self.parse_additive()?;
        
        while let Some(kind) = self.peek_kind() {
            match kind {
                TokenKind::LeftShift | TokenKind::RightShift => {
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
                TokenKind::Plus | TokenKind::Minus | TokenKind::Dot => {
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
        let mut expr = self.parse_power()?;
        
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
    
    /// Parse power expression
    fn parse_power(&mut self) -> ParseResult<Node> {
        let mut expr = self.parse_unary()?;
        
        while self.peek_kind() == Some(TokenKind::Power) {
            let op_token = self.tokens.next()?;
            let right = self.parse_unary()?; // Right associative
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
    
    /// Parse unary expression
    fn parse_unary(&mut self) -> ParseResult<Node> {
        if let Some(kind) = self.peek_kind() {
            match kind {
                TokenKind::Minus => {
                    let op_token = self.tokens.next()?;
                    let start = op_token.start;
                    
                    // Check for file test operators (-e, -f, -d, etc.)
                    if let Some(TokenKind::Identifier) = self.peek_kind() {
                        let next_token = self.tokens.peek()?;
                        if next_token.text.len() == 1 {
                            // It's a file test operator
                            let test_token = self.tokens.next()?;
                            let file_test = format!("-{}", test_token.text);
                            
                            // File test can be used without operand (tests $_)
                            let operand = if self.is_at_statement_end() {
                                // No operand, test $_
                                Node::new(
                                    NodeKind::Variable { sigil: "$".to_string(), name: "_".to_string() },
                                    SourceLocation { start: test_token.end, end: test_token.end }
                                )
                            } else {
                                self.parse_unary()?
                            };
                            
                            let end = operand.location.end;
                            return Ok(Node::new(
                                NodeKind::Unary {
                                    op: file_test,
                                    operand: Box::new(operand),
                                },
                                SourceLocation { start, end }
                            ));
                        }
                    }
                    
                    // Regular unary minus
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
                TokenKind::Plus | TokenKind::Not | TokenKind::Backslash | TokenKind::BitwiseNot | TokenKind::Star => {
                    let op_token = self.tokens.next()?;
                    let start = op_token.start;
                    
                    // Check if we're at EOF or a terminator (for standalone operators)
                    if self.tokens.is_eof() || self.is_at_statement_end() {
                        // Create a placeholder for standalone operator
                        let end = op_token.end;
                        return Ok(Node::new(
                            NodeKind::Unary {
                                op: op_token.text.clone(),
                                operand: Box::new(Node::new(
                                    NodeKind::Undef,
                                    SourceLocation { start: end, end }
                                )),
                            },
                            SourceLocation { start, end }
                        ));
                    }
                    
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
                TokenKind::Increment | TokenKind::Decrement => {
                    // Pre-increment and pre-decrement
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
                TokenKind::SmartMatch => {
                    // Smart match can be used as a unary operator
                    let op_token = self.tokens.next()?;
                    let start = op_token.start;
                    
                    // Check if we're at EOF or a terminator (for standalone operators)
                    if self.tokens.is_eof() || self.is_at_statement_end() {
                        // Create a placeholder for standalone operator
                        let end = op_token.end;
                        return Ok(Node::new(
                            NodeKind::Unary {
                                op: op_token.text.clone(),
                                operand: Box::new(Node::new(
                                    NodeKind::Undef,
                                    SourceLocation { start: end, end }
                                )),
                            },
                            SourceLocation { start, end }
                        ));
                    }
                    
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
                    
                    // Check for postfix dereference operators
                    match self.peek_kind() {
                        Some(TokenKind::ArraySigil) => {
                            // ->@* or ->@[...]
                            self.tokens.next()?; // consume @
                            
                            if self.peek_kind() == Some(TokenKind::Star) {
                                // ->@*
                                self.tokens.next()?; // consume *
                                let start = expr.location.start;
                                let end = self.previous_position();
                                
                                expr = Node::new(
                                    NodeKind::Unary {
                                        op: "->@*".to_string(),
                                        operand: Box::new(expr),
                                    },
                                    SourceLocation { start, end }
                                );
                            } else if self.peek_kind() == Some(TokenKind::LeftBracket) {
                                // ->@[...] array slice
                                self.tokens.next()?; // consume [
                                let index = self.parse_expression()?;
                                self.expect(TokenKind::RightBracket)?;
                                
                                let start = expr.location.start;
                                let end = self.previous_position();
                                
                                // Represent as a special binary operation for array slice dereference
                                expr = Node::new(
                                    NodeKind::Binary {
                                        op: "->@[]".to_string(),
                                        left: Box::new(expr),
                                        right: Box::new(index),
                                    },
                                    SourceLocation { start, end }
                                );
                            }
                        }
                        
                        Some(TokenKind::HashSigil) => {
                            // ->%* or ->%{...}
                            self.tokens.next()?; // consume %
                            
                            if self.peek_kind() == Some(TokenKind::Star) {
                                // ->%*
                                self.tokens.next()?; // consume *
                                let start = expr.location.start;
                                let end = self.previous_position();
                                
                                expr = Node::new(
                                    NodeKind::Unary {
                                        op: "->%*".to_string(),
                                        operand: Box::new(expr),
                                    },
                                    SourceLocation { start, end }
                                );
                            } else if self.peek_kind() == Some(TokenKind::LeftBrace) {
                                // ->%{...} hash slice
                                self.tokens.next()?; // consume {
                                let key = self.parse_expression()?;
                                self.expect(TokenKind::RightBrace)?;
                                
                                let start = expr.location.start;
                                let end = self.previous_position();
                                
                                // Represent as a special binary operation for hash slice dereference
                                expr = Node::new(
                                    NodeKind::Binary {
                                        op: "->%{}".to_string(),
                                        left: Box::new(expr),
                                        right: Box::new(key),
                                    },
                                    SourceLocation { start, end }
                                );
                            }
                        }
                        
                        Some(TokenKind::ScalarSigil) => {
                            // ->$*
                            self.tokens.next()?; // consume $
                            
                            if self.peek_kind() == Some(TokenKind::Star) {
                                self.tokens.next()?; // consume *
                                let start = expr.location.start;
                                let end = self.previous_position();
                                
                                expr = Node::new(
                                    NodeKind::Unary {
                                        op: "->$*".to_string(),
                                        operand: Box::new(expr),
                                    },
                                    SourceLocation { start, end }
                                );
                            }
                        }
                        
                        Some(TokenKind::SubSigil) | Some(TokenKind::BitwiseAnd) => {
                            // ->&* (code dereference)
                            self.tokens.next()?; // consume &
                            
                            if self.peek_kind() == Some(TokenKind::Star) {
                                self.tokens.next()?; // consume *
                                let start = expr.location.start;
                                let end = self.previous_position();
                                
                                expr = Node::new(
                                    NodeKind::Unary {
                                        op: "->&*".to_string(),
                                        operand: Box::new(expr),
                                    },
                                    SourceLocation { start, end }
                                );
                            }
                        }
                        
                        Some(TokenKind::Star) => {
                            // ->** (glob dereference)
                            self.tokens.next()?; // consume first *
                            
                            if self.peek_kind() == Some(TokenKind::Star) {
                                self.tokens.next()?; // consume second *
                                let start = expr.location.start;
                                let end = self.previous_position();
                                
                                expr = Node::new(
                                    NodeKind::Unary {
                                        op: "->**".to_string(),
                                        operand: Box::new(expr),
                                    },
                                    SourceLocation { start, end }
                                );
                            }
                        }
                        
                        Some(TokenKind::Identifier) | Some(TokenKind::Method) => {
                            // Method call
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
                        
                        _ => {
                            // Just the arrow by itself - could be an error or incomplete
                            // For now, we'll leave expr unchanged
                        }
                    }
                }
                
                Some(TokenKind::LeftBracket) => {
                    // Array indexing - can be a single index or slice with multiple indices
                    self.tokens.next()?; // consume [
                    
                    // Check if this might be a slice (multiple indices)
                    let mut indices = vec![self.parse_expression()?];
                    
                    // Look for comma-separated indices
                    while self.peek_kind() == Some(TokenKind::Comma) {
                        self.tokens.next()?; // consume comma
                        indices.push(self.parse_expression()?);
                    }
                    
                    self.expect(TokenKind::RightBracket)?;
                    
                    // Create the index node - either single index or array of indices
                    let index = if indices.len() == 1 {
                        indices.into_iter().next().unwrap()
                    } else {
                        // Multiple indices - create an array literal node
                        let start = indices.first().unwrap().location.start;
                        let end = indices.last().unwrap().location.end;
                        Node::new(
                            NodeKind::ArrayLiteral { elements: indices },
                            SourceLocation { start, end }
                        )
                    };
                    
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
                    // Check if this is a builtin function that needs special handling
                    if let NodeKind::Identifier { name } = &expr.kind {
                        if Self::is_builtin_function(name) {
                            // This is a builtin function with {} as argument
                            // Parse arguments without parentheses
                            let mut args = Vec::new();
                            
                            // Special handling for bless {} - parse it as a hash
                            if name == "bless" {
                                args.push(self.parse_hash_or_block()?);
                                
                                // Parse remaining arguments separated by commas
                                while self.peek_kind() == Some(TokenKind::Comma) {
                                    self.consume_token()?; // consume comma
                                    if self.is_at_statement_end() {
                                        break;
                                    }
                                    args.push(self.parse_comma()?);
                                }
                            } else if matches!(name.as_str(), "sort" | "map" | "grep") {
                                // Parse block expression as first argument
                                let block_start = self.current_position();
                                self.expect(TokenKind::LeftBrace)?;
                                
                                // Parse the expression inside the block (if any)
                                let mut statements = Vec::new();
                                if self.peek_kind() != Some(TokenKind::RightBrace) {
                                    statements.push(self.parse_expression()?);
                                }
                                
                                self.expect(TokenKind::RightBrace)?;
                                let block_end = self.previous_position();
                                
                                // Wrap the expression in a block node
                                let block = Node::new(
                                    NodeKind::Block { statements },
                                    SourceLocation { start: block_start, end: block_end }
                                );
                                
                                args.push(block);
                                
                                // Parse remaining arguments
                                while self.peek_kind() == Some(TokenKind::Comma) {
                                    self.consume_token()?; // consume comma
                                    if self.is_at_statement_end() {
                                        break;
                                    }
                                    args.push(self.parse_comma()?);
                                }
                            } else {
                                // Other builtins - parse {} as first argument
                                args.push(self.parse_hash_or_block()?);
                                
                                // Parse remaining arguments separated by commas
                                while self.peek_kind() == Some(TokenKind::Comma) {
                                    self.consume_token()?; // consume comma
                                    if self.is_at_statement_end() {
                                        break;
                                    }
                                    args.push(self.parse_comma()?);
                                }
                            }
                            
                            let start = expr.location.start;
                            let end = args.last().unwrap().location.end;
                            
                            expr = Node::new(
                                NodeKind::FunctionCall { 
                                    name: name.clone(), 
                                    args 
                                },
                                SourceLocation { start, end }
                            );
                            continue; // Continue the loop
                        }
                    }
                    
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
                        
                        // Special handling for qw()
                        if name == "qw" {
                            let words = self.parse_qw_list()?;
                            let start = expr.location.start;
                            let end = self.previous_position();
                            
                            expr = Node::new(
                                NodeKind::ArrayLiteral { elements: words },
                                SourceLocation { start, end }
                            );
                        } else {
                            let args = self.parse_args()?;
                            let start = expr.location.start;
                            let end = self.previous_position();
                            
                            expr = Node::new(
                                NodeKind::FunctionCall { name, args },
                                SourceLocation { start, end }
                            );
                        }
                    }
                }
                
                _ => {
                    // Check if this is a builtin function that can take bare arguments
                    if let NodeKind::Identifier { name } = &expr.kind {
                        // Check for quote operators first
                        if matches!(name.as_str(), "q" | "qq" | "qw" | "qr" | "qx" | "m" | "s") {
                            // This was already parsed as a quote operator in parse_primary
                            // Don't try to parse arguments
                        } else if Self::is_builtin_function(name) {
                            // Builtins always become function calls, even with no arguments
                            // This ensures they work correctly in expressions like "return $x or die"
                            if self.is_at_statement_end() {
                                // Bare builtin with no arguments
                                expr = Node::new(
                                    NodeKind::FunctionCall { 
                                        name: name.clone(), 
                                        args: vec![]
                                    },
                                    expr.location
                                );
                            } else {
                                // Parse arguments without parentheses
                                let mut args = Vec::new();
                                
                                // Special handling for sort, map, grep with block first argument
                                if matches!(name.as_str(), "sort" | "map" | "grep") && 
                                   self.peek_kind() == Some(TokenKind::LeftBrace) {
                                    // Parse block expression as first argument
                                    let block_start = self.current_position();
                                    self.expect(TokenKind::LeftBrace)?;
                                    
                                    // Parse the expression inside the block (if any)
                                    let mut statements = Vec::new();
                                    if self.peek_kind() != Some(TokenKind::RightBrace) {
                                        statements.push(self.parse_expression()?);
                                    }
                                    
                                    self.expect(TokenKind::RightBrace)?;
                                    let block_end = self.previous_position();
                                    
                                    // Wrap the expression in a block node
                                    let block = Node::new(
                                        NodeKind::Block { statements },
                                        SourceLocation { start: block_start, end: block_end }
                                    );
                                    
                                    args.push(block);
                                    
                                    // Parse remaining arguments
                                    while self.peek_kind() == Some(TokenKind::Comma) {
                                        self.consume_token()?; // consume comma
                                        if self.is_at_statement_end() {
                                            break;
                                        }
                                        args.push(self.parse_comma()?);
                                    }
                                } else if name == "bless" && self.peek_kind() == Some(TokenKind::LeftBrace) {
                                    // Special handling for bless {} - parse it as a hash
                                    args.push(self.parse_hash_or_block()?);
                                    
                                    // Parse remaining arguments separated by commas
                                    while self.peek_kind() == Some(TokenKind::Comma) {
                                        self.consume_token()?; // consume comma
                                        if self.is_at_statement_end() {
                                            break;
                                        }
                                        args.push(self.parse_comma()?);
                                    }
                                } else {
                                    // Parse the first argument
                                    args.push(self.parse_comma()?);
                                    
                                    // Parse remaining arguments separated by commas
                                    while self.peek_kind() == Some(TokenKind::Comma) {
                                        self.consume_token()?; // consume comma
                                        if self.is_at_statement_end() {
                                            break;
                                        }
                                        args.push(self.parse_comma()?);
                                    }
                                }
                                
                                let start = expr.location.start;
                                let end = args.last().unwrap().location.end;
                                
                                expr = Node::new(
                                    NodeKind::FunctionCall { 
                                        name: name.clone(), 
                                        args 
                                    },
                                    SourceLocation { start, end }
                                );
                            }
                        }
                    }
                    break;
                }
            }
        }
        
        Ok(expr)
    }
    
    /// Check if we're at a statement boundary
    fn is_at_statement_end(&mut self) -> bool {
        matches!(
            self.peek_kind(),
            Some(TokenKind::Semicolon) | Some(TokenKind::RightBrace) | 
            Some(TokenKind::RightParen) | Some(TokenKind::RightBracket) |
            Some(TokenKind::If) | Some(TokenKind::Unless) | 
            Some(TokenKind::While) | Some(TokenKind::Until) |
            Some(TokenKind::For) | Some(TokenKind::Foreach) |
            Some(TokenKind::Eof) | None
        )
    }
    
    /// Parse quote operator (q, qq, qw, qr, qx)
    fn parse_quote_operator(&mut self) -> ParseResult<Node> {
        let op_token = self.consume_token()?; // consume q/qq/qw/qr/qx
        let start = op_token.start;
        let op = op_token.text.as_ref();
        
        // Get the delimiter - it might be a bracket token or other punctuation
        let delim_token = self.consume_token()?;
        let delim_char = match delim_token.kind {
            TokenKind::LeftBrace => '{',
            TokenKind::LeftBracket => '[',
            TokenKind::LeftParen => '(',
            TokenKind::Less => '<',
            _ => delim_token.text.chars().next()
                .ok_or_else(|| ParseError::syntax("Expected delimiter after quote operator", delim_token.start))?
        };
        
        // Determine closing delimiter
        let close_delim = match delim_char {
            '{' => '}',
            '[' => ']',
            '(' => ')',
            '<' => '>',
            _ => delim_char, // For other delimiters like / or |, use the same char
        };
        
        // Store delimiters for later use
        let opening_delim = delim_char;
        let closing_delim = close_delim;
        
        // Collect content until closing delimiter
        let mut content = String::new();
        let mut depth = 1;
        
        // For regex operators (m, s), we need to preserve the exact pattern
        let preserve_exact_content = matches!(op, "m" | "s" | "qr");
        
        while depth > 0 && !self.tokens.is_eof() {
            // Check token kind first
            let token_kind = self.peek_kind();
            
            // Check for matching delimiter tokens
            if matches!(delim_char, '{' | '[' | '(' | '<') {
                // Handle bracket-based delimiters
                match (delim_char, token_kind) {
                    ('{', Some(TokenKind::LeftBrace)) => {
                        self.consume_token()?;
                        content.push('{');
                        depth += 1;
                    }
                    ('{', Some(TokenKind::RightBrace)) => {
                        self.consume_token()?;
                        depth -= 1;
                        if depth > 0 {
                            content.push('}');
                        }
                    }
                    ('[', Some(TokenKind::LeftBracket)) => {
                        self.consume_token()?;
                        content.push('[');
                        depth += 1;
                    }
                    ('[', Some(TokenKind::RightBracket)) => {
                        self.consume_token()?;
                        depth -= 1;
                        if depth > 0 {
                            content.push(']');
                        }
                    }
                    ('(', Some(TokenKind::LeftParen)) => {
                        self.consume_token()?;
                        content.push('(');
                        depth += 1;
                    }
                    ('(', Some(TokenKind::RightParen)) => {
                        self.consume_token()?;
                        depth -= 1;
                        if depth > 0 {
                            content.push(')');
                        }
                    }
                    ('<', Some(TokenKind::Less)) => {
                        self.consume_token()?;
                        content.push('<');
                        depth += 1;
                    }
                    ('<', Some(TokenKind::Greater)) => {
                        self.consume_token()?;
                        depth -= 1;
                        if depth > 0 {
                            content.push('>');
                        }
                    }
                    _ => {
                        // Regular token, add to content
                        let token = self.consume_token()?;
                        content.push_str(&token.text);
                        if !preserve_exact_content && !self.tokens.is_eof() && !content.is_empty() {
                            content.push(' ');
                        }
                    }
                }
            } else {
                // For non-bracket delimiters, just look for the closing delimiter
                let token = self.consume_token()?;
                if token.text.contains(close_delim) {
                    let pos = token.text.find(close_delim).unwrap();
                    content.push_str(&token.text[..pos]);
                    break;
                } else {
                    content.push_str(&token.text);
                    if !preserve_exact_content && !self.tokens.is_eof() {
                        content.push(' ');
                    }
                }
            }
        }
        
        // Parse modifiers for regex operators
        let mut modifiers = String::new();
        if matches!(op, "m" | "s" | "qr") {
            // Check for modifiers (letters after closing delimiter)
            while let Ok(token) = self.tokens.peek() {
                if token.kind == TokenKind::Identifier && token.text.len() == 1 {
                    // Single letter identifier could be a modifier
                    let ch = token.text.chars().next().unwrap();
                    if ch.is_ascii_alphabetic() {
                        modifiers.push(ch);
                        self.tokens.next()?;
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            }
        }
        
        let end = self.previous_position();
        
        // Create appropriate node based on operator
        match op {
            "qq" => {
                // Double-quoted string with interpolation
                Ok(Node::new(
                    NodeKind::String {
                        value: format!("\"{}\"", content),
                        interpolated: true,
                    },
                    SourceLocation { start, end }
                ))
            }
            "q" => {
                // Single-quoted string without interpolation
                Ok(Node::new(
                    NodeKind::String {
                        value: format!("'{}'", content),
                        interpolated: false,
                    },
                    SourceLocation { start, end }
                ))
            }
            "qw" => {
                // Word list - split on whitespace
                let words: Vec<Node> = content.split_whitespace()
                    .map(|word| Node::new(
                        NodeKind::String {
                            value: format!("'{}'", word),
                            interpolated: false,
                        },
                        SourceLocation { start, end }
                    ))
                    .collect();
                    
                Ok(Node::new(
                    NodeKind::ArrayLiteral { elements: words },
                    SourceLocation { start, end }
                ))
            }
            "qr" => {
                // Regular expression
                Ok(Node::new(
                    NodeKind::Regex {
                        pattern: format!("{}{}{}", opening_delim, content, closing_delim),
                        modifiers,
                    },
                    SourceLocation { start, end }
                ))
            }
            "qx" => {
                // Backticks/command execution
                Ok(Node::new(
                    NodeKind::String {
                        value: format!("`{}`", content),
                        interpolated: true,
                    },
                    SourceLocation { start, end }
                ))
            }
            "m" => {
                // Match operator with pattern
                Ok(Node::new(
                    NodeKind::Regex {
                        pattern: format!("m{}{}{}", opening_delim, content, closing_delim),
                        modifiers,
                    },
                    SourceLocation { start, end }
                ))
            }
            "s" => {
                // Substitution operator - for now just parse as regex
                // TODO: Parse replacement and modifiers
                Ok(Node::new(
                    NodeKind::Regex {
                        pattern: format!("s{}{}{}", opening_delim, content, closing_delim),
                        modifiers: String::new(),
                    },
                    SourceLocation { start, end }
                ))
            }
            _ => {
                Err(ParseError::syntax(
                    format!("Unknown quote operator: {}", op),
                    start
                ))
            }
        }
    }
    
    /// Parse qualified identifier (may contain ::)
    fn parse_qualified_identifier(&mut self) -> ParseResult<Node> {
        let start_token = self.consume_token()?;
        let start = start_token.start;
        let mut name = if start_token.kind == TokenKind::DoubleColon {
            // Handle absolute path like ::Foo::Bar
            "::".to_string()
        } else {
            start_token.text.clone()
        };
        
        // Keep consuming :: and identifiers
        while self.peek_kind() == Some(TokenKind::DoubleColon) {
            self.consume_token()?; // consume ::
            name.push_str("::");
            
            // In Perl, trailing :: is valid (e.g., Foo::Bar::)
            // Only consume identifier if there is one
            if self.peek_kind() == Some(TokenKind::Identifier) {
                let next_part = self.consume_token()?;
                name.push_str(&next_part.text);
            }
            // No error for trailing :: - it's valid in Perl
        }
        
        let end = self.previous_position();
        Ok(Node::new(
            NodeKind::Identifier { name },
            SourceLocation { start, end }
        ))
    }
    
    /// Check if an identifier is a builtin function that can take arguments without parens
    fn is_builtin_function(name: &str) -> bool {
        matches!(name, 
            "print" | "say" | "die" | "warn" | "return" | "defined" | 
            "undef" | "ref" | "chomp" | "chop" | "split" | "join" |
            "push" | "pop" | "shift" | "unshift" | "sort" | "map" |
            "grep" | "keys" | "values" | "each" | "delete" | "exists" |
            "open" | "close" | "read" | "write" | "printf" | "sprintf" |
            "exit" | "next" | "last" | "redo" | "goto" | "dump" | 
            "caller" | "import" | "unimport" | "require" | "bless" |
            "tie" | "tied" | "untie" | "scalar" | "wantarray"
        )
    }
    
    /// Parse primary expression
    fn parse_primary(&mut self) -> ParseResult<Node> {
        let token = self.tokens.peek()?;
        let token_kind = token.kind;
        
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
                let (pattern, modifiers) = extract_regex_parts(&token.text);
                Ok(Node::new(
                    NodeKind::Regex { 
                        pattern,
                        modifiers,
                    },
                    SourceLocation { start: token.start, end: token.end }
                ))
            }
            
            TokenKind::Substitution => {
                let token = self.tokens.next()?;
                let (pattern, replacement, modifiers) = parse_substitution_parts(&token.text);
                
                // Substitution as a standalone expression (will be used with =~ later)
                Ok(Node::new(
                    NodeKind::Substitution { 
                        expr: Box::new(Node::new(
                            NodeKind::Identifier { name: String::from("$_") },
                            SourceLocation { start: token.start, end: token.start }
                        )),
                        pattern,
                        replacement,
                        modifiers,
                    },
                    SourceLocation { start: token.start, end: token.end }
                ))
            }
            
            TokenKind::Transliteration => {
                let token = self.tokens.next()?;
                let (search, replace, modifiers) = parse_transliteration_parts(&token.text);
                
                // Transliteration as a standalone expression (will be used with =~ later)
                Ok(Node::new(
                    NodeKind::Transliteration { 
                        expr: Box::new(Node::new(
                            NodeKind::Identifier { name: String::from("$_") },
                            SourceLocation { start: token.start, end: token.start }
                        )),
                        search,
                        replace,
                        modifiers,
                    },
                    SourceLocation { start: token.start, end: token.end }
                ))
            }
            
            TokenKind::HeredocStart => {
                let start_token = self.tokens.next()?;
                let text = &start_token.text;
                let start = start_token.start;
                
                // Parse heredoc delimiter from the token text
                let (delimiter, interpolated, indented) = parse_heredoc_delimiter(text);
                
                // Collect heredoc body content
                let mut content = String::new();
                let mut end = start_token.end;
                
                // Look for HeredocBody tokens
                while let Ok(token) = self.tokens.peek() {
                    if token.kind == TokenKind::HeredocBody {
                        let body_token = self.tokens.next()?;
                        // Extract content from the token text
                        // The lexer includes the content in the token text
                        content.push_str(&body_token.text);
                        end = body_token.end;
                    } else {
                        break;
                    }
                }
                
                Ok(Node::new(
                    NodeKind::Heredoc {
                        delimiter: delimiter.to_string(),
                        content,
                        interpolated,
                        indented,
                    },
                    SourceLocation { start, end }
                ))
            }
            
            TokenKind::Eval => {
                self.parse_eval()
            }
            
            TokenKind::Do => {
                self.parse_do()
            }
            
            // Note: TokenKind::Sub is handled in the keyword-as-identifier case below
            // This allows 'sub' to be used as a hash key or identifier in expressions
            
            TokenKind::Try => {
                self.parse_try()
            }
            
            TokenKind::Less => {
                // Could be diamond operator <> or <FILEHANDLE>
                let start = self.consume_token()?.start; // consume <
                
                if self.peek_kind() == Some(TokenKind::Greater) {
                    // Diamond operator <>
                    self.consume_token()?; // consume >
                    let end = self.previous_position();
                    Ok(Node::new(
                        NodeKind::Diamond,
                        SourceLocation { start, end }
                    ))
                } else {
                    // Try to parse content until >
                    let mut pattern = String::new();
                    let mut has_glob_chars = false;
                    
                    while self.peek_kind() != Some(TokenKind::Greater) && !self.tokens.is_eof() {
                        let token = self.consume_token()?;
                        
                        // Check if this looks like a glob pattern
                        if token.text.contains('*') || token.text.contains('?') || 
                           token.text.contains('[') || token.text.contains('.') {
                            has_glob_chars = true;
                        }
                        
                        pattern.push_str(&token.text);
                    }
                    
                    if self.peek_kind() == Some(TokenKind::Greater) {
                        self.consume_token()?; // consume >
                        let end = self.previous_position();
                        
                        if pattern.is_empty() {
                            // Empty <> is diamond operator
                            Ok(Node::new(
                                NodeKind::Diamond,
                                SourceLocation { start, end }
                            ))
                        } else if has_glob_chars || pattern.contains('/') {
                            // Looks like a glob pattern
                            Ok(Node::new(
                                NodeKind::Glob { pattern },
                                SourceLocation { start, end }
                            ))
                        } else if pattern.chars().all(|c| c.is_uppercase() || c == '_') {
                            // Looks like a filehandle
                            Ok(Node::new(
                                NodeKind::Readline { filehandle: Some(pattern) },
                                SourceLocation { start, end }
                            ))
                        } else {
                            // Default to glob
                            Ok(Node::new(
                                NodeKind::Glob { pattern },
                                SourceLocation { start, end }
                            ))
                        }
                    } else {
                        Err(ParseError::syntax(
                            "Expected '>' to close angle bracket construct",
                            self.current_position()
                        ))
                    }
                }
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
                    // Check if it's a quote operator (q, qq, qw, qr, qx, m, s)
                    match token.text.as_ref() {
                        "q" | "qq" | "qw" | "qr" | "qx" | "m" | "s" => {
                            self.parse_quote_operator()
                        }
                        _ => {
                            // Regular identifier (possibly qualified with ::)
                            self.parse_qualified_identifier()
                        }
                    }
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
                
                if self.peek_kind() == Some(TokenKind::Comma) || self.peek_kind() == Some(TokenKind::FatArrow) {
                    // It's a list
                    let mut elements = vec![first];
                    
                    // Handle fat arrow after first element
                    if self.peek_kind() == Some(TokenKind::FatArrow) {
                        self.tokens.next()?; // consume =>
                        elements.push(self.parse_expression()?);
                    }
                    
                    while self.peek_kind() == Some(TokenKind::Comma) || self.peek_kind() == Some(TokenKind::FatArrow) {
                        if self.peek_kind() == Some(TokenKind::Comma) {
                            self.tokens.next()?; // consume comma
                        }
                        
                        if self.peek_kind() == Some(TokenKind::RightParen) {
                            break;
                        }
                        
                        let elem = self.parse_expression()?;
                        
                        // Check for fat arrow after element
                        if self.peek_kind() == Some(TokenKind::FatArrow) {
                            self.tokens.next()?; // consume =>
                            elements.push(elem);
                            if self.peek_kind() != Some(TokenKind::RightParen) {
                                elements.push(self.parse_expression()?);
                            }
                        } else {
                            elements.push(elem);
                        }
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
            
            // Handle & as sigil when at primary position
            TokenKind::BitwiseAnd => {
                // This is a subroutine call or code dereference
                // Convert to SubSigil behavior
                self.parse_variable_from_sigil()
            }
            
            TokenKind::LeftBrace => {
                // Could be hash literal or block
                // Try to parse as hash literal first
                self.parse_hash_or_block()
            }
            
            TokenKind::Ellipsis => {
                let token = self.tokens.next()?;
                Ok(Node::new(
                    NodeKind::Ellipsis,
                    SourceLocation { start: token.start, end: token.end }
                ))
            }
            
            TokenKind::Undef => {
                let token = self.tokens.next()?;
                Ok(Node::new(
                    NodeKind::Undef,
                    SourceLocation { start: token.start, end: token.end }
                ))
            }
            
            // Handle 'sub' specially - it might be an anonymous subroutine
            TokenKind::Sub => {
                // Check if this is an anonymous subroutine
                let next = self.peek_kind();
                if matches!(next, Some(TokenKind::LeftBrace) | Some(TokenKind::LeftParen)) {
                    // It's an anonymous subroutine
                    self.parse_subroutine()
                } else {
                    // It's used as an identifier
                    let token = self.tokens.next()?;
                    Ok(Node::new(
                        NodeKind::Identifier { name: token.text.to_string() },
                        SourceLocation { start: token.start, end: token.end }
                    ))
                }
            }
            
            // Handle keywords that can be used as identifiers in certain contexts
            // Note: Statement-level keywords (if, unless, while, return, etc.) should NOT be here
            TokenKind::My | TokenKind::Our | TokenKind::Local | TokenKind::State |
            TokenKind::Package | TokenKind::Use | TokenKind::No |
            TokenKind::Begin | TokenKind::End | TokenKind::Check |
            TokenKind::Init | TokenKind::Unitcheck |
            TokenKind::Given | TokenKind::When | TokenKind::Default |
            TokenKind::Catch | TokenKind::Finally |
            TokenKind::Continue | TokenKind::Class | TokenKind::Method | TokenKind::Format => {
                // In expression context, some keywords can be used as barewords/identifiers
                // This happens in hash keys, method names, etc.
                // But NOT for statement modifiers like if, unless, while, etc.
                let token = self.tokens.next()?;
                Ok(Node::new(
                    NodeKind::Identifier { name: token.text.to_string() },
                    SourceLocation { start: token.start, end: token.end }
                ))
            }
            
            TokenKind::DoubleColon => {
                // Absolute package path like ::Foo::Bar
                self.parse_qualified_identifier()
            }
            
            _ => {
                // Get position before consuming
                let pos = self.current_position();
                Err(ParseError::unexpected(
                    "expression",
                    format!("{:?}", token_kind),
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
    #[allow(dead_code)]
    fn peek_token(&mut self) -> ParseResult<&Token> {
        self.tokens.peek()
    }
    
    /// Check if we're at the start of a labeled statement (LABEL: ...)
    fn is_label_start(&mut self) -> bool {
        // We need an identifier followed by a colon
        if self.peek_kind() != Some(TokenKind::Identifier) {
            return false;
        }
        
        // Check if the second token is a colon
        match self.tokens.peek_second() {
            Ok(token) => token.kind == TokenKind::Colon,
            Err(_) => false,
        }
    }
    
    /// Parse a labeled statement (LABEL: statement)
    fn parse_labeled_statement(&mut self) -> ParseResult<Node> {
        let start = self.current_position();
        
        // Parse the label
        let label_token = self.expect(TokenKind::Identifier)?;
        let label = label_token.text.clone();
        
        // Consume the colon
        self.expect(TokenKind::Colon)?;
        
        // Parse the statement after the label
        let statement = Box::new(self.parse_statement()?);
        
        let end = self.previous_position();
        Ok(Node::new(
            NodeKind::LabeledStatement { label, statement },
            SourceLocation { start, end }
        ))
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
                format!("{:?}", kind),
                format!("{:?}", token.kind),
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
    
    
    /// Parse qw() word list
    fn parse_qw_list(&mut self) -> ParseResult<Vec<Node>> {
        self.expect(TokenKind::LeftParen)?; // consume (
        
        let mut words = Vec::new();
        
        // Parse space-separated words until )
        while self.peek_kind() != Some(TokenKind::RightParen) && !self.tokens.is_eof() {
            if let Some(TokenKind::Identifier) = self.peek_kind() {
                let token = self.tokens.next()?;
                words.push(Node::new(
                    NodeKind::String { 
                        value: format!("'{}'", token.text), // qw produces single-quoted strings
                        interpolated: false,
                    },
                    SourceLocation { start: token.start, end: token.end }
                ));
            } else {
                let pos = self.current_position();
                return Err(ParseError::syntax("Expected word in qw() list", pos));
            }
        }
        
        self.expect(TokenKind::RightParen)?;
        Ok(words)
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
        
        // For non-empty braces, we need to check if it contains hash-like content
        // Save position to potentially backtrack
        let _saved_pos = self.current_position();
        
        // Try to parse as expression (which might be hash contents)
        let first_expr = match self.parse_expression() {
            Ok(expr) => expr,
            Err(_) => {
                // If we can't parse an expression, parse as block statements
                let mut statements = Vec::new();
                while self.peek_kind() != Some(TokenKind::RightBrace) && !self.tokens.is_eof() {
                    statements.push(self.parse_statement()?);
                }
                
                self.expect(TokenKind::RightBrace)?;
                let end = self.previous_position();
                
                return Ok(Node::new(
                    NodeKind::Block { statements },
                    SourceLocation { start, end }
                ));
            }
        };
        
        // Check if we should close the brace now
        if self.peek_kind() == Some(TokenKind::RightBrace) {
            self.tokens.next()?; // consume }
            let end = self.previous_position();
            
            // Check if the expression is an array literal that should be a hash
            // This happens when parse_comma creates an array from key => value pairs
            if let NodeKind::ArrayLiteral { elements } = &first_expr.kind {
                // Check if this looks like hash pairs (even number of elements)
                if elements.len() % 2 == 0 && !elements.is_empty() {
                    // Convert array elements to hash pairs
                    let mut pairs = Vec::new();
                    for i in (0..elements.len()).step_by(2) {
                        pairs.push((elements[i].clone(), elements[i + 1].clone()));
                    }
                    
                    return Ok(Node::new(
                        NodeKind::HashLiteral { pairs },
                        SourceLocation { start, end }
                    ));
                }
            }
            
            // Otherwise it's a block with a single expression
            return Ok(Node::new(
                NodeKind::Block { statements: vec![first_expr] },
                SourceLocation { start, end }
            ));
        }
        
        // If there's more content, we need to determine if it's hash pairs or block statements
        let mut pairs = Vec::new();
        let mut _is_hash = false;
        
        // Check if next token is => or ,
        let next_kind = self.peek_kind();
        
        // Parse as hash if we see => or comma-separated pairs
        if next_kind == Some(TokenKind::FatArrow) || next_kind == Some(TokenKind::Comma) {
            // Parse as hash
            _is_hash = true;
            
            if self.peek_kind() == Some(TokenKind::FatArrow) {
                // key => value pattern
                self.tokens.next()?; // consume =>
                let value = self.parse_expression()?;
                pairs.push((first_expr, value));
            } else if self.peek_kind() == Some(TokenKind::Comma) {
                // comma-separated pattern: key, value, key2, value2
                self.tokens.next()?; // consume comma
                
                if self.peek_kind() != Some(TokenKind::RightBrace) {
                    let second = self.parse_expression()?;
                    pairs.push((first_expr, second));
                } else {
                    // Trailing comma - treat as single element hash with undef value
                    let undef = Node::new(
                        NodeKind::Identifier { name: "undef".to_string() },
                        SourceLocation { start: self.current_position(), end: self.current_position() }
                    );
                    pairs.push((first_expr, undef));
                }
            }
            
            // Parse remaining pairs
            while self.peek_kind() == Some(TokenKind::Comma) || self.peek_kind() == Some(TokenKind::FatArrow) {
                if self.peek_kind() == Some(TokenKind::Comma) {
                    self.tokens.next()?; // consume comma
                }
                
                if self.peek_kind() == Some(TokenKind::RightBrace) {
                    break;
                }
                
                let key = self.parse_expression()?;
                
                // Check for => or comma after key
                if self.peek_kind() == Some(TokenKind::FatArrow) {
                    self.tokens.next()?; // consume =>
                    let value = self.parse_expression()?;
                    pairs.push((key, value));
                } else if self.peek_kind() == Some(TokenKind::Comma) {
                    self.tokens.next()?; // consume comma
                    
                    if self.peek_kind() == Some(TokenKind::RightBrace) {
                        // Odd number of elements - last one becomes undef value
                        let undef = Node::new(
                            NodeKind::Identifier { name: "undef".to_string() },
                            SourceLocation { start: self.current_position(), end: self.current_position() }
                        );
                        pairs.push((key, undef));
                        break;
                    }
                    
                    let value = self.parse_expression()?;
                    pairs.push((key, value));
                } else if self.peek_kind() == Some(TokenKind::RightBrace) {
                    // Key without value at end - add undef
                    let undef = Node::new(
                        NodeKind::Identifier { name: "undef".to_string() },
                        SourceLocation { start: self.current_position(), end: self.current_position() }
                    );
                    pairs.push((key, undef));
                    break;
                } else {
                    // No comma or => after key - might be missing
                    let value = self.parse_expression()?;
                    pairs.push((key, value));
                }
            }
            
            self.expect(TokenKind::RightBrace)?;
            let end = self.previous_position();
            
            Ok(Node::new(
                NodeKind::HashLiteral { pairs },
                SourceLocation { start, end }
            ))
        } else {
            // Not a hash - parse as block
            if self.peek_kind() == Some(TokenKind::RightBrace) {
                // Single expression block
                self.tokens.next()?; // consume }
                let end = self.previous_position();
                
                return Ok(Node::new(
                    NodeKind::Block { statements: vec![first_expr] },
                    SourceLocation { start, end }
                ));
            }
            
            // Multiple statement block
            let mut statements = vec![first_expr];
            
            // Might need a semicolon
            if self.peek_kind() == Some(TokenKind::Semicolon) {
                self.tokens.next()?;
            }
            
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
    
    /// Check if the parenthesized content after sub name is a prototype (not a signature)
    #[allow(dead_code)]
    fn is_prototype(&mut self) -> bool {
        // Peek at the next token after (
        match self.tokens.peek_second() {
            Ok(token) => {
                // Check if it starts with prototype characters or looks like a prototype
                matches!(token.kind, 
                    TokenKind::ScalarSigil | TokenKind::ArraySigil | 
                    TokenKind::HashSigil | TokenKind::SubSigil |
                    TokenKind::Star | TokenKind::Semicolon |
                    TokenKind::Backslash) ||
                // Check for special vars that look like prototypes ($$, $#, etc)
                (token.kind == TokenKind::Identifier && 
                 token.text.chars().all(|c| matches!(c, '$' | '@' | '%' | '*' | '&' | ';' | '\\')))
            }
            Err(_) => false,
        }
    }
    
    /// Check if the parentheses likely contain a prototype rather than a signature
    fn is_likely_prototype(&mut self) -> ParseResult<bool> {
        // We need to peek past the opening paren without consuming
        // First, ensure we're at a left paren
        if self.tokens.peek()?.kind != TokenKind::LeftParen {
            return Ok(false);
        }
        
        // Use peek_second to look at the token after the paren
        match self.tokens.peek_second() {
            Ok(token) => {
                Ok(match token.kind {
                    // These are definitely prototype sigils
                    TokenKind::ScalarSigil | TokenKind::ArraySigil | TokenKind::HashSigil | 
                    TokenKind::Star | TokenKind::Backslash | TokenKind::Semicolon |
                    TokenKind::BitwiseAnd | TokenKind::GlobSigil => true,
                    // Empty prototype
                    TokenKind::RightParen => true,
                    // Colon indicates named parameter (:$foo), so it's a signature
                    TokenKind::Colon => false,
                    // Identifiers usually mean signature, but could be a special case
                    TokenKind::Identifier => {
                        // Check if it's a sigil-only identifier like "$" or "@"
                        // or the special underscore prototype
                        token.text == "_" || 
                        token.text.chars().all(|c| matches!(c, '$' | '@' | '%' | '*' | '&'))
                    }
                    // Anything else suggests a signature
                    _ => false,
                })
            }
            Err(_) => Ok(false),
        }
    }
    
    /// Parse old-style prototype
    fn parse_prototype(&mut self) -> ParseResult<String> {
        self.expect(TokenKind::LeftParen)?; // consume (
        let mut prototype = String::new();
        
        while !self.tokens.is_eof() {
            let token = self.tokens.next()?;
            
            match token.kind {
                TokenKind::RightParen => {
                    // End of prototype
                    break;
                }
                TokenKind::ScalarSigil => prototype.push('$'),
                TokenKind::ArraySigil => prototype.push('@'),
                TokenKind::HashSigil => prototype.push('%'),
                TokenKind::GlobSigil | TokenKind::Star => prototype.push('*'),
                TokenKind::SubSigil | TokenKind::BitwiseAnd => prototype.push('&'),
                TokenKind::Semicolon => prototype.push(';'),
                TokenKind::Backslash => prototype.push('\\'),
                _ => {
                    // For any other token, just add its text
                    // This handles cases where sigils might be parsed differently
                    prototype.push_str(&token.text);
                }
            }
        }
        
        Ok(prototype)
    }
}

/// Parse substitution parts from a string like "s/pattern/replacement/flags"
fn parse_substitution_parts(s: &str) -> (String, String, String) {
    // Skip 's' and first delimiter
    let mut chars = s.chars();
    chars.next(); // skip 's'
    let delimiter = chars.next().unwrap_or('/');
    
    let is_paired = matches!(delimiter, '{' | '[' | '(' | '<');
    let closing = match delimiter {
        '{' => '}',
        '[' => ']',
        '(' => ')',
        '<' => '>',
        _ => delimiter,
    };
    
    let mut pattern = String::new();
    let mut replacement = String::new();
    let mut modifiers = String::new();
    let mut in_escape = false;
    let mut phase = 0; // 0 = pattern, 1 = replacement, 2 = modifiers
    let mut depth = if is_paired { 1 } else { 0 };
    
    for ch in chars {
        if phase == 2 {
            modifiers.push(ch);
            continue;
        }
        
        if in_escape {
            if phase == 0 {
                pattern.push('\\');
                pattern.push(ch);
            } else {
                replacement.push('\\');
                replacement.push(ch);
            }
            in_escape = false;
            continue;
        }
        
        if ch == '\\' {
            in_escape = true;
            continue;
        }
        
        if is_paired {
            if ch == delimiter {
                depth += 1;
                if phase == 0 {
                    pattern.push(ch);
                } else {
                    replacement.push(ch);
                }
            } else if ch == closing {
                depth -= 1;
                if depth == 0 {
                    if phase == 0 {
                        phase = 1;
                        // Skip whitespace and expect opening delimiter for replacement
                        continue;
                    } else {
                        phase = 2;
                        continue;
                    }
                } else if phase == 0 {
                    pattern.push(ch);
                } else {
                    replacement.push(ch);
                }
            } else if phase == 1 && depth == 0 && ch == delimiter {
                // Start of replacement part in paired delimiters
                depth = 1;
            } else if phase == 1 && depth == 0 && ch.is_whitespace() {
                // Skip whitespace between paired delimiters
                continue;
            } else if phase == 0 {
                pattern.push(ch);
            } else if phase == 1 && depth > 0 {
                replacement.push(ch);
            }
        } else if ch == delimiter {
            phase += 1;
        } else if phase == 0 {
            pattern.push(ch);
        } else if phase == 1 {
            replacement.push(ch);
        }
    }
    
    (pattern, replacement, modifiers)
}

/// Parse heredoc delimiter from a string like "<<EOF", "<<'EOF'", "<<~EOF"
fn parse_heredoc_delimiter(s: &str) -> (&str, bool, bool) {
    let mut chars = s.chars();
    
    // Skip <<
    chars.next();
    chars.next();
    
    // Check for indented heredoc
    let indented = if chars.as_str().starts_with('~') {
        chars.next();
        true
    } else {
        false
    };
    
    let rest = chars.as_str().trim();
    
    // Check quoting to determine interpolation
    let (delimiter, interpolated) = if rest.starts_with('"') && rest.ends_with('"') {
        // Double-quoted: interpolated
        (&rest[1..rest.len()-1], true)
    } else if rest.starts_with('\'') && rest.ends_with('\'') {
        // Single-quoted: not interpolated
        (&rest[1..rest.len()-1], false)
    } else {
        // Bare word: interpolated
        (rest, true)
    };
    
    (delimiter, interpolated, indented)
}

/// Extract pattern and modifiers from a regex string like "/pattern/modifiers"
fn extract_regex_parts(s: &str) -> (String, String) {
    let chars: Vec<char> = s.chars().collect();
    if chars.is_empty() {
        return (s.to_string(), String::new());
    }
    
    // Get the delimiter
    let delimiter = chars[0];
    let closing = match delimiter {
        '{' => '}',
        '[' => ']',
        '(' => ')',
        '<' => '>',
        _ => delimiter,
    };
    
    // Find the closing delimiter
    let mut i = 1;
    let mut escaped = false;
    let mut depth = 1;
    let is_paired = delimiter != closing;
    
    while i < chars.len() {
        if !escaped {
            if chars[i] == delimiter && is_paired {
                depth += 1;
            } else if chars[i] == closing {
                if is_paired {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                } else {
                    break;
                }
            }
        }
        escaped = !escaped && chars[i] == '\\';
        i += 1;
    }
    
    if i < chars.len() {
        let pattern = chars[0..=i].iter().collect();
        let modifiers = chars[i+1..].iter().collect();
        (pattern, modifiers)
    } else {
        (s.to_string(), String::new())
    }
}

/// Parse transliteration parts from a string like "tr/search/replace/flags"
fn parse_transliteration_parts(s: &str) -> (String, String, String) {
    // Skip 'tr' or 'y' and first delimiter
    let mut chars = s.chars();
    if s.starts_with("tr") {
        chars.next(); // skip 't'
        chars.next(); // skip 'r'
    } else {
        chars.next(); // skip 'y'
    }
    let delimiter = chars.next().unwrap_or('/');
    
    let is_paired = matches!(delimiter, '{' | '[' | '(' | '<');
    let closing = match delimiter {
        '{' => '}',
        '[' => ']',
        '(' => ')',
        '<' => '>',
        _ => delimiter,
    };
    
    let mut search = String::new();
    let mut replace = String::new();
    let mut modifiers = String::new();
    let mut in_escape = false;
    let mut phase = 0; // 0 = search, 1 = replace, 2 = modifiers
    let mut depth = if is_paired { 1 } else { 0 };
    
    for ch in chars {
        if phase == 2 {
            modifiers.push(ch);
            continue;
        }
        
        if in_escape {
            if phase == 0 {
                search.push('\\');
                search.push(ch);
            } else {
                replace.push('\\');
                replace.push(ch);
            }
            in_escape = false;
            continue;
        }
        
        if ch == '\\' {
            in_escape = true;
            continue;
        }
        
        if is_paired {
            if ch == delimiter {
                depth += 1;
                if phase == 0 {
                    search.push(ch);
                } else {
                    replace.push(ch);
                }
            } else if ch == closing {
                depth -= 1;
                if depth == 0 {
                    if phase == 0 {
                        phase = 1;
                        // Skip whitespace and expect opening delimiter for replacement
                        continue;
                    } else {
                        phase = 2;
                        continue;
                    }
                } else if phase == 0 {
                    search.push(ch);
                } else {
                    replace.push(ch);
                }
            } else if phase == 1 && depth == 0 && ch == delimiter {
                // Start of replace part in paired delimiters
                depth = 1;
            } else if phase == 1 && depth == 0 && ch.is_whitespace() {
                // Skip whitespace between paired delimiters
                continue;
            } else if phase == 0 {
                search.push(ch);
            } else if phase == 1 && depth > 0 {
                replace.push(ch);
            }
        } else if ch == delimiter {
            phase += 1;
        } else if phase == 0 {
            search.push(ch);
        } else if phase == 1 {
            replace.push(ch);
        }
    }
    
    (search, replace, modifiers)
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
    
    #[test]
    fn test_list_declarations() {
        // Test simple list declaration
        let mut parser = Parser::new("my ($x, $y);");
        let result = parser.parse();
        assert!(result.is_ok());
        let ast = result.unwrap();
        println!("List declaration AST: {}", ast.to_sexp());
        
        // Test list declaration with initialization
        let mut parser = Parser::new("state ($a, $b) = (1, 2);");
        let result = parser.parse();
        assert!(result.is_ok());
        let ast = result.unwrap();
        println!("List declaration with init AST: {}", ast.to_sexp());
        
        // Test mixed sigils
        let mut parser = Parser::new("our ($scalar, @array, %hash);");
        let result = parser.parse();
        assert!(result.is_ok());
        let ast = result.unwrap();
        println!("Mixed sigils AST: {}", ast.to_sexp());
        
        // Test empty list
        let mut parser = Parser::new("my ();");
        let result = parser.parse();
        assert!(result.is_ok());
        let ast = result.unwrap();
        println!("Empty list AST: {}", ast.to_sexp());
    }
}