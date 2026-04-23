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
                // `undef` is valid as a placeholder in list destructuring:
                //   my ($self, undef, $src) = @_;
                let var = if self.peek_kind() == Some(TokenKind::Undef) {
                    let undef_token = self.consume_token()?;
                    Node::new(
                        NodeKind::Undef,
                        SourceLocation { start: undef_token.start, end: undef_token.end },
                    )
                } else {
                    self.parse_variable()?
                };

                // Parse optional attributes for this specific variable
                let mut var_attributes = Vec::new();
                while self.peek_kind() == Some(TokenKind::Colon) {
                    self.tokens.next()?; // consume colon
                    let attr_token = self.expect(TokenKind::Identifier)?;
                    var_attributes.push(attr_token.text.to_string());
                }

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
            // For 'local', parse only the localized target expression, not an
            // assignment expression. The initializer is handled below via
            // `assign_op` parsing, and consuming assignment here would swallow
            // the RHS into `variable` and leave a `MissingExpression`.
            //
            // e.g. `local $SIG{__WARN__} = sub { ... };`
            let variable = if declarator == "local" {
                self.parse_ternary()?
            } else {
                // For my/our/state, parse a simple variable
                let var = self.parse_variable()?;
                // If -> follows the declared variable, treat it as an lvalue subscript chain
                // e.g. my $cache->{key} = expr  or  my $foo->method()
                if self.peek_kind() == Some(TokenKind::Arrow) {
                    self.parse_postfix_chain(var)?
                } else {
                    var
                }
            };

            // Parse optional attributes
            let mut attributes = Vec::new();
            while self.peek_kind() == Some(TokenKind::Colon) {
                self.tokens.next()?; // consume colon
                let attr_token = self.expect(TokenKind::Identifier)?;
                attributes.push(attr_token.text.to_string());
            }

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

    /// Parse local statement (can localize any lvalue, not just simple variables)
    fn parse_local_statement(&mut self) -> ParseResult<Node> {
        let start = self.current_position();
        let declarator_token = self.consume_token()?; // consume 'local'
        let declarator = declarator_token.text.to_string();

        // Parse only the localized target expression (lhs), not an assignment.
        // `local $SIG{__WARN__} = sub { ... };` should keep the assignment RHS
        // in `initializer` below.
        let variable = Box::new(self.parse_ternary()?);

        let initializer = if self.peek_kind() == Some(TokenKind::Assign) {
            self.tokens.next()?; // consume =
            Some(Box::new(self.parse_assignment()?))
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
            let expr = self.parse_expression()?;

            self.expect(TokenKind::RightBrace)?;
            let end = self.previous_position();

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

        // Handle sigil + partial deref: when the lexer produces e.g. `%{shift` as one
        // token (name starts with `{` but doesn't end with `}`), this is a dereference
        // expression like `%{shift()}` where the lexer consumed `%{shift` greedily.
        // We need to create the inner expression from the identifier after `{`, parse
        // any trailing postfix (like `()` for function calls), then expect `}`.
        if name.starts_with('{') && !name.ends_with('}') {
            let inner_name = &name[1..]; // strip leading {
            let inner_start = token.start + sigil.len() + 1; // after sigil and {
            let inner_end = token.end;

            // Create an identifier node for the captured name
            let mut inner = Node::new(
                NodeKind::Identifier { name: inner_name.to_string() },
                SourceLocation { start: inner_start, end: inner_end },
            );

            // Parse postfix chain (handles function call parens, method calls, etc.)
            inner = self.parse_postfix_chain(inner)?;

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
                self.expect(TokenKind::RightBrace)?;
                let brace_end = self.previous_position();
                return Ok(Node::new(
                    NodeKind::Unary { op: "$#".to_string(), operand: Box::new(inner) },
                    SourceLocation { start: token.start, end: brace_end },
                ));
            }
        }

        // The lexer may hand us a sigil-only token (`&`) or a precombined `$$`
        // token followed by the referenced identifier. Preserve the full target
        // name instead of leaving the tail as a stray identifier node.
        if (full_name.is_empty() || (sigil == "$" && full_name == "$"))
            && self.peek_kind() == Some(TokenKind::Identifier)
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
        } else {
            Ok(Node::new(
                NodeKind::Variable { sigil, name: full_name },
                SourceLocation { start: token.start, end },
            ))
        }
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

        let (name, end) = if next_kind == Some(TokenKind::Identifier) ||
                             // Keywords can be used as variable names with any sigil
                             // e.g., %try, $default, @for, &try are all valid Perl
                             matches!(next_kind, Some(k) if Self::can_be_sub_name(k))
        {
            let name_token = self.tokens.next()?;
            let mut name = name_token.text.to_string();
            let mut end = name_token.end;

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
                    if self.peek_kind() == Some(TokenKind::Identifier) {
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
                    ("::".to_string(), dc_token.end)
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
            let expr = self.parse_expression()?;

            self.expect(TokenKind::RightBrace)?;
            let end = self.previous_position();

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

        while self.peek_kind() != Some(TokenKind::RightParen) && !self.tokens.is_eof() {
            // Parse parameter
            let param = self.parse_signature_param()?;
            params.push(param);

            // Check for comma or end of signature
            if self.peek_kind() == Some(TokenKind::Comma) {
                self.tokens.next()?; // consume comma
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

        // Check for default value (= expression)
        let default_value = if self.peek_kind() == Some(TokenKind::Assign) {
            self.tokens.next()?; // consume =
            // Parse a primary expression for default value to avoid parsing too far
            Some(Box::new(self.parse_primary()?))
        } else {
            None
        };

        let end = if let Some(ref default) = default_value {
            default.location.end
        } else {
            variable.location.end
        };

        // Check if variable is slurpy (@args or %hash)
        let is_slurpy = matches!(&variable.kind, NodeKind::Variable { sigil, .. } if sigil == "@" || sigil == "%");

        // Create the appropriate parameter node type
        let param_kind = if named {
            NodeKind::NamedParameter { variable: Box::new(variable) }
        } else if is_slurpy {
            NodeKind::SlurpyParameter { variable: Box::new(variable) }
        } else if let Some(default) = default_value {
            NodeKind::OptionalParameter { variable: Box::new(variable), default_value: default }
        } else {
            NodeKind::MandatoryParameter { variable: Box::new(variable) }
        };

        Ok(Node::new(param_kind, SourceLocation { start, end }))
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
        // Perl only allows: $ @ % & * \ ; + _ and ASCII space.
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
}

/// Return `true` if `c` is a character that Perl permits in old-style prototypes.
///
/// Valid characters (from perlsub):
/// `$` `@` `%` `&` `*` `\` `;` `+` `_` and ASCII space.
fn is_valid_prototype_char(c: char) -> bool {
    matches!(c, '$' | '@' | '%' | '&' | '*' | '\\' | ';' | '+' | '_' | ' ')
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
        assert!(!sexp.contains("ERROR"), "Parse of `{}` produced ERROR nodes: {}", code, sexp,);
    }

    #[test]
    fn code_deref_empty_args() {
        // &{$coderef}() - code dereference with empty args
        let code = "&{$coderef}();";
        assert_no_errors(code);
        let ast = parse_program(code);
        let sexp = ast.to_sexp();
        // Should contain the &{} operator and a function call structure
        assert!(sexp.contains("&{}"), "Expected &{{}} dereference in sexp, got: {}", sexp,);
    }

    #[test]
    fn code_deref_with_args() {
        // &{$coderef}($arg) - code dereference with args
        let code = "&{$coderef}($arg);";
        assert_no_errors(code);
        let ast = parse_program(code);
        let sexp = ast.to_sexp();
        assert!(sexp.contains("&{}"), "Expected &{{}} dereference in sexp, got: {}", sexp,);
        assert!(sexp.contains("arg"), "Expected argument in sexp, got: {}", sexp,);
    }

    #[test]
    fn code_deref_complex_expr() {
        // &{$hash{callback}}($arg) - code dereference with complex expression
        let code = "&{$hash{callback}}($arg);";
        assert_no_errors(code);
        let ast = parse_program(code);
        let sexp = ast.to_sexp();
        assert!(sexp.contains("&{}"), "Expected &{{}} dereference in sexp, got: {}", sexp,);
        assert!(sexp.contains("callback"), "Expected 'callback' key in sexp, got: {}", sexp,);
    }

    #[test]
    fn code_deref_simple_form_with_args() {
        // &$coderef($arg) - simple form (no braces), should already work
        let code = "&$coderef($arg);";
        assert_no_errors(code);
        let ast = parse_program(code);
        let sexp = ast.to_sexp();
        assert!(sexp.contains("call"), "Expected function call in sexp, got: {}", sexp,);
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
        assert!(sexp.contains("&{}"), "Expected &{{}} dereference in sexp, got: {}", sexp,);
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
                assert!(!args.is_empty(), "Expected at least 1 arg (the deref node)",);
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
