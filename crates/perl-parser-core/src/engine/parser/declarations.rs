impl<'a> Parser<'a> {
    fn parse_qualified_name(&mut self, allow_trailing_separator: bool) -> ParseResult<(String, SourceLocation)> {
        // Accept keywords as package names (e.g., `package if;`, `package next;`)
        // Same pattern as parse_subroutine — keywords are valid barewords in Perl
        let first = if self.peek_kind().is_some_and(Self::can_be_sub_name) {
            self.consume_token()?
        } else {
            self.expect(TokenKind::Identifier)?
        };
        let mut name = first.text.to_string();
        let name_start = first.start;
        let mut name_end = first.end;

        if !allow_trailing_separator && name.ends_with("::") {
            return Err(ParseError::UnexpectedToken {
                expected: "identifier after ::".to_string(),
                found: "class body".to_string(),
                location: name_end,
            });
        }

        while self.peek_kind() == Some(TokenKind::DoubleColon)
            || (self.peek_kind() == Some(TokenKind::Colon)
                && self.tokens.peek_second().map(|t| t.kind) == Ok(TokenKind::Colon))
        {
            if self.peek_kind() == Some(TokenKind::DoubleColon) {
                let double_colon = self.tokens.next()?; // consume ::
                name_end = double_colon.end;
            } else {
                self.tokens.next()?; // consume first :
                let second_colon = self.tokens.next()?; // consume second :
                name_end = second_colon.end;
            }

            name.push_str("::");

            if self.peek_kind().is_some_and(Self::can_be_sub_name) {
                let next = self.consume_token()?;
                name_end = next.end;
                name.push_str(&next.text);
            } else if allow_trailing_separator {
                break;
            } else {
                return Err(ParseError::UnexpectedToken {
                    expected: "identifier after ::".to_string(),
                    found: self
                        .peek_kind()
                        .map(|kind| kind.display_name().to_string())
                        .unwrap_or_else(|| "EOF".to_string()),
                    location: self.current_position(),
                });
            }
        }

        Ok((name, SourceLocation { start: name_start, end: name_end }))
    }

    /// Built-in subroutine attributes defined by perlsub / the Perl core.
    ///
    /// Attributes not in this list are valid only if the caller has loaded the
    /// `attributes` pragma or a framework that installs a custom `MODIFY_CODE_ATTRIBUTES`
    /// handler.  Unknown attributes are warned about (pushed to `self.errors`) but do
    /// **not** cause a hard parse failure — custom attribute usage is widespread in
    /// CPAN code (e.g. Moose `:ro`, Catalyst `:Private`).
    const BUILTIN_SUB_ATTRIBUTES: &'static [&'static str] = &["lvalue", "method", "prototype", "const"];

    /// Built-in class-level attributes defined by Perl 5.38+ `use feature 'class'`.
    ///
    /// These are valid on `class` declarations (not subroutines) and must not be
    /// warned about when parsing class headers.  `:isa(Parent)` declares inheritance;
    /// `:does(Role)` is reserved for future Perl versions.
    const BUILTIN_CLASS_ATTRIBUTES: &'static [&'static str] = &["isa", "does"];

    /// Return `true` if `name` is a known built-in subroutine attribute.
    fn is_builtin_sub_attribute(name: &str) -> bool {
        Self::BUILTIN_SUB_ATTRIBUTES.contains(&name)
    }

    /// Return `true` if `Attribute::Handlers` has been enabled in this file.
    fn has_attribute_handlers_support(&self) -> bool {
        self.attribute_handlers_enabled
    }

    /// Register a custom attribute handler for later `:MyAttr` uses.
    fn register_custom_attribute_handler(&mut self, handler: &str) {
        if self.has_attribute_handlers_support() {
            self.custom_attribute_handlers.insert(handler.to_string());
        }
    }

    /// Return `true` if `name` was registered as a custom Attribute::Handlers attribute.
    fn is_custom_attribute_handler(&self, name: &str) -> bool {
        self.has_attribute_handlers_support() && self.custom_attribute_handlers.contains(name)
    }

    /// Parse declaration attributes like `:lvalue` or `:prototype($)`.
    ///
    /// `extra_known` lists additional attribute names that should not trigger the
    /// "unknown subroutine attribute" warning — used by `parse_class` to whitelist
    /// class-level attributes such as `:isa(Parent)`.
    ///
    /// Unknown attributes produce a soft warning pushed to `self.errors` but
    /// parsing continues — custom attributes are legal in Perl via the
    /// `attributes` module or framework hooks.
    fn parse_declaration_attributes_with_extras(
        &mut self,
        extra_known: &[&str],
    ) -> ParseResult<Vec<String>> {
        let mut attributes = Vec::new();

        while self.peek_kind() == Some(TokenKind::Colon) {
            self.tokens.next()?; // consume colon
            let mut parsed_any = false;

            loop {
                let attr_token = match self.peek_kind() {
                    Some(TokenKind::Identifier | TokenKind::Method) => self.tokens.next()?,
                    _ if parsed_any => break,
                    _ => {
                        return Err(ParseError::syntax(
                            "Expected attribute name after ':'",
                            self.current_position(),
                        ));
                    }
                };
                parsed_any = true;

                let base_name = attr_token.text.to_string();
                let attr_start = attr_token.start;
                let mut attr_name = base_name.clone();

                if self.peek_kind() == Some(TokenKind::LeftParen) {
                    self.consume_token()?; // consume (
                    attr_name.push('(');

                    let mut paren_depth = 1;
                    while paren_depth > 0 && !self.tokens.is_eof() {
                        let token = self.tokens.next()?;
                        attr_name.push_str(&token.text);

                        match token.kind {
                            TokenKind::LeftParen => paren_depth += 1,
                            TokenKind::RightParen => paren_depth -= 1,
                            _ => {}
                        }
                    }

                    if paren_depth != 0 {
                        return Err(ParseError::syntax(
                            "Unterminated attribute argument list",
                            self.current_position(),
                        ));
                    }
                }

                // Warn (but do not error) if the attribute is not a known built-in
                // and not an extra-known context-specific attribute.
                // Custom attributes are valid when `attributes` or a framework hook is
                // in scope, so a warning is the correct diagnostic level.
                let is_known = Self::is_builtin_sub_attribute(&base_name)
                    || extra_known.contains(&base_name.as_str())
                    || self.is_custom_attribute_handler(&base_name)
                    || (self.has_attribute_handlers_support() && base_name == "ATTR");
                if !is_known {
                    self.errors.push(ParseError::syntax(
                        format!(
                            "unknown subroutine attribute ':{base_name}'; \
                             did you mean one of: lvalue, method, prototype, const?"
                        ),
                        attr_start,
                    ));
                }

                attributes.push(attr_name);

                match self.peek_kind() {
                    Some(TokenKind::Identifier | TokenKind::Method) => continue,
                    _ => break,
                }
            }
        }

        Ok(attributes)
    }

    /// Parse declaration attributes like `:lvalue` or `:prototype($)`.
    ///
    /// Convenience wrapper for the common subroutine/method case (no extra-known attributes).
    fn parse_declaration_attributes(&mut self) -> ParseResult<Vec<String>> {
        self.parse_declaration_attributes_with_extras(&[])
    }

    /// Parse subroutine definition
    fn parse_subroutine(&mut self) -> ParseResult<Node> {
        let start = self.current_position();
        self.tokens.next()?; // consume 'sub'

        let (name, name_span) = if self.peek_kind() == Some(TokenKind::DoubleColon) {
            // Leading :: qualifier — subroutine in the main package (e.g., sub ::PCDATA { })
            let dc_token = self.tokens.next()?; // consume '::'
            let name_start = dc_token.start;
            if self.peek_kind().is_some_and(Self::can_be_sub_name) {
                // sub ::PCDATA or sub ::DB_File::splice
                let ident_token = self.tokens.next()?;
                let full_name = format!("::{}", ident_token.text);
                (
                    Some(full_name),
                    Some(SourceLocation { start: name_start, end: ident_token.end }),
                )
            } else {
                // sub :: with no following name — treat as name "::"
                (
                    Some("::".to_string()),
                    Some(SourceLocation { start: name_start, end: dc_token.end }),
                )
            }
        } else if self.peek_kind().is_some_and(Self::can_be_sub_name) {
            let token = self.tokens.next()?;
            (
                Some(token.text.to_string()),
                Some(SourceLocation { start: token.start, end: token.end }),
            )
        } else {
            // No name - anonymous subroutine (next token is {, (, :, or similar)
            (None, None)
        };

        // Parse optional attributes before the prototype/signature.
        // Perl allows both `sub foo :lvalue ($)` and `sub foo ($) :lvalue`,
        // so we collect attributes on both sides and merge them.
        let mut attributes = self.parse_declaration_attributes()?;

        if let Some(handler_name) = name.as_deref() {
            if attributes.iter().any(|attr| attr.starts_with("ATTR(") || attr == "ATTR") {
                self.register_custom_attribute_handler(handler_name);
            }
        }

        // Parse optional prototype or signature after leading attributes.
        let (prototype, signature) = if self.peek_kind() == Some(TokenKind::LeftParen) {
            // Look ahead to determine if this is a prototype or signature
            if self.is_likely_prototype()? {
                // Parse as prototype
                let proto_start = self.current_position();
                let proto_content = self.parse_prototype()?;
                let proto_node = Node::new(
                    NodeKind::Prototype { content: proto_content },
                    SourceLocation { start: proto_start, end: self.previous_position() },
                );
                (Some(Box::new(proto_node)), None)
            } else {
                // Parse as signature
                let sig_start = self.current_position();
                let params = self.parse_signature()?;
                let sig_node = Node::new(
                    NodeKind::Signature { parameters: params },
                    SourceLocation { start: sig_start, end: self.previous_position() },
                );
                (None, Some(Box::new(sig_node)))
            }
        } else {
            (None, None)
        };

        // Parse optional trailing attributes after the prototype/signature.
        if self.peek_kind() == Some(TokenKind::Colon) {
            attributes.extend(self.parse_declaration_attributes()?);
        }

        // Check for forward declaration: sub foo; or sub foo(@); or sub foo :method;
        // Forward declarations have no block body — they end with a semicolon
        let body = if self.peek_kind() == Some(TokenKind::Semicolon) {
            // Forward declaration — return an empty block as the body
            let pos = self.current_position();
            Node::new(
                NodeKind::Block { statements: vec![] },
                SourceLocation { start: pos, end: pos },
            )
        } else {
            self.parse_block()?
        };

        let end = self.previous_position();
        Ok(Node::new(
            NodeKind::Subroutine {
                name,
                name_span,
                prototype,
                signature,
                attributes,
                body: Box::new(body),
            },
            SourceLocation { start, end },
        ))
    }

    /// Parse class declaration (Perl 5.38+)
    ///
    /// Handles the full syntax: `class Name :isa(Parent) :isa(Parent2) { ... }`
    /// Multiple `:isa(...)` attributes may appear; each contributes a parent class.
    fn parse_class(&mut self) -> ParseResult<Node> {
        let start = self.current_position();
        self.tokens.next()?; // consume 'class'

        let (name, _) = self.parse_qualified_name(false)?;

        // Parse class-level attributes (e.g. `:isa(Parent)`).
        // Pass BUILTIN_CLASS_ATTRIBUTES so `:isa` and `:does` don't produce
        // "unknown subroutine attribute" warnings.
        let attributes =
            self.parse_declaration_attributes_with_extras(Self::BUILTIN_CLASS_ATTRIBUTES)?;

        // Extract parent class names from `:isa(Parent)` attributes.
        // `parse_declaration_attributes` stores `:isa(Parent)` as "isa(Parent)".
        let parents: Vec<String> = attributes
            .iter()
            .filter_map(|attr| {
                let trimmed = attr.trim();
                if let Some(inner) = trimmed.strip_prefix("isa(") {
                    // inner is "Parent)" — drop trailing ')'
                    inner.strip_suffix(')').map(|s| s.trim().to_string())
                } else {
                    None
                }
            })
            .filter(|s| !s.is_empty())
            .collect();

        self.in_class_body += 1;
        let body = self.parse_block();
        self.in_class_body -= 1;
        let body = body?;

        let end = self.previous_position();
        Ok(Node::new(
            NodeKind::Class { name, parents, body: Box::new(body) },
            SourceLocation { start, end },
        ))
    }

    /// Parse method declaration (Perl 5.38+)
    fn parse_method(&mut self) -> ParseResult<Node> {
        let start = self.current_position();
        self.tokens.next()?; // consume 'method'

        let name_token = self.expect(TokenKind::Identifier)?;
        let name = name_token.text.to_string();

        let attributes = self.parse_declaration_attributes()?;

        // Parse optional signature
        let signature = if self.peek_kind() == Some(TokenKind::LeftParen) {
            let sig_start = self.current_position();
            let params = self.parse_signature()?;
            Some(Box::new(Node::new(
                NodeKind::Signature { parameters: params },
                SourceLocation { start: sig_start, end: self.previous_position() },
            )))
        } else {
            None
        };

        let body = self.parse_block()?;

        let end = self.previous_position();
        Ok(Node::new(
            NodeKind::Method { name, signature, attributes, body: Box::new(body) },
            SourceLocation { start, end },
        ))
    }

    /// Parse an Object::Pad `ADJUST` block as a method-like class body node.
    fn parse_adjust_block(&mut self) -> ParseResult<Node> {
        let start = self.current_position();
        self.tokens.next()?; // consume 'ADJUST'

        let body = self.parse_block()?;

        let end = self.previous_position();
        Ok(Node::new(
            NodeKind::Method {
                name: "ADJUST".to_string(),
                signature: None,
                attributes: Vec::new(),
                body: Box::new(body),
            },
            SourceLocation { start, end },
        ))
    }

    /// Parse format declaration
    fn parse_format(&mut self) -> ParseResult<Node> {
        let start = self.current_position();
        self.tokens.next()?; // consume 'format'

        // Parse format name (optional - can be anonymous)
        let name = if self.peek_kind() == Some(TokenKind::Assign) {
            // Anonymous format
            String::new()
        } else {
            // Named format
            let name_token = self.expect(TokenKind::Identifier)?;
            name_token.text.to_string()
        };

        // Expect =
        self.expect(TokenKind::Assign)?;

        // Tell the lexer to enter format body mode
        self.tokens.enter_format_mode();

        // Get the format body
        let body_token = self.tokens.next()?;
        let body = if body_token.kind == TokenKind::FormatBody {
            body_token.text.to_string()
        } else {
            return Err(ParseError::UnexpectedToken {
                expected: "format body".to_string(),
                found: body_token.kind.display_name().to_string(),
                location: body_token.start,
            });
        };

        let end = self.previous_position();
        Ok(Node::new(NodeKind::Format { name, body }, SourceLocation { start, end }))
    }

    /// Parse package declaration
    fn parse_package(&mut self) -> ParseResult<Node> {
        let start = self.current_position();
        self.tokens.next()?; // consume 'package'

        let (mut name, name_span) = self.parse_qualified_name(true)?;

        // Check for optional version number or v-string
        let version = if self.peek_kind() == Some(TokenKind::Number) {
            Some(self.tokens.next()?.text.to_string())
        } else if let Some(TokenKind::Identifier) = self.peek_kind() {
            // Check if it's a v-string version
            if let Ok(token) = self.tokens.peek() {
                if token.text.starts_with('v') && token.text.len() > 1 {
                    // It's a v-string like v1 or v5
                    let mut version_str = self.tokens.next()?.text.to_string();

                    // Collect the rest of the v-string (e.g., .2.3)
                    while let Some(TokenKind::Number) = self.peek_kind() {
                        if let Ok(num_token) = self.tokens.peek() {
                            if num_token.text.starts_with('.') {
                                version_str.push_str(&self.tokens.next()?.text);
                            } else {
                                break;
                            }
                        } else {
                            break;
                        }
                    }

                    Some(version_str)
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };

        // If we have a version, append it to the name for now
        // (In a real AST, you'd probably want these as separate fields)
        if let Some(ver) = version {
            name.push(' ');
            name.push_str(&ver);
        }

        let block = if self.peek_kind() == Some(TokenKind::LeftBrace) {
            Some(Box::new(self.parse_block()?))
        } else {
            // Don't consume semicolon here - let parse_statement handle it uniformly
            None
        };

        let end = self.previous_position();
        Ok(Node::new(NodeKind::Package { name, name_span, block }, SourceLocation { start, end }))
    }

    /// Parse use statement
    fn parse_use(&mut self) -> ParseResult<Node> {
        let start = self.current_position();
        self.consume_token()?; // consume 'use'

        // Parse module name, version, or identifier
        let mut module = if matches!(
            self.peek_kind(),
            Some(TokenKind::Number) | Some(TokenKind::VString)
        ) {
            // Numeric version like 5.036 or v-string like v5.14, v5.12.0
            self.consume_token()?.text.to_string()
        } else {
            let first_token = self.consume_token()?;

            // Check for version strings
            if first_token.kind == TokenKind::Identifier
                && first_token.text.starts_with('v')
                && first_token.text.chars().skip(1).all(|c| c.is_numeric())
            {
                // Version identifier like v5 or v536
                let mut version = first_token.text.to_string();

                // Check if followed by dot and more numbers (e.g., v5.36)
                if self.peek_kind() == Some(TokenKind::Unknown) {
                    if let Ok(dot_token) = self.tokens.peek() {
                        if dot_token.text.as_ref() == "." {
                            self.consume_token()?; // consume dot
                            if self.peek_kind() == Some(TokenKind::Number) {
                                let num = self.consume_token()?;
                                version.push('.');
                                version.push_str(&num.text);
                            }
                        }
                    }
                }
                version
            } else if first_token.text.as_ref() == "v" && self.peek_kind() == Some(TokenKind::Number) {
                // Version string like v5.36 (tokenized as "v" followed by number)
                let version = self.expect(TokenKind::Number)?;
                format!("v{}", version.text)
            } else if first_token.kind == TokenKind::Identifier {
                first_token.text.to_string()
            } else if first_token.text.chars().all(|c| c.is_alphanumeric() || c == '_') && !first_token.text.is_empty() {
                // Keyword-named pragmas: `use if COND, MODULE`, `use unless COND, MODULE`, etc.
                // The token kind is a keyword (e.g., TokenKind::If) but the text is a valid
                // Perl module name (all word chars). Accept it as-is.
                first_token.text.to_string()
            } else {
                return Err(ParseError::syntax(
                    format!("Expected module name or version, found {}", first_token.kind.display_name()),
                    first_token.start,
                ));
            }
        };

        // Handle :: in module names
        // Handle both DoubleColon tokens and separate Colon tokens (in case lexer sends :: as separate colons)
        while self.peek_kind() == Some(TokenKind::DoubleColon)
            || (self.peek_kind() == Some(TokenKind::Colon)
                && self.tokens.peek_second().map(|t| t.kind) == Ok(TokenKind::Colon))
        {
            if self.peek_kind() == Some(TokenKind::DoubleColon) {
                self.consume_token()?; // consume ::
                module.push_str("::");
            } else {
                // Handle two separate Colon tokens as ::
                self.consume_token()?; // consume first :
                self.consume_token()?; // consume second :
                module.push_str("::");
            }
            // In Perl, trailing :: is valid (e.g., Foo::Bar::)
            // Only consume identifier if there is one
            if self.peek_kind() == Some(TokenKind::Identifier) {
                let next_part = self.consume_token()?;
                module.push_str(&next_part.text);
            }
            // No error for trailing :: - it's valid in Perl
        }

        // Parse optional version number
        if self.peek_kind() == Some(TokenKind::Number) {
            module.push(' ');
            module.push_str(&self.consume_token()?.text);
        }

        if module.split_whitespace().next() == Some("Attribute::Handlers") {
            self.attribute_handlers_enabled = true;
        }

        // `use if CONDITION, MODULE [, ARGS]` and `use unless ...` are special:
        // the condition is an arbitrary expression that may contain braces
        // (e.g. `eval { ... }`), so we need brace-depth-aware token consumption.
        if module == "if" || module == "unless" {
            let mut cond_args = Vec::new();
            let mut brace_depth: usize = 0;
            while !self.tokens.is_eof() {
                // At brace depth 0, stop at statement terminators
                if brace_depth == 0 && Self::is_statement_terminator(self.peek_kind()) {
                    break;
                }
                // At brace depth 0, commas separate arguments — consume the comma
                // and continue to collect remaining args
                if self.peek_kind() == Some(TokenKind::Comma) && brace_depth == 0 {
                    self.consume_token()?; // consume ','
                    continue;
                }
                match self.peek_kind() {
                    Some(TokenKind::LeftBrace) => {
                        brace_depth = brace_depth.saturating_add(1);
                    }
                    Some(TokenKind::RightBrace) => {
                        brace_depth = brace_depth.saturating_sub(1);
                    }
                    _ => {}
                }
                cond_args.push(self.consume_token()?.text.to_string());
            }
            let end = self.previous_position();
            let has_filter_risk = Self::is_filter_module(&module);
            return Ok(Node::new(
                NodeKind::Use {
                    module,
                    args: cond_args,
                    has_filter_risk,
                    // `use if COND, MODULE, ARGS` does not use parens-style import lists;
                    // args here are the conditional expression and module args, not a parenthesized import list.
                    has_explicit_import_list: false,
                },
                SourceLocation { start, end },
            ));
        }

        // Parse optional import list
        let mut args = Vec::new();
        // Track whether we saw `(`...`)` parens around the import list. Both
        // `use Module ()` (empty parens) and `use Module (foo)` set this to true,
        // distinguishing them from bare `use Module;` (no parens — auto-import @EXPORT).
        let mut has_explicit_import_list = false;

        // Loop to handle multiple argument groups separated by commas
        // e.g., qw(FOO) => 1, qw(BAR BAZ) => 2
        loop {
            // Special case: ALWAYS check for qw FIRST before any other parsing
            // Check if next token is "qw" - this is critical to handle before bare args
            let is_qw = self.tokens.peek().map(|t| t.text.as_ref() == "qw").unwrap_or(false);
            if is_qw {
                self.consume_token()?; // consume 'qw'

                // Try to parse qw words, but if it fails (e.g., unknown delimiter),
                // fall back to simple token consumption
                let list = match self.parse_qw_words() {
                    Ok(words) => words,
                    Err(_) => {
                        // Fallback: just consume tokens until semicolon
                        let mut words = Vec::new();
                        while !Self::is_statement_terminator(self.peek_kind())
                            && !self.tokens.is_eof()
                        {
                            if let Ok(tok) = self.tokens.next() {
                                if matches!(tok.kind, TokenKind::Identifier | TokenKind::Number) {
                                    words.push(tok.text.to_string());
                                }
                            } else {
                                break;
                            }
                        }
                        words
                    }
                };
                // Format as "qw(FOO BAR BAZ)" so DeclarationProvider can recognize it
                // We use parentheses regardless of original delimiter for consistency
                let qw_str = format!("qw({})", list.join(" "));
                args.push(qw_str);
                // optional: qw(...) => <value>
                if self.peek_kind() == Some(TokenKind::FatArrow) {
                    self.consume_token()?; // =>
                    if let Some(TokenKind::String | TokenKind::Number | TokenKind::Identifier) =
                        self.peek_kind()
                    {
                        args.push(self.consume_token()?.text.to_string());
                    } else {
                        // best-effort: slurp tokens until ',' or ';'
                        while !Self::is_statement_terminator(self.peek_kind())
                            && self.peek_kind() != Some(TokenKind::Comma)
                        {
                            args.push(self.consume_token()?.text.to_string());
                        }
                    }
                }
                // Check if there's a comma and more args
                if self.peek_kind() == Some(TokenKind::Comma) {
                    self.consume_token()?; // consume ','
                    continue; // Loop to parse next argument group
                } else {
                    // No more args, we're done
                    break;
                }
            } else {
                // Not qw, break out to handle other argument types
                break;
            }
        }

        // Handle unary plus forcing hash syntax: use constant +{ FOO => 42 }
        if self.peek_kind() == Some(TokenKind::Plus) {
            let plus = self.consume_token()?;
            args.push(plus.text.to_string());
            // Next should be a hash
            if self.peek_kind() == Some(TokenKind::LeftBrace) {
                // Consume the hash expression
                let mut depth = 0;
                while !self.tokens.is_eof() {
                    match self.peek_kind() {
                        Some(TokenKind::LeftBrace) => {
                            depth += 1;
                            args.push(self.consume_token()?.text.to_string());
                        }
                        Some(TokenKind::RightBrace) => {
                            args.push(self.consume_token()?.text.to_string());
                            depth -= 1;
                            if depth == 0 {
                                break;
                            }
                        }
                        _ => {
                            args.push(self.consume_token()?.text.to_string());
                        }
                    }
                }
            }
        }
        // Handle hash syntax for pragmas like: use constant { FOO => 42, BAR => 43 }
        else if self.peek_kind() == Some(TokenKind::LeftBrace) {
            loop {
                // consume one { ... } block (track depth)
                let mut depth = 0;
                self.consume_token()?; // '{'
                depth += 1;
                args.push("{".into());
                while !self.tokens.is_eof() && depth > 0 {
                    match self.peek_kind() {
                        Some(TokenKind::LeftBrace) => {
                            depth += 1;
                            args.push(self.consume_token()?.text.to_string());
                        }
                        Some(TokenKind::RightBrace) => {
                            args.push(self.consume_token()?.text.to_string());
                            depth -= 1;
                        }
                        _ => {
                            args.push(self.consume_token()?.text.to_string());
                        }
                    }
                }
                // optional: => "ignored"
                if self.peek_kind() == Some(TokenKind::FatArrow) {
                    self.consume_token()?; // =>
                    if let Some(TokenKind::String | TokenKind::Number | TokenKind::Identifier) =
                        self.peek_kind()
                    {
                        args.push(self.consume_token()?.text.to_string());
                    } else {
                        while !Self::is_statement_terminator(self.peek_kind())
                            && self.peek_kind() != Some(TokenKind::Comma)
                        {
                            args.push(self.consume_token()?.text.to_string());
                        }
                    }
                }
                // another block after comma?
                if self.peek_kind() == Some(TokenKind::Comma) {
                    self.consume_token()?; // ','
                    if self.peek_kind() == Some(TokenKind::LeftBrace) {
                        continue; // loop for the next { ... }
                    }
                }
                break;
            }
        }
        // Handle anonymous sub as use argument: use Filter::Simple sub { ... };
        else if self.peek_kind() == Some(TokenKind::Sub) {
            args.push(self.consume_token()?.text.to_string()); // consume 'sub'
            if self.peek_kind() == Some(TokenKind::LeftBrace) {
                let mut depth: u32 = 0;
                while !self.tokens.is_eof() {
                    match self.peek_kind() {
                        Some(TokenKind::LeftBrace) => {
                            depth = depth.saturating_add(1);
                            args.push(self.consume_token()?.text.to_string());
                        }
                        Some(TokenKind::RightBrace) => {
                            args.push(self.consume_token()?.text.to_string());
                            depth = depth.saturating_sub(1);
                            if depth == 0 {
                                break;
                            }
                        }
                        _ => {
                            args.push(self.consume_token()?.text.to_string());
                        }
                    }
                }
            }
        }
        // Handle bare arguments (no parentheses)
        else if matches!(self.peek_kind(), Some(k) if matches!(k, TokenKind::String | TokenKind::Identifier | TokenKind::Minus | TokenKind::QuoteWords | TokenKind::QuoteSingle | TokenKind::QuoteDouble))
            && !Self::is_statement_terminator(self.peek_kind())
        {
            // Parse bare arguments like: use warnings 'void' or use constant FOO => 42
            // Also handle -strict flag and comma forms
            loop {
                // Check for qw BEFORE the match to avoid it being consumed as a generic identifier
                if let Ok(tok) = self.tokens.peek() {
                    if tok.text.as_ref() == "qw" {
                        self.consume_token()?; // consume 'qw'
                        let list = self.parse_qw_words()?;
                        // Format as "qw(FOO BAR BAZ)" so DeclarationProvider can recognize it
                        // We use parentheses regardless of original delimiter for consistency
                        let qw_str = format!("qw({})", list.join(" "));
                        args.push(qw_str);
                        // optional: qw(...) => <value>
                        if self.peek_kind() == Some(TokenKind::FatArrow) {
                            self.consume_token()?; // =>
                            if let Some(
                                TokenKind::String | TokenKind::Number | TokenKind::Identifier,
                            ) = self.peek_kind()
                            {
                                args.push(self.consume_token()?.text.to_string());
                            } else {
                                // best-effort: slurp tokens until ',' or ';'
                                while !Self::is_statement_terminator(self.peek_kind())
                                    && self.peek_kind() != Some(TokenKind::Comma)
                                {
                                    args.push(self.consume_token()?.text.to_string());
                                }
                            }
                        }
                        continue; // Don't fall through to the match below
                    }
                }

                match self.peek_kind() {
                    Some(TokenKind::String) => {
                        args.push(self.consume_token()?.text.to_string());

                        // Handle fat arrow after string key (e.g. use overload '""' => \&stringify)
                        match self.peek_kind() {
                            Some(TokenKind::Comma) => {
                                self.consume_token()?; // consume comma
                                // Continue to parse next argument
                            }
                            Some(TokenKind::FatArrow) => {
                                self.consume_token()?; // consume =>
                                // Consume the value after =>
                                self.consume_use_import_value(&mut args)?;
                            }
                            _ => {
                                // No separator, just continue
                            }
                        }
                    }
                    Some(TokenKind::QuoteSingle) | Some(TokenKind::QuoteDouble) => {
                        // Handle q{} / qq{} quote operators in use import lists
                        // e.g. use overload q{""} => sub { ... };
                        args.push(self.consume_token()?.text.to_string());

                        // Handle fat arrow after quote key (same as String handling)
                        match self.peek_kind() {
                            Some(TokenKind::Comma) => {
                                self.consume_token()?; // consume comma
                            }
                            Some(TokenKind::FatArrow) => {
                                self.consume_token()?; // consume =>
                                self.consume_use_import_value(&mut args)?;
                            }
                            _ => {}
                        }
                    }
                    Some(TokenKind::QuoteWords) => {
                        // Handle qw(...) in use statements
                        // Format it as "qw(FOO BAR)" for consistency with DeclarationProvider
                        let qw_token = self.consume_token()?;
                        let text: &str = qw_token.text.as_ref();
                        if let Some(content) = text.strip_prefix("qw").and_then(|s| {
                            // Extract content between delimiters
                            if s.starts_with('(') && s.ends_with(')') {
                                Some(&s[1..s.len() - 1])
                            } else if s.starts_with('[') && s.ends_with(']') {
                                Some(&s[1..s.len() - 1])
                            } else if s.starts_with('{') && s.ends_with('}') {
                                Some(&s[1..s.len() - 1])
                            } else if s.starts_with('<') && s.ends_with('>') {
                                Some(&s[1..s.len() - 1])
                            } else {
                                None
                            }
                        }) {
                            // Reformat as "qw(FOO BAR)" for consistency, stripping # comments first.
                            let cleaned = strip_qw_comments(content);
                            let words: Vec<&str> = cleaned.split_whitespace().collect();
                            let qw_str = format!("qw({})", words.join(" "));
                            args.push(qw_str);
                        } else {
                            // Fallback: just add the whole token as string
                            args.push(qw_token.text.to_string());
                        }

                        match self.peek_kind() {
                            Some(TokenKind::Comma) => {
                                self.consume_token()?;
                            }
                            Some(TokenKind::FatArrow) => {
                                self.consume_token()?;
                                self.consume_use_import_value(&mut args)?;
                            }
                            _ => {}
                        }
                    }
                    Some(TokenKind::Minus) => {
                        // Handle -strict, -dist => 'value', -conflicts => { ... }, etc.
                        let minus = self.consume_token()?;
                        if self.peek_kind() == Some(TokenKind::Identifier) {
                            let flag = self.consume_token()?;
                            // Combine minus and identifier as a single flag
                            args.push(format!("-{}", flag.text));
                            // Handle fat arrow after -flag (e.g. -dist => 'Module::Name')
                            if self.peek_kind() == Some(TokenKind::FatArrow) {
                                self.consume_token()?; // consume =>
                                self.consume_use_import_value(&mut args)?;
                            }
                        } else {
                            // Just a minus sign (shouldn't happen in use statements)
                            args.push(minus.text.to_string());
                        }
                    }
                    Some(TokenKind::Identifier) => {
                        // Check if this might be a constant declaration
                        let ident = self.consume_token()?;
                        let ident_text = ident.text.to_string();

                        // Variable-led import expressions need to consume through the end
                        // of the statement, e.g. `use warnings $ENV{X} ? qw(FATAL all) : ()`.
                        if ident_text.starts_with('$')
                            || ident_text.starts_with('@')
                            || ident_text.starts_with('%')
                        {
                            args.push(ident_text);
                            while !Self::is_statement_terminator(self.peek_kind())
                                && !self.tokens.is_eof()
                            {
                                args.push(self.consume_token()?.text.to_string());
                            }
                            break;
                        }

                        args.push(ident_text);

                        // Check for comma or fat arrow
                        match self.peek_kind() {
                            Some(TokenKind::Comma) => {
                                self.consume_token()?; // consume comma
                                // Continue to parse next argument
                            }
                            Some(TokenKind::FatArrow) => {
                                self.consume_token()?; // consume =>
                                // Consume the value after =>
                                self.consume_use_import_value(&mut args)?;
                            }
                            _ => {
                                // No separator, just continue
                            }
                        }
                    }
                    Some(TokenKind::Comma) => {
                        // Skip standalone commas (already handled after identifiers)
                        self.consume_token()?;
                    }
                    _ => break,
                }

                // Check if we should continue parsing arguments
                if Self::is_statement_terminator(self.peek_kind()) {
                    break;
                }
            }
        } else if self.peek_kind() == Some(TokenKind::LeftParen) {
            self.consume_token()?; // consume (
            // Mark that this `use` has an explicit parenthesized import list.
            // This is the key discriminator between `use Module;` (auto-import)
            // and `use Module ();` / `use Module (foo);` (explicit list — no auto-import).
            has_explicit_import_list = true;

            // Perl import lists can contain arbitrary expressions: backslash refs,
            // sub blocks, nested parens, fat-arrow pairs, etc.  Instead of
            // enumerating every legal token we use a depth-tracking consumer that
            // collects everything until the matching closing paren.
            let mut depth: u32 = 1;
            while depth > 0 && !self.tokens.is_eof() {
                match self.peek_kind() {
                    Some(TokenKind::LeftParen) => {
                        depth = depth.saturating_add(1);
                        args.push(self.consume_token()?.text.to_string());
                    }
                    Some(TokenKind::RightParen) => {
                        depth = depth.saturating_sub(1);
                        if depth > 0 {
                            args.push(self.consume_token()?.text.to_string());
                        } else {
                            self.consume_token()?; // consume final )
                        }
                    }
                    Some(_) => {
                        args.push(self.consume_token()?.text.to_string());
                    }
                    None => break,
                }
            }
        } else if !Self::is_statement_terminator(self.peek_kind()) {
            // Complex `use` arguments can start with tokens outside the bare-arg
            // fast path, such as sigils in `use warnings $ENV{X} ? ...`.
            while !Self::is_statement_terminator(self.peek_kind()) && !self.tokens.is_eof() {
                args.push(self.consume_token()?.text.to_string());
            }
        }

        // Don't consume semicolon here - let parse_statement handle it uniformly

        let end = self.previous_position();
        let has_filter_risk = Self::is_filter_module(&module);
        Ok(Node::new(
            NodeKind::Use { module, args, has_filter_risk, has_explicit_import_list },
            SourceLocation { start, end },
        ))
    }

    /// Parse special block (AUTOLOAD, DESTROY, etc.)
    fn parse_special_block(&mut self) -> ParseResult<Node> {
        let start = self.current_position();
        let name_token = self.consume_token()?;
        let name = name_token.text.to_string();

        // Capture name_span from token for precise LSP navigation
        let name_span = Some(SourceLocation { start: name_token.start, end: name_token.end });

        let block = self.parse_block()?;
        let end = block.location.end;

        // Treat as a special subroutine
        Ok(Node::new(
            NodeKind::Subroutine {
                name: Some(name),
                name_span,
                prototype: None,
                signature: None,
                attributes: vec![],
                body: Box::new(block),
            },
            SourceLocation { start, end },
        ))
    }

    /// Parse phase block (BEGIN, END, CHECK, INIT, UNITCHECK)
    fn parse_phase_block(&mut self) -> ParseResult<Node> {
        let start = self.current_position();
        let phase_token = self.consume_token()?;
        let phase = phase_token.text.to_string();

        // Capture phase_span from token for precise LSP navigation
        let phase_span = Some(SourceLocation { start: phase_token.start, end: phase_token.end });

        // Phase blocks must be followed by a block
        if self.peek_kind() != Some(TokenKind::LeftBrace) {
            return Err(ParseError::syntax(
                format!("{} must be followed by a block", phase),
                self.current_position(),
            ));
        }

        let block = self.parse_block()?;
        let end = block.location.end;

        // Create a special node for phase blocks
        Ok(Node::new(
            NodeKind::PhaseBlock { phase, phase_span, block: Box::new(block) },
            SourceLocation { start, end },
        ))
    }

    /// Parse data section (__DATA__ or __END__)
    fn parse_data_section(&mut self) -> ParseResult<Node> {
        let start = self.current_position();

        // Consume the data marker token
        let marker_token = self.consume_token()?;
        let marker = marker_token.text.to_string();

        // Check if there's a data body token
        let body = if self.peek_kind() == Some(TokenKind::DataBody) {
            let body_token = self.consume_token()?;
            Some(body_token.text.to_string())
        } else {
            None
        };

        let end = self.previous_position();

        // Create a data section node
        Ok(Node::new(NodeKind::DataSection { marker, body }, SourceLocation { start, end }))
    }

    /// Parse no statement (similar to use but disables pragmas/modules)
    fn parse_no(&mut self) -> ParseResult<Node> {
        let start = self.current_position();
        self.tokens.next()?; // consume 'no'

        // Parse module name — accepts bare identifiers or keyword-named pragmas
        // (e.g. `no if COND, warnings`, `no feature`).
        let first_token = self.consume_token()?;
        let mut module = if first_token.kind == TokenKind::Identifier {
            first_token.text.to_string()
        } else if first_token.text.chars().all(|c| c.is_alphanumeric() || c == '_')
            && !first_token.text.is_empty()
        {
            // Keyword-named pragmas: `no if COND, MODULE`, etc.
            first_token.text.to_string()
        } else {
            return Err(ParseError::syntax(
                format!(
                    "Expected module name after 'no', found {}",
                    first_token.kind.display_name()
                ),
                first_token.start,
            ));
        };

        // Handle :: in module names
        // Handle both DoubleColon tokens and separate Colon tokens (in case lexer sends :: as separate colons)
        while self.peek_kind() == Some(TokenKind::DoubleColon)
            || (self.peek_kind() == Some(TokenKind::Colon)
                && self.tokens.peek_second().map(|t| t.kind) == Ok(TokenKind::Colon))
        {
            if self.peek_kind() == Some(TokenKind::DoubleColon) {
                self.consume_token()?; // consume ::
                module.push_str("::");
            } else {
                // Handle two separate Colon tokens as ::
                self.consume_token()?; // consume first :
                self.consume_token()?; // consume second :
                module.push_str("::");
            }
            // In Perl, trailing :: is valid (e.g., Foo::Bar::)
            // Only consume identifier if there is one
            if self.peek_kind() == Some(TokenKind::Identifier) {
                let next_part = self.consume_token()?;
                module.push_str(&next_part.text);
            }
            // No error for trailing :: - it's valid in Perl
        }

        // Parse optional version number
        if self.peek_kind() == Some(TokenKind::Number) {
            module.push(' ');
            module.push_str(&self.consume_token()?.text);
        }

        // `no if CONDITION, MODULE [, ARGS]` and `no unless ...` are special:
        // the condition is an arbitrary expression that may contain braces
        // (e.g. `eval { ... }`), so we need brace-depth-aware token consumption.
        if module == "if" || module == "unless" {
            let mut cond_args = Vec::new();
            let mut brace_depth: usize = 0;
            while !self.tokens.is_eof() {
                if brace_depth == 0 && Self::is_statement_terminator(self.peek_kind()) {
                    break;
                }
                if self.peek_kind() == Some(TokenKind::Comma) && brace_depth == 0 {
                    self.consume_token()?;
                    continue;
                }
                match self.peek_kind() {
                    Some(TokenKind::LeftBrace) => {
                        brace_depth = brace_depth.saturating_add(1);
                    }
                    Some(TokenKind::RightBrace) => {
                        brace_depth = brace_depth.saturating_sub(1);
                    }
                    _ => {}
                }
                cond_args.push(self.consume_token()?.text.to_string());
            }
            let end = self.previous_position();
            let has_filter_risk = Self::is_filter_module(&module);
            return Ok(Node::new(
                NodeKind::No { module, args: cond_args, has_filter_risk },
                SourceLocation { start, end },
            ));
        }

        // Parse optional arguments list
        let mut args = Vec::new();

        // Handle bare arguments (no parentheses)
        if matches!(self.peek_kind(), Some(TokenKind::String) | Some(TokenKind::Identifier))
            && !matches!(self.peek_kind(), Some(TokenKind::Semicolon) | Some(TokenKind::Eof) | None)
        {
            // Parse bare arguments like: no warnings 'void'
            loop {
                // Check for qw BEFORE the match to avoid it being consumed as a generic identifier
                if let Ok(tok) = self.tokens.peek() {
                    if tok.text.as_ref() == "qw" {
                        self.consume_token()?; // consume 'qw'
                        let list = self.parse_qw_words()?;
                        // Format as "qw(FOO BAR BAZ)" so DeclarationProvider can recognize it
                        // We use parentheses regardless of original delimiter for consistency
                        let qw_str = format!("qw({})", list.join(" "));
                        args.push(qw_str);
                        // optional: qw(...) => <value>
                        if self.peek_kind() == Some(TokenKind::FatArrow) {
                            self.consume_token()?; // =>
                            if let Some(
                                TokenKind::String | TokenKind::Number | TokenKind::Identifier,
                            ) = self.peek_kind()
                            {
                                args.push(self.consume_token()?.text.to_string());
                            } else {
                                // best-effort: slurp tokens until ',' or ';'
                                while !Self::is_statement_terminator(self.peek_kind())
                                    && self.peek_kind() != Some(TokenKind::Comma)
                                {
                                    args.push(self.consume_token()?.text.to_string());
                                }
                            }
                        }
                        continue; // Don't fall through to the match below
                    }
                }

                match self.peek_kind() {
                    Some(TokenKind::String) => {
                        args.push(self.consume_token()?.text.to_string());

                        // Handle fat arrow or comma after a string key
                        // (e.g. `no overload '==' => \&func` or `no overload '+', '-'`)
                        match self.peek_kind() {
                            Some(TokenKind::Comma) => {
                                self.consume_token()?; // consume comma, continue loop
                            }
                            Some(TokenKind::FatArrow) => {
                                self.consume_token()?; // consume =>
                                // Best-effort: slurp the value until ',' or ';'
                                while !Self::is_statement_terminator(self.peek_kind())
                                    && self.peek_kind() != Some(TokenKind::Comma)
                                    && !self.tokens.is_eof()
                                {
                                    args.push(self.consume_token()?.text.to_string());
                                }
                                // Consume trailing comma if present
                                if self.peek_kind() == Some(TokenKind::Comma) {
                                    self.consume_token()?;
                                }
                            }
                            _ => {
                                // No separator; continue to terminator check
                            }
                        }
                    }
                    Some(TokenKind::Identifier) => {
                        args.push(self.consume_token()?.text.to_string());

                        // Handle comma or fat arrow after identifier
                        match self.peek_kind() {
                            Some(TokenKind::Comma) => {
                                self.consume_token()?; // consume comma, continue loop
                            }
                            Some(TokenKind::FatArrow) => {
                                self.consume_token()?; // consume =>
                                // Best-effort: slurp the value until ',' or ';'
                                while !Self::is_statement_terminator(self.peek_kind())
                                    && self.peek_kind() != Some(TokenKind::Comma)
                                    && !self.tokens.is_eof()
                                {
                                    args.push(self.consume_token()?.text.to_string());
                                }
                                // Consume trailing comma if present
                                if self.peek_kind() == Some(TokenKind::Comma) {
                                    self.consume_token()?;
                                }
                            }
                            _ => {
                                // No separator; continue to terminator check
                            }
                        }
                    }
                    Some(TokenKind::Comma) => {
                        // Standalone comma — skip and continue
                        self.consume_token()?;
                    }
                    _ => break,
                }

                // Check if we should continue parsing arguments
                if Self::is_statement_terminator(self.peek_kind()) {
                    break;
                }
            }
        } else if self.peek_kind() == Some(TokenKind::LeftParen) {
            self.consume_token()?; // consume (

            // `no` import lists can contain arbitrary expressions: backslash refs,
            // sub blocks, nested parens, fat-arrow pairs, sigil-prefixed variables,
            // etc.  Instead of enumerating every legal token we use a depth-tracking
            // consumer that collects everything until the matching closing paren.
            let mut depth: u32 = 1;
            while depth > 0 && !self.tokens.is_eof() {
                match self.peek_kind() {
                    Some(TokenKind::LeftParen) => {
                        depth = depth.saturating_add(1);
                        args.push(self.consume_token()?.text.to_string());
                    }
                    Some(TokenKind::RightParen) => {
                        depth = depth.saturating_sub(1);
                        if depth > 0 {
                            args.push(self.consume_token()?.text.to_string());
                        } else {
                            self.consume_token()?; // consume final )
                        }
                    }
                    Some(_) => {
                        args.push(self.consume_token()?.text.to_string());
                    }
                    None => break,
                }
            }
        }

        // Don't consume semicolon here - let parse_statement handle it uniformly

        let end = self.previous_position();
        let has_filter_risk = Self::is_filter_module(&module);
        Ok(Node::new(NodeKind::No { module, args, has_filter_risk }, SourceLocation { start, end }))
    }

    /// Consume a value expression on the right-hand side of `=>` inside a `use`
    /// import list. Values are stored as raw tokens, so this consumer needs to
    /// keep nested delimiters and ternaries balanced instead of bailing on the
    /// first operator or inner fat arrow.
    fn consume_use_import_value(&mut self, args: &mut Vec<String>) -> ParseResult<()> {
        let mut paren_depth = 0usize;
        let mut bracket_depth = 0usize;
        let mut brace_depth = 0usize;
        let mut ternary_depth = 0usize;
        let mut consumed_any = false;

        while !self.tokens.is_eof() {
            let kind = self.peek_kind();
            let at_top_level = paren_depth == 0 && bracket_depth == 0 && brace_depth == 0;

            // EOF/None: always stop to avoid infinite loops.
            // Semicolon: only stop at top level so that `eval { ...; ... }`
            // blocks inside use declarations are consumed fully.
            if matches!(kind, Some(TokenKind::Eof) | None) {
                break;
            }
            if kind == Some(TokenKind::Semicolon) && at_top_level {
                break;
            }

            if at_top_level && ternary_depth == 0 {
                match kind {
                    Some(
                        TokenKind::Comma
                        | TokenKind::FatArrow
                        | TokenKind::RightParen
                        | TokenKind::RightBracket
                        | TokenKind::RightBrace,
                    ) => break,
                    Some(TokenKind::Identifier)
                        if !consumed_any
                            && self.tokens.peek_second().map(|t| t.kind)
                                == Ok(TokenKind::FatArrow) =>
                    {
                        break;
                    }
                    _ => {}
                }
            }

            let token = self.consume_token()?;
            match token.kind {
                TokenKind::LeftParen => paren_depth += 1,
                TokenKind::RightParen => paren_depth = paren_depth.saturating_sub(1),
                TokenKind::LeftBracket => bracket_depth += 1,
                TokenKind::RightBracket => bracket_depth = bracket_depth.saturating_sub(1),
                TokenKind::LeftBrace => brace_depth += 1,
                TokenKind::RightBrace => brace_depth = brace_depth.saturating_sub(1),
                TokenKind::Question => ternary_depth += 1,
                TokenKind::Colon => ternary_depth = ternary_depth.saturating_sub(1),
                _ => {}
            }

            args.push(token.text.to_string());
            consumed_any = true;
        }

        Ok(())
    }
}
