impl<'a> Parser<'a> {
    /// Parse block specifically for builtin functions (map, grep, sort)
    /// These always parse {} as blocks, never as hashes
    fn parse_builtin_block(&mut self) -> ParseResult<Node> {
        self.with_recursion_guard(|s| {
            let start_token = s.tokens.next()?; // consume {
            let start = start_token.start;

            // Parse the expression inside the block (if any)
            let mut statements = Vec::new();
            if s.peek_kind() != Some(TokenKind::RightBrace) {
                statements.push(s.parse_expression()?);
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
        let start_token = self.tokens.next()?; // consume {
        let start = start_token.start;

        // Peek ahead to determine if it's a hash or block
        // For empty {}, decide based on context
        if self.peek_kind() == Some(TokenKind::RightBrace) {
            self.tokens.next()?; // consume }
            let end = self.previous_position();

            // For empty braces, default to hash (correct for most functions)
            // Functions like sort/map/grep have special handling that creates blocks
            return Ok(Node::new(
                NodeKind::HashLiteral { pairs: Vec::new() },
                SourceLocation { start, end },
            ));
        }

        // For non-empty braces, we need to check if it contains hash-like content
        // Save position to potentially backtrack
        let _saved_pos = self.current_position();

        // Try to parse as expression (which might be hash contents)
        let first_expr = match self.parse_expression() {
            Ok(expr) => expr,
            Err(e) => {
                // Propagate RecursionLimit immediately - don't try alternative parse
                if matches!(e, ParseError::RecursionLimit) {
                    return Err(e);
                }
                // If we can't parse an expression, parse as block statements
                let mut statements = Vec::new();
                while self.peek_kind() != Some(TokenKind::RightBrace) && !self.tokens.is_eof() {
                    statements.push(self.parse_statement()?);
                }

                self.expect(TokenKind::RightBrace)?;
                let end = self.previous_position();

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
                        SourceLocation { start, end },
                    ));
                }
            }

            // If the expression is already a HashLiteral, return it directly
            // This happens when parse_comma creates a HashLiteral from key => value pairs
            if matches!(first_expr.kind, NodeKind::HashLiteral { .. }) {
                return Ok(first_expr);
            }

            // Otherwise it's a block with a single expression
            return Ok(Node::new(
                NodeKind::Block { statements: vec![first_expr] },
                SourceLocation { start, end },
            ));
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

            if self.peek_kind() != /* ~ changed by cargo-mutants ~ */ Some(TokenKind::FatArrow) {
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

            Ok(Node::new(NodeKind::HashLiteral { pairs }, SourceLocation { start, end }))
        } else {
            // Not a hash - parse as block
            if self.peek_kind() == Some(TokenKind::RightBrace) {
                // Single expression block
                self.tokens.next()?; // consume }
                let end = self.previous_position();

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

            Ok(Node::new(NodeKind::Block { statements }, SourceLocation { start, end }))
        }
    }

}
