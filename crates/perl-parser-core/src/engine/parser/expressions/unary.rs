impl<'a> Parser<'a> {
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
                                    NodeKind::Variable {
                                        sigil: "$".to_string(),
                                        name: "_".to_string(),
                                    },
                                    SourceLocation { start: test_token.end, end: test_token.end },
                                )
                            } else {
                                self.parse_unary()?
                            };

                            let end = operand.location.end;
                            return Ok(Node::new(
                                NodeKind::Unary { op: file_test, operand: Box::new(operand) },
                                SourceLocation { start, end },
                            ));
                        }
                    }

                    // Regular unary minus
                    let operand = self.parse_unary()?;
                    let end = operand.location.end;

                    return Ok(Node::new(
                        NodeKind::Unary { op: op_token.text, operand: Box::new(operand) },
                        SourceLocation { start, end },
                    ));
                }
                TokenKind::Plus => {
                    let op_token = self.tokens.next()?;
                    let start = op_token.start;

                    // Special case: +{ ... } forces a hash constructor (not a block)
                    if self.peek_kind() == Some(TokenKind::LeftBrace) {
                        // Parse as hash literal
                        let hash = self.parse_hash_or_block()?;
                        let end = hash.location.end;

                        // Wrap the hash in a unary plus to preserve the explicit disambiguation
                        return Ok(Node::new(
                            NodeKind::Unary { op: op_token.text, operand: Box::new(hash) },
                            SourceLocation { start, end },
                        ));
                    }

                    // Check if we're at EOF or a terminator (for standalone operators)
                    if self.tokens.is_eof() || self.is_at_statement_end() {
                        // Create a placeholder for standalone operator
                        let end = op_token.end;
                        return Ok(Node::new(
                            NodeKind::Unary {
                                op: op_token.text,
                                operand: Box::new(Node::new(
                                    NodeKind::Undef,
                                    SourceLocation { start: end, end },
                                )),
                            },
                            SourceLocation { start, end },
                        ));
                    }

                    let operand = self.parse_unary()?;
                    let end = operand.location.end;

                    return Ok(Node::new(
                        NodeKind::Unary { op: op_token.text, operand: Box::new(operand) },
                        SourceLocation { start, end },
                    ));
                }
                TokenKind::Not | TokenKind::Backslash | TokenKind::BitwiseNot | TokenKind::Star => {
                    let op_token = self.tokens.next()?;
                    let start = op_token.start;

                    // Check if we're at EOF or a terminator (for standalone operators)
                    if self.tokens.is_eof() || self.is_at_statement_end() {
                        // Create a placeholder for standalone operator
                        let end = op_token.end;
                        return Ok(Node::new(
                            NodeKind::Unary {
                                op: op_token.text,
                                operand: Box::new(Node::new(
                                    NodeKind::Undef,
                                    SourceLocation { start: end, end },
                                )),
                            },
                            SourceLocation { start, end },
                        ));
                    }

                    let operand = self.parse_unary()?;
                    let end = operand.location.end;

                    return Ok(Node::new(
                        NodeKind::Unary { op: op_token.text, operand: Box::new(operand) },
                        SourceLocation { start, end },
                    ));
                }
                TokenKind::Increment | TokenKind::Decrement => {
                    // Pre-increment and pre-decrement
                    let op_token = self.tokens.next()?;
                    let start = op_token.start;
                    let operand = self.parse_unary()?;
                    let end = operand.location.end;

                    return Ok(Node::new(
                        NodeKind::Unary { op: op_token.text, operand: Box::new(operand) },
                        SourceLocation { start, end },
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
                                op: op_token.text,
                                operand: Box::new(Node::new(
                                    NodeKind::Undef,
                                    SourceLocation { start: end, end },
                                )),
                            },
                            SourceLocation { start, end },
                        ));
                    }

                    let operand = self.parse_unary()?;
                    let end = operand.location.end;

                    return Ok(Node::new(
                        NodeKind::Unary { op: op_token.text, operand: Box::new(operand) },
                        SourceLocation { start, end },
                    ));
                }
                _ => {}
            }
        }

        self.parse_postfix()
    }

}
