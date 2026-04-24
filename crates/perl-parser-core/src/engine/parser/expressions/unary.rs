/// Returns `true` when `kind` is a token that can immediately follow a
/// single-punctuation typeglob such as `*<`, `*>`, `*(`, or `*)`.
///
/// This lets the parser distinguish the typeglob form (e.g. `*REAL_USER_ID = *<;`)
/// from a dereference expression (e.g. `*<EXPR>`), without consuming the
/// punctuation character first.
fn is_typeglob_punct_terminator(kind: Option<TokenKind>) -> bool {
    matches!(
        kind,
        Some(
            TokenKind::Semicolon
                | TokenKind::Comma
                | TokenKind::RightParen
                | TokenKind::RightBrace
                | TokenKind::RightBracket
                | TokenKind::Eof
        ) | None
    )
}

impl<'a> Parser<'a> {
    fn is_contextual_await_start(&mut self) -> bool {
        if self.peek_kind() != Some(TokenKind::Identifier)
            || !self.tokens.peek().is_ok_and(|token| token.text.as_ref() == "await")
        {
            return false;
        }

        let second_kind = self.tokens.peek_second().ok().map(|token| token.kind);
        if matches!(
            second_kind,
            Some(TokenKind::FatArrow | TokenKind::Arrow | TokenKind::DoubleColon)
        ) {
            return false;
        }

        if second_kind == Some(TokenKind::Colon)
            && self.tokens.peek_third().is_ok_and(|token| token.kind == TokenKind::Colon)
        {
            return false;
        }

        true
    }

    /// Parse unary expression
    fn parse_unary(&mut self) -> ParseResult<Node> {
        if self.peek_kind() == Some(TokenKind::Slash) {
            self.tokens.relex_as_term();
        }

        if self.is_contextual_await_start() {
            let op_token = self.tokens.next()?;
            let start = op_token.start;

            if self.tokens.is_eof() || self.is_at_statement_end() {
                let end = op_token.end;
                return Ok(Node::new(
                    NodeKind::Unary {
                        op: op_token.text.to_string(),
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
                NodeKind::Unary { op: op_token.text.to_string(), operand: Box::new(operand) },
                SourceLocation { start, end },
            ));
        }

        if let Some(kind) = self.peek_kind() {
            match kind {
                TokenKind::Minus => {
                    let op_token = self.tokens.next()?;
                    let start = op_token.start;

                    // Check for file test operators (-e, -f, -d, etc.)
                    if let Some(TokenKind::Identifier) = self.peek_kind() {
                        let next_token = self.tokens.peek()?;
                        if next_token.text.len() == 1 {
                            // Before a fat arrow, `-G`, `-r`, etc. are hash/list keys, not
                            // file-test operators.
                            if self
                                .tokens
                                .peek_second()
                                .is_ok_and(|token| token.kind == TokenKind::FatArrow)
                            {
                                let test_token = self.tokens.next()?;
                                let end = test_token.end;
                                return Ok(Node::new(
                                    NodeKind::Identifier {
                                        name: format!("-{}", test_token.text),
                                    },
                                    SourceLocation { start, end },
                                ));
                            }

                            // It's a file test operator
                            let test_token = self.tokens.next()?;
                            let file_test = format!("-{}", test_token.text);

                            // File test can be used without operand (tests $_).
                            // Treat a following comma as end-of-expression so that
                            // `grep -e, @INC` and `grep !/pat/ && -d, @list` parse
                            // correctly: the comma is the EXPR/LIST separator for
                            // grep/map, not an argument to the file-test operator.
                            let operand = if self.is_at_statement_end()
                                || self.peek_kind() == Some(TokenKind::Comma)
                                || matches!(
                                    self.peek_kind(),
                                    Some(
                                        TokenKind::And
                                            | TokenKind::Or
                                            | TokenKind::DefinedOr
                                            | TokenKind::Greater
                                            | TokenKind::Less
                                            | TokenKind::GreaterEqual
                                            | TokenKind::LessEqual
                                            | TokenKind::Equal
                                            | TokenKind::NotEqual
                                    )
                                )
                            {
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
                        NodeKind::Unary { op: op_token.text.to_string(), operand: Box::new(operand) },
                        SourceLocation { start, end },
                    ));
                }
                TokenKind::Plus => {
                    let op_token = self.tokens.next()?;
                    let start = op_token.start;

                    // Special case: +{ ... } forces a hash constructor (not a block)
                    // If followed by ->, allow postfix chaining: +{@_}->{key}
                    if self.peek_kind() == Some(TokenKind::LeftBrace) {
                        // Parse as hash literal
                        let hash = self.parse_hash_or_block()?;
                        let end = hash.location.end;

                        // Wrap the hash in a unary plus to preserve the explicit disambiguation
                        let node = Node::new(
                            NodeKind::Unary { op: op_token.text.to_string(), operand: Box::new(hash) },
                            SourceLocation { start, end },
                        );
                        return self.parse_postfix_chain(node);
                    }

                    // Check if we're at EOF or a terminator (for standalone operators)
                    if self.tokens.is_eof() || self.is_at_statement_end() {
                        // Create a placeholder for standalone operator
                        let end = op_token.end;
                        return Ok(Node::new(
                            NodeKind::Unary {
                                op: op_token.text.to_string(),
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
                        NodeKind::Unary { op: op_token.text.to_string(), operand: Box::new(operand) },
                        SourceLocation { start, end },
                    ));
                }
                // Handle 'not' keyword as a unary prefix at expression level.
                // This lets `$a && not $b` parse correctly.
                TokenKind::WordNot => {
                    let op_token = self.tokens.next()?;
                    let start = op_token.start;

                    if self.tokens.is_eof() || self.is_at_statement_end() {
                        let end = op_token.end;
                        return Ok(Node::new(
                            NodeKind::Unary {
                                op: op_token.text.to_string(),
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
                        NodeKind::Unary {
                            op: op_token.text.to_string(),
                            operand: Box::new(operand),
                        },
                        SourceLocation { start, end },
                    ));
                }
                TokenKind::Not | TokenKind::Backslash | TokenKind::BitwiseNot | TokenKind::Star => {
                    let op_token = self.tokens.next()?;
                    let start = op_token.start;

                    // AC1: Disambiguate typeglob (*foo) from multiplication (*)
                    // If TokenKind is Star and it is followed by an identifier or {
                    if op_token.kind == TokenKind::Star {
                        if let Some(next_kind) = self.peek_kind() {
                            match next_kind {
                                kind if Self::can_be_sub_name(kind) => {
                                    let id_token = self.tokens.next()?;
                                    let end = id_token.end;
                                    let node = Node::new(
                                        NodeKind::Typeglob { name: id_token.text.to_string() },
                                        SourceLocation { start, end },
                                    );
                                    // Allow postfix chaining: *$self->{key}
                                    return self.parse_postfix_chain(node);
                                }
                                TokenKind::LeftBrace => {
                                    // Dynamic typeglob *{$name}
                                    let brace_expr = self.parse_postfix()?; // This will handle { ... }
                                    let end = brace_expr.location.end;
                                    return Ok(Node::new(
                                        NodeKind::Unary { op: "*".to_string(), operand: Box::new(brace_expr) },
                                        SourceLocation { start, end },
                                    ));
                                }
                                TokenKind::BitwiseXor => {
                                    // *^X typeglob for control variable $^X (e.g. *^N, *^W, *^F)
                                    self.consume_token()?; // consume ^
                                    if let Some(TokenKind::Identifier) = self.peek_kind() {
                                        let id_token = self.tokens.next()?;
                                        let name = format!("^{}", id_token.text);
                                        let end = id_token.end;
                                        return Ok(Node::new(
                                            NodeKind::Typeglob { name },
                                            SourceLocation { start, end },
                                        ));
                                    }
                                    // Standalone *^ — fall through to parse operand
                                }
                                // Typeglobs for Perl's process-identity punctuation variables.
                                // English.pm aliases: *REAL_USER_ID = *<; *EFFECTIVE_USER_ID = *>;
                                //                    *REAL_GROUP_ID = *(; *EFFECTIVE_GROUP_ID = *);
                                // Use 2-token lookahead so *<EXPR> and *(EXPR) still fall through
                                // when a real sub-expression follows the punctuation character.
                                TokenKind::Less => {
                                    let second_kind =
                                        self.tokens.peek_second().ok().map(|t| t.kind);
                                    if is_typeglob_punct_terminator(second_kind) {
                                        let t = self.tokens.next()?;
                                        return Ok(Node::new(
                                            NodeKind::Typeglob { name: "<".to_string() },
                                            SourceLocation { start, end: t.end },
                                        ));
                                    }
                                }
                                TokenKind::Greater => {
                                    let second_kind =
                                        self.tokens.peek_second().ok().map(|t| t.kind);
                                    if is_typeglob_punct_terminator(second_kind) {
                                        let t = self.tokens.next()?;
                                        return Ok(Node::new(
                                            NodeKind::Typeglob { name: ">".to_string() },
                                            SourceLocation { start, end: t.end },
                                        ));
                                    }
                                }
                                TokenKind::LeftParen => {
                                    let second_kind =
                                        self.tokens.peek_second().ok().map(|t| t.kind);
                                    if is_typeglob_punct_terminator(second_kind) {
                                        let t = self.tokens.next()?;
                                        return Ok(Node::new(
                                            NodeKind::Typeglob { name: "(".to_string() },
                                            SourceLocation { start, end: t.end },
                                        ));
                                    }
                                }
                                TokenKind::RightParen => {
                                    // *) is always a typeglob for $) (effective GID).
                                    // RightParen cannot start a valid sub-expression in this
                                    // context, so no lookahead disambiguation is needed.
                                    let t = self.tokens.next()?;
                                    return Ok(Node::new(
                                        NodeKind::Typeglob { name: ")".to_string() },
                                        SourceLocation { start, end: t.end },
                                    ));
                                }
                                // *? = typeglob for $? (child process status).
                                // Question cannot start an expression after *, so no lookahead.
                                TokenKind::Question => {
                                    let t = self.tokens.next()?;
                                    return Ok(Node::new(
                                        NodeKind::Typeglob { name: "?".to_string() },
                                        SourceLocation { start, end: t.end },
                                    ));
                                }
                                // *, = typeglob for $, (output field separator).
                                // Comma cannot start an expression after *, so no lookahead.
                                TokenKind::Comma => {
                                    let t = self.tokens.next()?;
                                    return Ok(Node::new(
                                        NodeKind::Typeglob { name: ",".to_string() },
                                        SourceLocation { start, end: t.end },
                                    ));
                                }
                                // *= — the lexer emits StarAssign for the compound assignment
                                // operator, so Star followed by bare Assign is always the typeglob
                                // *=  (for $= "format lines per page").  No lookahead needed.
                                TokenKind::Assign => {
                                    let t = self.tokens.next()?;
                                    return Ok(Node::new(
                                        NodeKind::Typeglob { name: "=".to_string() },
                                        SourceLocation { start, end: t.end },
                                    ));
                                }
                                // */ = typeglob for $/ (input record separator).
                                // Use lookahead: if followed by a statement terminator, it's a
                                // typeglob; otherwise fall through (could be multiply + regex).
                                TokenKind::Slash => {
                                    let second_kind =
                                        self.tokens.peek_second().ok().map(|t| t.kind);
                                    if is_typeglob_punct_terminator(second_kind) {
                                        let t = self.tokens.next()?;
                                        return Ok(Node::new(
                                            NodeKind::Typeglob { name: "/".to_string() },
                                            SourceLocation { start, end: t.end },
                                        ));
                                    }
                                }
                                // *. = typeglob for $. (input line number).
                                // Use lookahead: if followed by a statement terminator, it's a
                                // typeglob; otherwise fall through (could be multiply + concat).
                                TokenKind::Dot => {
                                    let second_kind =
                                        self.tokens.peek_second().ok().map(|t| t.kind);
                                    if is_typeglob_punct_terminator(second_kind) {
                                        let t = self.tokens.next()?;
                                        return Ok(Node::new(
                                            NodeKind::Typeglob { name: ".".to_string() },
                                            SourceLocation { start, end: t.end },
                                        ));
                                    }
                                }
                                // *| = typeglob for $| (output autoflush flag).
                                // Use lookahead: if followed by a statement terminator, it's a
                                // typeglob; otherwise fall through (could be multiply + bitwise-or).
                                TokenKind::BitwiseOr => {
                                    let second_kind =
                                        self.tokens.peek_second().ok().map(|t| t.kind);
                                    if is_typeglob_punct_terminator(second_kind) {
                                        let t = self.tokens.next()?;
                                        return Ok(Node::new(
                                            NodeKind::Typeglob { name: "|".to_string() },
                                            SourceLocation { start, end: t.end },
                                        ));
                                    }
                                }
                                // *: = typeglob for $: (format line-break characters).
                                // Use lookahead: if followed by a statement terminator, it's a
                                // typeglob; otherwise fall through (could be label or ternary colon).
                                TokenKind::Colon => {
                                    let second_kind =
                                        self.tokens.peek_second().ok().map(|t| t.kind);
                                    if is_typeglob_punct_terminator(second_kind) {
                                        let t = self.tokens.next()?;
                                        return Ok(Node::new(
                                            NodeKind::Typeglob { name: ":".to_string() },
                                            SourceLocation { start, end: t.end },
                                        ));
                                    }
                                }
                                _ => {}
                            }
                        }
                    }

                    // Check if we're at EOF or a terminator (for standalone operators)
                    if self.tokens.is_eof() || self.is_at_statement_end() {
                        // Create a placeholder for standalone operator
                        let end = op_token.end;
                        return Ok(Node::new(
                            NodeKind::Unary {
                                op: op_token.text.to_string(),
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

                    let node = Node::new(
                        NodeKind::Unary { op: op_token.text.to_string(), operand: Box::new(operand) },
                        SourceLocation { start, end },
                    );

                    // For typeglob (*), allow postfix chaining: *$self->{field}
                    if op_token.kind == TokenKind::Star {
                        return self.parse_postfix_chain(node);
                    }

                    return Ok(node);
                }
                TokenKind::Increment | TokenKind::Decrement => {
                    // Pre-increment and pre-decrement
                    let op_token = self.tokens.next()?;
                    let start = op_token.start;
                    let operand = self.parse_unary()?;
                    let end = operand.location.end;

                    return Ok(Node::new(
                        NodeKind::Unary { op: op_token.text.to_string(), operand: Box::new(operand) },
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
                                op: op_token.text.to_string(),
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
                        NodeKind::Unary { op: op_token.text.to_string(), operand: Box::new(operand) },
                        SourceLocation { start, end },
                    ));
                }
                _ => {}
            }
        }

        self.parse_postfix()
    }

}
