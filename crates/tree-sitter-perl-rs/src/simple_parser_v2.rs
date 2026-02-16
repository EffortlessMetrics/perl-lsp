//! Simple recursive descent parser for Perl with context-aware lexing
//!
//! This demonstrates the token-based approach with proper slash disambiguation

use crate::context_lexer_simple::ContextLexer;
use crate::simple_token::Token;
use crate::token_ast::AstNode;
use std::sync::Arc;

pub struct SimpleParser<'source> {
    lexer: ContextLexer<'source>,
    current: Option<Token>,
    source: &'source str,
}

impl<'source> SimpleParser<'source> {
    pub fn new(input: &'source str) -> Self {
        let mut lexer = ContextLexer::new(input);
        let current = lexer.next();

        Self { lexer, current, source: input }
    }

    /// Get current token or EOF
    fn current_token(&self) -> Token {
        self.current.clone().unwrap_or(Token::Eof)
    }

    /// Peek at current token
    fn peek(&self) -> Token {
        self.current_token()
    }

    /// Consume current token and advance to next
    fn next(&mut self) -> Token {
        let token = self.current_token();
        self.current = self.lexer.next();
        token
    }

    /// Check if current token matches expected
    fn check(&self, expected: &Token) -> bool {
        self.peek() == *expected
    }

    /// Consume token if it matches expected
    fn consume(&mut self, expected: Token) -> Result<(), String> {
        if self.check(&expected) {
            self.next();
            Ok(())
        } else {
            Err(format!("Expected {:?}, got {:?}", expected, self.peek()))
        }
    }

    /// Skip newlines
    fn skip_newlines(&mut self) {
        while self.check(&Token::Newline) {
            self.next();
        }
    }

    pub fn parse(&mut self) -> Result<AstNode, String> {
        self.parse_statements()
    }

    fn parse_statements(&mut self) -> Result<AstNode, String> {
        let mut statements = Vec::new();

        loop {
            self.skip_newlines();

            if self.check(&Token::Eof) {
                break;
            }

            statements.push(self.parse_statement()?);
        }

        Ok(AstNode {
            node_type: "program".to_string(),
            start_position: 0,
            end_position: self.source.len(),
            value: None,
            children: statements,
        })
    }

    fn parse_statement(&mut self) -> Result<AstNode, String> {
        match self.peek() {
            Token::My | Token::Our | Token::Local | Token::State => {
                self.parse_variable_declaration()
            }
            Token::If => self.parse_if_statement(),
            Token::Unless => self.parse_unless_statement(),
            Token::While => self.parse_while_statement(),
            Token::Until => self.parse_until_statement(),
            Token::For | Token::Foreach => self.parse_for_statement(),
            Token::Sub => self.parse_subroutine(),
            Token::Return => self.parse_return_statement(),
            Token::Use => self.parse_use_statement(),
            Token::Package => self.parse_package_statement(),
            _ => {
                // Expression statement
                let expr = self.parse_expression()?;
                self.consume_statement_terminator()?;
                Ok(expr)
            }
        }
    }

    fn parse_variable_declaration(&mut self) -> Result<AstNode, String> {
        let decl_type = match self.next() {
            Token::My => "my",
            Token::Our => "our",
            Token::Local => "local",
            Token::State => "state",
            unexpected => {
                // Error: Unexpected token in variable declaration context
                // Expected one of: my, our, local, state
                // This error occurs when the parser encounters an invalid token after parsing
                // statement-level context that requires a variable declaration keyword.
                return Err(format!(
                    "Expected variable declaration keyword (my/our/local/state), found {:?}",
                    unexpected
                ));
            }
        };

        let var = self.parse_variable()?;

        let value = if self.check(&Token::Assign) {
            self.next(); // consume =
            Some(Box::new(self.parse_expression()?))
        } else {
            None
        };

        self.consume_statement_terminator()?;

        Ok(AstNode {
            node_type: format!("{}_declaration", decl_type),
            start_position: 0,
            end_position: 0,
            value: None,
            children: if let Some(val) = value { vec![var, *val] } else { vec![var] },
        })
    }

    fn parse_variable(&mut self) -> Result<AstNode, String> {
        let (node_type, value) = match self.peek() {
            Token::ScalarVar => ("scalar_variable", "$var"),
            Token::ArrayVar => ("array_variable", "@var"),
            Token::HashVar => ("hash_variable", "%var"),
            _ => return Err(format!("Expected variable, got {:?}", self.peek())),
        };

        self.next();

        Ok(AstNode {
            node_type: node_type.to_string(),
            start_position: 0,
            end_position: 0,
            value: Some(Arc::from(value)),
            children: vec![],
        })
    }

    fn parse_expression(&mut self) -> Result<AstNode, String> {
        self.parse_assignment()
    }

    fn parse_assignment(&mut self) -> Result<AstNode, String> {
        let mut left = self.parse_ternary()?;

        if matches!(
            self.peek(),
            Token::Assign
                | Token::PlusAssign
                | Token::MinusAssign
                | Token::StarAssign
                | Token::SlashAssign
                | Token::PercentAssign
                | Token::DotAssign
                | Token::AndAssign
                | Token::OrAssign
                | Token::XorAssign
                | Token::LshiftAssign
                | Token::RshiftAssign
        ) {
            let op = self.next();
            let right = self.parse_assignment()?;

            left = AstNode {
                node_type: "assignment".to_string(),
                start_position: 0,
                end_position: 0,
                value: Some(Arc::from(format!("{:?}", op))),
                children: vec![left, right],
            };
        }

        Ok(left)
    }

    fn parse_ternary(&mut self) -> Result<AstNode, String> {
        let mut expr = self.parse_logical_or()?;

        if self.check(&Token::Question) {
            self.next();
            let then_expr = self.parse_expression()?;
            self.consume(Token::Colon)?;
            let else_expr = self.parse_ternary()?;

            expr = AstNode {
                node_type: "ternary_expression".to_string(),
                start_position: 0,
                end_position: 0,
                value: None,
                children: vec![expr, then_expr, else_expr],
            };
        }

        Ok(expr)
    }

    fn parse_logical_or(&mut self) -> Result<AstNode, String> {
        let mut left = self.parse_logical_and()?;

        while matches!(self.peek(), Token::LogOr | Token::BitOr) {
            let op = self.next();
            let right = self.parse_logical_and()?;

            left = AstNode {
                node_type: "binary_expression".to_string(),
                start_position: 0,
                end_position: 0,
                value: Some(Arc::from(format!("{:?}", op))),
                children: vec![left, right],
            };
        }

        Ok(left)
    }

    fn parse_logical_and(&mut self) -> Result<AstNode, String> {
        let mut left = self.parse_equality()?;

        while matches!(self.peek(), Token::LogAnd | Token::BitAnd) {
            let op = self.next();
            let right = self.parse_equality()?;

            left = AstNode {
                node_type: "binary_expression".to_string(),
                start_position: 0,
                end_position: 0,
                value: Some(Arc::from(format!("{:?}", op))),
                children: vec![left, right],
            };
        }

        Ok(left)
    }

    fn parse_equality(&mut self) -> Result<AstNode, String> {
        let mut left = self.parse_relational()?;

        while matches!(
            self.peek(),
            Token::NumEq
                | Token::NumNe
                | Token::StrEq
                | Token::StrNe
                | Token::Cmp
                | Token::Spaceship
        ) {
            let op = self.next();
            let right = self.parse_relational()?;

            left = AstNode {
                node_type: "binary_expression".to_string(),
                start_position: 0,
                end_position: 0,
                value: Some(Arc::from(format!("{:?}", op))),
                children: vec![left, right],
            };
        }

        Ok(left)
    }

    fn parse_relational(&mut self) -> Result<AstNode, String> {
        let mut left = self.parse_additive()?;

        while matches!(
            self.peek(),
            Token::NumLt | Token::NumGt | Token::NumLe | Token::NumGe | Token::Isa
        ) {
            let op = self.next();
            let right = self.parse_additive()?;

            left = AstNode {
                node_type: "binary_expression".to_string(),
                start_position: 0,
                end_position: 0,
                value: Some(Arc::from(format!("{:?}", op))),
                children: vec![left, right],
            };
        }

        Ok(left)
    }

    fn parse_additive(&mut self) -> Result<AstNode, String> {
        let mut left = self.parse_multiplicative()?;

        while matches!(self.peek(), Token::Plus | Token::Minus | Token::Dot) {
            let op = self.next();
            let right = self.parse_multiplicative()?;

            left = AstNode {
                node_type: "binary_expression".to_string(),
                start_position: 0,
                end_position: 0,
                value: Some(Arc::from(format!("{:?}", op))),
                children: vec![left, right],
            };
        }

        Ok(left)
    }

    fn parse_multiplicative(&mut self) -> Result<AstNode, String> {
        let mut left = self.parse_regex_match()?;

        while matches!(self.peek(), Token::Multiply | Token::Divide | Token::Modulo) {
            let op = self.next();
            let right = self.parse_regex_match()?;

            left = AstNode {
                node_type: "binary_expression".to_string(),
                start_position: 0,
                end_position: 0,
                value: Some(Arc::from(format!("{:?}", op))),
                children: vec![left, right],
            };
        }

        Ok(left)
    }

    fn parse_regex_match(&mut self) -> Result<AstNode, String> {
        let mut left = self.parse_unary()?;

        while matches!(self.peek(), Token::BinMatch | Token::BinNotMatch) {
            let op = self.next();
            let right = self.parse_unary()?;

            left = AstNode {
                node_type: "regex_match".to_string(),
                start_position: 0,
                end_position: 0,
                value: Some(Arc::from(format!("{:?}", op))),
                children: vec![left, right],
            };
        }

        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<AstNode, String> {
        match self.peek() {
            Token::Not | Token::BitNot | Token::Plus | Token::Minus => {
                let op = self.next();
                let expr = self.parse_unary()?;

                Ok(AstNode {
                    node_type: "unary_expression".to_string(),
                    start_position: 0,
                    end_position: 0,
                    value: Some(Arc::from(format!("{:?}", op))),
                    children: vec![expr],
                })
            }
            _ => self.parse_postfix(),
        }
    }

    fn parse_postfix(&mut self) -> Result<AstNode, String> {
        let mut expr = self.parse_primary()?;

        loop {
            match self.peek() {
                Token::LBracket => {
                    self.next();
                    let index = self.parse_expression()?;
                    self.consume(Token::RBracket)?;

                    expr = AstNode {
                        node_type: "array_access".to_string(),
                        start_position: 0,
                        end_position: 0,
                        value: None,
                        children: vec![expr, index],
                    };
                }
                Token::LBrace => {
                    self.next();
                    let key = self.parse_expression()?;
                    self.consume(Token::RBrace)?;

                    expr = AstNode {
                        node_type: "hash_access".to_string(),
                        start_position: 0,
                        end_position: 0,
                        value: None,
                        children: vec![expr, key],
                    };
                }
                Token::Arrow => {
                    self.next();

                    if self.check(&Token::LBracket) {
                        self.next();
                        let index = self.parse_expression()?;
                        self.consume(Token::RBracket)?;

                        expr = AstNode {
                            node_type: "deref_array_access".to_string(),
                            start_position: 0,
                            end_position: 0,
                            value: None,
                            children: vec![expr, index],
                        };
                    } else if self.check(&Token::LBrace) {
                        self.next();
                        let key = self.parse_expression()?;
                        self.consume(Token::RBrace)?;

                        expr = AstNode {
                            node_type: "deref_hash_access".to_string(),
                            start_position: 0,
                            end_position: 0,
                            value: None,
                            children: vec![expr, key],
                        };
                    } else if self.check(&Token::Identifier) {
                        let method = self.next();

                        expr = AstNode {
                            node_type: "method_call".to_string(),
                            start_position: 0,
                            end_position: 0,
                            value: Some(Arc::from(format!("{:?}", method))),
                            children: vec![expr],
                        };

                        // Parse method arguments if present
                        if self.check(&Token::LParen) {
                            self.next();
                            let mut args = vec![];

                            while !self.check(&Token::RParen) {
                                args.push(self.parse_expression()?);

                                if !self.check(&Token::RParen) {
                                    self.consume(Token::Comma)?;
                                }
                            }

                            self.consume(Token::RParen)?;
                            expr.children.extend(args);
                        }
                    }
                }
                _ => break,
            }
        }

        Ok(expr)
    }

    fn parse_primary(&mut self) -> Result<AstNode, String> {
        match self.peek() {
            Token::IntegerLiteral | Token::FloatLiteral => {
                self.next();
                Ok(AstNode {
                    node_type: "number".to_string(),
                    start_position: 0,
                    end_position: 0,
                    value: Some(Arc::from("42")), // Placeholder
                    children: vec![],
                })
            }
            Token::StringLiteral => {
                self.next();
                Ok(AstNode {
                    node_type: "string".to_string(),
                    start_position: 0,
                    end_position: 0,
                    value: Some(Arc::from("string")), // Placeholder
                    children: vec![],
                })
            }
            Token::Backtick => {
                self.next();
                Ok(AstNode {
                    node_type: "backtick".to_string(),
                    start_position: 0,
                    end_position: 0,
                    value: Some(Arc::from("cmd")), // Placeholder
                    children: vec![],
                })
            }
            Token::ScalarVar | Token::ArrayVar | Token::HashVar => self.parse_variable(),
            Token::LParen => {
                self.next();
                let expr = self.parse_expression()?;
                self.consume(Token::RParen)?;
                Ok(expr)
            }
            Token::LBracket => self.parse_array_literal(),
            Token::LBrace => self.parse_hash_literal(),
            Token::Regex => {
                self.next();
                Ok(AstNode {
                    node_type: "regex_literal".to_string(),
                    start_position: 0,
                    end_position: 0,
                    value: Some(Arc::from("/pattern/")), // Placeholder
                    children: vec![],
                })
            }
            Token::BinMatch => {
                self.next();
                // Handle regex on right side
                let regex = self.parse_unary()?;
                Ok(regex)
            }
            Token::Identifier | Token::Bareword => {
                let name = self.next();

                // Check if it's a function call
                if self.check(&Token::LParen) {
                    self.next();
                    let mut args = vec![];

                    while !self.check(&Token::RParen) {
                        args.push(self.parse_expression()?);

                        if !self.check(&Token::RParen) {
                            self.consume(Token::Comma)?;
                        }
                    }

                    self.consume(Token::RParen)?;

                    Ok(AstNode {
                        node_type: "function_call".to_string(),
                        start_position: 0,
                        end_position: 0,
                        value: Some(Arc::from(format!("{:?}", name))),
                        children: args,
                    })
                } else {
                    Ok(AstNode {
                        node_type: "bareword".to_string(),
                        start_position: 0,
                        end_position: 0,
                        value: Some(Arc::from(format!("{:?}", name))),
                        children: vec![],
                    })
                }
            }
            _ => Err(format!("Unexpected token: {:?}", self.peek())),
        }
    }

    fn parse_array_literal(&mut self) -> Result<AstNode, String> {
        self.consume(Token::LBracket)?;
        let mut elements = vec![];

        while !self.check(&Token::RBracket) {
            elements.push(self.parse_expression()?);

            if !self.check(&Token::RBracket) {
                self.consume(Token::Comma)?;
            }
        }

        self.consume(Token::RBracket)?;

        Ok(AstNode {
            node_type: "array_literal".to_string(),
            start_position: 0,
            end_position: 0,
            value: None,
            children: elements,
        })
    }

    fn parse_hash_literal(&mut self) -> Result<AstNode, String> {
        self.consume(Token::LBrace)?;
        let mut pairs = vec![];

        while !self.check(&Token::RBrace) {
            let key = self.parse_expression()?;
            self.consume(Token::FatArrow)?;
            let value = self.parse_expression()?;

            pairs.push(AstNode {
                node_type: "hash_pair".to_string(),
                start_position: 0,
                end_position: 0,
                value: None,
                children: vec![key, value],
            });

            if !self.check(&Token::RBrace) {
                self.consume(Token::Comma)?;
            }
        }

        self.consume(Token::RBrace)?;

        Ok(AstNode {
            node_type: "hash_literal".to_string(),
            start_position: 0,
            end_position: 0,
            value: None,
            children: pairs,
        })
    }

    fn parse_if_statement(&mut self) -> Result<AstNode, String> {
        self.consume(Token::If)?;
        self.consume(Token::LParen)?;
        let condition = self.parse_expression()?;
        self.consume(Token::RParen)?;
        let then_block = self.parse_block()?;

        let mut children = vec![condition, then_block];

        // Handle elsif/else
        while self.check(&Token::Elsif) {
            self.next();
            self.consume(Token::LParen)?;
            let elsif_cond = self.parse_expression()?;
            self.consume(Token::RParen)?;
            let elsif_block = self.parse_block()?;

            children.push(AstNode {
                node_type: "elsif_clause".to_string(),
                start_position: 0,
                end_position: 0,
                value: None,
                children: vec![elsif_cond, elsif_block],
            });
        }

        if self.check(&Token::Else) {
            self.next();
            let else_block = self.parse_block()?;
            children.push(else_block);
        }

        Ok(AstNode {
            node_type: "if_statement".to_string(),
            start_position: 0,
            end_position: 0,
            value: None,
            children,
        })
    }

    fn parse_unless_statement(&mut self) -> Result<AstNode, String> {
        self.consume(Token::Unless)?;
        self.consume(Token::LParen)?;
        let condition = self.parse_expression()?;
        self.consume(Token::RParen)?;
        let body = self.parse_block()?;

        Ok(AstNode {
            node_type: "unless_statement".to_string(),
            start_position: 0,
            end_position: 0,
            value: None,
            children: vec![condition, body],
        })
    }

    fn parse_while_statement(&mut self) -> Result<AstNode, String> {
        self.consume(Token::While)?;
        self.consume(Token::LParen)?;
        let condition = self.parse_expression()?;
        self.consume(Token::RParen)?;
        let body = self.parse_block()?;

        Ok(AstNode {
            node_type: "while_statement".to_string(),
            start_position: 0,
            end_position: 0,
            value: None,
            children: vec![condition, body],
        })
    }

    fn parse_until_statement(&mut self) -> Result<AstNode, String> {
        self.consume(Token::Until)?;
        self.consume(Token::LParen)?;
        let condition = self.parse_expression()?;
        self.consume(Token::RParen)?;
        let body = self.parse_block()?;

        Ok(AstNode {
            node_type: "until_statement".to_string(),
            start_position: 0,
            end_position: 0,
            value: None,
            children: vec![condition, body],
        })
    }

    fn parse_for_statement(&mut self) -> Result<AstNode, String> {
        let _keyword = self.next(); // for or foreach

        // C-style for loop
        if self.check(&Token::LParen) {
            self.next();

            // Init
            let init =
                if !self.check(&Token::Semicolon) { Some(self.parse_expression()?) } else { None };
            self.consume(Token::Semicolon)?;

            // Condition
            let cond =
                if !self.check(&Token::Semicolon) { Some(self.parse_expression()?) } else { None };
            self.consume(Token::Semicolon)?;

            // Update
            let update =
                if !self.check(&Token::RParen) { Some(self.parse_expression()?) } else { None };
            self.consume(Token::RParen)?;

            let body = self.parse_block()?;

            let mut children = vec![];
            if let Some(i) = init {
                children.push(i);
            }
            if let Some(c) = cond {
                children.push(c);
            }
            if let Some(u) = update {
                children.push(u);
            }
            children.push(body);

            Ok(AstNode {
                node_type: "c_style_for".to_string(),
                start_position: 0,
                end_position: 0,
                value: None,
                children,
            })
        } else {
            // Perl-style foreach
            let var = if matches!(self.peek(), Token::My | Token::Our | Token::Local) {
                self.parse_variable_declaration()?
            } else if matches!(self.peek(), Token::ScalarVar) {
                self.parse_variable()?
            } else {
                return Err("Expected variable in foreach".to_string());
            };

            self.consume(Token::LParen)?;
            let list = self.parse_expression()?;
            self.consume(Token::RParen)?;

            let body = self.parse_block()?;

            Ok(AstNode {
                node_type: "foreach_statement".to_string(),
                start_position: 0,
                end_position: 0,
                value: None,
                children: vec![var, list, body],
            })
        }
    }

    fn parse_subroutine(&mut self) -> Result<AstNode, String> {
        self.consume(Token::Sub)?;

        let name = if self.check(&Token::Identifier) {
            let n = self.next();
            Some(Arc::from(format!("{:?}", n)))
        } else {
            None
        };

        // Parse signature if present
        let signature = if self.check(&Token::LParen) {
            self.next();
            let mut params = vec![];

            while !self.check(&Token::RParen) {
                params.push(self.parse_expression()?);

                if !self.check(&Token::RParen) {
                    self.consume(Token::Comma)?;
                }
            }

            self.consume(Token::RParen)?;
            Some(params)
        } else {
            None
        };

        let body = self.parse_block()?;

        let mut children = vec![body];
        if let Some(sig) = signature {
            children.extend(sig);
        }

        Ok(AstNode {
            node_type: "subroutine".to_string(),
            start_position: 0,
            end_position: 0,
            value: name,
            children,
        })
    }

    fn parse_return_statement(&mut self) -> Result<AstNode, String> {
        self.consume(Token::Return)?;

        let value = if !matches!(self.peek(), Token::Semicolon | Token::Newline | Token::Eof) {
            Some(self.parse_expression()?)
        } else {
            None
        };

        self.consume_statement_terminator()?;

        Ok(AstNode {
            node_type: "return_statement".to_string(),
            start_position: 0,
            end_position: 0,
            value: None,
            children: value.map(|v| vec![v]).unwrap_or_default(),
        })
    }

    fn parse_use_statement(&mut self) -> Result<AstNode, String> {
        self.consume(Token::Use)?;

        let module = if self.check(&Token::Identifier) || self.check(&Token::Bareword) {
            let name = self.next();
            Some(Arc::from(format!("{:?}", name)))
        } else {
            return Err("Expected module name after 'use'".to_string());
        };

        self.consume_statement_terminator()?;

        Ok(AstNode {
            node_type: "use_statement".to_string(),
            start_position: 0,
            end_position: 0,
            value: module,
            children: vec![],
        })
    }

    fn parse_package_statement(&mut self) -> Result<AstNode, String> {
        self.consume(Token::Package)?;

        let name = if self.check(&Token::Identifier) || self.check(&Token::Bareword) {
            let n = self.next();
            Some(Arc::from(format!("{:?}", n)))
        } else {
            return Err("Expected package name".to_string());
        };

        self.consume_statement_terminator()?;

        Ok(AstNode {
            node_type: "package_statement".to_string(),
            start_position: 0,
            end_position: 0,
            value: name,
            children: vec![],
        })
    }

    fn parse_block(&mut self) -> Result<AstNode, String> {
        self.consume(Token::LBrace)?;

        let mut statements = Vec::new();

        loop {
            self.skip_newlines();

            if self.check(&Token::RBrace) {
                self.next();
                break;
            }

            statements.push(self.parse_statement()?);
        }

        Ok(AstNode {
            node_type: "block".to_string(),
            start_position: 0,
            end_position: 0,
            value: None,
            children: statements,
        })
    }

    fn consume_statement_terminator(&mut self) -> Result<(), String> {
        match self.peek() {
            Token::Semicolon => {
                self.next();
                Ok(())
            }
            Token::Newline | Token::Eof => Ok(()), // Perl allows newline/EOF as terminator
            token => Err(format!("Expected ; or newline, got {:?}", token)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slash_as_division() {
        let input = "my $x = 10 / 2;";
        use perl_tdd_support::must;
        let mut parser = SimpleParser::new(input);
        let ast = must(parser.parse());

        assert_eq!(ast.node_type, "program");
        assert_eq!(ast.children.len(), 1);

        let decl = &ast.children[0];
        assert_eq!(decl.node_type, "my_declaration");
        assert_eq!(decl.children.len(), 2);

        let expr = &decl.children[1];
        assert_eq!(expr.node_type, "binary_expression");
        assert_eq!(expr.value.as_ref().map(|s| s.as_ref()), Some("Divide"));
    }

    #[test]
    fn test_slash_as_regex() {
        let input = "if (/test/) { print; }";
        use perl_tdd_support::must;
        let mut parser = SimpleParser::new(input);
        let ast = must(parser.parse());

        assert_eq!(ast.node_type, "program");
        assert_eq!(ast.children.len(), 1);

        let if_stmt = &ast.children[0];
        assert_eq!(if_stmt.node_type, "if_statement");

        let condition = &if_stmt.children[0];
        assert_eq!(condition.node_type, "regex_literal");
    }

    #[test]
    fn test_regex_match_operator() {
        let input = "$text =~ /pattern/;";
        use perl_tdd_support::must;
        let mut parser = SimpleParser::new(input);
        let ast = must(parser.parse());

        assert_eq!(ast.node_type, "program");
        assert_eq!(ast.children.len(), 1);

        let match_expr = &ast.children[0];
        assert_eq!(match_expr.node_type, "regex_match");
        assert_eq!(match_expr.children.len(), 2);

        assert_eq!(match_expr.children[0].node_type, "scalar_variable");
        assert_eq!(match_expr.children[1].node_type, "regex_literal");
    }

    #[test]
    fn test_method_calls() {
        let input = "$obj->method();";
        use perl_tdd_support::must;
        let mut parser = SimpleParser::new(input);
        let ast = must(parser.parse());

        assert_eq!(ast.node_type, "program");
        assert_eq!(ast.children.len(), 1);

        let call = &ast.children[0];
        assert_eq!(call.node_type, "method_call");
    }

    #[test]
    fn test_complex_expression() {
        let input = "my $result = $a + $b * $c / 2 - ($d =~ /test/);";
        use perl_tdd_support::must;
        let mut parser = SimpleParser::new(input);
        let ast = must(parser.parse());

        assert_eq!(ast.node_type, "program");
        assert_eq!(ast.children.len(), 1);

        let decl = &ast.children[0];
        assert_eq!(decl.node_type, "my_declaration");
        assert_eq!(decl.children.len(), 2);

        // The expression should have proper precedence
        let expr = &decl.children[1];
        assert_eq!(expr.node_type, "binary_expression");
        assert_eq!(expr.value.as_ref().map(|s| s.as_ref()), Some("Minus"));
    }
}
