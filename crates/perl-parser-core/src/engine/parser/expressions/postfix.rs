impl<'a> Parser<'a> {
    /// Parse postfix expression
    fn parse_postfix(&mut self) -> ParseResult<Node> {
        let mut expr = self.parse_primary()?;

        loop {
            match self.peek_kind() {
                Some(k) if Self::is_postfix_op(Some(k)) => {
                    let op_token = self.consume_token()?;
                    let start = expr.location.start;
                    let end = op_token.end;

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

                                expr = Node::new(
                                    NodeKind::Unary {
                                        op: "->**".to_string(),
                                        operand: Box::new(expr),
                                    },
                                    SourceLocation { start, end },
                                );
                            }
                        }

                        Some(TokenKind::Identifier | TokenKind::Method) => {
                            // Method call
                            let method = self.tokens.next()?.text.to_string();

                            let args = if self.peek_kind() == Some(TokenKind::LeftParen) {
                                self.parse_args()?
                            } else {
                                Vec::new()
                            };

                            let start = expr.location.start;
                            let end = self.previous_position();

                            expr = Node::new(
                                NodeKind::MethodCall { object: Box::new(expr), method, args },
                                SourceLocation { start, end },
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
                        self.consume_token()?; // consume comma
                        indices.push(self.parse_expression()?);
                    }

                    self.expect(TokenKind::RightBracket)?;

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
                                    SourceLocation { start: block_start, end: block_end },
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
                            } else if matches!(name.as_str(), "sort" | "map" | "grep") {
                                // These builtins should parse {} as blocks, not hashes
                                args.push(self.parse_builtin_block()?);
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
                            //
                            // For nullary builtins like shift, pop, caller, wantarray, etc.,
                            // when followed by a binary operator, they should be treated as
                            // having no arguments (e.g., "shift || 2" means shift() || 2)
                            let is_nullary_without_args = Self::is_nullary_builtin(name)
                                && self.peek_kind().is_some_and(Self::is_binary_operator);

                            if self.is_at_statement_end() || is_nullary_without_args {
                                // Bare builtin with no arguments
                                expr = Node::new(
                                    NodeKind::FunctionCall { name: name.clone(), args: vec![] },
                                    expr.location,
                                );
                            } else {
                                // Parse arguments without parentheses
                                let mut args = Vec::new();

                                // Special handling for sort, map, grep with block first argument
                                if matches!(name.as_str(), "sort" | "map" | "grep")
                                    && self.peek_kind() == Some(TokenKind::LeftBrace)
                                {
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
                                        SourceLocation { start: block_start, end: block_end },
                                    );

                                    args.push(block);

                                    // Parse remaining arguments for map/grep/sort without requiring commas
                                    // But respect statement boundaries including ] and )
                                    while !self.is_at_statement_end() {
                                        // Skip comma if present
                                        if self.peek_kind() == Some(TokenKind::Comma) {
                                            self.consume_token()?;
                                        }
                                        // Check again after potential comma
                                        if self.is_at_statement_end() {
                                            break;
                                        }
                                        args.push(self.parse_ternary()?);
                                    }
                                } else if name == "bless"
                                    && self.peek_kind() == Some(TokenKind::LeftBrace)
                                {
                                    // Special handling for bless {} - parse it as a hash
                                    args.push(self.parse_hash_or_block()?);

                                    // Parse remaining arguments separated by commas
                                    while self.peek_kind() == Some(TokenKind::Comma) {
                                        self.consume_token()?; // consume comma
                                        if self.is_at_statement_end() {
                                            break;
                                        }
                                        args.push(self.parse_assignment()?);
                                    }
                                } else {
                                    // Parse the first argument
                                    args.push(self.parse_ternary()?);

                                    // Parse remaining arguments separated by commas
                                    while self.peek_kind() == Some(TokenKind::Comma) {
                                        self.consume_token()?; // consume comma
                                        if self.is_at_statement_end() {
                                            break;
                                        }
                                        args.push(self.parse_ternary()?);
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
                | Some(TokenKind::Eof)
                | None
        )
    }

}
