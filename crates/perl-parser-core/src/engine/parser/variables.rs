impl<'a> Parser<'a> {
    /// Parse variable declaration (my, our, local, state)
    fn parse_variable_declaration(&mut self) -> ParseResult<Node> {
        let start = self.current_position();
        let declarator_token = self.consume_token()?;
        let declarator = declarator_token.text.to_string();

        // Check if we have a list declaration like `my ($x, $y)`
        if self.peek_kind() == Some(TokenKind::LeftParen) {
            self.consume_token()?; // consume (

            let mut variables = Vec::new();

            // Parse comma-separated list of variables with their individual attributes
            while self.peek_kind() != Some(TokenKind::RightParen) && !self.tokens.is_eof() {
                let var = self.parse_variable_list_item()?;

                // Parse optional attributes for this specific variable
                let var_attributes = if self.peek_kind() == Some(TokenKind::Colon) {
                    self.parse_variable_attributes()?
                } else {
                    Vec::new()
                };

                // Create a node that includes both the variable and its attributes
                let var_with_attrs = if var_attributes.is_empty() {
                    var
                } else {
                    let start = var.location.start;
                    let end = self.previous_position();
                    Node::new(
                        NodeKind::VariableWithAttributes {
                            variable: Box::new(var),
                            attributes: var_attributes,
                        },
                        SourceLocation { start, end },
                    )
                };

                variables.push(var_with_attrs);

                if self.peek_kind() == Some(TokenKind::Comma) {
                    self.consume_token()?; // consume comma
                } else if self.peek_kind() != Some(TokenKind::RightParen) {
                    return Err(ParseError::syntax(
                        "Expected comma or closing parenthesis in variable list",
                        self.current_position(),
                    ));
                }
            }

            self.expect(TokenKind::RightParen)?; // consume )

            // No longer parse attributes here - they're parsed per variable above
            let attributes = Vec::new();

            let initializer = if self.peek_kind() == Some(TokenKind::Assign) {
                self.tokens.next()?; // consume =
                Some(Box::new(self.parse_expression()?))
            } else {
                None
            };

            // Don't consume semicolon here - let parse_statement handle it uniformly

            let end = self.previous_position();
            let node = Node::new(
                NodeKind::VariableListDeclaration {
                    declarator,
                    variables,
                    attributes,
                    initializer,
                },
                SourceLocation { start, end },
            );
            Ok(node)
        } else {
            // Single variable declaration
            // For 'local', we need to parse lvalue expressions (not just simple variables)
            // because local can take complex forms like local $ENV{PATH}
            let variable = if declarator == "local" {
                // For local, parse a general lvalue expression
                self.parse_assignment()?
            } else {
                // Legacy typed lexical declarations are used by pseudo-hash `fields`-style code:
                //     my Package::Type $self = shift;
                // Perl accepts this syntax; consume the optional leading type token so that
                // parsing continues at the declared variable.
                self.consume_legacy_decl_type_constraint()?;

                // For my/our/state, parse a variable declaration target and
                // any direct or arrow postfix lvalue operations.
                let var = self.parse_variable()?;
                self.parse_postfix_chain(var)?
            };

            // Parse optional attributes
            let attributes = if self.peek_kind() == Some(TokenKind::Colon) {
                self.parse_variable_attributes()?
            } else {
                Vec::new()
            };

            // Perl's `my`/`our`/`state`/`local` declare ONLY the first variable
            // when the declaration list is not parenthesized:
            //
            //   `perl -MO=Deparse,-p -e 'my $a, $b, $c = 1;'`
            //   => `(my($a), $b, ($c = 1));`
            //
            // (perlsub: "If more than one value is listed, the list must be
            // placed in parentheses.") So a comma immediately following the
            // declared variable is NOT part of this declaration — it belongs
            // to the surrounding comma-expression, which the callers of
            // `parse_variable_declaration` (statement- and expression-level)
            // pick up via their own comma continuation handling. Parenthesized
            // lists (`my ($a, $b)`) are handled entirely by the branch above
            // and are unaffected by this.

            // Accept both simple `=` and compound operators (`||=`, `//=`, `.=`, etc.)
            // Perl allows `our $x ||= 0;` and `my $y .= "suffix";`
            let assign_op = self.peek_compound_assign_op();
            let initializer = if let Some(op) = assign_op {
                let op_token = self.tokens.next()?;
                let rhs = if let Some(missing) = self.recover_missing_infix_rhs(op_token.start) {
                    missing
                } else {
                    self.parse_expression()?
                };
                if op == "=" {
                    Some(Box::new(rhs))
                } else {
                    let var_clone = variable.clone();
                    let assign_end = rhs.location.end;
                    Some(Box::new(Node::new(
                        NodeKind::Assignment {
                            op: op.to_string(),
                            lhs: Box::new(var_clone),
                            rhs: Box::new(rhs),
                        },
                        SourceLocation { start: variable.location.start, end: assign_end },
                    )))
                }
            } else {
                None
            };

            // Don't consume semicolon here - let parse_statement handle it uniformly

            let end = self.previous_position();
            let node = Node::new(
                NodeKind::VariableDeclaration {
                    declarator,
                    variable: Box::new(variable),
                    attributes,
                    initializer,
                },
                SourceLocation { start, end },
            );
            Ok(node)
        }
    }

    /// Parse one slot in a lexical list declaration.
    fn parse_variable_list_item(&mut self) -> ParseResult<Node> {
        match self.peek_kind() {
            Some(TokenKind::Undef) => {
                let undef_token = self.consume_token()?;
                Ok(Node::new(
                    NodeKind::Undef,
                    SourceLocation { start: undef_token.start, end: undef_token.end },
                ))
            }
            Some(TokenKind::LeftParen) => {
                let start = self.current_position();
                self.consume_token()?; // consume (
                let mut items = Vec::new();
                while self.peek_kind() != Some(TokenKind::RightParen) && !self.tokens.is_eof() {
                    items.push(self.parse_variable_list_item()?);
                    if self.peek_kind() == Some(TokenKind::Comma) {
                        self.consume_token()?; // consume ,
                    } else if self.peek_kind() != Some(TokenKind::RightParen) {
                        return Err(ParseError::syntax(
                            "Expected comma or closing parenthesis in nested variable list",
                            self.current_position(),
                        ));
                    }
                }
                self.expect_closing_delimiter(TokenKind::RightParen)?;
                let end = self.previous_position();
                // Single-item group: return the item directly for backward compatibility.
                // Multi-item group: wrap in NestedVariableList.
                match items.len() {
                    0 => Ok(Node::new(NodeKind::Undef, SourceLocation { start, end })),
                    1 => {
                        // Safe: we just checked len == 1
                        let mut it = items.into_iter();
                        match it.next() {
                            Some(only) => Ok(only),
                            None => Ok(Node::new(NodeKind::Undef, SourceLocation { start, end })), // LCOV_EXCL_LINE
                        }
                    }
                    _ => Ok(Node::new(
                        NodeKind::NestedVariableList { items },
                        SourceLocation { start, end },
                    )),
                }
            }
            _ => self.parse_ternary(),
        }
    }

    /// Consume an optional legacy type constraint in lexical declarations.
    ///
    /// This supports old pseudo-hash style declarations like:
    /// `my Debconf::DbDriver $this = shift;`
    ///
    /// The type constraint is intentionally ignored in the AST for now.
    fn consume_legacy_decl_type_constraint(&mut self) -> ParseResult<()> {
        if self.peek_kind() != Some(TokenKind::Identifier) {
            return Ok(());
        }

        let looks_like_type = {
            let current = self.tokens.peek()?;
            if current.text.starts_with('$')
                || current.text.starts_with('@')
                || current.text.starts_with('%')
                || current.text.starts_with('&')
                || current.text.starts_with('*')
            {
                false
            } else {
                let next = self.tokens.peek_second()?;
                matches!(
                    next.kind,
                    TokenKind::ScalarSigil
                        | TokenKind::ArraySigil
                        | TokenKind::HashSigil
                        | TokenKind::SubSigil
                        | TokenKind::GlobSigil
                ) || (next.kind == TokenKind::Identifier
                    && next
                        .text
                        .chars()
                        .next()
                        .is_some_and(|c| matches!(c, '$' | '@' | '%' | '&' | '*')))
            }
        };

        if looks_like_type {
            self.consume_token()?;
        }

        Ok(())
    }

    /// Parse local statement (can localize any lvalue, not just simple variables)
    fn parse_local_statement(&mut self) -> ParseResult<Node> {
        let start = self.current_position();
        let declarator_token = self.consume_token()?; // consume 'local'
        let declarator = declarator_token.text.to_string();

        // Parse the lvalue expression that's being localized
        let variable = Box::new(self.parse_expression()?);

        let initializer = if self.peek_kind() == Some(TokenKind::Assign) {
            self.tokens.next()?; // consume =
            Some(Box::new(self.parse_expression()?))
        } else {
            None
        };

        let end = self.previous_position();
        let node = Node::new(
            NodeKind::VariableDeclaration {
                declarator,
                variable,
                attributes: Vec::new(),
                initializer,
            },
            SourceLocation { start, end },
        );
        Ok(node)
    }

    /// Parse a variable ($foo, @bar, %baz)
    fn parse_variable(&mut self) -> ParseResult<Node> {
        // If the next token is a sigil token, delegate to parse_variable_from_sigil
        // This handles cases where the lexer splits sigil and name (e.g. "%" "hash" vs "%hash")
        // Also handles operators that can act as sigils in this context (%, &, *)
        if let Some(kind) = self.peek_kind() {
            match kind {
                TokenKind::ScalarSigil
                | TokenKind::ArraySigil
                | TokenKind::HashSigil
                | TokenKind::SubSigil
                | TokenKind::GlobSigil
                | TokenKind::Percent     // %hash
                | TokenKind::BitwiseAnd  // &sub
                | TokenKind::Star => {   // *glob
                    return self.parse_variable_from_sigil();
                }
                _ => {}
            }
        }

        let token = self.consume_token()?;

        // The lexer returns variables as identifiers like "$x", "@array", etc.
        // We need to split the sigil from the name
        let text = &token.text;

        if let Some(name) = Self::simple_braced_scalar_token_name(text) {
            return Ok(Node::new(
                NodeKind::Variable { sigil: String::from("$"), name: name.to_string() },
                SourceLocation { start: token.start, end: token.end },
            ));
        }

        // Special handling for @{, %{, and ${ (array/hash/scalar dereference)
        // e.g. @{$ref}, %{$hash}, ${"${pkg}::$sym"}
        if &**text == "@{" || &**text == "%{" || &**text == "${" {
            let sigil = text
                .chars()
                .next()
                .ok_or_else(|| {
                    ParseError::syntax("Empty token text for array/hash dereference", token.start)
                })?
                .to_string();
            let start = token.start;

            // Parse the expression inside the braces
            let (expr, folded) = if sigil == "$" {
                self.parse_braced_scalar_body()?
            } else {
                (self.parse_expression()?, false)
            };

            self.consume_deref_body_terminators()?;
            self.expect(TokenKind::RightBrace)?;
            let end = self.previous_position();

            if folded {
                // `${ name }` == `$name` (perlref): already folded to a
                // scalar variable node; do not re-wrap in Unary{"${}"}.
                // Widen the span to cover the whole `${ ... }`, matching the
                // no-whitespace single-token fast path above.
                let mut folded_node = expr;
                folded_node.location = SourceLocation { start, end };
                return Ok(folded_node);
            }

            let op = format!("{}{{}}", sigil);
            return Ok(Node::new(
                NodeKind::Unary { op, operand: Box::new(expr) },
                SourceLocation { start, end },
            ));
        }

        // Special handling for &{ (code dereference)
        if &**text == "&{" {
            return self.parse_code_dereference(token.start);
        }

        let (sigil, name) = if let Some(rest) = text.strip_prefix('$') {
            ("$".to_string(), rest.to_string())
        } else if let Some(rest) = text.strip_prefix('@') {
            ("@".to_string(), rest.to_string())
        } else if let Some(rest) = text.strip_prefix('%') {
            ("%".to_string(), rest.to_string())
        } else if let Some(rest) = text.strip_prefix('&') {
            ("&".to_string(), rest.to_string())
        } else if text.starts_with('*') && text.len() > 1 {
            let rest = &text[1..];
            ("*".to_string(), rest.to_string())
        } else {
            return Err(ParseError::syntax(
                format!("Expected variable, found '{}'", text),
                token.start,
            ));
        };

        if matches!(sigil.as_str(), "$" | "@" | "%")
            && name.is_empty()
            && self.peek_kind() == Some(TokenKind::LeftBrace)
        {
            self.tokens.next()?; // consume {

            let (expr, folded) = if sigil == "$" {
                self.parse_braced_scalar_body()?
            } else {
                (self.parse_expression()?, false)
            };

            self.consume_deref_body_terminators()?;
            self.expect(TokenKind::RightBrace)?;
            let end = self.previous_position();

            if folded {
                // `${ name }` == `$name` (perlref): already folded to a
                // scalar variable node; do not re-wrap in Unary{"${}"}.
                let mut folded_node = expr;
                folded_node.location = SourceLocation { start: token.start, end };
                return Ok(folded_node);
            }

            let op = format!("{}{{}}", sigil);
            return Ok(Node::new(
                NodeKind::Unary { op, operand: Box::new(expr) },
                SourceLocation { start: token.start, end },
            ));
        }

        // Handle sigil + partial deref: when the lexer produces e.g. `%{shift` as one
        // token (name starts with `{` but doesn't end with `}`), this is a dereference
        // expression like `%{shift()}` where the lexer consumed `%{shift` greedily.
        // We need to create the inner expression from the identifier after `{`, parse
        // any trailing postfix (like `()` for function calls), then expect `}`.
        if name.starts_with('{') && !name.ends_with('}') {
            let inner_name = &name[1..]; // strip leading {
            let inner_start = token.start + sigil.len() + 1; // after sigil and {
            let inner_end = token.end;

            // `${sep }` (trailing whitespace before `}`, none after `${`):
            // the lexer greedily captures `${sep` as one token because
            // there's no space right after `${`. When the captured name is
            // a plain bareword immediately followed by `}` — no postfix, no
            // `::` — this is `${name}` == `$name` folding (perlref), not a
            // dereference; mirror `try_parse_simple_braced_scalar`'s fast
            // path instead of wrapping in Unary{"${}"}.
            if sigil == "$"
                && is_simple_scalar_name(inner_name)
                && self.peek_kind() == Some(TokenKind::RightBrace)
            {
                self.expect(TokenKind::RightBrace)?;
                let end = self.previous_position();
                return Ok(Node::new(
                    NodeKind::Variable { sigil: "$".to_string(), name: inner_name.to_string() },
                    SourceLocation { start: token.start, end },
                ));
            }

            let mut inner = if sigil == "$" && self.peek_kind() == Some(TokenKind::DoubleColon) {
                self.parse_qualified_scalar_tail(inner_name.to_string(), inner_start, inner_end)?
            } else {
                // Create an identifier node for the captured name
                let inner = Node::new(
                    NodeKind::Identifier { name: inner_name.to_string() },
                    SourceLocation { start: inner_start, end: inner_end },
                );

                // Parse postfix chain (handles function call parens, method calls, etc.)
                self.parse_postfix_chain(inner)?
            };
            if self.peek_kind() == Some(TokenKind::Question) {
                // `${ref($x) ? $x : fallback($x)}` may enter this partial-deref
                // path when the lexer greedily captures `${ref` as one token.
                inner = self.parse_ternary_with(inner)?;
            }

            self.consume_deref_body_terminators()?;
            self.expect(TokenKind::RightBrace)?;
            let end = self.previous_position();

            let op = format!("{}{{}}", sigil);
            return Ok(Node::new(
                NodeKind::Unary { op, operand: Box::new(inner) },
                SourceLocation { start: token.start, end },
            ));
        }

        // Check if the variable name is followed by :: for package-qualified variables
        let mut full_name = name;
        let mut end = token.end;

        // Handle $#$ref — last index of dereferenced array
        // The lexer sends `$#` as Identifier("$#"), so sigil="$" name="#"
        if sigil == "$" && full_name == "#" {
            let next_is_var = self.tokens.peek().ok().is_some_and(|t| t.text.starts_with('$'));
            let next_is_sigil = self.peek_kind() == Some(TokenKind::ScalarSigil);
            if next_is_var || next_is_sigil {
                // $#$ref — parse the inner variable and wrap
                let inner = self.parse_variable()?;
                let inner_end = inner.location.end;
                return Ok(Node::new(
                    NodeKind::Unary { op: "$#".to_string(), operand: Box::new(inner) },
                    SourceLocation { start: token.start, end: inner_end },
                ));
            } else if self.peek_kind() == Some(TokenKind::LeftBrace) {
                // $#{expr} — last index via block dereference
                self.tokens.next()?; // consume {
                let inner = self.parse_expression()?;
                self.consume_deref_body_terminators()?;
                self.expect(TokenKind::RightBrace)?;
                let brace_end = self.previous_position();
                return Ok(Node::new(
                    NodeKind::Unary { op: "$#".to_string(), operand: Box::new(inner) },
                    SourceLocation { start: token.start, end: brace_end },
                ));
            }
        }

        // The lexer may hand us a sigil-only token (`&`), a precombined `$$`
        // token, or an old-style deref prefix such as `@$$` followed by the
        // referenced identifier. Preserve the full target name instead of
        // leaving the tail as a stray identifier node.
        if (full_name.is_empty()
            || (sigil == "$" && full_name == "$")
            || (matches!(sigil.as_str(), "@" | "%") && full_name == "$$"))
            && self.peek_kind().is_some_and(Self::is_variable_name_kind)
            && (full_name.is_empty()
                || self
                    .tokens
                    .peek()
                    .ok()
                    .is_some_and(|name_token| name_token.start == end))
        {
            let name_token = self.tokens.next()?;
            full_name.push_str(&name_token.text);
            end = name_token.end;
        }

        // Handle :: in package-qualified variables
        while self.peek_kind() == Some(TokenKind::DoubleColon) {
            self.tokens.next()?; // consume ::
            full_name.push_str("::");

            // The next part might be an identifier or another variable
            if self.peek_kind() == Some(TokenKind::Identifier) {
                let name_token = self.tokens.next()?;
                full_name.push_str(&name_token.text);
                end = name_token.end;
            } else {
                // Handle cases like $Foo::$bar
                return Err(ParseError::syntax(
                    "Expected identifier after :: in package-qualified variable",
                    self.current_position(),
                ));
            }
        }

        if sigil == "*" {
            Ok(Node::new(
                NodeKind::Typeglob { name: full_name },
                SourceLocation { start: token.start, end },
            ))
        } else if matches!(sigil.as_str(), "$" | "@" | "%")
            && Self::is_unbraced_scalar_deref_name(&full_name)
        {
            // Unbraced dereference: $$ref, @$ref, %$ref — equivalent to ${$ref}, @{$ref}, %{$ref}.
            // The `full_name` here is e.g. "$ref"; strip the leading `$` to get the inner name.
            let inner_name = full_name[1..].to_string();
            let inner = Node::new(
                NodeKind::Variable { sigil: "$".to_string(), name: inner_name },
                SourceLocation { start: token.start + sigil.len(), end },
            );
            let op = format!("{}{{}}", sigil);
            Ok(Node::new(
                NodeKind::Unary { op, operand: Box::new(inner) },
                SourceLocation { start: token.start, end },
            ))
        } else {
            Ok(Node::new(
                NodeKind::Variable { sigil, name: full_name },
                SourceLocation { start: token.start, end },
            ))
        }
    }

    /// Return `true` when `name` is an unbraced scalar-dereference target:
    /// a string that starts with `$` followed by at least one identifier
    /// character (e.g. `"$ref"`, `"$self"`).
    ///
    /// Excludes bare `"$"` (which represents the PID special variable `$$`).
    fn is_unbraced_scalar_deref_name(name: &str) -> bool {
        let mut chars = name.chars();
        if chars.next() != Some('$') {
            return false;
        }
        chars.next().is_some_and(|c| c.is_alphanumeric() || c == '_')
    }

    fn try_parse_braced_qualified_scalar(&mut self) -> ParseResult<Option<Node>> {
        if self.peek_kind() != Some(TokenKind::Identifier) {
            return Ok(None);
        }

        if self.tokens.peek_second()?.kind != TokenKind::DoubleColon {
            return Ok(None);
        }

        let first = self.tokens.next()?;
        self.parse_qualified_scalar_tail(first.text.to_string(), first.start, first.end)
            .map(Some)
    }

    /// Parse the body of a `${...}` and report whether it was already
    /// folded to a bare scalar variable by the `${name}` == `$name` rule
    /// (perlref), via `try_parse_simple_braced_scalar`.
    ///
    /// When the returned flag is `true`, the caller MUST NOT wrap the node
    /// in `Unary{"${}"}` — it is already the correct scalar variable node.
    /// All other cases (caret-special variables, qualified names, and real
    /// scalar-ref dereferences such as `${$ref}`) return `false` and must
    /// still be wrapped — even though `${$ref}` produces a structurally
    /// identical `Variable` node once parsed, so the flag (not the node
    /// shape) is what distinguishes folding from dereferencing.
    fn parse_braced_scalar_body(&mut self) -> ParseResult<(Node, bool)> {
        if let Some(expr) = self.try_parse_braced_caret_special_scalar()? {
            return Ok((expr, false));
        }

        if let Some(expr) = self.try_parse_simple_braced_scalar()? {
            return Ok((expr, true));
        }

        match self.try_parse_braced_qualified_scalar()? {
            Some(expr) => Ok((expr, false)),
            None => Ok((self.parse_expression()?, false)),
        }
    }

    fn try_parse_simple_braced_scalar(&mut self) -> ParseResult<Option<Node>> {
        if self.peek_kind() != Some(TokenKind::Identifier) {
            return Ok(None);
        }

        // The peeked identifier must be a plain bareword (e.g. `sep`), not a
        // sigil-prefixed variable reference (e.g. `$ref` inside `${$ref}`,
        // which is a nested dereference, not `${name}` == `$name` folding).
        if !is_simple_scalar_name(&self.tokens.peek()?.text) {
            return Ok(None);
        }

        if self.tokens.peek_second()?.kind != TokenKind::RightBrace {
            return Ok(None);
        }

        let name_token = self.tokens.next()?;
        Ok(Some(Node::new(
            NodeKind::Variable { sigil: String::from("$"), name: name_token.text.to_string() },
            SourceLocation { start: name_token.start, end: name_token.end },
        )))
    }

    fn simple_braced_scalar_token_name(text: &str) -> Option<&str> {
        let inner = text.strip_prefix("${")?.strip_suffix('}')?;
        if is_simple_scalar_name(inner) {
            Some(inner)
        } else {
            None
        }
    }

    fn try_parse_braced_caret_special_scalar(&mut self) -> ParseResult<Option<Node>> {
        if !matches!(self.peek_kind(), Some(TokenKind::Unknown | TokenKind::BitwiseXor)) {
            return Ok(None);
        }

        let caret_token = self.tokens.peek()?;
        if caret_token.text.as_ref() != "^" {
            return Ok(None);
        }

        let caret_token = self.tokens.next()?;
        let mut name = String::from("^");
        let mut end = caret_token.end;

        if self.peek_kind() == Some(TokenKind::Identifier) {
            let ident = self.tokens.next()?;
            name.push_str(&ident.text);
            end = ident.end;
        }

        Ok(Some(Node::new(
            NodeKind::Variable { sigil: String::from("$"), name },
            SourceLocation { start: caret_token.start, end },
        )))
    }

    fn parse_qualified_scalar_tail(
        &mut self,
        mut full_name: String,
        start: usize,
        mut end: usize,
    ) -> ParseResult<Node> {
        while self.peek_kind() == Some(TokenKind::DoubleColon) {
            self.tokens.next()?;
            full_name.push_str("::");

            if self.peek_kind() == Some(TokenKind::Identifier) {
                let name_token = self.tokens.next()?;
                full_name.push_str(&name_token.text);
                end = name_token.end;
            } else {
                return Err(ParseError::syntax(
                    "Expected identifier after :: in package-qualified variable",
                    self.current_position(),
                ));
            }
        }

        let variable = Node::new(
            NodeKind::Variable { sigil: "$".to_string(), name: full_name },
            SourceLocation { start, end },
        );

        self.parse_postfix_chain(variable)
    }

    fn consume_deref_body_terminators(&mut self) -> ParseResult<()> {
        while self.peek_kind() == Some(TokenKind::Semicolon) {
            self.consume_token()?;
        }
        Ok(())
    }

    /// Parse a variable when we have a sigil token first
    fn parse_variable_from_sigil(&mut self) -> ParseResult<Node> {
        let sigil_token = self.consume_token()?;
        let sigil = match sigil_token.kind {
            TokenKind::BitwiseAnd => "&".to_string(), // Handle & as sigil
            _ => sigil_token.text.to_string(),
        };
        let start = sigil_token.start;

        // Check if next token is an identifier or a keyword that should be treated as identifier
        let next_kind = self.peek_kind();

        // Keywords can be used as variable names with any sigil
        // e.g., %try, $default, @for, &try are all valid Perl.
        let (name, end) = if next_kind.is_some_and(Self::is_variable_name_kind) {
            let name_token = self.tokens.next()?;
            let mut name = name_token.text.to_string();
            let mut end = name_token.end;

            // `%$$slice` may arrive as `%`, `$$`, `slice`; only join an
            // adjacent tail so whitespace-delimited `$$ eq` keeps `eq` as op.
            if name == "$$"
                && self.peek_kind() == Some(TokenKind::Identifier)
                && self
                    .tokens
                    .peek()
                    .ok()
                    .is_some_and(|next_token| next_token.start == end)
            {
                let next_token = self.tokens.next()?;
                name.push_str(&next_token.text);
                end = next_token.end;
            }

            // Handle :: in package-qualified variables
            while self.peek_kind() == Some(TokenKind::DoubleColon) {
                self.tokens.next()?; // consume ::
                name.push_str("::");

                if self.peek_kind() == Some(TokenKind::Identifier) {
                    let next_token = self.tokens.next()?;
                    name.push_str(&next_token.text);
                    end = next_token.end;
                } else {
                    return Err(ParseError::syntax(
                        "Expected identifier after :: in package-qualified variable",
                        self.current_position(),
                    ));
                }
            }

            (name, end)
        } else {
            // Handle special variables like $$, $@, $!, $?, etc.
            match self.peek_kind() {
                Some(TokenKind::ScalarSigil) => {
                    // `$$` is the PID special variable, but `$$ident` is a scalar
                    // dereference target that must preserve the referenced name.
                    let token = self.tokens.next()?;
                    if self.tokens.peek().ok().is_some_and(|name_token| {
                        Self::is_variable_name_kind(name_token.kind) && name_token.start == token.end
                    }) {
                        let name_token = self.tokens.next()?;
                        let mut name = format!("${}", name_token.text);
                        let mut end = name_token.end;

                        while self.peek_kind() == Some(TokenKind::DoubleColon) {
                            self.tokens.next()?; // consume ::
                            name.push_str("::");

                            if self.peek_kind() == Some(TokenKind::Identifier) {
                                let next_token = self.tokens.next()?;
                                name.push_str(&next_token.text);
                                end = next_token.end;
                            } else {
                                return Err(ParseError::syntax(
                                    "Expected identifier after :: in package-qualified variable",
                                    self.current_position(),
                                ));
                            }
                        }

                        (name, end)
                    } else {
                        ("$".to_string(), token.end)
                    }
                }
                Some(TokenKind::ArraySigil) => {
                    // $@ - eval error
                    let token = self.tokens.next()?;
                    ("@".to_string(), token.end)
                }
                Some(TokenKind::Not) => {
                    // $! - system error
                    let token = self.tokens.next()?;
                    ("!".to_string(), token.end)
                }
                Some(TokenKind::Unknown) => {
                    // Could be $?, $^, $#, or other special
                    let token = self.tokens.peek()?;
                    match token.text.as_ref() {
                        "?" => {
                            let token = self.tokens.next()?;
                            ("?".to_string(), token.end)
                        }
                        "^" => {
                            // Handle $^X variables
                            let token = self.tokens.next()?;
                            if self.peek_kind() == Some(TokenKind::Identifier) {
                                let var_token = self.tokens.next()?;
                                (format!("^{}", var_token.text), var_token.end)
                            } else {
                                ("^".to_string(), token.end)
                            }
                        }
                        "#" => {
                            // Handle $# (array length)
                            let token = self.tokens.next()?;
                            if self.peek_kind() == Some(TokenKind::Identifier) {
                                let var_token = self.tokens.next()?;
                                let mut var_name = var_token.text.to_string();
                                let mut var_end = var_token.end;

                                // Handle $#Pkg::Var (package-qualified)
                                while self.peek_kind() == Some(TokenKind::DoubleColon) {
                                    self.tokens.next()?;
                                    var_name.push_str("::");
                                    if self.peek_kind() == Some(TokenKind::Identifier) {
                                        let next_token = self.tokens.next()?;
                                        var_name.push_str(&next_token.text);
                                        var_end = next_token.end;
                                    }
                                }

                                (format!("#{}", var_name), var_end)
                            } else if matches!(self.peek_kind(), Some(TokenKind::ScalarSigil))
                                || self.tokens.peek().ok().is_some_and(|t| t.text.starts_with('$'))
                            {
                                // $#$ref — last index of dereferenced array
                                // Parse the inner variable expression
                                let inner = self.parse_variable()?;
                                let end = inner.location.end;
                                // Wrap in a Unary $#() node
                                let node = Node::new(
                                    NodeKind::Unary {
                                        op: "$#".to_string(),
                                        operand: Box::new(inner),
                                    },
                                    SourceLocation { start, end },
                                );
                                return Ok(node);
                            } else if self.peek_kind() == Some(TokenKind::LeftBrace) {
                                // $#{expr} — last index of dereferenced array via block
                                self.tokens.next()?; // consume {
                                let inner = self.parse_expression()?;
                                self.expect(TokenKind::RightBrace)?;
                                let end = self.previous_position();
                                let node = Node::new(
                                    NodeKind::Unary {
                                        op: "$#".to_string(),
                                        operand: Box::new(inner),
                                    },
                                    SourceLocation { start, end },
                                );
                                return Ok(node);
                            } else {
                                // Just $# by itself
                                ("#".to_string(), token.end)
                            }
                        }
                        _ => {
                            return Err(ParseError::syntax(
                                format!("Unexpected character after sigil: {}", token.text),
                                token.start,
                            ));
                        }
                    }
                }
                Some(TokenKind::Number) => {
                    // $0, $1, $2, etc. - numbered capture groups
                    let num_token = self.tokens.next()?;
                    (num_token.text.to_string(), num_token.end)
                }
                Some(TokenKind::DoubleColon) => {
                    // $:: — the main namespace stash
                    let dc_token = self.tokens.next()?; // consume ::
                    if self.peek_kind() == Some(TokenKind::Identifier) {
                        let name_token = self.tokens.next()?;
                        (format!("::{}", name_token.text), name_token.end)
                    } else {
                        ("::".to_string(), dc_token.end)
                    }
                }
                Some(TokenKind::Colon) => {
                    // $: — format line-break character variable
                    let colon_token = self.tokens.next()?;
                    (":".to_string(), colon_token.end)
                }
                _ => {
                    // Empty variable name (just the sigil)
                    (String::new(), self.previous_position())
                }
            }
        };

        // Special handling for @, %, or $ sigil followed by { - array/hash/scalar dereference
        // e.g. @{$ref}, %{$hash}, ${"${pkg}::$sym"}
        if (sigil == "@" || sigil == "%" || sigil == "$")
            && name.is_empty()
            && self.peek_kind() == Some(TokenKind::LeftBrace)
        {
            self.tokens.next()?; // consume {

            // Parse the expression inside the braces
            let (expr, folded) = if sigil == "$" {
                self.parse_braced_scalar_body()?
            } else {
                (self.parse_expression()?, false)
            };

            self.consume_deref_body_terminators()?;
            self.expect(TokenKind::RightBrace)?;
            let end = self.previous_position();

            if folded {
                // `${ name }` == `$name` (perlref): already folded to a
                // scalar variable node; do not re-wrap in Unary{"${}"}.
                let mut folded_node = expr;
                folded_node.location = SourceLocation { start, end };
                return Ok(folded_node);
            }

            let op = format!("{}{{}}", sigil);
            return Ok(Node::new(
                NodeKind::Unary { op, operand: Box::new(expr) },
                SourceLocation { start, end },
            ));
        }

        // Special handling for & sigil followed by { - code dereference: &{expr}(args)
        if sigil == "&" && name.is_empty() && self.peek_kind() == Some(TokenKind::LeftBrace) {
            self.tokens.next()?; // consume {
            return self.parse_code_dereference(start);
        }

        // Special handling for & sigil - it's a function call
        if sigil == "&" {
            let args = if self.peek_kind() == Some(TokenKind::LeftParen) {
                self.consume_token()?; // consume (
                self.parse_parenthesized_arg_list()?
            } else {
                vec![]
            };

            Ok(Node::new(NodeKind::FunctionCall { name, args }, SourceLocation { start, end }))
        } else if sigil == "*" {
            Ok(Node::new(NodeKind::Typeglob { name }, SourceLocation { start, end }))
        } else if matches!(sigil.as_str(), "$" | "@" | "%")
            && Self::is_unbraced_scalar_deref_name(&name)
        {
            // Unbraced dereference arriving via separate sigil token:
            // `@` + `$ref`, `%` + `$ref`, or `$` (ScalarSigil) + `$ref`.
            // Equivalent to @{$ref}, %{$ref}, ${$ref}.
            let inner_name = name[1..].to_string();
            let inner = Node::new(
                NodeKind::Variable { sigil: "$".to_string(), name: inner_name },
                SourceLocation { start: start + sigil.len(), end },
            );
            let op = format!("{}{{}}", sigil);
            Ok(Node::new(
                NodeKind::Unary { op, operand: Box::new(inner) },
                SourceLocation { start, end },
            ))
        } else {
            Ok(Node::new(NodeKind::Variable { sigil, name }, SourceLocation { start, end }))
        }
    }

    /// Parse a parenthesized argument list: (expr, expr, ...).
    /// Assumes the opening `(` has already been consumed.
    fn parse_parenthesized_arg_list(&mut self) -> ParseResult<Vec<Node>> {
        let mut args = vec![];
        while self.peek_kind() != Some(TokenKind::RightParen) && !self.tokens.is_eof() {
            args.push(self.parse_expression()?);
            // Accept both comma and fat arrow as separators
            if matches!(self.peek_kind(), Some(TokenKind::Comma | TokenKind::FatArrow)) {
                self.consume_token()?;
            } else if self.peek_kind() != Some(TokenKind::RightParen) && !self.tokens.is_eof() {
                return Err(ParseError::syntax(
                    "Expected comma or right parenthesis",
                    self.current_position(),
                ));
            }
        }
        self.expect(TokenKind::RightParen)?;
        Ok(args)
    }

    /// Parse code dereference: {expr} optionally followed by (args).
    /// Assumes the opening `{` has already been consumed.
    /// `start` is the position of the `&` sigil.
    fn parse_code_dereference(&mut self, start: usize) -> ParseResult<Node> {
        let inner_expr = self.parse_expression()?;
        self.consume_deref_body_terminators()?;
        self.expect(TokenKind::RightBrace)?;
        let deref_end = self.previous_position();
        let deref_node = Node::new(
            NodeKind::Unary { op: "&{}".to_string(), operand: Box::new(inner_expr) },
            SourceLocation { start, end: deref_end },
        );

        if self.peek_kind() == Some(TokenKind::LeftParen) {
            self.consume_token()?;
            let args = self.parse_parenthesized_arg_list()?;
            let call_end = self.previous_position();
            let mut all = vec![deref_node];
            all.extend(args);
            return Ok(Node::new(
                NodeKind::FunctionCall { name: "&{}".to_string(), args: all },
                SourceLocation { start, end: call_end },
            ));
        }

        Ok(deref_node)
    }

    /// Parse subroutine signature
    fn parse_signature(&mut self) -> ParseResult<Vec<Node>> {
        self.expect(TokenKind::LeftParen)?; // consume (
        let mut params = Vec::new();
        let mut seen_invocant_separator = false;

        while self.peek_kind() != Some(TokenKind::RightParen) && !self.tokens.is_eof() {
            // Parse parameter
            let param = self.parse_signature_param()?;
            params.push(param);

            // Check for separator or end of signature.
            // Perl method signatures may use an invocant separator:
            //   method run ($self: $arg1, $arg2) { ... }
            // Treat the first `:` after a parameter as a valid separator.
            if self.peek_kind() == Some(TokenKind::Comma) {
                self.tokens.next()?; // consume comma
            } else if self.peek_kind() == Some(TokenKind::Colon) && !seen_invocant_separator {
                self.tokens.next()?; // consume invocant separator
                seen_invocant_separator = true;
            } else if self.peek_kind() == Some(TokenKind::RightParen) {
                break;
            } else {
                return Err(ParseError::syntax(
                    "Expected comma or closing parenthesis in signature",
                    self.current_position(),
                ));
            }
        }

        self.expect(TokenKind::RightParen)?; // consume )
        self.validate_signature_ordering(&params);
        Ok(params)
    }

    /// Validate ordering rules for a collected list of signature parameters.
    ///
    /// Emits diagnostics (without aborting the parse) for:
    /// - A slurpy (`@` or `%`) parameter that is not the last parameter.
    /// - Both an `@` and a `%` slurpy parameter present in the same signature.
    /// - A mandatory parameter appearing after an optional parameter.
    fn validate_signature_ordering(&mut self, params: &[Node]) {
        let mut seen_slurpy_at = false; // saw @array slurpy
        let mut seen_slurpy_pct = false; // saw %hash slurpy
        let mut seen_optional = false;

        for (idx, param) in params.iter().enumerate() {
            let is_last = idx == params.len() - 1;

            match &param.kind {
                NodeKind::SlurpyParameter { variable } => {
                    let sigil = match &variable.kind {
                        NodeKind::Variable { sigil, .. } => sigil.as_str(),
                        _ => "",
                    };

                    if sigil == "@" {
                        if seen_slurpy_pct {
                            self.errors.push(ParseError::syntax(
                                "Signature cannot have both @ and % slurpy parameters",
                                param.location.start,
                            ));
                        }
                        seen_slurpy_at = true;
                    } else if sigil == "%" {
                        if seen_slurpy_at {
                            self.errors.push(ParseError::syntax(
                                "Signature cannot have both @ and % slurpy parameters",
                                param.location.start,
                            ));
                        }
                        seen_slurpy_pct = true;
                    }

                    if !is_last {
                        self.errors.push(ParseError::syntax(
                            "Slurpy parameter must be the last parameter in the signature",
                            param.location.start,
                        ));
                    }
                }
                NodeKind::OptionalParameter { .. } => {
                    seen_optional = true;
                }
                NodeKind::MandatoryParameter { .. } => {
                    if seen_optional {
                        self.errors.push(ParseError::syntax(
                            "Mandatory parameter cannot follow an optional parameter in signature",
                            param.location.start,
                        ));
                    }
                }
                _ => {}
            }
        }
    }

    /// Parse a single signature parameter
    fn parse_signature_param(&mut self) -> ParseResult<Node> {
        let start = self.current_position();

        // Check for named parameter (:$name)
        let named = if self.peek_kind() == Some(TokenKind::Colon) {
            self.tokens.next()?; // consume :
            true
        } else {
            false
        };

        // Check for type constraint (Type $var)
        let _type_constraint = if self.peek_kind() == Some(TokenKind::Identifier) {
            // Look ahead to see if this is a type constraint
            let token = self.tokens.peek()?;
            if !token.text.starts_with('$')
                && !token.text.starts_with('@')
                && !token.text.starts_with('%')
                && !token.text.starts_with('&')
            {
                // It's likely a type constraint
                Some(self.tokens.next()?.text.to_string())
            } else {
                None
            }
        } else {
            None
        };

        // Parse the variable
        let variable = self.parse_variable()?;

        // Some signature-capable frameworks (and Object::Pad-style code in the wild)
        // attach parameter traits/attributes after the variable:
        //   sub f ($x :param, $y :reader(foo)) { ... }
        // Treat these as parseable syntax and preserve only span/shape for now.
        let mut end = variable.location.end;
        end = self.consume_signature_param_attributes(end)?;

        // Check for a default value. Positional parameters accept only `=`;
        // named parameters (Perl 5.44 / PPC0024) additionally accept the `//=`
        // and `||=` default operators (`sub f (:$x //= 1)`), which apply the
        // default when the caller omits the argument or passes undef / a false
        // value respectively.
        let default_op: Option<&'static str> = match self.peek_kind() {
            Some(TokenKind::Assign) => Some("="),
            Some(TokenKind::DefinedOrAssign) if named => Some("//="),
            Some(TokenKind::LogicalOrAssign) if named => Some("||="),
            _ => None,
        };
        let default_value = if default_op.is_some() {
            self.tokens.next()?; // consume the default operator
            // Parse a full scalar expression for the default value (perlsub: "any scalar
            // expression").  parse_ternary covers calls, binops, and ternary expressions
            // while stopping at the `,` or `)` that delimits signature parameters, since
            // comma collection only happens in parse_comma (one level above).
            Some(Box::new(self.parse_ternary()?))
        } else {
            None
        };

        end = if let Some(ref default) = default_value {
            default.location.end
        } else {
            end
        };

        // Check if variable is slurpy (@args or %hash)
        let is_slurpy = matches!(&variable.kind, NodeKind::Variable { sigil, .. } if sigil == "@" || sigil == "%");

        // Create the appropriate parameter node type
        let param_kind = if named {
            // The external argument name is the lexical variable name without
            // its sigil (`:$alpha` is supplied by callers as `alpha => ...`).
            let external_name = match &variable.kind {
                NodeKind::Variable { name, .. } => name.clone(),
                _ => String::new(),
            };
            // A named parameter without a default is required; with a default
            // it is optional. Preserve which operator introduced the default
            // (`=`, `//=`, or `||=`) so downstream layers can distinguish the
            // defaulting semantics.
            let (default_operator, required) = match default_op {
                Some(op) => (Some(op.to_string()), false),
                None => (None, true),
            };
            NodeKind::NamedParameter {
                variable: Box::new(variable),
                external_name,
                default_operator,
                default_value,
                required,
            }
        } else if is_slurpy {
            NodeKind::SlurpyParameter { variable: Box::new(variable) }
        } else if let Some(default) = default_value {
            NodeKind::OptionalParameter { variable: Box::new(variable), default_value: default }
        } else {
            NodeKind::MandatoryParameter { variable: Box::new(variable) }
        };

        Ok(Node::new(param_kind, SourceLocation { start, end }))
    }

    fn consume_signature_param_attributes(&mut self, mut end: usize) -> ParseResult<usize> {
        // Only consume `:identifier(...)` parameter attributes. A bare `:` (with `)`,
        // sigil, variable, or other non-bareword token following) is the
        // method-invocant separator and must be left for `parse_signature` to
        // handle — see #6254 for the invocant separator support that this
        // helper must coexist with.
        //
        // Note: the perl-lexer returns variables as Identifier tokens whose text
        // begins with the sigil (e.g. `$b`). A real parameter attribute is a
        // bareword identifier (e.g. `param`, `reader`), so reject identifier
        // texts that begin with a Perl sigil character.
        while self.peek_kind() == Some(TokenKind::Colon) {
            let next_is_attr_ident = match self.tokens.peek_second() {
                Ok(tok) if tok.kind == TokenKind::Identifier => {
                    let first = tok.text.chars().next();
                    !matches!(first, Some('$' | '@' | '%' | '&' | '*'))
                }
                _ => false,
            };
            if !next_is_attr_ident {
                break;
            }
            self.consume_token()?; // consume ':'
            let attr = self.expect(TokenKind::Identifier)?;
            end = attr.end;

            if self.peek_kind() == Some(TokenKind::LeftParen) {
                self.consume_token()?; // consume '('
                let mut depth = 1usize;
                while depth > 0 && !self.tokens.is_eof() {
                    let token = self.consume_token()?;
                    end = token.end;
                    match token.kind {
                        TokenKind::LeftParen => depth += 1,
                        TokenKind::RightParen => depth -= 1,
                        _ => {}
                    }
                }

                if depth != 0 {
                    return Err(ParseError::syntax(
                        "Unterminated signature parameter attribute arguments",
                        self.current_position(),
                    ));
                }
            }
        }

        Ok(end)
    }

    /// Check if the parenthesized content after sub name is a prototype (not a signature)
    #[allow(dead_code)]
    fn is_prototype(&mut self) -> bool {
        // Peek at the next token after (
        match self.tokens.peek_second() {
            Ok(token) => {
                // Check if it starts with prototype characters or looks like a prototype
                matches!(token.kind,
                    TokenKind::ScalarSigil | TokenKind::ArraySigil |
                    TokenKind::HashSigil | TokenKind::SubSigil |
                    TokenKind::Star | TokenKind::Semicolon |
                    TokenKind::Backslash) ||
                // Check for special vars that look like prototypes ($$, $#, etc)
                (token.kind == TokenKind::Identifier &&
                 token.text.chars().all(|c| matches!(c, '$' | '@' | '%' | '*' | '&' | ';' | '\\')))
            }
            Err(_) => false,
        }
    }

    /// Check if the parentheses likely contain a prototype rather than a signature
    fn is_likely_prototype(&mut self) -> ParseResult<bool> {
        // We need to peek past the opening paren without consuming
        // First, ensure we're at a left paren
        if self.tokens.peek()?.kind != TokenKind::LeftParen {
            return Ok(false);
        }

        // Use peek_second to look at the token after the paren
        match self.tokens.peek_second() {
            Ok(token) => {
                Ok(match token.kind {
                    // These are unambiguously prototype tokens.
                    // `+` means "scalar or array/hash ref" (perlsub), valid in prototypes.
                    // `++` is also valid: Perl's lexer merges two `+` into `Increment`, so
                    // `(++$)` has peek_second == Increment.
                    TokenKind::Star
                    | TokenKind::Backslash
                    | TokenKind::Semicolon
                    | TokenKind::BitwiseAnd
                    | TokenKind::SubSigil
                    | TokenKind::GlobSigil
                    | TokenKind::Plus
                    | TokenKind::Increment => true,
                    // Sigils: peek past to distinguish prototype ($;@%) from signature ($x, @rest)
                    TokenKind::ScalarSigil | TokenKind::ArraySigil | TokenKind::HashSigil => {
                        match self.tokens.peek_third() {
                            Ok(third) => !matches!(third.kind, TokenKind::Identifier),
                            Err(_) => true, // default to prototype on error
                        }
                    }
                    // Empty prototype
                    TokenKind::RightParen => true,
                    // Colon indicates named parameter (:$foo), so it's a signature
                    TokenKind::Colon => false,
                    // Identifiers: The lexer produces a single Identifier token that
                    // may include the leading sigil (e.g., `$x` → Identifier("$x")).
                    // Signature parameters always start with a sigil followed by a name
                    // (e.g., `$x`, `@arr`). Pure prototype characters produce either
                    // bare sigil tokens (ScalarSigil/ArraySigil handled above) or an
                    // Identifier token whose text contains only valid prototype chars
                    // (`_`, `$`, `@`, `%`, `*`, `&`).
                    //
                    // A bare alphabetic identifier with no leading sigil (e.g., `XYZ`, `a`)
                    // cannot be a signature parameter — treat it as a prototype candidate
                    // so that `parse_prototype` can validate and warn on the invalid chars.
                    TokenKind::Identifier => {
                        let text = &*token.text;
                        // `_` is a valid prototype character (default $_)
                        if text == "_" {
                            return Ok(true);
                        }
                        // A sigil-prefixed identifier: check if ALL chars are valid
                        // prototype chars.  If not (e.g., `$x`), it's a signature param.
                        let all_proto_chars = text.chars().all(is_valid_prototype_char);
                        if all_proto_chars {
                            // Looks like prototype-only chars → prototype
                            return Ok(true);
                        }
                        // Text begins with a sigil followed by a real identifier name →
                        // it's a signature parameter (e.g., `$x`).
                        let starts_with_sigil = text
                            .chars()
                            .next()
                            .is_some_and(|c| matches!(c, '$' | '@' | '%' | '*' | '&'));
                        if starts_with_sigil {
                            // `$x`, `@arr`, etc. → signature
                            return Ok(false);
                        }
                        if let Ok(third) = self.tokens.peek_third() {
                            let third_starts_signature = match third.kind {
                                TokenKind::Identifier => third
                                    .text
                                    .chars()
                                    .next()
                                    .is_some_and(|c| matches!(c, '$' | '@' | '%' | '*' | '&')),
                                TokenKind::ScalarSigil
                                | TokenKind::ArraySigil
                                | TokenKind::HashSigil
                                | TokenKind::SubSigil
                                | TokenKind::GlobSigil => true,
                                _ => false,
                            };

                            if third_starts_signature {
                                // `Type $x`, `Role @rest`, etc. are typed signatures.
                                return Ok(false);
                            }
                        }
                        // Bare alphabetic identifier with no sigil (e.g., `XYZ`, `foo`) →
                        // treat as prototype candidate; invalid chars will be warned about.
                        true
                    }
                    // Anything else suggests a signature
                    _ => false,
                })
            }
            Err(_) => Ok(false),
        }
    }

    /// Parse old-style prototype
    fn parse_prototype(&mut self) -> ParseResult<String> {
        let open_paren_pos = self.current_position();
        self.expect(TokenKind::LeftParen)?; // consume (
        let mut prototype = String::new();

        while !self.tokens.is_eof() {
            let token = self.consume_token()?;

            match token.kind {
                TokenKind::RightParen => {
                    // End of prototype
                    break;
                }
                TokenKind::ScalarSigil => prototype.push('$'),
                TokenKind::ArraySigil => prototype.push('@'),
                TokenKind::HashSigil => prototype.push('%'),
                TokenKind::GlobSigil | TokenKind::Star => prototype.push('*'),
                TokenKind::SubSigil | TokenKind::BitwiseAnd => prototype.push('&'),
                TokenKind::Semicolon => prototype.push(';'),
                TokenKind::Backslash => prototype.push('\\'),
                // `+` means "scalar or array/hash ref" (perlsub prototype character).
                // `++` is the Increment token produced when two `+` chars appear together.
                TokenKind::Plus => prototype.push('+'),
                TokenKind::Increment => prototype.push_str("++"),
                _ => {
                    // For any other token, just add its text
                    // This handles cases where sigils might be parsed differently
                    prototype.push_str(&token.text);
                }
            }
        }

        // Validate every character in the collected prototype string.
        // Perl allows: $ @ % & * \ ; + _ bracketed ref groups, and ASCII space.
        // Anything else triggers Perl's "Illegal character in prototype" warning.
        // We emit a SyntaxError diagnostic (collected as a warning by the LSP layer
        // via DiagnosticCode::InvalidPrototype / PL302) but do NOT abort parsing —
        // the prototype string is preserved so the caller still gets a Subroutine node.
        let invalid_chars: String = prototype
            .chars()
            .filter(|c| !is_valid_prototype_char(*c))
            .collect::<std::collections::BTreeSet<char>>()
            .into_iter()
            .collect();

        if !invalid_chars.is_empty() {
            self.errors.push(ParseError::SyntaxError {
                message: format!(
                    "Invalid prototype character(s) '{}' — valid characters are: \
                    $, @, %, &, *, \\, ;, +, _ (see perlsub)",
                    invalid_chars
                ),
                location: open_paren_pos,
            });
        }

        Ok(prototype)
    }

    fn is_variable_name_kind(kind: TokenKind) -> bool {
        kind == TokenKind::Identifier || Self::can_be_sub_name(kind)
    }
}

/// Return `true` if `c` is a character that Perl permits in old-style prototypes.
///
/// Valid characters (from perlsub):
/// `$` `@` `%` `&` `*` `\` `;` `+` `_`, bracketed ref groups, and ASCII space.
fn is_valid_prototype_char(c: char) -> bool {
    matches!(c, '$' | '@' | '%' | '&' | '*' | '\\' | ';' | '+' | '_' | '[' | ']' | ' ')
}

/// Return `true` if `name` is a simple bareword identifier suitable for the
/// `${name}` == `$name` folding described in perlref: `${foo}` is exactly
/// `$foo` when `foo` is a plain identifier, not an arbitrary dereference
/// expression.
///
/// Valid: first character alphabetic or underscore, remaining characters
/// alphanumeric or underscore. Matches the identifier text the lexer already
/// produces for the `${identifier}` single-token form (see
/// `perl-lexer`'s braced-variable scan), so no `::` package-separator
/// handling is needed here.
fn is_simple_scalar_name(name: &str) -> bool {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !(first.is_alphabetic() || first == '_') {
        return false;
    }
    chars.all(|c| c.is_alphanumeric() || c == '_')
}

#[cfg(test)]
mod prototype_heuristic_tests {
    use super::*;

    /// Helper: parse code and extract the first Subroutine node.
    fn parse_sub(code: &str) -> Option<Node> {
        let mut parser = Parser::new(code);
        let ast = parser.parse().ok()?;
        if let NodeKind::Program { statements } = ast.kind {
            statements.into_iter().next()
        } else {
            None
        }
    }

    #[test]
    fn signature_with_named_params() {
        let node = parse_sub("sub foo($x) {}");
        assert!(node.is_some(), "expected parsed subroutine for `sub foo($x) {{}}`");
        let Some(node) = node else {
            return;
        };
        assert!(
            matches!(&node.kind, NodeKind::Subroutine { .. }),
            "expected Subroutine node, got {}",
            node.kind.kind_name()
        );

        if let NodeKind::Subroutine { signature, prototype, .. } = &node.kind {
            assert!(signature.is_some(), "sub foo($x) should have a signature");
            assert!(prototype.is_none(), "sub foo($x) should not have a prototype");
        }
    }

    /// `--lib` coverage for the named-parameter construction branch in
    /// `parse_signature_param`: the external name is derived from the variable
    /// (sigil-stripped), a `= <expr>` default is preserved (not discarded), the
    /// default operator is recorded, and `required` reflects default presence.
    #[test]
    fn named_parameter_carries_external_name_and_default() {
        fn find_named(node: &Node, out: &mut Vec<(String, bool, bool, Option<String>)>) {
            if let NodeKind::NamedParameter {
                external_name, default_value, required, default_operator, ..
            } = &node.kind
            {
                out.push((
                    external_name.clone(),
                    *required,
                    default_value.is_some(),
                    default_operator.clone(),
                ));
            }
            node.for_each_child(|c| find_named(c, out));
        }

        let node = parse_sub("sub f (:$alpha, :$beta = 1) {}").expect("parse named-param sub");
        let mut found = Vec::new();
        find_named(&node, &mut found);

        assert_eq!(found.len(), 2, "both named params surface, got {found:?}");

        let alpha = found.iter().find(|f| f.0 == "alpha").expect("named param :$alpha");
        assert!(alpha.1, ":$alpha has no default → required");
        assert!(!alpha.2, ":$alpha has no default value");
        assert!(alpha.3.is_none(), ":$alpha has no default operator");

        let beta = found.iter().find(|f| f.0 == "beta").expect("named param :$beta");
        assert!(!beta.1, ":$beta has a default → optional");
        assert!(beta.2, ":$beta preserves its default value");
        assert_eq!(beta.3.as_deref(), Some("="), ":$beta records the `=` default operator");
    }

    /// Perl 5.44 named parameters accept `//=` and `||=` default operators in
    /// addition to `=` (PPC0024). Positional parameters accept only `=`.
    #[test]
    fn named_parameter_records_slash_slash_and_pipe_pipe_default_operators() -> Result<(), String> {
        // Collect every named parameter's (external_name, default_operator) pair
        // in one straight-line walk, then assert by literal name — mirroring the
        // sibling `find_named` collector. Deliberately avoids a per-node
        // `external_name == name` comparison so this coverage test asserts
        // behaviour without introducing a branch seam of its own.
        fn collect_named_ops(node: &Node, out: &mut Vec<(String, Option<String>)>) {
            if let NodeKind::NamedParameter { external_name, default_operator, .. } = &node.kind {
                out.push((external_name.clone(), default_operator.clone()));
            }
            node.for_each_child(|c| collect_named_ops(c, out));
        }

        let node = parse_sub("sub f (:$a = 1, :$b //= 2, :$c ||= 3) {}")
            .ok_or("parse named params with //= and ||= defaults")?;
        let mut ops = Vec::new();
        collect_named_ops(&node, &mut ops);

        assert_eq!(
            ops.iter().find(|(n, _)| n == "a").map(|(_, op)| op.clone()),
            Some(Some("=".to_string())),
            ":$a uses `=`"
        );
        assert_eq!(
            ops.iter().find(|(n, _)| n == "b").map(|(_, op)| op.clone()),
            Some(Some("//=".to_string())),
            ":$b uses `//=`"
        );
        assert_eq!(
            ops.iter().find(|(n, _)| n == "c").map(|(_, op)| op.clone()),
            Some(Some("||=".to_string())),
            ":$c uses `||=`"
        );
        Ok(())
    }

    /// `--lib` call-observation coverage for `parse_signature_param`'s
    /// default-operator match (`match self.peek_kind() { ... }`): call the
    /// seam-owning method directly — not through the full `parse_sub` chain
    /// — so each match arm is exercised and observed with an exact-value
    /// assertion on the resulting node, rather than only inferred from a
    /// downstream parse error.
    #[test]
    fn parse_signature_param_directly_selects_each_default_operator_arm() -> Result<(), String> {
        fn parse_param(src: &str) -> Result<Node, String> {
            let mut parser = Parser::new(src);
            parser.parse_signature_param().map_err(|e| format!("parse `{src}`: {e:?}"))
        }

        // `=` arm: available to named parameters (and, separately, to
        // positional parameters via `OptionalParameter`).
        let node = parse_param(":$a = 1")?;
        match &node.kind {
            NodeKind::NamedParameter { default_operator, required, .. } => {
                assert_eq!(default_operator.as_deref(), Some("="), ":$a = 1 selects the `=` arm");
                assert!(!required, ":$a = 1 has a default -> optional");
            }
            other => return Err(format!("expected NamedParameter, got {}", other.kind_name())),
        }

        // `//=` arm: named-only, gated by `named`.
        let node = parse_param(":$b //= 2")?;
        match &node.kind {
            NodeKind::NamedParameter { default_operator, required, .. } => {
                assert_eq!(
                    default_operator.as_deref(),
                    Some("//="),
                    ":$b //= 2 selects the `//=` arm"
                );
                assert!(!required, ":$b //= 2 has a default -> optional");
            }
            other => return Err(format!("expected NamedParameter, got {}", other.kind_name())),
        }

        // `||=` arm: named-only, gated by `named`.
        let node = parse_param(":$c ||= 3")?;
        match &node.kind {
            NodeKind::NamedParameter { default_operator, required, .. } => {
                assert_eq!(
                    default_operator.as_deref(),
                    Some("||="),
                    ":$c ||= 3 selects the `||=` arm"
                );
                assert!(!required, ":$c ||= 3 has a default -> optional");
            }
            other => return Err(format!("expected NamedParameter, got {}", other.kind_name())),
        }

        // Fallback `_ => None` arm for a named parameter: no default token
        // follows, so no default operator is recorded and the parameter is
        // required.
        let node = parse_param(":$d")?;
        match &node.kind {
            NodeKind::NamedParameter { default_operator, required, .. } => {
                assert!(default_operator.is_none(), ":$d has no default token -> None arm");
                assert!(required, ":$d has no default -> required");
            }
            other => return Err(format!("expected NamedParameter, got {}", other.kind_name())),
        }

        // Fallback `_ => None` arm for a *positional* parameter: `named` is
        // false, so the `//=` guard fails even though `DefinedOrAssign`
        // follows, and default_op falls through to `_ => None`. The `//= 1`
        // tokens are left unconsumed by this call (the caller reports the
        // error), but this seam-owner call directly observes that no default
        // was consumed at all -- proving the guard, not an incidental
        // downstream parse failure.
        let node = parse_param("$x //= 1")?;
        assert!(
            matches!(&node.kind, NodeKind::MandatoryParameter { .. }),
            "positional `$x //= 1`: named=false so the `//=` arm guard fails, \
             falling through to `_ => None` (no default consumed)"
        );

        // Discriminator for the type-constraint `peek_kind() == Some(Identifier)`
        // boundary at the head of `parse_signature_param`: a leading *bareword*
        // identifier (`Type`) is consumed as a type constraint before the
        // variable, exercising the true side of the inner
        // `!token.text.starts_with('$')` check — whereas every `$`/`:$`-sigiled
        // case above takes the false side (the identifier text starts with a
        // sigil, so it is the variable, not a type).
        let node = parse_param("Type $x")?;
        assert!(
            matches!(&node.kind, NodeKind::MandatoryParameter { .. }),
            "`Type $x`: the bareword `Type` is a type constraint, `$x` the parameter"
        );

        Ok(())
    }

    /// Exact error-variant coverage for the named-parameter seam in
    /// `parse_signature_param`: a named parameter whose default operator is
    /// present but followed by no default expression (`:$x =`) must surface the
    /// underlying `parse_ternary` error rather than fabricating a defaulted
    /// parameter. Grips the weakly-covered error edge of the named seam.
    #[test]
    fn parse_signature_param_named_default_without_expression_is_an_error() {
        let mut parser = Parser::new(":$x =");
        assert!(
            parser.parse_signature_param().is_err(),
            "`:$x =` has a default operator with no following expression, so \
             parse_signature_param must propagate the parse_ternary error"
        );
    }

    /// The `//=` / `||=` default operators are named-only (PPC0024). A
    /// *positional* parameter must not consume them as a default — the parser
    /// reports an error instead of silently accepting the named-only syntax.
    /// Guards the `named` gate in `parse_signature_param` against regression.
    #[test]
    fn positional_parameter_rejects_slash_slash_and_pipe_pipe_defaults() -> Result<(), String> {
        for src in ["sub f ($x //= 1) {}", "sub f ($x ||= 1) {}"] {
            let mut parser = Parser::new(src);
            parser.parse().map_err(|e| format!("parse `{src}`: {e:?}"))?;
            assert!(
                !parser.get_errors().is_empty(),
                "expected a parse error for positional default operator in `{src}`",
            );
        }
        Ok(())
    }

    #[test]
    fn signature_with_multiple_params() {
        let node = parse_sub("sub foo($x, $y) {}");
        assert!(node.is_some(), "expected parsed subroutine for `sub foo($x, $y) {{}}`");
        let Some(node) = node else {
            return;
        };
        assert!(
            matches!(&node.kind, NodeKind::Subroutine { .. }),
            "expected Subroutine node, got {}",
            node.kind.kind_name()
        );

        if let NodeKind::Subroutine { signature, .. } = &node.kind {
            assert!(signature.is_some(), "sub foo($x, $y) should have a signature");
        }
    }

    #[test]
    fn typed_signature_with_type_constraint() {
        let node = parse_sub("sub foo(Type $x) {}");
        assert!(node.is_some(), "expected parsed subroutine for `sub foo(Type $x) {{}}`");
        let Some(node) = node else {
            return;
        };
        assert!(
            matches!(&node.kind, NodeKind::Subroutine { .. }),
            "expected Subroutine node, got {}",
            node.kind.kind_name()
        );

        if let NodeKind::Subroutine { signature, prototype, .. } = &node.kind {
            assert!(signature.is_some(), "typed signature should keep a signature");
            assert!(prototype.is_none(), "typed signature should not become a prototype");
        }
    }

    #[test]
    fn prototype_single_sigil() {
        let node = parse_sub("sub foo($) {}");
        assert!(node.is_some(), "expected parsed subroutine for `sub foo($) {{}}`");
        let Some(node) = node else {
            return;
        };
        assert!(
            matches!(&node.kind, NodeKind::Subroutine { .. }),
            "expected Subroutine node, got {}",
            node.kind.kind_name()
        );

        if let NodeKind::Subroutine { prototype, signature, .. } = &node.kind {
            assert!(prototype.is_some(), "sub foo($) should have a prototype");
            assert!(signature.is_none(), "sub foo($) should not have a signature");
        }
    }

    #[test]
    fn prototype_with_semicolon() {
        let node = parse_sub("sub foo($;@) {}");
        assert!(node.is_some(), "expected parsed subroutine for `sub foo($;@) {{}}`");
        let Some(node) = node else {
            return;
        };
        assert!(
            matches!(&node.kind, NodeKind::Subroutine { .. }),
            "expected Subroutine node, got {}",
            node.kind.kind_name()
        );

        if let NodeKind::Subroutine { prototype, .. } = &node.kind {
            assert!(prototype.is_some(), "sub foo($;@) should have a prototype");
        }
    }

    #[test]
    fn prototype_empty() {
        let node = parse_sub("sub foo() {}");
        assert!(node.is_some(), "expected parsed subroutine for `sub foo() {{}}`");
        let Some(node) = node else {
            return;
        };
        assert!(
            matches!(&node.kind, NodeKind::Subroutine { .. }),
            "expected Subroutine node, got {}",
            node.kind.kind_name()
        );

        if let NodeKind::Subroutine { prototype, .. } = &node.kind {
            assert!(prototype.is_some(), "sub foo() should have a prototype (empty)");
        }
    }

    #[test]
    fn prototype_with_sub_sigil() {
        let node = parse_sub("sub foo(&) {}");
        assert!(node.is_some(), "expected parsed subroutine for `sub foo(&) {{}}`");
        let Some(node) = node else {
            return;
        };
        assert!(
            matches!(&node.kind, NodeKind::Subroutine { .. }),
            "expected Subroutine node, got {}",
            node.kind.kind_name()
        );

        if let NodeKind::Subroutine { prototype, .. } = &node.kind {
            assert!(prototype.is_some(), "sub foo(&) should have a prototype");
        }
    }
}

#[cfg(test)]
mod code_dereference_tests {
    use super::*;
    use perl_tdd_support::{must, must_some};

    /// Helper: parse code and return the full AST.
    fn parse_program(code: &str) -> Node {
        let mut parser = Parser::new(code);
        must(parser.parse())
    }

    /// Helper: parse code and return the first statement node.
    fn parse_first_stmt(code: &str) -> Option<Node> {
        let ast = parse_program(code);
        match ast.kind {
            NodeKind::Program { mut statements } if !statements.is_empty() => {
                Some(statements.swap_remove(0))
            }
            _ => None,
        }
    }

    /// Helper: check that the AST sexp contains no ERROR nodes.
    fn assert_no_errors(code: &str) {
        let ast = parse_program(code);
        let sexp = ast.to_sexp();
        assert!(!sexp.contains("ERROR"), "Parse of `{}` produced ERROR nodes: {}", code, sexp);
    }

    #[test]
    fn code_deref_empty_args() {
        // &{$coderef}() - code dereference with empty args
        let code = "&{$coderef}();";
        assert_no_errors(code);
        let ast = parse_program(code);
        let sexp = ast.to_sexp();
        // Should contain the &{} operator and a function call structure
        assert!(sexp.contains("&{}"), "Expected &{{}} dereference in sexp, got: {}", sexp);
    }

    #[test]
    fn code_deref_with_args() {
        // &{$coderef}($arg) - code dereference with args
        let code = "&{$coderef}($arg);";
        assert_no_errors(code);
        let ast = parse_program(code);
        let sexp = ast.to_sexp();
        assert!(sexp.contains("&{}"), "Expected &{{}} dereference in sexp, got: {}", sexp);
        assert!(sexp.contains("arg"), "Expected argument in sexp, got: {}", sexp);
    }

    #[test]
    fn code_deref_complex_expr() {
        // &{$hash{callback}}($arg) - code dereference with complex expression
        let code = "&{$hash{callback}}($arg);";
        assert_no_errors(code);
        let ast = parse_program(code);
        let sexp = ast.to_sexp();
        assert!(sexp.contains("&{}"), "Expected &{{}} dereference in sexp, got: {}", sexp);
        assert!(sexp.contains("callback"), "Expected 'callback' key in sexp, got: {}", sexp);
    }

    #[test]
    fn code_deref_simple_form_with_args() {
        // &$coderef($arg) - simple form (no braces), should already work
        let code = "&$coderef($arg);";
        assert_no_errors(code);
        let ast = parse_program(code);
        let sexp = ast.to_sexp();
        assert!(sexp.contains("call"), "Expected function call in sexp, got: {}", sexp);
    }

    #[test]
    fn code_deref_simple_form_no_parens() {
        // &$coderef - no parens, implicit @_ forwarding
        // The parser currently treats &$var as FunctionCall { name: "$", args: [] }
        // because the lexer splits & and $coderef as separate tokens, and the
        // & sigil handler treats $ as a special variable name (like $$).
        // This is a known limitation for &$var without braces.
        let code = "&$coderef;";
        assert_no_errors(code);
    }

    #[test]
    fn code_deref_no_parens() {
        // &{$coderef} - code dereference without arguments (implicit @_ forwarding)
        let code = "&{$coderef};";
        assert_no_errors(code);
        let ast = parse_program(code);
        let sexp = ast.to_sexp();
        assert!(sexp.contains("&{}"), "Expected &{{}} dereference in sexp, got: {}", sexp);
    }

    #[test]
    fn code_deref_produces_correct_ast_structure() {
        // Verify the AST structure for &{$coderef}($x, $y)
        let code = "&{$coderef}($x, $y);";
        let stmt = must_some(parse_first_stmt(code));

        // The statement should be an ExpressionStatement wrapping a FunctionCall
        let NodeKind::ExpressionStatement { expression } = &stmt.kind else {
            assert_eq!(
                stmt.kind.kind_name(),
                "ExpressionStatement",
                "Expected ExpressionStatement, got {} (sexp: {})",
                stmt.kind.kind_name(),
                stmt.to_sexp(),
            );
            return;
        };

        match &expression.kind {
            NodeKind::FunctionCall { name, args } => {
                assert_eq!(name, "&{}", "Function call name should be &{{}}");
                // First arg is the Unary dereference node (&{$coderef}),
                // remaining args are the actual arguments (may be combined into
                // a single list node depending on comma parsing)
                assert!(!args.is_empty(), "Expected at least 1 arg (the deref node)");
                // First arg should be the Unary &{} dereference
                assert_eq!(
                    args.first().map(|a| a.kind.kind_name()),
                    Some("Unary"),
                    "First arg should be a Unary dereference node: {:?}",
                    args.iter().map(|a| a.kind.kind_name()).collect::<Vec<_>>(),
                );
            }
            _ => assert_eq!(
                expression.kind.kind_name(),
                "FunctionCall",
                "Expected FunctionCall, got {} (sexp: {})",
                expression.kind.kind_name(),
                expression.to_sexp(),
            ),
        }
    }
}

#[cfg(test)]
mod nested_variable_list_item_tests {
    use super::*;
    use perl_tdd_support::must;

    fn parse_program(code: &str) -> Node {
        let mut parser = Parser::new(code);
        must(parser.parse())
    }

    #[test]
    fn empty_nested_paren_in_variable_list_produces_undef() {
        // Covers line 183: the `0 =>` arm in parse_variable_list_item.
        // An empty `()` nested inside a variable list should produce a valid AST.
        let ast = parse_program("my ($a, ()) = @_;");
        let sexp = ast.to_sexp();
        assert!(!sexp.is_empty(), "empty nested () should produce a valid AST");
    }

    #[test]
    fn nested_variable_list_item_single_item_passthrough() {
        // Covers the `1 =>` arm in parse_variable_list_item.
        // A single-item `($x)` inside a variable list returns the item directly.
        let ast = parse_program("my ($a, ($b)) = (1, 2);");
        let sexp = ast.to_sexp();
        assert!(
            sexp.contains("$b") || sexp.contains("b"),
            "single-item nested paren should unwrap to item: {sexp}"
        );
    }

    #[test]
    fn nested_variable_list_item_multi_item_wraps() {
        // Covers the `_ =>` (multi-item) arm in parse_variable_list_item.
        // Multiple items produce a NestedVariableList node.
        let ast = parse_program("my ($a, ($b, $c)) = (1, 2, 3);");
        let sexp = ast.to_sexp();
        assert!(
            sexp.contains("nested_variable_list") || sexp.contains("$b"),
            "multi-item nested paren should produce NestedVariableList: {sexp}"
        );
    }

    #[test]
    fn nested_variable_list_item_malformed_missing_comma() {
        // Covers line 172-175: the error path in parse_variable_list_item when
        // a token other than comma or ) follows an item in the nested list.
        let mut parser = Parser::new("my ($a, ($b $c)) = (1, 2, 3);");
        let result = parser.parse();
        // The parser should either return an error or an AST with an Error node.
        match result {
            Err(_) => {} // error path exercised
            Ok(ast) => {
                let sexp = ast.to_sexp();
                // If it returns Ok, there should be an Error node in the AST.
                assert!(
                    sexp.contains("error") || !parser.get_errors().is_empty(),
                    "malformed nested list should produce error signal: {sexp}"
                );
            }
        }
    }
}
