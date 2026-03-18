impl<'a> Parser<'a> {
    /// Parse comma operator (lowest precedence except for word operators)
    fn parse_comma(&mut self) -> ParseResult<Node> {
        let mut expr = self.parse_assignment()?;

        // In scalar context, comma creates a list
        // For now, we'll just parse it as sequential expressions
        // Also handle fat arrow (=>) which acts like comma
        if self.peek_kind() == Some(TokenKind::Comma)
            || self.peek_kind() == Some(TokenKind::FatArrow)
        {
            let mut expressions = vec![expr];
            let mut saw_fat_comma = false;

            // Handle initial fat arrow — auto-quote the key if it is a bare identifier
            if self.peek_kind() == Some(TokenKind::FatArrow) {
                saw_fat_comma = true;
                // Auto-quote bare identifiers before =>
                let last_idx = expressions.len() - 1;
                if let NodeKind::Identifier { ref name } = expressions[last_idx].kind {
                    let loc = expressions[last_idx].location;
                    expressions[last_idx] = Node::new(
                        NodeKind::String { value: name.clone(), interpolated: false },
                        loc,
                    );
                }
                self.tokens.next()?; // consume =>
                expressions.push(self.parse_assignment()?);
            }

            while self.peek_kind() == Some(TokenKind::Comma)
                || self.peek_kind() == Some(TokenKind::FatArrow)
            {
                if self.peek_kind() == Some(TokenKind::Comma) {
                    self.consume_token()?; // consume comma
                }

                // Check for end of expression (includes statement modifier
                // keywords so that `$a, $b if $cond` does not try to parse the
                // modifier as another comma element).
                match self.peek_kind() {
                    Some(TokenKind::Semicolon)
                    | Some(TokenKind::RightParen)
                    | Some(TokenKind::RightBrace)
                    | Some(TokenKind::RightBracket) => break,
                    Some(k) if Self::is_stmt_modifier_kind(k) => break,
                    _ => {}
                }

                let mut elem = self.parse_assignment()?;

                // Check for fat arrow after element — auto-quote bare identifiers
                if self.peek_kind() == Some(TokenKind::FatArrow) {
                    saw_fat_comma = true;
                    if let NodeKind::Identifier { ref name } = elem.kind {
                        elem = Node::new(
                            NodeKind::String { value: name.clone(), interpolated: false },
                            elem.location,
                        );
                    }
                    self.tokens.next()?; // consume =>
                    expressions.push(elem);

                    // Check again for end of expression
                    match self.peek_kind() {
                        Some(TokenKind::Semicolon)
                        | Some(TokenKind::RightParen)
                        | Some(TokenKind::RightBrace)
                        | Some(TokenKind::RightBracket) => break,
                        Some(k) if Self::is_stmt_modifier_kind(k) => break,
                        _ => expressions.push(self.parse_assignment()?),
                    }
                } else {
                    expressions.push(elem);
                }
            }

            // Convert to hash literal if we saw fat comma and have even number of elements
            let start = expressions[0].location.start;
            let end = expressions
                .last()
                .ok_or_else(|| ParseError::syntax("Empty expression list", start))?
                .location
                .end;
            expr = Self::build_list_or_hash(expressions, saw_fat_comma, start, end);
        }

        // Now handle word operators (or, xor, and, not) which have the lowest precedence
        expr = self.parse_word_or_expr(expr)?;

        Ok(expr)
    }

    /// Parse word or expression (or, xor) - takes an existing expr and applies word operators
    fn parse_word_or_expr(&mut self, mut expr: Node) -> ParseResult<Node> {
        // First handle 'and' which has higher precedence than 'or'/'xor'
        expr = self.parse_word_and_expr_with(expr)?;

        // Then handle 'or' and 'xor' which have lowest precedence
        while let Some(kind) = self.peek_kind() {
            match kind {
                TokenKind::WordOr | TokenKind::WordXor => {
                    let op_token = self.tokens.next()?;
                    // Parse the right side as a full expression starting with assignment
                    let right = self.parse_assignment()?;
                    // Apply any 'and' operators to the right side
                    let right = self.parse_word_and_expr_with(right)?;

                    let start = expr.location.start;
                    let end = right.location.end;

                    expr = Node::new(
                        NodeKind::Binary {
                            op: op_token.text.to_string(),
                            left: Box::new(expr),
                            right: Box::new(right),
                        },
                        SourceLocation { start, end },
                    );
                }
                _ => break,
            }
        }

        Ok(expr)
    }

    /// Parse word and expression with existing left side
    fn parse_word_and_expr_with(&mut self, mut expr: Node) -> ParseResult<Node> {
        while self.peek_kind() == Some(TokenKind::WordAnd) {
            let op_token = self.tokens.next()?;
            // Parse right side as a 'not' expression or assignment
            let right = self.parse_word_not_expr()?;
            let start = expr.location.start;
            let end = right.location.end;

            expr = Node::new(
                NodeKind::Binary {
                    op: op_token.text.to_string(),
                    left: Box::new(expr),
                    right: Box::new(right),
                },
                SourceLocation { start, end },
            );
        }

        Ok(expr)
    }

    /// Parse word not expression - handles 'not' operator
    fn parse_word_not_expr(&mut self) -> ParseResult<Node> {
        if self.peek_kind() == Some(TokenKind::WordNot) {
            let op_token = self.tokens.next()?;
            let start = op_token.start;
            let operand = self.parse_word_not_expr()?;
            let end = operand.location.end;

            return Ok(Node::new(
                NodeKind::Unary { op: op_token.text.to_string(), operand: Box::new(operand) },
                SourceLocation { start, end },
            ));
        }

        // The right side of a word operator should be a full expression
        self.parse_assignment()
    }

    /// Parse assignment expression
    fn parse_assignment(&mut self) -> ParseResult<Node> {
        // Check if we have a 'not' operator first
        if self.peek_kind() == Some(TokenKind::WordNot) {
            return self.parse_word_not_expr();
        }

        // Handle 'return' as an expression in expression context
        // This allows patterns like: open $fh, $file or return;
        // But NOT when followed by => (autoquoted hash key: return => $val)
        if self.peek_kind() == Some(TokenKind::Return)
            && !self.is_keyword_before_fat_arrow()
        {
            return self.parse_return_expr();
        }

        let mut expr = self.parse_ternary()?;

        // Check for assignment operators
        if let Some(kind) = self.peek_kind() {
            let op = match kind {
                TokenKind::Assign => Some("="),
                TokenKind::PlusAssign => Some("+="),
                TokenKind::MinusAssign => Some("-="),
                TokenKind::StarAssign => Some("*="),
                TokenKind::SlashAssign => Some("/="),
                TokenKind::PercentAssign => Some("%="),
                TokenKind::DotAssign => Some(".="),
                TokenKind::AndAssign => Some("&="),
                TokenKind::OrAssign => Some("|="),
                TokenKind::XorAssign => Some("^="),
                TokenKind::PowerAssign => Some("**="),
                TokenKind::LeftShiftAssign => Some("<<="),
                TokenKind::RightShiftAssign => Some(">>="),
                TokenKind::LogicalAndAssign => Some("&&="),
                TokenKind::LogicalOrAssign => Some("||="),
                TokenKind::DefinedOrAssign => Some("//="),
                _ => None,
            };

            if let Some(op) = op {
                self.tokens.next()?; // consume operator
                // The RHS can be a 'not' expression
                let rhs = if self.peek_kind() == Some(TokenKind::WordNot) {
                    self.parse_word_not_expr()?
                } else {
                    self.parse_assignment()?
                };
                let start = expr.location.start;
                let end = rhs.location.end;

                expr = Node::new(
                    NodeKind::Assignment {
                        lhs: Box::new(expr),
                        rhs: Box::new(rhs),
                        op: op.to_string(),
                    },
                    SourceLocation { start, end },
                );
            }
        }

        Ok(expr)
    }

    /// Parse ternary conditional expression
    /// Right-associative: `$a ? $b ? $c : $d : $e` parses as `$a ? ($b ? $c : $d) : $e`
    ///
    /// In Perl the colon `:` acts as a delimiter for the then-branch, so the
    /// expression between `?` and `:` may contain assignment operators
    /// (e.g. `$a ? $b = 1 : $c`).  The then-branch therefore uses
    /// `parse_assignment` which recurses into `parse_ternary` for nested
    /// conditionals.  The else-branch uses `parse_ternary` directly so that
    /// chained ternaries (`$a ? $b : $c ? $d : $e`) are right-associative
    /// without accidentally capturing a surrounding assignment.
    fn parse_ternary(&mut self) -> ParseResult<Node> {
        let mut expr = self.parse_or()?;

        if self.peek_kind() == Some(TokenKind::Question) {
            self.tokens.next()?; // consume ?
            // The then-branch (between ? and :) allows full assignment
            // expressions because : acts as a terminator.  parse_assignment
            // calls parse_ternary internally, so nested ternaries still work.
            let then_expr = self.parse_assignment()?;
            self.expect(TokenKind::Colon)?;
            let else_expr = self.parse_ternary()?;

            let start = expr.location.start;
            let end = else_expr.location.end;

            expr = Node::new(
                NodeKind::Ternary {
                    condition: Box::new(expr),
                    then_expr: Box::new(then_expr),
                    else_expr: Box::new(else_expr),
                },
                SourceLocation { start, end },
            );
        }

        Ok(expr)
    }

    /// Continue parsing ternary conditional when we already have the condition
    /// expression.  Used by the statement-level named-unary tail so that
    /// `defined $var ? then : else` is correctly wrapped in a Ternary node.
    fn parse_ternary_with(&mut self, mut expr: Node) -> ParseResult<Node> {
        if self.peek_kind() == Some(TokenKind::Question) {
            self.tokens.next()?; // consume ?
            let then_expr = self.parse_assignment()?;
            self.expect(TokenKind::Colon)?;
            let else_expr = self.parse_ternary()?;

            let start = expr.location.start;
            let end = else_expr.location.end;

            expr = Node::new(
                NodeKind::Ternary {
                    condition: Box::new(expr),
                    then_expr: Box::new(then_expr),
                    else_expr: Box::new(else_expr),
                },
                SourceLocation { start, end },
            );
        }

        Ok(expr)
    }

    /// Parse logical OR expression
    fn parse_or(&mut self) -> ParseResult<Node> {
        let expr = self.parse_and()?;
        self.parse_or_with(expr)
    }

    /// Parse logical AND expression
    fn parse_and(&mut self) -> ParseResult<Node> {
        let expr = self.parse_bitwise_or()?;
        self.parse_and_with(expr)
    }

    /// Parse bitwise OR expression
    fn parse_bitwise_or(&mut self) -> ParseResult<Node> {
        let expr = self.parse_bitwise_xor()?;
        self.parse_bitwise_or_with(expr)
    }

    /// Parse bitwise XOR expression
    fn parse_bitwise_xor(&mut self) -> ParseResult<Node> {
        let expr = self.parse_bitwise_and()?;
        self.parse_bitwise_xor_with(expr)
    }

    /// Parse range expression
    fn parse_range(&mut self) -> ParseResult<Node> {
        let expr = self.parse_equality()?;
        self.parse_range_with(expr)
    }

    /// Parse bitwise AND expression
    fn parse_bitwise_and(&mut self) -> ParseResult<Node> {
        let expr = self.parse_range()?;
        self.parse_bitwise_and_with(expr)
    }

    /// Parse equality expression
    fn parse_equality(&mut self) -> ParseResult<Node> {
        let expr = self.parse_relational()?;
        self.parse_equality_with(expr)
    }

    /// Parse relational expression
    fn parse_relational(&mut self) -> ParseResult<Node> {
        let expr = self.parse_shift()?;
        self.parse_relational_with(expr)
    }

    fn parse_or_with(&mut self, mut expr: Node) -> ParseResult<Node> {
        while Self::is_logical_or(self.peek_kind()) {
            let op_token = self.tokens.next()?;
            let right = self.parse_and()?;
            let start = expr.location.start;
            let end = right.location.end;

            expr = Node::new(
                NodeKind::Binary {
                    op: op_token.text.to_string(),
                    left: Box::new(expr),
                    right: Box::new(right),
                },
                SourceLocation { start, end },
            );
        }

        Ok(expr)
    }

    fn parse_and_with(&mut self, mut expr: Node) -> ParseResult<Node> {
        while self.peek_kind() == Some(TokenKind::And) {
            let op_token = self.tokens.next()?;
            let right = self.parse_bitwise_or()?;
            let start = expr.location.start;
            let end = right.location.end;

            expr = Node::new(
                NodeKind::Binary {
                    op: op_token.text.to_string(),
                    left: Box::new(expr),
                    right: Box::new(right),
                },
                SourceLocation { start, end },
            );
        }

        Ok(expr)
    }

    fn parse_bitwise_or_with(&mut self, mut expr: Node) -> ParseResult<Node> {
        while self.peek_kind() == Some(TokenKind::BitwiseOr) {
            let op_token = self.tokens.next()?;
            let right = self.parse_bitwise_xor()?;
            let start = expr.location.start;
            let end = right.location.end;

            expr = Node::new(
                NodeKind::Binary {
                    op: op_token.text.to_string(),
                    left: Box::new(expr),
                    right: Box::new(right),
                },
                SourceLocation { start, end },
            );
        }

        Ok(expr)
    }

    fn parse_bitwise_xor_with(&mut self, mut expr: Node) -> ParseResult<Node> {
        while self.peek_kind() == Some(TokenKind::BitwiseXor) {
            let op_token = self.tokens.next()?;
            let right = self.parse_bitwise_and()?;
            let start = expr.location.start;
            let end = right.location.end;

            expr = Node::new(
                NodeKind::Binary {
                    op: op_token.text.to_string(),
                    left: Box::new(expr),
                    right: Box::new(right),
                },
                SourceLocation { start, end },
            );
        }

        Ok(expr)
    }

    fn parse_range_with(&mut self, mut expr: Node) -> ParseResult<Node> {
        while self.peek_kind() == Some(TokenKind::Range) {
            let op_token = self.tokens.next()?;
            let right = self.parse_equality()?;
            let start = expr.location.start;
            let end = right.location.end;

            expr = Node::new(
                NodeKind::Binary {
                    op: op_token.text.to_string(),
                    left: Box::new(expr),
                    right: Box::new(right),
                },
                SourceLocation { start, end },
            );
        }

        Ok(expr)
    }

    fn parse_bitwise_and_with(&mut self, mut expr: Node) -> ParseResult<Node> {
        while self.peek_kind() == Some(TokenKind::BitwiseAnd) {
            let op_token = self.tokens.next()?;
            let right = self.parse_range()?;
            let start = expr.location.start;
            let end = right.location.end;

            expr = Node::new(
                NodeKind::Binary {
                    op: op_token.text.to_string(),
                    left: Box::new(expr),
                    right: Box::new(right),
                },
                SourceLocation { start, end },
            );
        }

        Ok(expr)
    }

    fn parse_equality_with(&mut self, mut expr: Node) -> ParseResult<Node> {
        while let Some(kind) = self.peek_kind() {
            match kind {
                TokenKind::Identifier => {
                    let next_text = self.tokens.peek()?.text.as_ref();
                    if matches!(next_text, "eq" | "ne" | "lt" | "le" | "gt" | "ge" | "cmp") {
                        let op_token = self.tokens.next()?;
                        let right = self.parse_relational()?;
                        let start = expr.location.start;
                        let end = right.location.end;

                        expr = Node::new(
                            NodeKind::Binary {
                                op: op_token.text.to_string(),
                                left: Box::new(expr),
                                right: Box::new(right),
                            },
                            SourceLocation { start, end },
                        );
                    } else {
                        break;
                    }
                }
                TokenKind::Equal
                | TokenKind::NotEqual
                | TokenKind::Match
                | TokenKind::NotMatch
                | TokenKind::SmartMatch => {
                    let op_token = self.tokens.next()?;
                    let right = self.parse_relational()?;
                    let start = expr.location.start;
                    let end = right.location.end;

                    if matches!(op_token.kind, TokenKind::Match | TokenKind::NotMatch) {
                        if let NodeKind::Substitution { pattern, replacement, modifiers, has_embedded_code, .. } =
                            &right.kind
                        {
                            let negated = matches!(op_token.kind, TokenKind::NotMatch);
                            expr = Node::new(
                                NodeKind::Substitution {
                                    expr: Box::new(expr),
                                    pattern: pattern.clone(),
                                    replacement: replacement.clone(),
                                    modifiers: modifiers.clone(),
                                    has_embedded_code: *has_embedded_code,
                                    negated,
                                },
                                SourceLocation { start, end },
                            );
                        } else if let NodeKind::Transliteration {
                            search, replace, modifiers, ..
                        } = &right.kind
                        {
                            let negated = matches!(op_token.kind, TokenKind::NotMatch);
                            expr = Node::new(
                                NodeKind::Transliteration {
                                    expr: Box::new(expr),
                                    search: search.clone(),
                                    replace: replace.clone(),
                                    modifiers: modifiers.clone(),
                                    negated,
                                },
                                SourceLocation { start, end },
                            );
                        } else if let NodeKind::Regex { pattern, replacement, modifiers, has_embedded_code } =
                            &right.kind
                        {
                            let negated = matches!(op_token.kind, TokenKind::NotMatch);
                            if let Some(replacement) = replacement {
                                let pat = if pattern.len() >= 2 {
                                    pattern[1..pattern.len() - 1].to_string()
                                } else {
                                    pattern.clone()
                                };
                                expr = Node::new(
                                    NodeKind::Substitution {
                                        expr: Box::new(expr),
                                        pattern: pat,
                                        replacement: replacement.clone(),
                                        modifiers: modifiers.clone(),
                                        has_embedded_code: *has_embedded_code,
                                        negated,
                                    },
                                    SourceLocation { start, end },
                                );
                            } else {
                                expr = Node::new(
                                    NodeKind::Match {
                                        expr: Box::new(expr),
                                        pattern: pattern.clone(),
                                        modifiers: modifiers.clone(),
                                        has_embedded_code: *has_embedded_code,
                                        negated,
                                    },
                                    SourceLocation { start, end },
                                );
                            }
                        } else {
                            expr = Node::new(
                                NodeKind::Binary {
                                    op: op_token.text.to_string(),
                                    left: Box::new(expr),
                                    right: Box::new(right),
                                },
                                SourceLocation { start, end },
                            );
                        }
                    } else {
                        expr = Node::new(
                            NodeKind::Binary {
                                op: op_token.text.to_string(),
                                left: Box::new(expr),
                                right: Box::new(right),
                            },
                            SourceLocation { start, end },
                        );
                    }
                }
                _ => break,
            }
        }

        Ok(expr)
    }

    fn parse_relational_with(&mut self, mut expr: Node) -> ParseResult<Node> {
        while let Some(kind) = self.peek_kind() {
            match kind {
                TokenKind::Less
                | TokenKind::Greater
                | TokenKind::LessEqual
                | TokenKind::GreaterEqual
                | TokenKind::Spaceship
                | TokenKind::StringCompare => {
                    let op_token = self.tokens.next()?;
                    let right = self.parse_shift()?;
                    let start = expr.location.start;
                    let end = right.location.end;

                    expr = Node::new(
                        NodeKind::Binary {
                            op: op_token.text.to_string(),
                            left: Box::new(expr),
                            right: Box::new(right),
                        },
                        SourceLocation { start, end },
                    );
                }
                TokenKind::Identifier => {
                    if self.tokens.peek()?.text.as_ref() == "ISA" {
                        let _op_token = self.tokens.next()?;
                        let right = self.parse_shift()?;
                        let start = expr.location.start;
                        let end = right.location.end;

                        expr = Node::new(
                            NodeKind::Binary {
                                op: "ISA".to_string(),
                                left: Box::new(expr),
                                right: Box::new(right),
                            },
                            SourceLocation { start, end },
                        );
                    } else {
                        break;
                    }
                }
                _ => break,
            }
        }

        Ok(expr)
    }

    /// Parse shift expression
    fn parse_shift(&mut self) -> ParseResult<Node> {
        let expr = self.parse_additive()?;
        self.parse_shift_with(expr)
    }

    /// Parse additive expression
    fn parse_additive(&mut self) -> ParseResult<Node> {
        let expr = self.parse_multiplicative()?;
        self.parse_additive_with(expr)
    }

    /// Parse multiplicative expression (including the `x` string repetition operator)
    fn parse_multiplicative(&mut self) -> ParseResult<Node> {
        let expr = self.parse_power()?;
        self.parse_multiplicative_with(expr)
    }

    /// Parse power expression
    fn parse_power(&mut self) -> ParseResult<Node> {
        let expr = self.parse_unary()?;
        self.parse_power_with(expr)
    }

    fn parse_shift_with(&mut self, mut expr: Node) -> ParseResult<Node> {
        while let Some(kind) = self.peek_kind() {
            match kind {
                TokenKind::LeftShift | TokenKind::RightShift => {
                    let op_token = self.tokens.next()?;
                    let right = self.parse_additive()?;
                    let start = expr.location.start;
                    let end = right.location.end;

                    expr = Node::new(
                        NodeKind::Binary {
                            op: op_token.text.to_string(),
                            left: Box::new(expr),
                            right: Box::new(right),
                        },
                        SourceLocation { start, end },
                    );
                }
                _ => break,
            }
        }

        Ok(expr)
    }

    fn parse_additive_with(&mut self, mut expr: Node) -> ParseResult<Node> {
        while let Some(kind) = self.peek_kind() {
            match kind {
                TokenKind::Plus | TokenKind::Minus | TokenKind::Dot => {
                    let op_token = self.tokens.next()?;
                    let right = self.parse_multiplicative()?;
                    let start = expr.location.start;
                    let end = right.location.end;

                    expr = Node::new(
                        NodeKind::Binary {
                            op: op_token.text.to_string(),
                            left: Box::new(expr),
                            right: Box::new(right),
                        },
                        SourceLocation { start, end },
                    );
                }
                _ => break,
            }
        }

        Ok(expr)
    }

    fn parse_multiplicative_with(&mut self, mut expr: Node) -> ParseResult<Node> {
        while let Some(kind) = self.peek_kind() {
            match kind {
                TokenKind::Star | TokenKind::Slash | TokenKind::Percent => {
                    let op_token = self.tokens.next()?;
                    let right = self.parse_unary()?;
                    let start = expr.location.start;
                    let end = right.location.end;

                    expr = Node::new(
                        NodeKind::Binary {
                            op: op_token.text.to_string(),
                            left: Box::new(expr),
                            right: Box::new(right),
                        },
                        SourceLocation { start, end },
                    );
                }
                TokenKind::Identifier => {
                    let peeked = self.tokens.peek()?;
                    if peeked.text.as_ref() != "x" {
                        break;
                    }
                    let is_operand_start = if let Ok(next) = self.tokens.peek_second() {
                        match next.kind {
                            TokenKind::Number
                            | TokenKind::ScalarSigil
                            | TokenKind::ArraySigil
                            | TokenKind::HashSigil
                            | TokenKind::LeftParen
                            | TokenKind::LeftBracket
                            | TokenKind::String
                            | TokenKind::QuoteSingle
                            | TokenKind::QuoteDouble => true,
                            TokenKind::Identifier => {
                                let t = next.text.as_ref();
                                t.starts_with('$')
                                    || t.starts_with('@')
                                    || t.starts_with('%')
                            }
                            _ => false,
                        }
                    } else {
                        false
                    };
                    if !is_operand_start {
                        break;
                    }
                    let op_token = self.tokens.next()?;
                    let right = self.parse_unary()?;
                    let start = expr.location.start;
                    let end = right.location.end;

                    expr = Node::new(
                        NodeKind::Binary {
                            op: op_token.text.to_string(),
                            left: Box::new(expr),
                            right: Box::new(right),
                        },
                        SourceLocation { start, end },
                    );
                }
                _ => break,
            }
        }

        Ok(expr)
    }

    fn parse_power_with(&mut self, mut expr: Node) -> ParseResult<Node> {
        while self.peek_kind() == Some(TokenKind::Power) {
            let op_token = self.tokens.next()?;
            let right = self.parse_unary()?;
            let start = expr.location.start;
            let end = right.location.end;

            expr = Node::new(
                NodeKind::Binary {
                    op: op_token.text.to_string(),
                    left: Box::new(expr),
                    right: Box::new(right),
                },
                SourceLocation { start, end },
            );
        }

        Ok(expr)
    }

}
