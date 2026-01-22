impl<'a> Parser<'a> {
    /// Parse a complete program
    fn parse_program(&mut self) -> ParseResult<Node> {
        let start = self.current_position();
        let mut statements = Vec::new();

        while !self.tokens.is_eof() {
            // Check for UnknownRest token (lexer budget exceeded)
            if matches!(self.peek_kind(), Some(TokenKind::UnknownRest)) {
                let t = self.consume_token()?;
                statements.push(Node::new(
                    NodeKind::UnknownRest,
                    SourceLocation { start: t.start, end: t.end },
                ));
                break; // Stop parsing but preserve earlier nodes
            }

            statements.push(self.parse_statement()?);
        }

        let end = self.previous_position();
        Ok(Node::new(NodeKind::Program { statements }, SourceLocation { start, end }))
    }

    /// Parse a single statement
    fn parse_statement(&mut self) -> ParseResult<Node> {
        self.check_recursion()?;
        let result = self.parse_statement_inner();
        self.exit_recursion();
        result
    }

    fn parse_statement_inner(&mut self) -> ParseResult<Node> {
        // Every new statement begins here
        self.at_stmt_start = true;

        let kind = self.tokens.peek()?.kind;

        // Don't check for labels here - it breaks regular identifier parsing
        // Labels will be handled differently

        let mut stmt = match kind {
            // Empty statement (lone semicolon) - just consume and return a no-op
            TokenKind::Semicolon => {
                let pos = self.current_position();
                self.consume_token()?;
                // Return an empty block as a no-op placeholder
                return Ok(Node::new(
                    NodeKind::Block { statements: vec![] },
                    SourceLocation { start: pos, end: pos },
                ));
            }

            // Variable declarations
            TokenKind::My | TokenKind::Our | TokenKind::State => self.parse_variable_declaration(),
            TokenKind::Local => self.parse_local_statement(),

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
            TokenKind::Sub => {
                let sub_node = self.parse_subroutine()?;
                // Check if this is an anonymous subroutine
                Ok(if let NodeKind::Subroutine { name, .. } = &sub_node.kind {
                    if name.is_none() {
                        // Wrap anonymous subroutines in expression statements
                        let location = sub_node.location;
                        Node::new(
                            NodeKind::ExpressionStatement { expression: Box::new(sub_node) },
                            location,
                        )
                    } else {
                        // Named subroutines are statements by themselves
                        sub_node
                    }
                } else {
                    // Shouldn't happen, but return as-is
                    sub_node
                })
            }
            TokenKind::Class => self.parse_class(),
            TokenKind::Method => self.parse_method(),

            // Package management
            TokenKind::Package => self.parse_package(),
            TokenKind::Use => self.parse_use(),
            TokenKind::No => self.parse_no(),

            // Format declarations
            TokenKind::Format => self.parse_format(),

            // Phase blocks
            TokenKind::Begin
            | TokenKind::End
            | TokenKind::Check
            | TokenKind::Init
            | TokenKind::Unitcheck => self.parse_phase_block(),

            // Data sections
            TokenKind::DataMarker => self.parse_data_section(),

            // Return statement
            TokenKind::Return => self.parse_return(),

            // Block
            TokenKind::LeftBrace => self.parse_block(),

            // Expression-ish statement
            _ => {
                // Check if this might be a labeled statement
                if self.is_label_start() {
                    return self.parse_labeled_statement();
                }

                // Either build via indirect-object path or the normal expression path
                if let TokenKind::Identifier = kind {
                    // We need the text for the indirect call check
                    // We must clone it because is_indirect_call_pattern takes &mut self
                    let text = self.tokens.peek()?.text.clone();
                    if self.is_indirect_call_pattern(&text) {
                        // Parse indirect call but DON'T return early - let it go through
                        // the same modifier/semicolon handling as other statements
                        self.parse_indirect_call()
                    } else {
                        self.parse_expression_statement()
                    }
                } else {
                    self.parse_expression_statement()
                }
            }
        }?;

        // Check for statement modifiers on ANY statement
        if matches!(self.peek_kind(), Some(k) if Self::is_stmt_modifier_kind(k)) {
            stmt = self.parse_statement_modifier(stmt)?;
        }

        // Check for optional semicolon
        // Don't use peek_fresh_kind() here as it can cause issues with nested blocks
        if self.peek_kind() == Some(TokenKind::Semicolon) {
            let semi_token = self.consume_token()?;
            // Track cursor after semicolon for heredoc content collection
            self.byte_cursor = semi_token.end;
        }

        // Drain pending heredocs after statement completion (Sprint A Day 5 - with AST attachment)
        self.drain_pending_heredocs(&mut stmt);

        Ok(stmt)
    }

    /// Mark that we're no longer at statement start (called after consuming statement head)
    fn mark_not_stmt_start(&mut self) {
        self.at_stmt_start = false;
    }

    /// Check if current token is a statement modifier keyword
    fn is_statement_modifier_keyword(&mut self) -> bool {
        matches!(self.peek_kind(), Some(k) if Self::is_stmt_modifier_kind(k))
    }

    /// Parse expression statement
    fn parse_expression_statement(&mut self) -> ParseResult<Node> {
        let start = self.current_position();

        // Check for special blocks like AUTOLOAD and DESTROY
        if let Ok(token) = self.tokens.peek() {
            if matches!(token.text.as_ref(), "AUTOLOAD" | "DESTROY" | "CLONE" | "CLONE_SKIP") {
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

        // Statement modifiers are handled at the statement level in parse_statement()

        let end = self.previous_position();

        // Wrap the expression in an ExpressionStatement node
        Ok(Node::new(
            NodeKind::ExpressionStatement { expression: Box::new(expr) },
            SourceLocation { start, end },
        ))
    }

    /// Parse simple statement (print, die, next, last, etc. with their arguments)
    fn parse_simple_statement(&mut self) -> ParseResult<Node> {
        // Check if it's a builtin that can take arguments without parens
        if let Ok(token) = self.tokens.peek() {
            match token.text.as_ref() {
                "print" | "say" | "die" | "warn" | "return" | "next" | "last" | "redo" | "open"
                | "tie" | "printf" | "close" | "pipe" | "sysopen" | "sysread" | "syswrite"
                | "truncate" | "fcntl" | "ioctl" | "flock" | "seek" | "tell" | "select"
                | "binmode" | "exec" | "system" | "bless" | "ref" | "defined" | "undef"
                | "keys" | "values" | "each" | "delete" | "exists" | "push" | "pop" | "shift"
                | "unshift" | "sort" | "map" | "grep" | "chomp" | "chop" | "split" | "join" => {
                    let start = token.start;
                    // We need to clone the text to check for indirect call pattern because
                    // is_indirect_call_pattern borrows self mutably to peek ahead
                    let text = token.text.clone();

                    // Check for indirect object syntax before consuming the token
                    if self.is_indirect_call_pattern(&text) {
                        return self.parse_indirect_call();
                    }

                    // Consume the function name token
                    let token = self.consume_token()?;
                    let func_name = token.text;

                    // We're consuming the function name, no longer at statement start
                    self.mark_not_stmt_start();

                    // Check if there are arguments (not followed by semicolon or modifier)
                    match self.peek_kind() {
                        Some(TokenKind::Semicolon)
                        | Some(TokenKind::If)
                        | Some(TokenKind::Unless)
                        | Some(TokenKind::While)
                        | Some(TokenKind::Until)
                        | Some(TokenKind::For)
                        | Some(TokenKind::Foreach)
                        | Some(TokenKind::RightBrace)
                        | Some(TokenKind::Eof)
                        | None => {
                            // No arguments - return as function call with empty args
                            let end = self.previous_position();
                            Ok(Node::new(
                                NodeKind::FunctionCall { name: func_name.to_string(), args: vec![] },
                                SourceLocation { start, end },
                            ))
                        }
                        _ => {
                            // Has arguments - parse them as a comma-separated list
                            let mut args = vec![];

                            // Parse first argument
                            // Special handling for open/pipe/socket/tie which can take my $var as first arg
                            if (&*func_name == "open"
                                || &*func_name == "pipe"
                                || &*func_name == "socket"
                                || &*func_name == "tie")
                                && self.peek_kind() == Some(TokenKind::My)
                            {
                                args.push(self.parse_variable_declaration()?);
                            } else if matches!(func_name.as_ref(), "map" | "grep" | "sort")
                                && self.peek_kind() == Some(TokenKind::LeftBrace)
                            {
                                // Special handling for map/grep/sort with block first argument
                                args.push(self.parse_builtin_block()?);
                            } else {
                                // For builtins, don't parse word operators as part of arguments
                                // Word operators should be handled at statement level
                                args.push(self.parse_assignment()?);
                            }

                            // Parse remaining arguments
                            // For map/grep/sort, parse list arguments without requiring commas
                            if matches!(func_name.as_ref(), "map" | "grep" | "sort") {
                                // Parse list arguments until statement boundary
                                while !Self::is_statement_terminator(self.peek_kind())
                                    && !self.is_statement_modifier_keyword()
                                {
                                    // Skip optional comma
                                    if self.peek_kind() == Some(TokenKind::Comma) {
                                        self.consume_token()?;
                                    }
                                    args.push(self.parse_assignment()?);
                                }
                            } else {
                                // For other functions, require commas between arguments
                                while self.peek_kind() == Some(TokenKind::Comma) {
                                    self.consume_token()?; // consume comma

                                    // Check if we hit a statement modifier
                                    match self.peek_kind() {
                                        Some(TokenKind::If)
                                        | Some(TokenKind::Unless)
                                        | Some(TokenKind::While)
                                        | Some(TokenKind::Until)
                                        | Some(TokenKind::For)
                                        | Some(TokenKind::Foreach) => break,
                                        _ => args.push(self.parse_assignment()?),
                                    }
                                }
                            }

                            let end = args.last().map(|a| a.location.end).unwrap_or(start);

                            Ok(Node::new(
                                NodeKind::FunctionCall { name: func_name.to_string(), args },
                                SourceLocation { start, end },
                            ))
                        }
                    }
                }
                "new" => {
                    // Check for indirect constructor syntax
                    let _start = token.start;
                    // Clone to satisfy borrow checker
                    let text = token.text.clone();

                    if self.is_indirect_call_pattern(&text) {
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
        let modifier = modifier_token.text.to_string();

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
            SourceLocation { start, end },
        ))
    }

    /// Parse a block statement
    fn parse_block(&mut self) -> ParseResult<Node> {
        self.check_recursion()?;
        let start = self.current_position();
        self.expect(TokenKind::LeftBrace)?;

        let mut statements = Vec::new();

        while self.peek_kind() != Some(TokenKind::RightBrace) && !self.tokens.is_eof() {
            let stmt = self.parse_statement()?;
            // Don't add empty blocks (from lone semicolons) to the statement list
            if !matches!(stmt.kind, NodeKind::Block { ref statements } if statements.is_empty()) {
                statements.push(stmt);
            }

            // parse_statement already invalidates peek, so we don't need to do it again

            // Swallow any stray semicolons before checking for the next statement or closing brace
            while self.peek_kind() == Some(TokenKind::Semicolon) {
                self.consume_token()?;
                self.tokens.invalidate_peek();
            }
        }

        self.expect(TokenKind::RightBrace)?;
        let end = self.previous_position();

        let result = Node::new(NodeKind::Block { statements }, SourceLocation { start, end });
        self.exit_recursion();
        Ok(result)
    }

    /// Check if we're at the start of a labeled statement (LABEL: ...)
    fn is_label_start(&mut self) -> bool {
        // We need an identifier followed by a colon
        if self.peek_kind() != Some(TokenKind::Identifier) {
            return false;
        }

        // Check if the second token is a colon
        if let Ok(second_token) = self.tokens.peek_second() {
            if second_token.kind == TokenKind::Colon {
                // To avoid conflict with qualified identifiers (Package::name),
                // we need to be more careful. A true label should be:
                // IDENTIFIER : STATEMENT
                // where STATEMENT doesn't start with another colon.
                //
                // For now, let's be conservative and disable label detection
                // when we see patterns that could be qualified identifiers.
                // This is a simple heuristic: if we see IDENTIFIER : and the
                // identifier looks like it could be a package name (starts with
                // uppercase), we'll let the expression parser handle it.
                if let Ok(first_token) = self.tokens.peek() {
                    let name = &first_token.text;
                    // If identifier starts with uppercase, it might be a package name
                    // so avoid treating it as a label
                    if name.chars().next().is_some_and(|c| c.is_uppercase()) {
                        return false;
                    }
                }
                return true;
            }
        }
        false
    }

    /// Parse a labeled statement (LABEL: statement)
    fn parse_labeled_statement(&mut self) -> ParseResult<Node> {
        let start = self.current_position();

        // Parse the label
        let label_token = self.expect(TokenKind::Identifier)?;
        let label = label_token.text.to_string();

        // Consume the colon
        self.expect(TokenKind::Colon)?;

        // Parse the statement after the label
        let statement = Box::new(self.parse_statement()?);

        let end = self.previous_position();
        Ok(Node::new(
            NodeKind::LabeledStatement { label, statement },
            SourceLocation { start, end },
        ))
    }

}
