impl<'a> Parser<'a> {
    /// Entry point for parsing postfix expressions.
    ///
    /// Parses a primary expression first, then applies any postfix operators
    /// (arrow chains, subscripts, etc.) to form a complete postfix expression.
    fn parse_postfix(&mut self) -> ParseResult<Node> {
        let expr = self.parse_primary()?;
        self.parse_postfix_chain(expr)
    }

    /// Apply postfix operators (arrow chains, subscripts, etc.) to an
    /// already-parsed expression.
    ///
    /// This function is factored out of `parse_postfix` so that callers who
    /// build an initial node outside the normal `parse_primary` path
    /// (e.g. typeglobs in `parse_unary`) can still participate in postfix chaining.
    ///
    /// The loop handles several postfix patterns in order of precedence:
    /// 1. Hash/array slice without arrow (`@hash{...}`, `%hash{...}`)
    /// 2. Increment/decrement operators (`++`, `--`)
    /// 3. Arrow dereference (`->`)
    /// 4. Array subscript (`[...]`)
    /// 5. Hash subscript with block handling (`{...}`)
    /// 6. Function call parentheses (`(...)`)
    pub(crate) fn parse_postfix_chain(&mut self, mut expr: Node) -> ParseResult<Node> {
        let mut postfix_chain_depth = 0usize;

        // Closure to track nesting depth and prevent stack overflow on deeply
        // nested postfix chains (e.g., `$a->[0]->[1]->[2]->...`).
        let mut record_postfix_layer = || -> ParseResult<()> {
            postfix_chain_depth += 1;
            if postfix_chain_depth > MAX_RECURSION_DEPTH {
                return Err(ParseError::NestingTooDeep {
                    depth: postfix_chain_depth,
                    max_depth: MAX_RECURSION_DEPTH,
                });
            }
            Ok(())
        };

        loop {
            // --------------------------------------------------------------------
            // Hash/array slice without arrow: @hash{...} or %hash{...}
            //
            // In Perl, `@hash{...}` and `%hash{...}` are valid hash/array slice
            // operations that do NOT require an intervening `->`.
            //
            // This must be checked BEFORE the Arrow arm (line 69) because the
            // Arrow arm's LeftBrace handling (line 295) is only reached when there
            // is a `->` preceding the `{`. Without this early check, `@hash{...}`
            // would fall through to the generic hash-element arm at line 428,
            // which would incorrectly parse `{...}` as a block instead of a subscript.
            //
            // The condition checks:
            // 1. The next token is `{` (not `->`)
            // 2. The current expression is a variable with `@` or `%` sigil
            //
            // Example: `@ops_seen{ map split(/ /), values %ops }` should parse as
            // a hash slice, not as `@ops_seen` followed by a block.
            // --------------------------------------------------------------------
            if self.peek_kind() == Some(TokenKind::LeftBrace) {
                if let NodeKind::Variable { sigil, .. } = &expr.kind {
                    if sigil == "@" || sigil == "%" {
                        // Hash/array slice: @hash{...} or %hash{...}
                        self.tokens.next()?; // consume {
                        let key = self.parse_hash_subscript_key()?;
                        self.expect(TokenKind::RightBrace)?;

                        let start = expr.location.start;
                        let end = self.previous_position();

                        record_postfix_layer()?;
                        expr = Node::new(
                            NodeKind::Binary {
                                op: "{}".to_string(),
                                left: Box::new(expr),
                                right: Box::new(key),
                            },
                            SourceLocation { start, end },
                        );
                        continue;
                    }
                }
            }

            match self.peek_kind() {
                Some(k) if Self::is_postfix_op(Some(k)) => {
                    let op_token = self.consume_token()?;
                    let start = expr.location.start;
                    let end = op_token.end;

                    record_postfix_layer()?;
                    expr = Node::new(
                        NodeKind::Unary { op: op_token.text.to_string(), operand: Box::new(expr) },
                        SourceLocation { start, end },
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

                                record_postfix_layer()?;
                                expr = Node::new(
                                    NodeKind::Unary {
                                        op: "->@*".to_string(),
                                        operand: Box::new(expr),
                                    },
                                    SourceLocation { start, end },
                                );
                            } else if self.peek_kind() == Some(TokenKind::LeftBracket) {
                                // ->@[...] array slice
                                self.tokens.next()?; // consume [
                                let index = self.parse_expression()?;
                                self.expect_closing_delimiter(TokenKind::RightBracket)?;

                                let start = expr.location.start;
                                let end = self.previous_position();

                                // Represent as a special binary operation for array slice dereference
                                record_postfix_layer()?;
                                expr = Node::new(
                                    NodeKind::Binary {
                                        op: "->@[]".to_string(),
                                        left: Box::new(expr),
                                        right: Box::new(index),
                                    },
                                    SourceLocation { start, end },
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

                                record_postfix_layer()?;
                                expr = Node::new(
                                    NodeKind::Unary {
                                        op: "->%*".to_string(),
                                        operand: Box::new(expr),
                                    },
                                    SourceLocation { start, end },
                                );
                            } else if self.peek_kind() == Some(TokenKind::LeftBrace) {
                                // ->%{...} hash slice
                                self.tokens.next()?; // consume {
                                let key = self.parse_hash_subscript_key()?;
                                self.expect(TokenKind::RightBrace)?;

                                let start = expr.location.start;
                                let end = self.previous_position();

                                // Represent as a special binary operation for hash slice dereference
                                record_postfix_layer()?;
                                expr = Node::new(
                                    NodeKind::Binary {
                                        op: "->%{}".to_string(),
                                        left: Box::new(expr),
                                        right: Box::new(key),
                                    },
                                    SourceLocation { start, end },
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

                                record_postfix_layer()?;
                                expr = Node::new(
                                    NodeKind::Unary {
                                        op: "->$*".to_string(),
                                        operand: Box::new(expr),
                                    },
                                    SourceLocation { start, end },
                                );
                            }
                        }

                        Some(TokenKind::SubSigil | TokenKind::BitwiseAnd) => {
                            // ->&* (code dereference)
                            self.tokens.next()?; // consume &

                            if self.peek_kind() == Some(TokenKind::Star) {
                                self.tokens.next()?; // consume *
                                let start = expr.location.start;
                                let end = self.previous_position();

                                record_postfix_layer()?;
                                expr = Node::new(
                                    NodeKind::Unary {
                                        op: "->&*".to_string(),
                                        operand: Box::new(expr),
                                    },
                                    SourceLocation { start, end },
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

                                record_postfix_layer()?;
                                expr = Node::new(
                                    NodeKind::Unary {
                                        op: "->**".to_string(),
                                        operand: Box::new(expr),
                                    },
                                    SourceLocation { start, end },
                                );
                            }
                        }

                        Some(kind) if Self::can_be_sub_name(kind) => {
                            // Check for ->$#* (postfix last-index dereference, Perl 5.20+).
                            // The lexer produces Identifier("$#") for `$#` when no array
                            // name follows, so we handle it here before the method-call path.
                            if self.tokens.peek().is_ok_and(|t| t.text.as_ref() == "$#") {
                                if self
                                    .tokens
                                    .peek_second()
                                    .is_ok_and(|t| t.kind == TokenKind::Star)
                                {
                                    self.tokens.next()?; // consume $#
                                    self.tokens.next()?; // consume *
                                    let start = expr.location.start;
                                    let end = self.previous_position();
                                    record_postfix_layer()?;
                                    expr = Node::new(
                                        NodeKind::Unary {
                                            op: "->$#*".to_string(),
                                            operand: Box::new(expr),
                                        },
                                        SourceLocation { start, end },
                                    );
                                    continue;
                                }
                            }

                            // Method call
                            let method = self.consume_token()?.text.to_string();

                            let args = if self.peek_kind() == Some(TokenKind::LeftParen) {
                                self.parse_args()?
                            } else {
                                Vec::new()
                            };

                            let start = expr.location.start;
                            let end = self.previous_position();

                            record_postfix_layer()?;
                            expr = Node::new(
                                NodeKind::MethodCall { object: Box::new(expr), method, args },
                                SourceLocation { start, end },
                            );
                        }

                        Some(TokenKind::LeftParen) => {
                            // Coderef invocation: $ref->(args)
                            let args = self.parse_args()?;
                            let start = expr.location.start;
                            let end = self.previous_position();

                            let mut all_args = vec![expr];
                            all_args.extend(args);

                            record_postfix_layer()?;
                            expr = Node::new(
                                NodeKind::FunctionCall { name: "->()".to_string(), args: all_args },
                                SourceLocation { start, end },
                            );
                        }

                        Some(TokenKind::LeftBracket) => {
                            // Arrow array dereference: $ref->[index]
                            self.tokens.next()?; // consume [
                            let index = self.parse_expression()?;
                            self.expect_closing_delimiter(TokenKind::RightBracket)?;

                            let start = expr.location.start;
                            let end = self.previous_position();

                            record_postfix_layer()?;
                            expr = Node::new(
                                NodeKind::Binary {
                                    op: "->[]".to_string(),
                                    left: Box::new(expr),
                                    right: Box::new(index),
                                },
                                SourceLocation { start, end },
                            );
                        }

                        Some(TokenKind::LeftBrace) => {
                            // Arrow hash dereference: $ref->{key}
                            self.tokens.next()?; // consume {
                            let key = self.parse_hash_subscript_key()?;
                            self.expect(TokenKind::RightBrace)?;

                            let start = expr.location.start;
                            let end = self.previous_position();

                            record_postfix_layer()?;
                            expr = Node::new(
                                NodeKind::Binary {
                                    op: "->{}".to_string(),
                                    left: Box::new(expr),
                                    right: Box::new(key),
                                },
                                SourceLocation { start, end },
                            );
                        }

                        _ => {
                            // `->` was consumed but the next token is not a valid
                            // postfix continuation (method name, paren, bracket, brace,
                            // or dereference sigil).  This is a truncated postfix chain.
                            //
                            // Emit a structured recovery annotation and wrap the
                            // partially-parsed expression in an error node so that
                            // LSP features can still use the prefix (e.g. `$obj`).
                            let start = expr.location.start;
                            let end = self.previous_position();
                            let pos = end;
                            self.errors.push(ParseError::Recovered {
                                site: RecoverySite::PostfixChain,
                                kind: RecoveryKind::TruncatedChain,
                                location: pos,
                            });
                            expr = Node::new(
                                NodeKind::Error {
                                    message: "Incomplete arrow expression".to_string(),
                                    expected: vec![],
                                    found: self.tokens.peek().ok().cloned(),
                                    partial: Some(Box::new(expr)),
                                },
                                SourceLocation { start, end },
                            );
                            // Exit the postfix loop — we cannot continue chaining
                            // after a malformed arrow.
                            break;
                        }
                    }
                }

                Some(TokenKind::LeftBracket) => {
                    // Builtin function identifiers treat [ as anonymous-arrayref argument.
                    if let NodeKind::Identifier { name } = &expr.kind {
                        if Self::is_builtin_function(name) || self.looks_like_bare_call(name) {
                            let name = name.clone();
                            let start = expr.location.start;
                            let mut args = vec![self.parse_ternary()?];
                            while matches!(
                                self.peek_kind(),
                                Some(TokenKind::Comma) | Some(TokenKind::FatArrow)
                            ) {
                                self.consume_token()?;
                                if self.is_at_statement_end() {
                                    break;
                                }
                                args.push(self.parse_ternary()?);
                            }
                            let end = args.last().map_or(expr.location.end, |a| a.location.end);
                            expr = Node::new(
                                NodeKind::FunctionCall { name, args },
                                SourceLocation { start, end },
                            );
                            continue;
                        }
                    }
                    // Array indexing - can be a single index or slice with multiple indices
                    self.tokens.next()?; // consume [

                    // Check if this might be a slice (multiple indices)
                    let mut indices = vec![self.parse_expression()?];

                    // Look for comma-separated indices
                    while self.peek_kind() == Some(TokenKind::Comma) {
                        self.consume_token()?; // consume comma
                        indices.push(self.parse_expression()?);
                    }

                    self.expect_closing_delimiter(TokenKind::RightBracket)?;

                    // Create the index node - either single index or array of indices
                    let index = if indices.len() == 1 {
                        indices.into_iter().next().ok_or_else(|| {
                            ParseError::syntax("Empty indices vector", expr.location.start)
                        })?
                    } else {
                        // Multiple indices - create an array literal node
                        let start = indices
                            .first()
                            .ok_or_else(|| {
                                ParseError::syntax("Empty indices vector", expr.location.start)
                            })?
                            .location
                            .start;
                        let end = indices
                            .last()
                            .ok_or_else(|| {
                                ParseError::syntax("Empty indices vector", expr.location.start)
                            })?
                            .location
                            .end;
                        Node::new(
                            NodeKind::ArrayLiteral { elements: indices },
                            SourceLocation { start, end },
                        )
                    };

                    let start = expr.location.start;
                    let end = self.previous_position();

                    // Represent as binary subscript operation
                    record_postfix_layer()?;
                    expr = Node::new(
                        NodeKind::Binary {
                            op: "[]".to_string(),
                            left: Box::new(expr),
                            right: Box::new(index),
                        },
                        SourceLocation { start, end },
                    );
                }

                Some(TokenKind::LeftBrace) => {
                    // Check if this is a builtin function (or block-list func like first/any/all)
                    // or a user-defined function with a block argument that needs special handling
                    if let NodeKind::Identifier { name } = &expr.kind {
                        let is_builtin = Self::is_builtin_function(name);
                        let is_block_list = Self::is_block_list_func(name);
                        // In Perl, hash element access ALWAYS requires a sigil ($hash{key}).
                        // A bare lowercase identifier followed by { is a function call with
                        // a block argument: capture { ... }, where { ... }, etc.
                        let is_bare_func =
                            !is_builtin && !is_block_list && Self::looks_like_block_call_name(name);

                        if is_builtin || is_block_list || is_bare_func {
                            // This is a builtin function with {} as argument
                            // Parse arguments without parentheses
                            let mut args = Vec::new();

                            // Special handling for bless {} - parse it as a hash
                            if name == "bless" {
                                args.push(self.parse_hash_or_block()?);

                                // Parse remaining arguments separated by commas or fat arrows
                                while matches!(
                                    self.peek_kind(),
                                    Some(TokenKind::Comma) | Some(TokenKind::FatArrow)
                                ) {
                                    self.consume_token()?; // consume comma or fat arrow
                                    if self.is_at_statement_end() {
                                        break;
                                    }
                                    args.push(self.parse_comma()?);
                                }
                            } else if is_block_list || is_bare_func {
                                // Parse block (may contain multiple statements) as first argument
                                // for map/grep/sort/first/any/all/none/reduce/etc.
                                args.push(self.parse_builtin_block()?);

                                // Parse trailing list arguments.
                                // In Perl, the block form does not require a comma
                                // before the list: `grep { ... } @array`
                                // First consume without a comma/fat arrow if present
                                if (if is_bare_func {
                                    self.should_continue_bare_call_after_block()
                                } else {
                                    !self.is_at_statement_end()
                                })
                                    && !matches!(
                                        self.peek_kind(),
                                        Some(TokenKind::Comma) | Some(TokenKind::FatArrow)
                                    )
                                {
                                    args.push(self.parse_assignment_or_declaration()?);
                                }

                                // Then consume any remaining comma/fat-arrow-separated arguments
                                while matches!(
                                    self.peek_kind(),
                                    Some(TokenKind::Comma) | Some(TokenKind::FatArrow)
                                ) {
                                    self.consume_token()?; // consume comma or fat arrow
                                    if is_bare_func {
                                        if !self.should_continue_bare_call_after_block() {
                                            break;
                                        }
                                    } else if self.is_at_statement_end() {
                                        break;
                                    }
                                    args.push(self.parse_assignment_or_declaration()?);
                                }
                            } else {
                                // Other builtins - parse {} as first argument (filehandle or hash)
                                args.push(self.parse_hash_or_block()?);

                                // For print/say/printf/exec/system, `{ $fh } $args` uses
                                // the block as a filehandle and args follow without a comma.
                                // Collect trailing args without requiring commas first.
                                let is_fh_builtin = matches!(
                                    name.as_str(),
                                    "print" | "say" | "printf" | "exec" | "system" | "send"
                                );
                                if is_fh_builtin
                                    && !self.is_at_statement_end()
                                    && !matches!(
                                        self.peek_kind(),
                                        Some(TokenKind::Comma | TokenKind::FatArrow)
                                    )
                                {
                                    // No comma — treat the block as a filehandle and parse the list.
                                    while !self.is_at_statement_end()
                                        && !matches!(
                                            self.peek_kind(),
                                            Some(
                                                TokenKind::WordOr
                                                    | TokenKind::WordAnd
                                                    | TokenKind::WordXor
                                                    | TokenKind::WordNot
                                                    | TokenKind::Question
                                            )
                                        )
                                    {
                                        if matches!(
                                            self.peek_kind(),
                                            Some(TokenKind::Comma) | Some(TokenKind::FatArrow)
                                        ) {
                                            self.consume_token()?;
                                        }
                                        if self.is_at_statement_end() {
                                            break;
                                        }
                                        args.push(self.parse_ternary()?);
                                    }
                                } else {
                                    // Parse remaining arguments separated by commas or fat arrows
                                    while matches!(
                                        self.peek_kind(),
                                        Some(TokenKind::Comma) | Some(TokenKind::FatArrow)
                                    ) {
                                        self.consume_token()?; // consume comma or fat arrow
                                        if self.is_at_statement_end() {
                                            break;
                                        }
                                        args.push(self.parse_comma()?);
                                    }
                                }
                            }

                            let start = expr.location.start;

                            let end = args
                                .last()
                                .ok_or_else(|| ParseError::syntax("Empty arguments list", start))?
                                .location
                                .end;

                            expr = Node::new(
                                NodeKind::FunctionCall { name: name.clone(), args },
                                SourceLocation { start, end },
                            );
                            continue; // Continue the loop
                        }
                    }

                    // Hash element access
                    self.tokens.next()?; // consume {
                    let key = self.parse_hash_subscript_key()?;
                    self.expect(TokenKind::RightBrace)?;

                    let start = expr.location.start;
                    let end = self.previous_position();

                    // Represent as binary subscript operation
                    record_postfix_layer()?;
                    expr = Node::new(
                        NodeKind::Binary {
                            op: "{}".to_string(),
                            left: Box::new(expr),
                            right: Box::new(key),
                        },
                        SourceLocation { start, end },
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
                                SourceLocation { start, end },
                            );
                        } else if matches!(name.as_str(), "print" | "say" | "printf" | "send") {
                            // `print( $fh EXPR )` — filehandle-style inside explicit parens.
                            // parse_args() treats every argument as comma-separated, so
                            // `print( $fh join(...) )` fails because $fh is parsed as the
                            // only argument and `join` is unexpected before `)`.
                            // Use a specialised parser that detects the indirect-filehandle
                            // pattern: scalar-variable followed by a non-comma expression.
                            let args = self.parse_print_parens_args()?;
                            let start = expr.location.start;
                            let end = self.previous_position();

                            expr = Node::new(
                                NodeKind::FunctionCall { name, args },
                                SourceLocation { start, end },
                            );
                        } else {
                            let args = self.parse_args()?;
                            let start = expr.location.start;
                            let end = self.previous_position();

                            expr = Node::new(
                                NodeKind::FunctionCall { name, args },
                                SourceLocation { start, end },
                            );
                        }
                    }
                }

                // `undef(LIST)` — undef with explicit argument list undefines variables.
                // `EXPR(args)` where EXPR is a subscript or dereference — implicit coderef call.
                // In Perl: `$h{cb}($arg)` and `$arr[0]($arg)` are valid coderef invocations
                // without a mandatory `->`.  We handle this for the patterns that arise in
                // real CPAN code; the test cases are driven by the expected_colon error bucket.
                Some(TokenKind::LeftParen)
                    if matches!(&expr.kind, NodeKind::Undef | NodeKind::Binary { .. }) =>
                {
                    // Disambiguate: `Undef` → `undef(LIST)` builtin call.
                    // Everything else (subscript / deref Binary) → implicit coderef call.
                    let args = self.parse_args()?;
                    let start = expr.location.start;
                    let end = self.previous_position();

                    record_postfix_layer()?;
                    expr = if matches!(&expr.kind, NodeKind::Undef) {
                        Node::new(
                            NodeKind::FunctionCall {
                                name: "undef".to_string(),
                                args,
                            },
                            SourceLocation { start, end },
                        )
                    } else {
                        let mut all_args = vec![expr];
                        all_args.extend(args);
                        Node::new(
                            NodeKind::FunctionCall {
                                name: "->()".to_string(),
                                args: all_args,
                            },
                            SourceLocation { start, end },
                        )
                    };
                }

                _ => {
                    // Check if this is a builtin function that can take bare arguments
                    if let NodeKind::Identifier { name } = &expr.kind {
                        // Check for quote operators first
                        if matches!(name.as_str(), "q" | "qq" | "qw" | "qr" | "qx" | "m" | "s") {
                            // This was already parsed as a quote operator in parse_primary
                            // Don't try to parse arguments
                        } else if self.peek_kind() == Some(TokenKind::FatArrow) {
                            // Identifier before => is a hash key — do NOT treat as
                            // a builtin function call.  Fall through to break.
                        } else if Self::is_nullary_builtin(name) {
                            // Nullary builtins (shift, pop, caller, wantarray, etc.) can also
                            // take an explicit sigil-starting argument, e.g. `shift @arr`.
                            // Special case: `caller N` — caller accepts an optional stack-level
                            // number (e.g. `caller 0`, `caller 1`).
                            let next_is_sigil_arg = self.tokens.peek().ok().is_some_and(|t| {
                                t.text.starts_with('@')
                                    || t.text.starts_with('$')
                                    || t.text.starts_with('%')
                            });
                            let next_is_caller_level =
                                name == "caller" && self.peek_kind() == Some(TokenKind::Number);
                            let args = if (next_is_sigil_arg || next_is_caller_level)
                                && !self.is_at_statement_end()
                            {
                                vec![self.parse_ternary()?]
                            } else {
                                vec![]
                            };
                            let start = expr.location.start;
                            let end = args
                                .last()
                                .map(|arg: &Node| arg.location.end)
                                .unwrap_or(expr.location.end);
                            expr = Node::new(
                                NodeKind::FunctionCall { name: name.clone(), args },
                                SourceLocation { start, end },
                            );
                        } else if !Self::is_builtin_function(name)
                            && !self.is_at_statement_end()
                            && self.peek_kind() != Some(TokenKind::FatArrow)
                            && self.tokens.peek().ok().is_some_and(|t| {
                                t.text.starts_with('$')
                                    || t.text.starts_with('@')
                                    || t.text.starts_with('%')
                            })
                        {
                            // Sigil-peek heuristic: non-builtin identifier followed by a
                            // sigil-starting argument is a bare function call.
                            // Handles `blessed $self`, `reftype $x`, `weaken $ref`, etc.
                            // (imported unary functions that look like builtins at the call site)
                            //
                            // Parse only a high-precedence argument expression here so
                            // lower-precedence operators remain outside the call.
                            // Example: `is_ready $obj ? 1 : 0` must parse as
                            // `(is_ready $obj) ? 1 : 0`, not `is_ready($obj ? 1 : 0)`.
                            let arg = self.parse_shift()?;
                            let start = expr.location.start;
                            let end = arg.location.end;
                            expr = Node::new(
                                NodeKind::FunctionCall { name: name.clone(), args: vec![arg] },
                                SourceLocation { start, end },
                            );
                        } else if name.contains("::")
                            && !self.is_at_statement_end()
                            && self.peek_kind() != Some(TokenKind::FatArrow)
                            && matches!(
                                self.peek_kind(),
                                Some(
                                    TokenKind::String
                                        | TokenKind::QuoteSingle
                                        | TokenKind::QuoteDouble
                                        | TokenKind::Number
                                )
                            )
                        {
                            // Qualified call with string/number literal argument — issue #2750 Pattern B.
                            // e.g. `(Carp::croak "error")`, `(utf8::downgrade $$buf or Carp::croak "Wide char")`
                            // In paren-expression context, qualified names followed by a literal argument
                            // are treated as function calls (same as unqualified `croak "err"` via looks_like_bare_call).
                            // Guard: NOT followed by => (would be a hash-key) and NOT at statement end.
                            let mut args = vec![self.parse_ternary()?];
                            // Collect additional comma-separated arguments
                            while matches!(
                                self.peek_kind(),
                                Some(TokenKind::Comma) | Some(TokenKind::FatArrow)
                            ) && !self.is_at_statement_end()
                            {
                                self.consume_token()?; // consume , or =>
                                if self.is_at_statement_end() {
                                    break;
                                }
                                args.push(self.parse_ternary()?);
                            }
                            let start = expr.location.start;
                            let end = self.previous_position();
                            expr = Node::new(
                                NodeKind::FunctionCall { name: name.clone(), args },
                                SourceLocation { start, end },
                            );
                        } else if Self::is_builtin_function(name)
                            || Self::core_qualified_builtin_name(name).is_some()
                            || self.looks_like_bare_call(name)
                        {
                            // In call argument lists, `builtin => value` should keep the lhs as a
                            // bareword key so parse_args can auto-quote it for fat-comma pairs.
                            // Example: `$obj->on(accept => sub { ... })`.
                            if self.peek_kind() == Some(TokenKind::FatArrow) {
                                break;
                            }

                            // For CORE::qualified names, use the bare name for downstream
                            // builtin classification so that e.g. `CORE::shift` is recognised
                            // as nullary and `CORE::grep { ... } @list` gets block handling.
                            let bare_name =
                                Self::core_qualified_builtin_name(name).unwrap_or(name.as_str());

                            // Builtins always become function calls, even with no arguments
                            // This ensures they work correctly in expressions like "return $x or die"
                            //
                            // For nullary builtins like shift, pop, caller, wantarray, etc.,
                            // when followed by a binary operator, they should be treated as
                            // having no arguments (e.g., "shift || 2" means shift() || 2).
                            // Also applies to optional-arg builtins (defined, length, ord, etc.)
                            // that implicitly use $_ when no explicit argument is given, so that
                            // `defined && ...`, `length > 0`, `ord >= 32` parse correctly.
                            let is_nullary_without_args = (Self::is_nullary_builtin(bare_name)
                                || Self::is_optional_arg_builtin(bare_name))
                                && self.peek_kind().is_some_and(Self::is_binary_operator);

                            // When a builtin is followed by a comma, it should be treated
                            // as having no arguments.  The comma belongs to an enclosing
                            // list context (e.g. `grep defined, @list`).
                            let is_comma_terminated = self.peek_kind() == Some(TokenKind::Comma);

                            // String comparison operators (eq, ne, lt, le, gt, ge) are
                            // tokenized as Identifiers. A builtin followed by one of these
                            // should be treated as having no arguments, so that
                            // `ref eq 'CODE'` parses as `ref() eq 'CODE'` (not `ref(eq)`).
                            // `cmp` is also a string comparison operator tokenized as Identifier.
                            let is_str_op_terminated = self.peek_kind()
                                == Some(TokenKind::Identifier)
                                && self.tokens.peek().ok().is_some_and(|t| {
                                    matches!(
                                        t.text.as_ref(),
                                        "eq" | "ne" | "lt" | "le" | "gt" | "ge" | "cmp"
                                    )
                                });

                            if self.is_at_statement_end()
                                || is_nullary_without_args
                                || is_comma_terminated
                                || is_str_op_terminated
                            {
                                // Bare builtin with no arguments
                                expr = Node::new(
                                    NodeKind::FunctionCall { name: name.clone(), args: vec![] },
                                    expr.location,
                                );
                            } else {
                                // Parse arguments without parentheses
                                let mut args = Vec::new();

                                // Special handling for sort/map/grep/first/any/all/etc.
                                // with block first argument
                                if Self::is_block_list_func(bare_name)
                                    && self.peek_kind() == Some(TokenKind::LeftBrace)
                                {
                                    // Parse block (may contain multiple statements) as first argument
                                    args.push(self.parse_builtin_block()?);

                                    // Parse remaining arguments without requiring commas
                                    // But respect statement boundaries including ] and )
                                    // Word operators terminate argument collection since
                                    // they bind less tightly than list operators.
                                    while !self.is_at_statement_end()
                                        && !matches!(
                                            self.peek_kind(),
                                            Some(
                                                TokenKind::WordOr
                                                    | TokenKind::WordAnd
                                                    | TokenKind::WordXor
                                                    | TokenKind::WordNot
                                                    | TokenKind::Question
                                            )
                                        )
                                    {
                                        // Skip comma or fat arrow if present
                                        if matches!(
                                            self.peek_kind(),
                                            Some(TokenKind::Comma) | Some(TokenKind::FatArrow)
                                        ) {
                                            self.consume_token()?;
                                        }
                                        // Check again after potential comma/fat arrow
                                        if self.is_at_statement_end() {
                                            break;
                                        }
                                        args.push(self.parse_ternary()?);
                                    }
                                } else if bare_name == "sort"
                                    && matches!(self.peek_kind(), Some(TokenKind::Identifier))
                                    && self.tokens.peek().ok().is_some_and(|t| {
                                        // Named comparator: lowercase identifier that's not a
                                        // binary string op and not a block-list function.
                                        // e.g. `sort cmp_events @list`
                                        // Block-list functions (grep, map, sort, etc.) cannot be
                                        // sort comparators — `sort grep { ... } @list` means
                                        // sort the result of grep, not `sort grep_func @list`.
                                        let txt: &str = &t.text;
                                        !txt.is_empty()
                                            && txt.starts_with(|c: char| {
                                                c.is_ascii_lowercase() || c == '_'
                                            })
                                            && !matches!(
                                                txt,
                                                "eq" | "ne"
                                                    | "lt"
                                                    | "le"
                                                    | "gt"
                                                    | "ge"
                                                    | "cmp"
                                                    | "x"
                                            )
                                            && !Self::is_block_list_func(txt)
                                    })
                                {
                                    // sort FUNCNAME LIST — `sort by_name @list`
                                    // Parse the comparator function name as the first arg,
                                    // then collect the list to sort.
                                    args.push(self.parse_ternary()?);

                                    while !self.is_at_statement_end()
                                        && !matches!(
                                            self.peek_kind(),
                                            Some(
                                                TokenKind::WordOr
                                                    | TokenKind::WordAnd
                                                    | TokenKind::WordXor
                                                    | TokenKind::WordNot
                                                    | TokenKind::Question
                                            )
                                        )
                                    {
                                        if matches!(
                                            self.peek_kind(),
                                            Some(TokenKind::Comma) | Some(TokenKind::FatArrow)
                                        ) {
                                            self.consume_token()?;
                                        }
                                        if self.is_at_statement_end() {
                                            break;
                                        }
                                        args.push(self.parse_ternary()?);
                                    }
                                } else if bare_name == "sort"
                                    && self.tokens.peek().ok().is_some_and(|t| {
                                        // Scalar-variable coderef: text starts with `$`
                                        // e.g. `sort $cmp @list`, `sort $keysort (keys %h)`
                                        t.kind == TokenKind::Identifier && t.text.starts_with('$')
                                    })
                                {
                                    // sort $coderef LIST — `sort $cmp @list`, `sort $cmp (keys %h)`
                                    // The scalar is a coderef comparator. Consume it as the first
                                    // arg, then collect the list to sort (issue #2750 Pattern C).
                                    args.push(self.parse_ternary()?);

                                    while !self.is_at_statement_end()
                                        && !matches!(
                                            self.peek_kind(),
                                            Some(
                                                TokenKind::WordOr
                                                    | TokenKind::WordAnd
                                                    | TokenKind::WordXor
                                                    | TokenKind::WordNot
                                                    | TokenKind::Question
                                            )
                                        )
                                    {
                                        if matches!(
                                            self.peek_kind(),
                                            Some(TokenKind::Comma) | Some(TokenKind::FatArrow)
                                        ) {
                                            self.consume_token()?;
                                        }
                                        if self.is_at_statement_end() {
                                            break;
                                        }
                                        args.push(self.parse_ternary()?);
                                    }
                                } else if bare_name == "bless"
                                    && self.peek_kind() == Some(TokenKind::LeftBrace)
                                {
                                    // Special handling for bless {} - parse it as a hash
                                    args.push(self.parse_hash_or_block()?);

                                    // Parse remaining arguments separated by commas or fat arrows
                                    while matches!(
                                        self.peek_kind(),
                                        Some(TokenKind::Comma) | Some(TokenKind::FatArrow)
                                    ) {
                                        self.consume_token()?; // consume comma or fat arrow
                                        if self.is_at_statement_end() {
                                            break;
                                        }
                                        args.push(self.parse_assignment()?);
                                    }
                                } else if matches!(
                                    bare_name,
                                    "print" | "say" | "printf" | "exec" | "system" | "send"
                                ) && self.peek_kind() == Some(TokenKind::LeftBrace)
                                {
                                    // print { $fh } ARGS — block-form filehandle in expr context
                                    // Parse the block as the filehandle, then the remaining args.
                                    args.push(self.parse_hash_or_block()?);
                                    while !self.is_at_statement_end()
                                        && !matches!(
                                            self.peek_kind(),
                                            Some(
                                                TokenKind::WordOr
                                                    | TokenKind::WordAnd
                                                    | TokenKind::WordXor
                                                    | TokenKind::WordNot
                                                    | TokenKind::Question
                                            )
                                        )
                                    {
                                        if matches!(
                                            self.peek_kind(),
                                            Some(TokenKind::Comma) | Some(TokenKind::FatArrow)
                                        ) {
                                            self.consume_token()?;
                                        }
                                        if self.is_at_statement_end() {
                                            break;
                                        }
                                        args.push(self.parse_ternary()?);
                                    }
                                } else if matches!(bare_name, "split" | "grep" | "map" | "sort")
                                    && self.peek_kind() == Some(TokenKind::Slash)
                                {
                                    // For `split /regex/, ...` and `grep /regex/, @list`,
                                    // re-lex `/` as regex delimiter
                                    self.tokens.relex_as_term();
                                    args.push(self.parse_ternary()?);

                                    // Parse remaining arguments separated by commas or fat arrows
                                    while matches!(
                                        self.peek_kind(),
                                        Some(TokenKind::Comma) | Some(TokenKind::FatArrow)
                                    ) {
                                        self.consume_token()?;
                                        if self.is_at_statement_end() {
                                            break;
                                        }
                                        args.push(self.parse_ternary()?);
                                    }
                                } else {
                                    // Parse the first argument
                                    args.push(self.parse_assignment_or_declaration()?);

                                    // Generic bare calls can also take implicit list arguments
                                    // after a leading block/hash argument, just like parse_args()
                                    // does for parenthesized calls.
                                    while matches!(
                                        args.last(),
                                        Some(n)
                                            if matches!(
                                                n.kind,
                                                NodeKind::Block { .. } | NodeKind::HashLiteral { .. }
                                            )
                                    ) && self.should_continue_bare_call_after_block()
                                    {
                                        args.push(self.parse_assignment_or_declaration()?);
                                    }

                                    // Special case: print/say/printf/exec/send with indirect object.
                                    // `print $fh $msg` / `send $sock $msg` — first arg is the
                                    // filehandle/socket (no comma before remaining args).
                                    // After the first arg, if next is not a comma/terminator,
                                    // treat first arg as indirect object and continue parsing the list.
                                    if matches!(
                                        bare_name,
                                        "print" | "say" | "printf" | "exec" | "system" | "send"
                                    ) && !self.is_at_statement_end()
                                        && !matches!(
                                            self.peek_kind(),
                                            Some(
                                                TokenKind::Comma
                                                    | TokenKind::FatArrow
                                                    | TokenKind::WordOr
                                                    | TokenKind::WordAnd
                                                    | TokenKind::WordXor
                                                    | TokenKind::WordNot
                                                    | TokenKind::Question
                                            )
                                        )
                                    {
                                        // No comma after first arg — it's an indirect object
                                        // (filehandle for print/say/printf, socket for send).
                                        // Parse the remaining args.
                                        while !self.is_at_statement_end()
                                            && !matches!(
                                                self.peek_kind(),
                                                Some(
                                                    TokenKind::WordOr
                                                        | TokenKind::WordAnd
                                                        | TokenKind::WordXor
                                                        | TokenKind::WordNot
                                                        | TokenKind::Question
                                                )
                                            )
                                        {
                                            if matches!(
                                                self.peek_kind(),
                                                Some(TokenKind::Comma) | Some(TokenKind::FatArrow)
                                            ) {
                                                self.consume_token()?;
                                            }
                                            if self.is_at_statement_end() {
                                                break;
                                            }
                                            args.push(self.parse_assignment_or_declaration()?);
                                        }
                                    }

                                    // Parse remaining arguments separated by commas or fat arrows
                                    // Perl allows `push @array => $value` as well as commas
                                    while matches!(
                                        self.peek_kind(),
                                        Some(TokenKind::Comma) | Some(TokenKind::FatArrow)
                                    ) {
                                        self.consume_token()?;
                                        if self.is_at_statement_end() {
                                            break;
                                        }
                                        args.push(self.parse_assignment_or_declaration()?);
                                    }
                                }

                                let start = expr.location.start;

                                let end = args
                                    .last()
                                    .ok_or_else(|| {
                                        ParseError::syntax("Empty arguments list", start)
                                    })?
                                    .location
                                    .end;

                                expr = Node::new(
                                    NodeKind::FunctionCall { name: name.clone(), args },
                                    SourceLocation { start, end },
                                );
                            }
                        }
                    } else if matches!(expr.kind, NodeKind::Undef) {
                        // `undef` is a keyword token (not Identifier) so it
                        // bypasses the generic builtin-function path above.
                        // When `undef` is followed by a sigil-starting argument
                        // (e.g. `undef $var`, `undef @arr`) in an expression context,
                        // treat it as `undef(EXPR)`.  This handles patterns like
                        // `close($f) or undef $ret` where `undef` is not at statement
                        // start and the argument is not separated by a comma.
                        let next_is_sigil_arg = !self.is_at_statement_end()
                            && self.tokens.peek().ok().is_some_and(|t| {
                                t.text.starts_with('$')
                                    || t.text.starts_with('@')
                                    || t.text.starts_with('%')
                            });
                        if next_is_sigil_arg {
                            let arg = self.parse_ternary()?;
                            let start = expr.location.start;
                            let end = arg.location.end;
                            expr = Node::new(
                                NodeKind::FunctionCall {
                                    name: "undef".to_string(),
                                    args: vec![arg],
                                },
                                SourceLocation { start, end },
                            );
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
            Some(TokenKind::Semicolon)
                | Some(TokenKind::RightBrace)
                | Some(TokenKind::RightParen)
                | Some(TokenKind::RightBracket)
                | Some(TokenKind::If)
                | Some(TokenKind::Unless)
                | Some(TokenKind::While)
                | Some(TokenKind::Until)
                | Some(TokenKind::For)
                | Some(TokenKind::Foreach)
                | Some(TokenKind::WordAnd)
                | Some(TokenKind::WordOr)
                | Some(TokenKind::WordXor)
                | Some(TokenKind::WordNot)
                | Some(TokenKind::Question)
                | Some(TokenKind::DataMarker)
                | Some(TokenKind::Eof)
                | None
        )
    }

    /// Check whether the current peek token is a quote-op name that should be
    /// treated as a bareword hash key inside a subscript.
    ///
    /// Returns `true` only when the token is `m|s|q|qq|qw|qr|qx|tr|y` AND
    /// the immediately following token is `}` or `,` — meaning the identifier
    /// cannot be the start of a real quote/regex expression.
    ///
    /// Contrast: `qw(a b)` has `qw` followed by `(`, so it returns `false`
    /// and the normal `parse_expression` path handles it as a `qw(...)` literal.
    fn peek_is_quote_op_bareword(&mut self) -> bool {
        if self.peek_kind() == Some(TokenKind::Identifier) {
            if let Ok(first) = self.tokens.peek() {
                let is_quote_op_name = matches!(
                    first.text.as_ref(),
                    "m" | "s" | "q" | "qq" | "qw" | "qr" | "qx" | "tr" | "y"
                );
                if is_quote_op_name {
                    // Only treat as a bareword key if the NEXT token is `}` or `,`
                    // (meaning there is no delimiter to start a real quote expression).
                    if let Ok(second) = self.tokens.peek_second() {
                        return matches!(second.kind, TokenKind::RightBrace | TokenKind::Comma);
                    }
                }
            }
        }
        false
    }

    /// Consume the next token as a bareword string node (for quote-op names used
    /// as hash keys, e.g. the `m` in `$h{m}` or `@h{m, s}`).
    fn consume_as_bareword_string(&mut self) -> ParseResult<Node> {
        let token = self.tokens.next()?;
        Ok(Node::new(
            NodeKind::String { value: token.text.to_string(), interpolated: false },
            SourceLocation { start: token.start, end: token.end },
        ))
    }

    /// Parse hash subscript key expression, treating lone keywords as bare
    /// identifiers when they appear as `$h->{keyword}` or `$h{keyword}`.
    ///
    /// Keywords like `not`, `and`, `or`, `xor`, `do`, `eval` would normally be
    /// consumed as operators or statement keywords. When one of these appears
    /// inside a hash subscript followed immediately by `}`, it should be treated
    /// as a bare hash key instead.
    ///
    /// Additionally handles quote-operator names (`m`, `s`, `q`, etc.) when used
    /// as hash subscript keys. The lexer suppresses quote-op detection inside
    /// hash subscripts (hash_brace_depth > 0), emitting them as Identifier tokens.
    /// This function builds a proper parse tree node for them, including support
    /// for hash slices like `@h{m, s}` which require a list node.
    fn parse_hash_subscript_key(&mut self) -> ParseResult<Node> {
        // Try keyword-as-bareword first (not, and, or, xor, do, eval)
        if let Some(node) = self.try_parse_keyword_bareword_key()? {
            return Ok(node);
        }

        // Try quote-op names as bareword hash keys
        if let Some(node) = self.try_parse_quote_op_bareword_key()? {
            return Ok(node);
        }

        // Default: parse as a general expression
        self.parse_expression()
    }

    /// Attempt to parse a keyword (`not`, `and`, `or`, `xor`, `do`, `eval`) as a
    /// bareword hash key when it appears directly before `}`.
    ///
    /// Returns `Some(Node)` if the current token is a keyword followed by `}`,
    /// otherwise returns `None` to fall through to general expression parsing.
    fn try_parse_keyword_bareword_key(&mut self) -> ParseResult<Option<Node>> {
        let Some(kind) = self.peek_kind() else {
            return Ok(None);
        };

        let is_simple_keyword_key = matches!(
            kind,
            TokenKind::WordNot
                | TokenKind::WordAnd
                | TokenKind::WordOr
                | TokenKind::WordXor
                | TokenKind::Do
                | TokenKind::Eval
        );

        if !is_simple_keyword_key {
            return Ok(None);
        }

        let Ok(second) = self.tokens.peek_second() else {
            return Ok(None);
        };

        if second.kind != TokenKind::RightBrace {
            return Ok(None);
        }

        let token = self.tokens.next()?;
        Ok(Some(Node::new(
            NodeKind::Identifier { name: token.text.to_string() },
            SourceLocation { start: token.start, end: token.end },
        )))
    }

    /// Attempt to parse a quote-operator name (`m`, `s`, `q`, `qq`, `qw`, `qr`,
    /// `qx`, `tr`, `y`) as a bareword hash key when used in a hash subscript.
    ///
    /// The lexer suppresses quote-op detection inside hash subscripts
    /// (hash_brace_depth > 0), emitting them as Identifier tokens.
    /// This method builds a proper parse tree node, including support for
    /// hash slices like `@h{m, s}` which require an ArrayLiteral node.
    ///
    /// Returns `Some(Node)` if the current token is a quote-op name followed by
    /// `}` or `,` (indicating it's being used as a bareword key rather than as
    /// the start of a quote expression). Returns `None` otherwise.
    fn try_parse_quote_op_bareword_key(&mut self) -> ParseResult<Option<Node>> {
        if !self.peek_is_quote_op_bareword() {
            return Ok(None);
        }

        let first = self.consume_as_bareword_string()?;
        let start = first.location.start;

        // Single key (common case): `$h{m}` — `}` immediately follows
        if self.peek_kind() != Some(TokenKind::Comma) {
            return Ok(Some(first));
        }

        // Slice case: `@h{m, s, q}` — build a list node from all bareword keys
        let mut elements = vec![first];
        while self.peek_kind() == Some(TokenKind::Comma) {
            self.consume_token()?; // consume `,`
            if self.peek_kind() == Some(TokenKind::RightBrace) {
                break; // trailing comma before `}` is fine
            }
            if self.peek_is_quote_op_bareword() {
                elements.push(self.consume_as_bareword_string()?);
            } else {
                // Mixed slice like `@h{m, $var}` — parse rest normally
                elements.push(self.parse_assignment()?);
            }
        }

        let end = elements.last().map(|n| n.location.end).unwrap_or(start);
        Ok(Some(Node::new(
            NodeKind::ArrayLiteral { elements },
            SourceLocation { start, end },
        )))
    }
}
