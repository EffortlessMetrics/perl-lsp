impl<'a> Parser<'a> {
    /// Parse comma operator (lowest precedence except for word operators)
    fn parse_comma(&mut self) -> ParseResult<Node> {
        let mut expr = self.parse_assignment()?;
        expr = self.collect_comma_fat_arrow_continuation(expr)?;

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
                    // Parse the right side as a full expression starting with assignment.
                    // In Perl, comma has higher precedence than word operators, so
                    // '\ or \ = 1, 0' parses as '\ or ((\ = 1), 0)'.
                    // After parsing the first assignment, collect trailing comma / fat-arrow
                    // elements before building the word-operator node.
                    let mut right = self.parse_assignment()?;
                    // Apply any 'and' operators to the right side
                    right = self.parse_word_and_expr_with(right)?;
                    right = self.collect_comma_fat_arrow_continuation(right)?;

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

        // After processing all or/xor operators, apply any trailing 'and' operators.
        // This handles patterns like `($a or $b, $c and $d)` where comma collection
        // inside `or` absorbs `$c` but leaves `and $d` for this level.
        expr = self.parse_word_and_expr_with(expr)?;

        Ok(expr)
    }

    /// Parse word and expression with existing left side
    fn parse_word_and_expr_with(&mut self, mut expr: Node) -> ParseResult<Node> {
        while self.peek_kind() == Some(TokenKind::WordAnd) {
            let op_token = self.tokens.next()?;
            // Parse right side as a 'not' expression or assignment.
            // In Perl, comma has higher precedence than word operators, so
            // `$a and $x = 1, last` parses as `$a and ($x = 1, last)`.
            // After parsing the first assignment, collect trailing comma / fat-arrow
            // elements before building the word-operator node.
            let mut right = self.parse_word_not_expr()?;
            right = self.collect_comma_fat_arrow_continuation(right)?;

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
        if let Some(kind) = self.peek_kind() {
            if matches!(
                kind,
                TokenKind::WordNot | TokenKind::WordAnd | TokenKind::WordOr | TokenKind::WordXor
            ) && self.is_keyword_before_fat_arrow()
            {
                let token = self.tokens.next()?;
                return Ok(Node::new(
                    NodeKind::Identifier { name: token.text.to_string() },
                    SourceLocation { start: token.start, end: token.end },
                ));
            }

            // Check if we have a 'not' operator first
            if kind == TokenKind::WordNot {
                return self.parse_word_not_expr();
            }
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
                let op_token = self.tokens.next()?; // consume operator
                // The RHS can be a 'not' expression, or missing (recovery)
                let rhs =
                    if let Some(missing) = self.recover_missing_infix_rhs(op_token.start) {
                        missing
                    } else if self.peek_kind() == Some(TokenKind::WordNot) {
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
            // Fat-comma pair as then-branch: `$cond ? key => val : ...`
            // parse_assignment returns only the first element; collect any
            // trailing fat-arrow / comma continuation stopping before `:`.
            let then_expr = self.collect_fat_arrow_ternary_branch(then_expr)?;
            self.expect(TokenKind::Colon)?;
            let else_expr = self.parse_ternary()?;
            // Likewise for the else-branch.
            let else_expr = self.collect_fat_arrow_ternary_branch(else_expr)?;

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
            let then_expr = self.collect_fat_arrow_ternary_branch(then_expr)?;
            self.expect(TokenKind::Colon)?;
            let else_expr = self.parse_ternary()?;
            let else_expr = self.collect_fat_arrow_ternary_branch(else_expr)?;

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

    /// After parsing the first element of a ternary branch with
    /// `parse_assignment`, check whether a fat-arrow or comma follows.
    ///
    /// If so, collect the remaining elements into a list/hash, stopping at
    /// `Colon` (the ternary separator), `Semicolon`, closing delimiters, and
    /// statement-modifier keywords.  This handles patterns such as:
    ///   `$cond ? key => val : other => val2`
    ///
    /// If no fat-arrow or comma follows, the first element is returned as-is
    /// (fast path for ordinary ternary branches).
    ///
    /// IMPORTANT: do NOT call `parse_comma` here — it does not stop at `:`
    /// and would consume the ternary separator.
    fn collect_fat_arrow_ternary_branch(&mut self, first: Node) -> ParseResult<Node> {
        // Fast path: no fat-arrow or comma following — nothing to collect.
        if self.peek_kind() != Some(TokenKind::FatArrow)
            && self.peek_kind() != Some(TokenKind::Comma)
        {
            return Ok(first);
        }

        let mut elements: Vec<Node> = vec![first];
        let mut saw_fat_arrow = false;

        // Handle an immediate fat-arrow after the first element.
        if self.peek_kind() == Some(TokenKind::FatArrow) {
            saw_fat_arrow = true;
            // Auto-quote a bare identifier before =>
            if let NodeKind::Identifier { ref name } = elements[0].kind {
                let loc = elements[0].location;
                elements[0] = Node::new(
                    NodeKind::String { value: name.clone(), interpolated: false },
                    loc,
                );
            }
            self.tokens.next()?; // consume =>
            elements.push(self.parse_assignment()?);
        }

        // Continue collecting comma/fat-arrow separated elements until we see
        // a ternary colon or other terminator.
        loop {
            match self.peek_kind() {
                Some(TokenKind::Comma) | Some(TokenKind::FatArrow) => {}
                // Stop at ternary colon and all other terminators.
                Some(TokenKind::Colon)
                | Some(TokenKind::Semicolon)
                | Some(TokenKind::RightParen)
                | Some(TokenKind::RightBrace)
                | Some(TokenKind::RightBracket)
                | None => break,
                Some(k) if Self::is_stmt_modifier_kind(k) => break,
                _ => break,
            }

            let was_fat_arrow = self.peek_kind() == Some(TokenKind::FatArrow);
            self.consume_token()?; // consume , or =>

            if was_fat_arrow {
                saw_fat_arrow = true;
                // Auto-quote the last element (the key before =>)
                if let Some(last) = elements.last_mut() {
                    if let NodeKind::Identifier { ref name } = last.kind {
                        *last = Node::new(
                            NodeKind::String { value: name.clone(), interpolated: false },
                            last.location,
                        );
                    }
                }
            }

            // Stop before parsing the value if we hit a terminator.
            match self.peek_kind() {
                Some(TokenKind::Colon)
                | Some(TokenKind::Semicolon)
                | Some(TokenKind::RightParen)
                | Some(TokenKind::RightBrace)
                | Some(TokenKind::RightBracket)
                | None => break,
                Some(k) if Self::is_stmt_modifier_kind(k) => break,
                _ => {}
            }

            let mut elem = self.parse_assignment()?;

            // If fat-arrow follows this element, auto-quote it and consume =>
            if self.peek_kind() == Some(TokenKind::FatArrow) {
                saw_fat_arrow = true;
                if let NodeKind::Identifier { ref name } = elem.kind {
                    elem = Node::new(
                        NodeKind::String { value: name.clone(), interpolated: false },
                        elem.location,
                    );
                }
                self.tokens.next()?; // consume =>
                elements.push(elem);

                match self.peek_kind() {
                    Some(TokenKind::Colon)
                    | Some(TokenKind::Semicolon)
                    | Some(TokenKind::RightParen)
                    | Some(TokenKind::RightBrace)
                    | Some(TokenKind::RightBracket)
                    | None => break,
                    Some(k) if Self::is_stmt_modifier_kind(k) => break,
                    _ => elements.push(self.parse_assignment()?),
                }
            } else {
                elements.push(elem);
            }
        }

        let start = elements[0].location.start;
        let end = elements
            .last()
            .ok_or_else(|| ParseError::syntax("Empty ternary branch list", start))?
            .location
            .end;
        Ok(Self::build_list_or_hash(elements, saw_fat_arrow, start, end))
    }

    /// Apply all binary operators below assignment precedence to an already-parsed
    /// left-hand-side node.
    ///
    /// This is used when a variable declaration (`my`/`our`/`local`/`state`) has
    /// no `=` initializer but is followed by a binary operator in expression context:
    ///
    ///   `(our $CAN_HAZ_XS && $ok)`   — `&&` after the declaration
    ///   `(our $AUTOLOAD =~ /pattern/)` — `=~` after the declaration
    ///   `(my $x || "default")`        — `||` after the declaration
    ///
    /// The chain applies operators from highest to lowest precedence (shift → add →
    /// mul → relational → equality → bitwise-and → range → bitwise-xor →
    /// bitwise-or → logical-and → logical-or → ternary) so that the declaration
    /// node is correctly used as the leftmost operand of whatever operator follows.
    ///
    /// Assignment operators (`=`, `+=`, …) are NOT applied here — they are handled
    /// by `parse_declaration_arg` itself (via the `Some(TokenKind::Assign)` branch).
    fn parse_below_assignment_with(&mut self, expr: Node) -> ParseResult<Node> {
        let expr = self.parse_multiplicative_with(expr)?;
        let expr = self.parse_additive_with(expr)?;
        let expr = self.parse_shift_with(expr)?;
        let expr = self.parse_relational_with(expr)?;
        let expr = self.parse_equality_with(expr)?;
        let expr = self.parse_bitwise_and_with(expr)?;
        let expr = self.parse_range_with(expr)?;
        let expr = self.parse_bitwise_xor_with(expr)?;
        let expr = self.parse_bitwise_or_with(expr)?;
        let expr = self.parse_and_with(expr)?;
        let expr = self.parse_or_with(expr)?;
        self.parse_ternary_with(expr)
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
            let right = if let Some(missing) = self.recover_missing_infix_rhs(op_token.start) {
                missing
            } else {
                self.parse_and()?
            };
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
            let right = if let Some(missing) = self.recover_missing_infix_rhs(op_token.start) {
                missing
            } else {
                self.parse_bitwise_or()?
            };
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
            let right = if let Some(missing) = self.recover_missing_infix_rhs(op_token.start) {
                missing
            } else {
                self.parse_bitwise_xor()?
            };
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
            let right = if let Some(missing) = self.recover_missing_infix_rhs(op_token.start) {
                missing
            } else {
                self.parse_bitwise_and()?
            };
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
            let right = if let Some(missing) = self.recover_missing_infix_rhs(op_token.start) {
                missing
            } else {
                self.parse_equality()?
            };
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
            let right = if let Some(missing) = self.recover_missing_infix_rhs(op_token.start) {
                missing
            } else {
                self.parse_range()?
            };
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
                        let right = if let Some(missing) =
                            self.recover_missing_infix_rhs(op_token.start)
                        {
                            missing
                        } else {
                            self.parse_relational()?
                        };
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
                    let right = if let Some(missing) =
                        self.recover_missing_infix_rhs(op_token.start)
                    {
                        missing
                    } else {
                        self.parse_relational()?
                    };
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
                    let right = if let Some(missing) =
                        self.recover_missing_infix_rhs(op_token.start)
                    {
                        missing
                    } else {
                        self.parse_shift()?
                    };
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
                        let op_token = self.tokens.next()?;
                        let right = if let Some(missing) =
                            self.recover_missing_infix_rhs(op_token.start)
                        {
                            missing
                        } else {
                            self.parse_shift()?
                        };
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
                    let right = if let Some(missing) =
                        self.recover_missing_infix_rhs(op_token.start)
                    {
                        missing
                    } else {
                        self.parse_additive()?
                    };
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
                    let right = if let Some(missing) =
                        self.recover_missing_infix_rhs(op_token.start)
                    {
                        missing
                    } else {
                        self.parse_multiplicative()?
                    };
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
                    // Use parse_power() so that `a * b**c` parses as `a * (b**c)`.
                    // Exponentiation binds more tightly than multiplication in Perl.
                    let right = if let Some(missing) =
                        self.recover_missing_infix_rhs(op_token.start)
                    {
                        missing
                    } else {
                        self.parse_power()?
                    };
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
                    let peeked_text = peeked.text.as_ref();
                    // Handle fused `x<digits>` token (e.g. `("")x4`): the lexer joins
                    // the `x` repetition operator with a following digit run into one
                    // Identifier token when there is no whitespace between them.
                    // Split them here: synthesize the operator and number nodes directly.
                    let fused_x_digits = peeked_text.len() > 1
                        && peeked_text.starts_with('x')
                        && peeked_text[1..].chars().all(|c| c.is_ascii_digit());
                    if fused_x_digits {
                        let op_token = self.tokens.next()?;
                        let num_str = op_token.text[1..].to_string();
                        let num_start = op_token.start + 1;
                        let num_end = op_token.end;
                        let right = Node::new(
                            NodeKind::Number { value: num_str },
                            SourceLocation { start: num_start, end: num_end },
                        );
                        let start = expr.location.start;
                        let end = right.location.end;
                        expr = Node::new(
                            NodeKind::Binary {
                                op: "x".to_string(),
                                left: Box::new(expr),
                                right: Box::new(right),
                            },
                            SourceLocation { start, end },
                        );
                        continue;
                    }
                    if peeked_text != "x" {
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
                            | TokenKind::QuoteDouble
                            | TokenKind::Not
                            | TokenKind::Minus
                            | TokenKind::Plus
                            | TokenKind::Backslash
                            | TokenKind::BitwiseNot => true,
                            TokenKind::Identifier => {
                                let t = next.text.as_ref();
                                // Sigil-prefixed pseudo-identifiers count as operand starts
                                if t.starts_with('$') || t.starts_with('@') || t.starts_with('%') {
                                    true
                                } else {
                                    // A plain identifier (e.g. an imported function like `width`)
                                    // can also be the start of the x-repetition RHS, provided it
                                    // is not a binary operator keyword (or, and, not, eq, ne, …).
                                    // This allows `'-' x width $n` inside parentheses.
                                    !matches!(
                                        t,
                                        "or" | "and" | "not" | "xor"
                                            | "eq" | "ne" | "lt" | "le" | "gt" | "ge"
                                            | "cmp" | "x"
                                    )
                                }
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
                    // Use parse_power() so that `a x b**c` parses as `a x (b**c)`.
                    // Exponentiation binds more tightly than repetition in Perl.
                    let right = self.parse_power()?;
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
            let right = if let Some(missing) = self.recover_missing_infix_rhs(op_token.start) {
                missing
            } else {
                self.parse_unary()?
            };
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
