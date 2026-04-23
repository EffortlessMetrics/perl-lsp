impl<'a> Parser<'a> {
    /// Parse a complete program
    fn parse_program(&mut self) -> ParseResult<Node> {
        let start = self.current_position();
        let mut statements = Vec::new();

        while !self.tokens.is_eof() {
            self.check_cancelled()?;

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
                    // Don't recover from these — propagate immediately
                    if matches!(
                        e,
                        ParseError::RecursionLimit
                            | ParseError::NestingTooDeep { .. }
                            | ParseError::Cancelled
                    ) {
                        return Err(e);
                    }

                    // Record the actual error
                    self.errors.push(e.clone());

                    // Create error node for failed statement
                    let error_location = self.current_position();
                    let error_msg = format!("{}", e);
                    // Collect peek_kind before mutable borrow in recover_from_error
                    let peek_display = self.peek_kind()
                        .map(|k| k.display_name())
                        .unwrap_or("end of input");
                    let error_node = self.recover_from_error(
                        error_msg,
                        "statement".to_string(),
                        peek_display.to_string(),
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

    /// Check if the current token is a keyword that is being used as an
    /// autoquoted hash key before a fat arrow (`=>`).
    ///
    /// In Perl, any bareword before `=>` is treated as a string:
    /// ```perl
    /// my %h = (if => 1, for => 2, return => 3);
    /// ```
    fn is_keyword_before_fat_arrow(&mut self) -> bool {
        self.tokens
            .peek_second()
            .ok()
            .map(|t| t.kind == TokenKind::FatArrow)
            .unwrap_or(false)
    }

    fn is_async_sub_start(&mut self) -> bool {
        self.peek_kind() == Some(TokenKind::Identifier)
            && self.tokens.peek().ok().is_some_and(|t| t.text.as_ref() == "async")
            && self
                .tokens
                .peek_second()
                .ok()
                .is_some_and(|t| t.kind == TokenKind::Sub)
    }

    fn is_adjust_block_start(&mut self) -> bool {
        self.in_class_body > 0
            && self.peek_kind() == Some(TokenKind::Identifier)
            && self.tokens.peek().ok().is_some_and(|t| t.text.as_ref() == "ADJUST")
            && self
                .tokens
                .peek_second()
                .ok()
                .is_some_and(|t| t.kind == TokenKind::LeftBrace)
    }

    fn finish_subroutine_statement(&mut self, sub_node: Node) -> ParseResult<Node> {
        Ok(if let NodeKind::Subroutine { name, .. } = &sub_node.kind {
            if name.is_none() {
                // Anonymous sub may be followed by arrow: sub { 42 }->()
                let expr = if self.peek_kind() == Some(TokenKind::Arrow) {
                    self.parse_postfix_chain(sub_node)?
                } else {
                    sub_node
                };
                // Wrap anonymous subroutines in expression statements
                let location = expr.location;
                Node::new(
                    NodeKind::ExpressionStatement { expression: Box::new(expr) },
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

    fn parse_statement_inner(&mut self) -> ParseResult<Node> {
        // Every new statement begins here
        self.at_stmt_start = true;

        // A `/` at statement start is always a regex delimiter, never division.
        // The lexer may be in ExpectOperator mode after a preceding block's `}`,
        // causing it to emit Division (Slash) instead of RegexMatch.  Roll back
        // and re-lex in ExpectTerm mode to get the correct token.
        if self.tokens.peek()?.kind == TokenKind::Slash {
            self.tokens.relex_as_term();
        }

        let kind = self.tokens.peek()?.kind;

        // Don't check for labels here - it breaks regular identifier parsing
        // Labels will be handled differently

        // In Perl, any bareword (including reserved keywords) before `=>` is
        // autoquoted as a string.  Detect this pattern early so that keyword
        // tokens such as `if`, `for`, `return`, `my`, etc. are NOT dispatched
        // to their keyword-specific parsers when they appear as hash keys.
        if Self::is_keyword_token(kind) && self.is_keyword_before_fat_arrow() {
            let token = self.consume_token()?;
            self.mark_not_stmt_start();
            // Produce a String node (autoquoting) and continue as an expression statement
            let key_node = Node::new(
                NodeKind::String { value: token.text.to_string(), interpolated: false },
                SourceLocation { start: token.start, end: token.end },
            );
            // Now parse the rest of the expression (=> value, more pairs, etc.)
            // Re-enter the comma parser with the key already consumed
            let mut stmt = self.finish_expression_from(key_node)?;
            // Check for statement modifiers on ANY statement
            if matches!(self.peek_kind(), Some(k) if Self::is_stmt_modifier_kind(k)) {
                stmt = self.parse_statement_modifier(stmt)?;
            }
            // Check for optional semicolon
            if self.peek_kind() == Some(TokenKind::Semicolon) {
                let semi_token = self.consume_token()?;
                self.byte_cursor = semi_token.end;
            }
            self.drain_pending_heredocs(&mut stmt);
            return Ok(stmt);
        }

        if kind == TokenKind::Identifier {
            let keyword_text = self.tokens.peek()?.text.clone();
            let next_kind = self.tokens.peek_second().ok().map(|t| t.kind);

            if keyword_text.as_ref() == "else" && next_kind == Some(TokenKind::LeftBrace) {
                return self.parse_orphaned_else();
            }

            if keyword_text.as_ref() == "elsif" && next_kind == Some(TokenKind::LeftParen) {
                return self.parse_orphaned_elsif();
            }

            if self.is_adjust_block_start() {
                return self.parse_adjust_block();
            }
        }

        let mut stmt = if self.is_async_sub_start() {
            let async_token = self.consume_token()?;
            let mut sub_node = self.parse_subroutine()?;
            sub_node.location.start = async_token.start;
            if let NodeKind::Subroutine { attributes, .. } = &mut sub_node.kind
                && !attributes.iter().any(|attr| attr == "async")
            {
                attributes.insert(0, "async".to_string());
            }
            self.finish_subroutine_statement(sub_node)
        } else {
            match kind {
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

            // Variable declarations (`my $x`, `our @y`, ...) and scoped sub declarations
            // (`my sub helper { ... }`, `our sub helper { ... }`, `state sub memo { ... }`).
            TokenKind::My | TokenKind::Our | TokenKind::State => {
                if matches!(self.tokens.peek_second().map(|t| t.kind), Ok(TokenKind::Sub)) {
                    let decl_token = self.consume_token()?;
                    let mut sub_node = self.parse_subroutine()?;
                    sub_node.location.start = decl_token.start;
                    self.finish_subroutine_statement(sub_node)
                } else {
                    let decl = self.parse_variable_declaration()?;
                    if self.peek_kind() == Some(TokenKind::FatArrow) {
                        self.finish_expression_from(decl)
                    } else {
                        Ok(self.parse_word_or_expr(decl)?)
                    }
                }
            }
            // `field` is a variable declarator only in Perl 5.38+ class bodies.
            // In legacy code it is commonly a regular identifier (function call,
            // hash key, etc.).  We disambiguate by peeking at the next token:
            // if it starts a variable directly (sigil or sigil-prefixed
            // identifier), treat it as a declaration; otherwise fall through
            // to expression parsing.
            TokenKind::Field if self.is_field_declaration_context() => {
                let decl = self.parse_variable_declaration()?;
                if self.peek_kind() == Some(TokenKind::FatArrow) {
                    let variable = match decl.kind {
                        NodeKind::VariableDeclaration { variable, .. } => *variable,
                        _ => decl,
                    };
                    let call_start = variable.location.start;
                    let mut args = vec![variable];

                    while matches!(self.peek_kind(), Some(TokenKind::Comma) | Some(TokenKind::FatArrow)) {
                        self.consume_token()?;

                        if self.peek_kind() == Some(TokenKind::FatArrow) {
                            self.consume_token()?;
                        }

                        if self.is_at_statement_end() {
                            break;
                        }

                        args.push(self.parse_assignment_or_declaration()?);
                    }

                    let end = args.last().map(|arg| arg.location.end).unwrap_or(call_start);
                    let call = Node::new(
                        NodeKind::FunctionCall { name: "field".to_string(), args },
                        SourceLocation { start: call_start, end },
                    );
                    Ok(self.parse_word_or_expr(call)?)
                } else {
                    Ok(self.parse_word_or_expr(decl)?)
                }
            }
            TokenKind::Local => self.parse_local_statement(),

            // Control flow
            TokenKind::If => self.parse_if_statement(),
            TokenKind::Unless => self.parse_unless_statement(),

            // Orphaned else/elsif — these appear at statement level when the
            // preceding if/unless block failed to parse or was consumed by
            // error recovery. Instead of crashing into expression parsing,
            // consume the else/elsif clause gracefully and wrap it in an
            // error-recovery If node so the rest of the file can keep parsing.
            TokenKind::Else => self.parse_orphaned_else(),
            TokenKind::Elsif => self.parse_orphaned_elsif(),
            TokenKind::While => self.parse_while_statement(),
            TokenKind::Until => self.parse_until_statement(),
            TokenKind::For => self.parse_for_statement(),
            TokenKind::Foreach => self.parse_foreach_statement(),
            TokenKind::Given => self.parse_given_statement(),
            TokenKind::Default => self.parse_default_statement(),
            TokenKind::Try => self.parse_try(),
                TokenKind::Defer => self.parse_defer(),

            // Loop control — next/last/redo can be followed by a word operator at statement level,
            // e.g. `last and die` means `(last) and (die)`.
            TokenKind::Next | TokenKind::Last | TokenKind::Redo => {
                let ctrl = self.parse_loop_control()?;
                Ok(self.parse_word_or_expr(ctrl)?)
            }

            // Subroutines and modern OOP
            TokenKind::Sub => {
                let sub_node = self.parse_subroutine()?;
                self.finish_subroutine_statement(sub_node)
            }
            TokenKind::Class => self.parse_class(),
            // `method NAME SIGNATURE BLOCK` is a Perl 5.38+ declaration.
            // Legacy code uses `method` as a function name; disambiguate by
            // checking the next token is an Identifier (the method name).
            TokenKind::Method
                if matches!(
                    self.tokens.peek_second().map(|t| t.kind),
                    Ok(TokenKind::Identifier)
                ) =>
            {
                self.parse_method()
            }

            // Package management
            TokenKind::Package => self.parse_package(),
            TokenKind::Use => self.parse_use(),
            TokenKind::No => self.parse_no(),

            // Format declarations
            TokenKind::Format => self.parse_format(),

            // Phase blocks — but first check for label syntax (CHECK: ..., BEGIN: ..., etc.)
            // In Perl, phase-block keywords are valid statement labels when followed by `:`.
            // e.g. `CHECK: for (my $i = 0; ...)` uses CHECK as a loop label, not a phase block.
            TokenKind::Begin
            | TokenKind::End
            | TokenKind::Check
            | TokenKind::Init
            | TokenKind::Unitcheck
                if self
                    .tokens
                    .peek_second()
                    .ok()
                    .map(|t| t.kind == TokenKind::Colon)
                    .unwrap_or(false) =>
            {
                self.parse_keyword_as_label()
            }
            TokenKind::Begin
            | TokenKind::End
            | TokenKind::Check
            | TokenKind::Init
            | TokenKind::Unitcheck => self.parse_phase_block(),

            // Data sections
            TokenKind::DataMarker => self.parse_data_section(),

            // Return statement — may be followed by a word operator at statement level,
            // e.g. `return or die` means `(return) or (die)`.
            TokenKind::Return => {
                let ret = self.parse_return()?;
                Ok(self.parse_word_or_expr(ret)?)
            }

            // Goto statement
            TokenKind::Goto => self.parse_goto(),

            // Block — or hashref/block constructor followed by arrow dereference
            // e.g. {key => "value"}->{key}
            TokenKind::LeftBrace => {
                let block = self.parse_block()?;
                if self.peek_kind() == Some(TokenKind::Arrow) {
                    // The block is actually an expression (hash constructor)
                    // followed by postfix arrow operators.
                    let chained = self.parse_postfix_chain(block)?;
                    let loc = chained.location;
                    Ok(Node::new(
                        NodeKind::ExpressionStatement { expression: Box::new(chained) },
                        loc,
                    ))
                } else {
                    Ok(block)
                }
            }

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
                        // the same modifier/semicolon handling as other statements.
                        // Word operators (or, and, xor) may follow: `print $fh "msg" or die`.
                        let call = self.parse_indirect_call()?;
                        Ok(self.parse_word_or_expr(call)?)
                    } else {
                        self.parse_expression_statement()
                    }
                } else {
                    self.parse_expression_statement()
                }
            }
            }
        }?;

        // Check for statement modifiers — only on non-compound statements.
        // Compound statements (if/while/for/foreach/given/default/try/sub/package)
        // cannot take postfix modifiers; the keyword that follows is a new statement.
        if !Self::is_compound_statement(&stmt)
            && matches!(self.peek_kind(), Some(k) if Self::is_stmt_modifier_kind(k))
        {
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

    /// Returns true if the node is a compound statement that cannot take a postfix modifier.
    /// In Perl, compound statements (if/while/for/foreach/given/default/try/sub/package)
    /// are terminated by their own closing brace — a modifier keyword that follows is a
    /// new top-level statement, not a modifier on the compound statement.
    ///
    /// A bare `Block` is also compound: `{ ... } for @arr` is a syntax error in Perl
    /// (verified: `perl -c` rejects it). Without this, `{ ... }\nfor my $x (...) { }`
    /// would be misread as `{ ... } for my` (postfix-for with `my` as the iterator
    /// expression), causing the `for my $x (LIST) { BLOCK }` form to fail.
    fn is_compound_statement(node: &Node) -> bool {
        matches!(
            node.kind,
            NodeKind::If { .. }
                | NodeKind::While { .. }
                | NodeKind::For { .. }
                | NodeKind::Foreach { .. }
                | NodeKind::Given { .. }
                | NodeKind::Default { .. }
                | NodeKind::Try { .. }
                | NodeKind::Defer { .. }
                | NodeKind::Subroutine { .. }
                | NodeKind::Package { .. }
                | NodeKind::Block { .. }
                | NodeKind::PhaseBlock { .. }
        )
    }

    /// Parse expression statement
    /// Resume comma-level expression parsing from an already-consumed first
    /// token (used when a keyword has been autoquoted before `=>`).
    ///
    /// Produces an `ExpressionStatement` wrapping the resulting list / hash
    /// expression.
    fn finish_expression_from(&mut self, first: Node) -> ParseResult<Node> {
        let start = first.location.start;
        let mut expr = self.collect_comma_fat_arrow_continuation(first)?;

        // Handle trailing word operators (or, and, xor)
        expr = self.parse_word_or_expr(expr)?;

        let end = self.previous_position();
        Ok(Node::new(
            NodeKind::ExpressionStatement { expression: Box::new(expr) },
            SourceLocation { start, end },
        ))
    }

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

    /// Continue parsing operators after a no-arg named-unary/nullary call.
    fn parse_call_statement_tail(&mut self, mut expr: Node) -> ParseResult<Node> {
        expr = self.parse_power_with(expr)?;
        expr = self.parse_multiplicative_with(expr)?;
        expr = self.parse_additive_with(expr)?;
        expr = self.parse_shift_with(expr)?;
        self.parse_named_unary_statement_tail(expr)
    }

    /// Continue parsing operators that bind less tightly than named-unary/list
    /// operator arguments after we have already constructed the call node.
    fn parse_named_unary_statement_tail(&mut self, mut expr: Node) -> ParseResult<Node> {
        expr = self.parse_relational_with(expr)?;
        expr = self.parse_equality_with(expr)?;
        expr = self.parse_range_with(expr)?;
        expr = self.parse_bitwise_and_with(expr)?;
        expr = self.parse_bitwise_xor_with(expr)?;
        expr = self.parse_bitwise_or_with(expr)?;
        expr = self.parse_and_with(expr)?;
        expr = self.parse_or_with(expr)?;
        expr = self.parse_ternary_with(expr)?;
        expr = self.collect_comma_fat_arrow_continuation(expr)?;
        self.parse_word_or_expr(expr)
    }

    fn parse_named_unary_statement_call(
        &mut self,
        start: usize,
        func_name: &str,
        allow_no_args: bool,
    ) -> ParseResult<Node> {
        let omit_optional_arg = allow_no_args
            && (self.peek_kind().is_some_and(Self::is_binary_operator)
                || self.peek_kind() == Some(TokenKind::Slash));

        // String comparison operators (ne, eq, lt, le, gt, ge) are tokenized as
        // Identifier tokens, so `is_binary_operator` won't catch them. When a
        // nullary builtin like `ref` is followed by one of these, don't consume
        // the operator as an argument -- let it become a binary operator instead.
        let next_is_str_cmp_op = self.peek_kind() == Some(TokenKind::Identifier)
            && self.tokens.peek().is_ok_and(|t| {
                matches!(t.text.as_ref(), "eq" | "ne" | "lt" | "le" | "gt" | "ge")
            });

        let args = if self.is_at_statement_end() || omit_optional_arg || next_is_str_cmp_op {
            vec![]
        } else {
            vec![self.parse_shift()?]
        };

        if args.is_empty() && !allow_no_args && !next_is_str_cmp_op {
            return Err(ParseError::unexpected(
                "expression".to_string(),
                format!("{:?}", self.peek_kind()),
                self.current_position(),
            ));
        }

        let had_args = !args.is_empty();
        let end = args
            .last()
            .map(|arg| arg.location.end)
            .unwrap_or_else(|| self.previous_position());
        let expr = Node::new(
            NodeKind::FunctionCall {
                name: func_name.to_string(),
                args,
            },
            SourceLocation { start, end },
        );

        if had_args {
            self.parse_named_unary_statement_tail(expr)
        } else {
            self.parse_call_statement_tail(expr)
        }
    }

    /// Parse simple statement (print, die, next, last, etc. with their arguments)
    fn parse_simple_statement(&mut self) -> ParseResult<Node> {
        // In Perl, any bareword before `=>` is autoquoted as a hash key.
        // When a builtin name (e.g. `log`, `abs`, `die`) appears before `=>`,
        // skip the builtin dispatch and fall through to expression parsing.
        // This handles patterns like `has log => sub { ... }`.
        if self.is_keyword_before_fat_arrow() {
            return self.parse_expression();
        }
        // Check if it's a builtin that can take arguments without parens
        if let Ok(token) = self.tokens.peek() {
            let token_text = token.text.clone();
            let token_start = token.start;

            match token_text.as_ref() {
                // Parenthesized nullary builtins are handled by the general
                // expression parser; this branch is for statement-start
                // bare calls like `shift @arr` or `caller 1 || die`.
                name if Self::is_nullary_builtin(name) => {
                    if self.tokens.peek_second().is_ok_and(|t| {
                        matches!(t.kind, TokenKind::LeftParen | TokenKind::Arrow)
                    }) {
                        self.parse_expression()
                    } else {
                        let token = self.consume_token()?;
                        self.mark_not_stmt_start();
                        self.parse_named_unary_statement_call(token_start, token.text.as_ref(), true)
                    }
                }
                // Special-cased builtins with dedicated AST nodes — must come
                // before the generic `is_builtin_function` guard below.
                "tie" => {
                    let start = token_start;
                    self.consume_token()?; // consume tie
                    self.mark_not_stmt_start();

                    // `tie(VARIABLE, CLASS, LIST)` is valid Perl syntax.
                    let has_parens = self.peek_kind() == Some(TokenKind::LeftParen);
                    if has_parens {
                        self.consume_token()?; // consume '('
                    }

                    // First argument to tie can be a variable declaration, e.g. tie my %hash, ...
                    let variable = if matches!(self.peek_kind(), Some(TokenKind::My | TokenKind::Our | TokenKind::Local | TokenKind::State)) {
                        Box::new(self.parse_variable_declaration()?)
                    } else {
                        Box::new(self.parse_assignment()?)
                    };

                    // Accept comma or fat arrow between variable and package
                    // (Perl treats `=>` as a synonym for `,`)
                    match self.peek_kind() {
                        Some(TokenKind::Comma) | Some(TokenKind::FatArrow) => {
                            self.consume_token()?;
                        }
                        _ => {
                            return Err(ParseError::unexpected(
                                "Comma".to_string(),
                                format!("{:?}", self.peek_kind()),
                                self.current_position(),
                            ));
                        }
                    }
                    let package = Box::new(self.parse_assignment()?);

                    let mut args = vec![];
                    while matches!(self.peek_kind(), Some(TokenKind::Comma) | Some(TokenKind::FatArrow)) {
                        self.consume_token()?; // consume , or =>
                        if has_parens && self.peek_kind() == Some(TokenKind::RightParen) {
                            break;
                        }
                        args.push(self.parse_assignment()?);
                    }

                    if has_parens {
                        self.expect(TokenKind::RightParen)?;
                    }

                    let end = self.previous_position();
                    Ok(Node::new(
                        NodeKind::Tie { variable, package, args },
                        SourceLocation { start, end },
                    ))
                }
                "untie" => {
                    let start = token_start;
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
                    let _start = token_start;
                    // Clone to satisfy borrow checker
                    let text = token.text.clone();

                    if self.is_indirect_call_pattern(&text) {
                        return self.parse_indirect_call();
                    }

                    // Otherwise parse as regular expression
                    self.parse_expression()
                }
                "goto" => {
                    // goto LABEL | goto &sub | goto $expr
                    self.parse_goto()
                }
                // Generic builtin functions that can take arguments without parens.
                // Uses the canonical builtin registry in `perl-builtins-phf`.
                name if Self::is_builtin_function(name) => {
                    let start = token_start;
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
                            // `defined` and `ref` at statement start without parens use
                            // parse_unary() for the single argument. This fixes
                            // the precedence issue: `ref $obj->{list} eq 'ARRAY'` must parse
                            // as `(eq (ref ...) 'ARRAY')` not `(ref (eq ...))`.
                            //
                            // Only these two are included because they specifically have the
                            // arrow-chain pattern (`defined $obj->{k}`, `ref $obj->{list}`)
                            // and should stop the indirect-call path from eating the `->`
                            // chain while still leaving surrounding comparisons outside the
                            // call node.
                            //
                            // When called WITH parens we fall through — parens already delimit.
                            if self.peek_kind() != Some(TokenKind::LeftParen)
                                && Self::is_optional_arg_builtin(func_name.as_ref())
                            {
                                return self.parse_named_unary_statement_call(
                                    start,
                                    func_name.as_ref(),
                                    true,
                                );
                            }

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
                            } else if Self::is_block_list_func(func_name.as_ref())
                                && self.peek_kind() == Some(TokenKind::LeftBrace)
                            {
                                // Special handling for map/grep/sort/first/any/all/etc.
                                // with block first argument
                                args.push(self.parse_builtin_block()?);
                                parsed_block_arg = true;
                            } else if matches!(func_name.as_ref(), "split" | "grep" | "map" | "sort")
                                && self.peek_kind() == Some(TokenKind::Slash)
                            {
                                // For `split /regex/, ...` and `grep /regex/, @list`,
                                // the `/` after these builtins is a regex delimiter, not
                                // division. Roll back the lexer to re-lex the `/` in
                                // ExpectTerm mode so it becomes a regex.
                                self.tokens.relex_as_term();
                                args.push(self.parse_assignment()?);
                            } else if self.peek_kind() == Some(TokenKind::LeftParen)
                                && (Self::is_block_list_func(func_name.as_ref())
                                    || matches!(
                                        func_name.as_ref(),
                                        "exec" | "system" | "print" | "say" | "printf" | "send"
                                    ))
                            {
                                // block-list and filehandle builtins followed by (...) use
                                // parse_args() so that `map({...} keys ...)` and
                                // `exec({...} @prog)` work: the block/hash inside the parens
                                // may be followed by the list without a separating comma.
                                // For print/say/printf, use the filehandle-aware variant so
                                // that `print( $fh EXPR )` works with no comma after $fh.
                                let paren_args = if matches!(
                                    func_name.as_ref(),
                                    "print" | "say" | "printf" | "send"
                                ) {
                                    self.parse_print_parens_args()?
                                } else {
                                    self.parse_args()?
                                };
                                args.extend(paren_args);
                            } else {
                                // For builtins, use parse_assignment_or_declaration to handle
                                // my/our/local/state declarations inside argument lists
                                args.push(self.parse_assignment_or_declaration()?);
                            }

                            // Handle map/grep/sort { block } LIST case where no comma separates block and list.
                            // Also skip an optional fat arrow (`=>`) which Perl treats as a comma synonym.
                            if parsed_block_arg && !self.is_at_statement_end() {
                                // Skip optional comma or fat arrow before the list
                                if matches!(self.peek_kind(), Some(TokenKind::Comma) | Some(TokenKind::FatArrow)) {
                                    self.consume_token()?;
                                }
                                if !self.is_at_statement_end() {
                                    args.push(self.parse_assignment()?);
                                }
                            }

                            // Parse remaining arguments
                            // For block-list builtins, parse list arguments without requiring
                            // commas. Use is_at_statement_end() so `map { ... } @arr` is
                            // accepted as the last statement before a block close even without
                            // a trailing semicolon.
                            //
                            // Word operators (or, and, xor, not) terminate the argument list
                            // because they bind less tightly than list operators.
                            // e.g., `sort @list or die` => (sort @list) or (die)
                            if Self::is_block_list_func(func_name.as_ref()) {
                                while !self.is_at_statement_end()
                                    && !matches!(
                                        self.peek_kind(),
                                        Some(
                                            TokenKind::WordOr
                                                | TokenKind::WordAnd
                                                | TokenKind::WordXor
                                                | TokenKind::WordNot
                                        )
                                    )
                                {
                                    // Skip optional comma or fat arrow
                                    if matches!(self.peek_kind(), Some(TokenKind::Comma) | Some(TokenKind::FatArrow)) {
                                        self.consume_token()?;
                                    }
                                    args.push(self.parse_assignment()?);
                                }
                            } else {
                                // For other functions, require commas (or fat arrows) between arguments
                                // Perl allows `push @array => $value` as well as `push @array, $value`
                                while matches!(self.peek_kind(), Some(TokenKind::Comma) | Some(TokenKind::FatArrow)) {
                                    self.consume_token()?; // consume comma or fat arrow

                                    // Handle `, =>` (comma then fat arrow) — consume
                                    // the redundant separator.
                                    if self.peek_kind() == Some(TokenKind::FatArrow) {
                                        self.consume_token()?;
                                    }

                                    if self.is_at_statement_end() {
                                        break;
                                    }

                                    // Check if we hit a statement modifier.
                                    match self.peek_kind() {
                                        Some(TokenKind::If)
                                        | Some(TokenKind::Unless)
                                        | Some(TokenKind::While)
                                        | Some(TokenKind::Until)
                                        | Some(TokenKind::For)
                                        | Some(TokenKind::Foreach) => break,
                                        _ => args.push(self.parse_assignment_or_declaration()?),
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
                s.check_cancelled()?;

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
                        // Don't recover from these — propagate immediately
                        if matches!(
                            e,
                            ParseError::RecursionLimit
                                | ParseError::NestingTooDeep { .. }
                                | ParseError::Cancelled
                        ) {
                            return Err(e);
                        }

                        // Record the actual error
                        s.errors.push(e.clone());

                        // Create error node for failed statement
                        let error_location = s.current_position();
                        let error_msg = format!("{}", e);
                        // Collect peek_kind before mutable borrow in recover_from_error
                        let peek_display = s.peek_kind()
                            .map(|k| k.display_name())
                            .unwrap_or("end of input");
                        let error_node = s.recover_from_error(
                            error_msg,
                            "statement".to_string(),
                            peek_display.to_string(),
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

            // Handle unclosed block at EOF: emit error but return partial block
            if s.peek_kind() == Some(TokenKind::RightBrace) {
                s.expect(TokenKind::RightBrace)?;
            } else {
                // Missing closing brace (EOF or recovery break)
                let pos = s.current_position();
                s.errors.push(ParseError::syntax(
                    "Unclosed block: expected '}' but reached end of input",
                    pos,
                ));
            }
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

    /// Parse a phase-block keyword token used as a statement label.
    ///
    /// Perl allows phase-block keywords (BEGIN, END, CHECK, INIT, UNITCHECK) as
    /// statement labels when followed by `:`.  Because these tokenise as their own
    /// `TokenKind` variants rather than `TokenKind::Identifier`, the standard
    /// `parse_labeled_statement` (which calls `expect(TokenKind::Identifier)`)
    /// cannot handle them.  This function consumes the keyword token, then the
    /// `:`, then the subordinate statement, producing a `LabeledStatement` node.
    fn parse_keyword_as_label(&mut self) -> ParseResult<Node> {
        let start = self.current_position();

        // Consume the phase-keyword token (BEGIN / END / CHECK / INIT / UNITCHECK)
        let label_token = self.consume_token()?;
        let label = label_token.text.to_string();

        // Consume the `:`
        self.expect(TokenKind::Colon)?;

        // Parse the statement that follows the label
        let statement = Box::new(self.parse_statement()?);

        let end = self.previous_position();
        Ok(Node::new(
            NodeKind::LabeledStatement { label, statement },
            SourceLocation { start, end },
        ))
    }

}
