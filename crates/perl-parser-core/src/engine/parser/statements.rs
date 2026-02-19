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

            // Parse statement with error recovery
            let stmt_result = self.parse_statement();
            match stmt_result {
                Ok(stmt) => statements.push(stmt),
                Err(e) => {
                    // Don't recover from recursion/nesting limits - propagate immediately
                    if matches!(e, ParseError::RecursionLimit | ParseError::NestingTooDeep { .. }) {
                        return Err(e);
                    }

                    // Record the actual error
                    self.errors.push(e.clone());

                    // Create error node for failed statement
                    let error_location = self.current_position();
                    let error_msg = format!("{}", e);
                    // Collect peek_kind before mutable borrow in recover_from_error
                    let peek_kind = format!("{:?}", self.peek_kind());
                    let error_node = self.recover_from_error(
                        error_msg,
                        "statement".to_string(),
                        peek_kind,
                        error_location
                    );
                    statements.push(error_node);

                    // Try to synchronize to next statement
                    if !self.synchronize() {
                        // If synchronization fails, we're likely at EOF
                        break;
                    }
                }
            }
        }

        let end = self.previous_position();
        Ok(Node::new(NodeKind::Program { statements }, SourceLocation { start, end }))
    }

    /// Parse a single statement
    fn parse_statement(&mut self) -> ParseResult<Node> {
        self.with_recursion_guard(|s| s.parse_statement_inner())
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

            // Loop control
            TokenKind::Next | TokenKind::Last | TokenKind::Redo => self.parse_loop_control(),

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
                    // We must clone it because is_indirect_call_pattern borrows self mutably to peek ahead
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

        // Drain pending heredocs after statement completion (attach content to AST)
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
                | "printf" | "close" | "pipe" | "sysopen" | "sysread" | "syswrite"
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
                            // Special handling for open/pipe/socket which can take my $var as first arg
                            let mut parsed_block_arg = false;
                            if (func_name.as_ref() == "open"
                                || func_name.as_ref() == "pipe"
                                || func_name.as_ref() == "socket")
                                && (self.peek_kind() == Some(TokenKind::My) 
                                    || self.peek_kind() == Some(TokenKind::Our)
                                    || self.peek_kind() == Some(TokenKind::Local)
                                    || self.peek_kind() == Some(TokenKind::State))
                            {
                                args.push(self.parse_variable_declaration()?);
                            } else if matches!(func_name.as_ref(), "map" | "grep" | "sort")
                                && self.peek_kind() == Some(TokenKind::LeftBrace)
                            {
                                // Special handling for map/grep/sort with block first argument
                                args.push(self.parse_builtin_block()?);
                                parsed_block_arg = true;
                            } else {
                                // For builtins, use parse_assignment to avoid consuming comma operators
                                args.push(self.parse_assignment()?);
                            }

                            // Handle map/grep/sort { block } LIST case where no comma separates block and list
                            if parsed_block_arg 
                                && self.peek_kind() != Some(TokenKind::Comma) 
                                && !self.is_at_statement_end() 
                            {
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

                            let end = self.previous_position();
                            Ok(Node::new(
                                NodeKind::FunctionCall { name: func_name.to_string(), args },
                                SourceLocation { start, end },
                            ))
                        }
                    }
                }
                "tie" => {
                    let start = token.start;
                    self.consume_token()?; // consume tie
                    self.mark_not_stmt_start();

                    // First argument to tie can be a variable declaration, e.g. tie my %hash, ...
                    let variable = if matches!(self.peek_kind(), Some(TokenKind::My | TokenKind::Our | TokenKind::Local | TokenKind::State)) {
                        Box::new(self.parse_variable_declaration()?)
                    } else {
                        Box::new(self.parse_assignment()?)
                    };

                    self.expect(TokenKind::Comma)?;
                    let package = Box::new(self.parse_assignment()?);

                    let mut args = vec![];
                    while self.peek_kind() == Some(TokenKind::Comma) {
                        self.consume_token()?; // consume ,
                        args.push(self.parse_assignment()?);
                    }

                    let end = self.previous_position();
                    Ok(Node::new(
                        NodeKind::Tie { variable, package, args },
                        SourceLocation { start, end },
                    ))
                }
                "untie" => {
                    let start = token.start;
                    self.consume_token()?; // consume untie
                    self.mark_not_stmt_start();

                    let variable = Box::new(self.parse_assignment()?);

                    let end = self.previous_position();
                    Ok(Node::new(
                        NodeKind::Untie { variable },
                        SourceLocation { start, end },
                    ))
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
        self.with_recursion_guard(|s| {
            let start = s.current_position();

            s.expect(TokenKind::LeftBrace)?;

            let mut statements = Vec::new();

            while s.peek_kind() != Some(TokenKind::RightBrace) && !s.tokens.is_eof() {
                // Parse statement with error recovery (AC3: Panic Mode Recovery inside blocks)
                let stmt_result = s.parse_statement();
                match stmt_result {
                    Ok(stmt) => {
                        // Don't add empty blocks (from lone semicolons) to the statement list
                        if !matches!(stmt.kind, NodeKind::Block { ref statements } if statements.is_empty()) {
                            statements.push(stmt);
                        }
                    }
                    Err(e) => {
                        // Don't recover from recursion/nesting limits - propagate immediately
                        if matches!(e, ParseError::RecursionLimit | ParseError::NestingTooDeep { .. }) {
                            return Err(e);
                        }

                        // Record the actual error
                        s.errors.push(e.clone());

                        // Create error node for failed statement
                        let error_location = s.current_position();
                        let error_msg = format!("{}", e);
                        // Collect peek_kind before mutable borrow in recover_from_error
                        let peek_kind = format!("{:?}", s.peek_kind());
                        let error_node = s.recover_from_error(
                            error_msg,
                            "statement".to_string(),
                            peek_kind,
                            error_location
                        );
                        statements.push(error_node);

                        // Try to synchronize to next statement
                        if !s.synchronize() {
                            // If synchronization fails, we check if we're at block end or EOF
                            if s.peek_kind() == Some(TokenKind::RightBrace) || s.tokens.is_eof() {
                                break;
                            }
                            // Otherwise stop to prevent infinite loop
                            break; 
                        }
                    }
                }

                // parse_statement already invalidates peek, so we don't need to do it again

                // Swallow any stray semicolons before checking for the next statement or closing brace
                while s.peek_kind() == Some(TokenKind::Semicolon) {
                    s.consume_token()?;
                    s.tokens.invalidate_peek();
                }
            }

            s.expect(TokenKind::RightBrace)?;
            let end = s.previous_position();

            Ok(Node::new(NodeKind::Block { statements }, SourceLocation { start, end }))
        })
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
                // Qualified identifiers use `::` which tokenizes as
                // DoubleColon, so `Identifier Colon` (single colon) is
                // unambiguously a label — even for uppercase names like
                // OUTER:, LOOP:, LINE: which are idiomatic Perl labels.
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

    /// Parse loop control statement (next, last, redo)
    fn parse_loop_control(&mut self) -> ParseResult<Node> {
        let start = self.current_position();
        let op_token = self.consume_token()?;
        let op = op_token.text.to_string();

        self.mark_not_stmt_start();

        // Check for optional label
        let label = if let Some(TokenKind::Identifier) = self.peek_kind() {
            let label_token = self.consume_token()?;
            Some(label_token.text.to_string())
        } else {
            None
        };

        let end = self.previous_position();
        Ok(Node::new(
            NodeKind::LoopControl { op, label },
            SourceLocation { start, end },
        ))
    }

}
