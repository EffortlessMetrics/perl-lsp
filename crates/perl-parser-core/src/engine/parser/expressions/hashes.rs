impl<'a> Parser<'a> {
    /// Parse block specifically for builtin functions (map, grep, sort)
    /// These always parse {} as blocks, never as hashes.
    ///
    /// The block may contain multiple statements separated by semicolons,
    /// e.g. `map { my $y = uc $_; $y } @list`.
    fn parse_builtin_block(&mut self) -> ParseResult<Node> {
        self.with_recursion_guard(|s| {
            let start_token = s.tokens.next()?; // consume {
            let start = start_token.start;

            let mut statements = Vec::new();

            while s.peek_kind() != Some(TokenKind::RightBrace) && !s.tokens.is_eof() {
                statements.push(s.parse_statement()?);

                // Swallow stray semicolons between statements
                while s.peek_kind() == Some(TokenKind::Semicolon) {
                    s.consume_token()?;
                }
            }

            s.expect(TokenKind::RightBrace)?;
            let end = s.previous_position();

            // Always return a block node for builtin functions
            Ok(Node::new(NodeKind::Block { statements }, SourceLocation { start, end }))
        })
    }

    /// Parse hash literal or block
    fn parse_hash_or_block(&mut self) -> ParseResult<Node> {
        self.parse_hash_or_block_with_context(false)
    }

    /// Parse hash literal or block with context about whether blocks are expected
    fn parse_hash_or_block_with_context(&mut self, expect_block: bool) -> ParseResult<Node> {
        self.with_recursion_guard(|s| s.parse_hash_or_block_inner(expect_block))
    }

    fn parse_hash_or_block_inner(&mut self, _expect_block: bool) -> ParseResult<Node> {
        self.check_recursion()?;
        let start_token = self.tokens.next()?; // consume {
        let start = start_token.start;

        // Peek ahead to determine if it's a hash or block
        // For empty {}, decide based on context
        if self.peek_kind() == Some(TokenKind::RightBrace) {
            self.tokens.next()?; // consume }
            let end = self.previous_position();

            // For empty braces, default to hash (correct for most functions)
            // Functions like sort/map/grep have special handling that creates blocks
            self.exit_recursion();
            return Ok(Node::new(
                NodeKind::HashLiteral { pairs: Vec::new() },
                SourceLocation { start, end },
            ));
        }

        // For non-empty braces, we need to check if it contains hash-like content
        // Save position to potentially backtrack
        let _saved_pos = self.current_position();

        // Track error count before parsing the first expression so we can detect
        // whether inner recovery occurred (e.g. an unclosed `[` inside the hash/block).
        // This is used in the "block" fallback path to prevent premature bail (#1352).
        let errors_before = self.errors.len();

        // Try to parse as expression (which might be hash contents)
        let first_expr = match self.parse_expression() {
            Ok(expr) => expr,
            Err(e) => {
                // Propagate recursion/nesting limits immediately - don't try alternative parse
                if matches!(e, ParseError::RecursionLimit | ParseError::NestingTooDeep { .. }) {
                    return Err(e);
                }
                // Fix #1352: If peek is at a statement boundary (;, sub, my, EOF, etc.)
                // and not the `}` we want, the `{` is unclosed. Recover here at the
                // expression boundary — do NOT enter the block-statement loop which would
                // swallow trailing sub/my/... declarations into this node.
                if self.peek_kind() != Some(TokenKind::RightBrace)
                    && self.is_delimiter_recovery_point()
                {
                    self.expect_closing_delimiter(TokenKind::RightBrace)?;
                    let end = self.previous_position();
                    self.exit_recursion();
                    // Emit a recovery marker so the AST reflects the incomplete brace
                    // content instead of a silently-empty block. Without this, an
                    // incomplete construct like `{ host => "localhost", port =>` parsed
                    // to `(block )` with no marker, hiding the error from AST consumers
                    // (the diagnostic itself is recorded by expect_closing_delimiter
                    // above). We intentionally do NOT enter the block-statement loop —
                    // that would swallow trailing `sub`/`my` declarations (#1352).
                    let marker = Node::new(
                        NodeKind::MissingExpression,
                        SourceLocation { start, end },
                    );
                    return Ok(Node::new(
                        NodeKind::Block { statements: vec![marker] },
                        SourceLocation { start, end },
                    ));
                }
                // If we can't parse an expression, parse as block statements
                let mut statements = Vec::new();
                while self.peek_kind() != Some(TokenKind::RightBrace) && !self.tokens.is_eof() {
                    statements.push(self.parse_statement()?);
                }

                self.expect(TokenKind::RightBrace)?;
                let end = self.previous_position();

                self.exit_recursion();
                return Ok(Node::new(
                    NodeKind::Block { statements },
                    SourceLocation { start, end },
                ));
            }
        };

        // Check if we should close the brace now
        if self.peek_kind() == Some(TokenKind::RightBrace) {
            self.tokens.next()?; // consume }
            let end = self.previous_position();

            // Destructure first_expr to consume its kind by move, avoiding clones
            let Node { kind: first_kind, location: first_loc } = first_expr;

            match first_kind {
                // Array literal that should be a hash: convert pairs via move
                // This happens when parse_comma creates an array from key => value pairs
                NodeKind::ArrayLiteral { elements }
                    if elements.len() % 2 == 0 && !elements.is_empty() =>
                {
                    let mut pairs = Vec::with_capacity(elements.len() / 2);
                    let mut iter = elements.into_iter();
                    while let Some(key) = iter.next() {
                        // Safety: len is even and non-zero, so values are always paired
                        if let Some(value) = iter.next() {
                            pairs.push((key, value));
                        }
                    }

                    self.exit_recursion();
                    return Ok(Node::new(
                        NodeKind::HashLiteral { pairs },
                        SourceLocation { start, end },
                    ));
                }

                // Already a HashLiteral — return it directly
                // This happens when parse_comma creates a HashLiteral from key => value pairs
                kind @ NodeKind::HashLiteral { .. } => {
                    self.exit_recursion();
                    return Ok(Node::new(kind, first_loc));
                }

                // Otherwise it's a block with a single expression
                other_kind => {
                    let expr_node = Node::new(other_kind, first_loc);
                    self.exit_recursion();
                    return Ok(Node::new(
                        NodeKind::Block { statements: vec![expr_node] },
                        SourceLocation { start, end },
                    ));
                }
            }
        }

        // If there's more content, we need to determine if it's hash pairs or block statements
        let mut pairs = Vec::new();
        let mut _is_hash = false;

        // Check if next token is => or ,
        let next_kind = self.peek_kind();

        // Parse as hash if we see => or comma-separated pairs
        if matches!(next_kind, Some(k) if matches!(k, TokenKind::FatArrow | TokenKind::Comma)) {
            // Parse as hash
            _is_hash = true;

            if self.peek_kind() == Some(TokenKind::FatArrow) {
                // key => value pattern
                self.tokens.next()?; // consume =>
                let value = self.parse_expression()?;
                pairs.push((first_expr, value));
            } else if self.peek_kind() == Some(TokenKind::Comma) {
                // comma-separated pattern: key, value, key2, value2
                self.consume_token()?; // consume comma
                self.consume_redundant_commas()?;

                if self.peek_kind() != Some(TokenKind::RightBrace) {
                    let second = self.parse_expression()?;
                    pairs.push((first_expr, second));
                } else {
                    // Trailing comma - treat as single element hash with undef value
                    let undef = Node::new(
                        NodeKind::Identifier { name: "undef".to_string() },
                        SourceLocation {
                            start: self.current_position(),
                            end: self.current_position(),
                        },
                    );
                    pairs.push((first_expr, undef));
                }
            }

            // Parse remaining pairs
            while self.peek_kind() == Some(TokenKind::Comma)
                || self.peek_kind() == Some(TokenKind::FatArrow)
            {
                if self.peek_kind() == Some(TokenKind::Comma) {
                    self.consume_token()?; // consume comma
                    self.consume_redundant_commas()?;
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
                    self.consume_token()?; // consume comma
                    self.consume_redundant_commas()?;

                    if self.peek_kind() == Some(TokenKind::RightBrace) {
                        // Odd number of elements - last one becomes undef value
                        let undef = Node::new(
                            NodeKind::Identifier { name: "undef".to_string() },
                            SourceLocation {
                                start: self.current_position(),
                                end: self.current_position(),
                            },
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
                        SourceLocation {
                            start: self.current_position(),
                            end: self.current_position(),
                        },
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

            self.exit_recursion();
            Ok(Node::new(NodeKind::HashLiteral { pairs }, SourceLocation { start, end }))
        } else {
            // Not a hash - parse as block
            if self.peek_kind() == Some(TokenKind::RightBrace) {
                // Single expression block
                self.tokens.next()?; // consume }
                let end = self.previous_position();

                self.exit_recursion();
                return Ok(Node::new(
                    NodeKind::Block { statements: vec![first_expr] },
                    SourceLocation { start, end },
                ));
            }

            // Fix #1352: Detect unclosed-delimiter situations at the expression boundary
            // before entering the multi-statement block loop that would swallow trailing
            // sub/my/... declarations into this node.
            //
            // Two signals that the `{` is unclosed (not a legitimate multi-statement block):
            //
            //   (a) first_expr is already a HashLiteral — this `{` is a hash opener;
            //       a legitimate block's first statement would never be a bare HashLiteral.
            //
            //   (b) inner errors occurred during parse_expression() (e.g. an unclosed `[`
            //       triggered `expect_closing_delimiter` recovery) AND peek is `;` — the
            //       semicolon without a preceding `}` confirms the `{` is unclosed.
            //
            // In both cases, use expect_closing_delimiter to record the missing `}` error
            // and return immediately, leaving `;` and subsequent tokens for the statement
            // parser to handle so they appear as top-level AST nodes.
            let had_inner_errors = self.errors.len() > errors_before;
            let unclosed_hash =
                matches!(first_expr.kind, NodeKind::HashLiteral { .. });
            let unclosed_after_inner_error =
                had_inner_errors && self.peek_kind() == Some(TokenKind::Semicolon);

            if unclosed_hash || unclosed_after_inner_error {
                self.expect_closing_delimiter(TokenKind::RightBrace)?;
                let end = self.previous_position();
                self.exit_recursion();
                return Ok(Node::new(
                    NodeKind::Block { statements: vec![first_expr] },
                    SourceLocation { start, end },
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

            self.exit_recursion();
            Ok(Node::new(NodeKind::Block { statements }, SourceLocation { start, end }))
        }
    }

}
