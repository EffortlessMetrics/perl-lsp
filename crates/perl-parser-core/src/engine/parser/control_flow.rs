impl<'a> Parser<'a> {
    /// Parse if statement
    fn parse_if_statement(&mut self) -> ParseResult<Node> {
        let start = self.current_position();
        self.tokens.next()?; // consume 'if'

        self.expect(TokenKind::LeftParen)?;

        // Check if this is a variable declaration in the condition
        let condition = if matches!(
            self.peek_kind(),
            Some(TokenKind::My)
                | Some(TokenKind::Our)
                | Some(TokenKind::Local)
                | Some(TokenKind::State)
        ) {
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
            let elsif_cond = if matches!(
                self.peek_kind(),
                Some(TokenKind::My)
                    | Some(TokenKind::Our)
                    | Some(TokenKind::Local)
                    | Some(TokenKind::State)
            ) {
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
            SourceLocation { start, end },
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
            NodeKind::Unary { op: "!".to_string(), operand: Box::new(condition) },
            SourceLocation { start, end: self.previous_position() },
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
            SourceLocation { start, end },
        ))
    }

    /// Parse while loop
    fn parse_while_statement(&mut self) -> ParseResult<Node> {
        let start = self.current_position();
        self.tokens.next()?; // consume 'while'

        self.expect(TokenKind::LeftParen)?;

        // Check if this is a variable declaration in the condition
        let condition = if matches!(
            self.peek_kind(),
            Some(TokenKind::My)
                | Some(TokenKind::Our)
                | Some(TokenKind::Local)
                | Some(TokenKind::State)
        ) {
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
            SourceLocation { start, end },
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
            NodeKind::Unary { op: "!".to_string(), operand: Box::new(condition) },
            SourceLocation { start, end: self.previous_position() },
        );

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
                condition: Box::new(negated_condition),
                body: Box::new(body),
                continue_block,
            },
            SourceLocation { start, end },
        ))
    }

    /// Parse for loop
    fn parse_for_statement(&mut self) -> ParseResult<Node> {
        let start = self.current_position();
        self.tokens.next()?; // consume 'for'

        // Check if it's a foreach-style for loop
        if matches!(self.peek_kind(), Some(TokenKind::My)) || self.is_variable_start() {
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
                    NodeKind::Variable { sigil: "$".to_string(), name: "_".to_string() },
                    SourceLocation { start, end: start },
                );

                return Ok(Node::new(
                    NodeKind::Foreach {
                        variable: Box::new(implicit_var),
                        list: Box::new(expr),
                        body: Box::new(body),
                        continue_block: None, // No continue block for implicit foreach
                    },
                    SourceLocation { start, end },
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
            NodeKind::For { init, condition, update, body: Box::new(body), continue_block },
            SourceLocation { start, end },
        ))
    }

    /// Parse foreach loop
    fn parse_foreach_statement(&mut self) -> ParseResult<Node> {
        let start = self.current_position();
        self.tokens.next()?; // consume 'foreach'

        // Set flag to prevent semicolon consumption in variable declaration
        self.in_for_loop_init = true;
        let variable = if self.peek_kind() == Some(TokenKind::My) {
            self.parse_variable_declaration()?
        } else {
            self.parse_variable()?
        };
        self.in_for_loop_init = false;

        self.expect(TokenKind::LeftParen)?;
        let list = self.parse_expression()?;
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
            NodeKind::Foreach {
                variable: Box::new(variable),
                list: Box::new(list),
                body: Box::new(body),
                continue_block,
            },
            SourceLocation { start, end },
        ))
    }

    /// Parse foreach-style for loop
    fn parse_foreach_style_for(&mut self) -> ParseResult<Node> {
        // Set flag to prevent semicolon consumption in variable declaration
        self.in_for_loop_init = true;
        let variable = if self.peek_kind() == Some(TokenKind::My) {
            self.parse_variable_declaration()?
        } else {
            self.parse_variable()?
        };
        self.in_for_loop_init = false;

        self.expect(TokenKind::LeftParen)?;
        let list = self.parse_expression()?;
        self.expect(TokenKind::RightParen)?;

        let body = self.parse_block()?;

        // Handle continue block
        let continue_block = if self.peek_kind() == Some(TokenKind::Continue) {
            self.tokens.next()?; // consume 'continue'
            Some(Box::new(self.parse_block()?))
        } else {
            None
        };

        let start = variable.location.start;
        let end = self.previous_position();

        Ok(Node::new(
            NodeKind::Foreach {
                variable: Box::new(variable),
                list: Box::new(list),
                body: Box::new(body),
                continue_block,
            },
            SourceLocation { start, end },
        ))
    }

    /// Parse format declaration
    /// Parse return statement
    fn parse_return(&mut self) -> ParseResult<Node> {
        let start = self.current_position();
        self.tokens.next()?; // consume 'return'

        // Check if we have a value to return - only stop at clear ends or statement modifiers
        let value = if Self::is_statement_terminator(self.peek_kind())
            || matches!(self.peek_kind(), Some(TokenKind::RightBrace))
            || matches!(self.peek_kind(), Some(k) if Self::is_stmt_modifier_kind(k))
        {
            None
        } else {
            // Parse the return value
            Some(Box::new(self.parse_expression()?))
        };

        let end = value.as_ref().map(|v| v.location.end).unwrap_or(self.previous_position());
        Ok(Node::new(NodeKind::Return { value }, SourceLocation { start, end }))
    }

    /// Parse eval expression/block
    fn parse_eval(&mut self) -> ParseResult<Node> {
        let start = self.consume_token()?.start; // consume 'eval'

        // Eval can take either a block or a string expression
        if self.peek_kind() == Some(TokenKind::LeftBrace) {
            // eval { ... }
            let block = self.parse_block()?;
            let end = block.location.end;
            Ok(Node::new(NodeKind::Eval { block: Box::new(block) }, SourceLocation { start, end }))
        } else {
            // eval "string" or eval $expr
            let expr = self.parse_expression()?;
            let end = expr.location.end;
            Ok(Node::new(NodeKind::Eval { block: Box::new(expr) }, SourceLocation { start, end }))
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
                let var_name = if self.peek_kind() == Some(TokenKind::ScalarSigil)
                    || self.tokens.peek()?.text.starts_with('$')
                {
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

        let end = finally_block
            .as_ref()
            .map(|b| b.location.end)
            .or_else(|| catch_blocks.last().map(|(_, b)| b.location.end))
            .unwrap_or(body.location.end);

        Ok(Node::new(
            NodeKind::Try {
                body: Box::new(body),
                catch_blocks: catch_blocks.into_iter().map(|(v, b)| (v, Box::new(b))).collect(),
                finally_block,
            },
            SourceLocation { start, end },
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
            Ok(Node::new(NodeKind::Do { block: Box::new(block) }, SourceLocation { start, end }))
        } else {
            // do "filename" or do $expr
            let expr = self.parse_expression()?;
            let end = expr.location.end;
            Ok(Node::new(NodeKind::Do { block: Box::new(expr) }, SourceLocation { start, end }))
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
            NodeKind::Given { expr: Box::new(expr), body: Box::new(body) },
            SourceLocation { start, end },
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
                        self.current_position(),
                    ));
                }
            }
        }

        self.expect(TokenKind::RightBrace)?;
        let end = self.previous_position();

        Ok(Node::new(NodeKind::Block { statements }, SourceLocation { start, end }))
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
            NodeKind::When { condition: Box::new(condition), body: Box::new(body) },
            SourceLocation { start, end },
        ))
    }

    /// Parse default statement
    fn parse_default_statement(&mut self) -> ParseResult<Node> {
        let start = self.consume_token()?.start; // consume 'default'

        // Parse the body block
        let body = self.parse_block()?;
        let end = body.location.end;

        Ok(Node::new(NodeKind::Default { body: Box::new(body) }, SourceLocation { start, end }))
    }

}
