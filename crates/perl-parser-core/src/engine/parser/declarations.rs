impl<'a> Parser<'a> {
    /// Parse subroutine definition
    fn parse_subroutine(&mut self) -> ParseResult<Node> {
        let start = self.current_position();
        self.tokens.next()?; // consume 'sub'

        let (name, name_span) = match self.peek_kind() {
            // Regular identifier
            Some(TokenKind::Identifier)
            | Some(TokenKind::Method)
            | Some(TokenKind::Class)
            | Some(TokenKind::Try)
            | Some(TokenKind::Catch)
            | Some(TokenKind::Finally)
            | Some(TokenKind::Given)
            | Some(TokenKind::When)
            | Some(TokenKind::Default)
            | Some(TokenKind::Continue)
            | Some(TokenKind::Format) => {
                let token = self.tokens.next()?;
                (
                    Some(token.text.to_string()),
                    Some(SourceLocation { start: token.start, end: token.end }),
                )
            }
            // No name - anonymous subroutine
            _ => (None, None),
        };

        // Parse optional attributes first (they come before signature in modern Perl)
        let mut attributes = Vec::new();
        while self.peek_kind() == Some(TokenKind::Colon) {
            self.tokens.next()?; // consume colon

            // Parse one or more space-separated attributes after the colon
            loop {
                // Attributes can be identifiers or certain keywords
                let attr_token = match self.peek_kind() {
                    Some(TokenKind::Identifier | TokenKind::Method) => self.tokens.next()?,
                    _ => {
                        // If it's not an attribute name, we're done with this attribute list
                        break;
                    }
                };

                let mut attr_name = attr_token.text.to_string();

                // Check if attribute has a value in parentheses (like :prototype($))
                if self.peek_kind() == Some(TokenKind::LeftParen) {
                    self.consume_token()?; // consume (
                    attr_name.push('(');

                    // Collect tokens until matching )
                    let mut paren_depth = 1;
                    while paren_depth > 0 && !self.tokens.is_eof() {
                        let token = self.tokens.next()?;
                        attr_name.push_str(&token.text);

                        match token.kind {
                            TokenKind::LeftParen => paren_depth += 1,
                            TokenKind::RightParen => {
                                paren_depth -= 1;
                                if paren_depth == 0 {
                                    attr_name.push(')');
                                }
                            }
                            _ => {}
                        }
                    }
                }

                attributes.push(attr_name);

                // Check if there's another attribute (not preceded by colon)
                match self.peek_kind() {
                    Some(TokenKind::Identifier | TokenKind::Method) => {
                        // Continue parsing more attributes
                        continue;
                    }
                    _ => break,
                }
            }
        }

        // Parse optional prototype or signature after attributes
        let (prototype, signature) = if self.peek_kind() == Some(TokenKind::LeftParen) {
            // Look ahead to determine if this is a prototype or signature
            if self.is_likely_prototype()? {
                // Parse as prototype
                let proto_content = self.parse_prototype()?;
                let proto_node = Node::new(
                    NodeKind::Prototype { content: proto_content },
                    SourceLocation { start: self.current_position(), end: self.current_position() },
                );
                (Some(Box::new(proto_node)), None)
            } else {
                // Parse as signature
                let params = self.parse_signature()?;
                let sig_node = Node::new(
                    NodeKind::Signature { parameters: params },
                    SourceLocation { start: self.current_position(), end: self.current_position() },
                );
                (None, Some(Box::new(sig_node)))
            }
        } else {
            (None, None)
        };

        let body = self.parse_block()?;

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
    fn parse_class(&mut self) -> ParseResult<Node> {
        let start = self.current_position();
        self.tokens.next()?; // consume 'class'

        let name_token = self.expect(TokenKind::Identifier)?;
        let name = name_token.text.to_string();

        let body = self.parse_block()?;

        let end = self.previous_position();
        Ok(Node::new(NodeKind::Class { name, body: Box::new(body) }, SourceLocation { start, end }))
    }

    /// Parse method declaration (Perl 5.38+)
    fn parse_method(&mut self) -> ParseResult<Node> {
        let start = self.current_position();
        self.tokens.next()?; // consume 'method'

        let name_token = self.expect(TokenKind::Identifier)?;
        let name = name_token.text.to_string();

        // Parse optional signature
        let signature = if self.peek_kind() == Some(TokenKind::LeftParen) {
            let params = self.parse_signature()?;
            Some(Box::new(Node::new(
                NodeKind::Signature { parameters: params },
                SourceLocation { start: self.current_position(), end: self.current_position() },
            )))
        } else {
            None
        };

        let body = self.parse_block()?;

        let end = self.previous_position();
        Ok(Node::new(
            NodeKind::Method { name, signature, attributes: Vec::new(), body: Box::new(body) },
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
                found: format!("{:?}", body_token.kind),
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

        // Parse package name (can include ::)
        let first = self.expect(TokenKind::Identifier)?;
        let mut name = first.text.to_string();
        let name_start = first.start;
        let mut name_end = first.end;

        // Handle :: in package names
        // Handle both DoubleColon tokens and separate Colon tokens (in case lexer sends :: as separate colons)
        while self.peek_kind() == Some(TokenKind::DoubleColon)
            || (self.peek_kind() == Some(TokenKind::Colon)
                && self.tokens.peek_second().map(|t| t.kind) == Ok(TokenKind::Colon))
        {
            if self.peek_kind() == Some(TokenKind::DoubleColon) {
                let dc = self.tokens.next()?; // consume ::
                name_end = dc.end;
                name.push_str("::");
            } else if self.peek_kind() == Some(TokenKind::Colon) {
                // Handle two separate Colon tokens as ::
                let _first_colon = self.tokens.next()?; // consume first :
                let second_colon = self.tokens.next()?; // consume second :
                name_end = second_colon.end;
                name.push_str("::");
            }

            // Check if there's an identifier after ::
            // If not, it's a trailing :: which is valid in Perl
            if self.peek_kind() == Some(TokenKind::Identifier) {
                let id = self.tokens.next()?;
                name_end = id.end;
                name.push_str(&id.text);
            } else {
                // Trailing :: is valid, just break
                break;
            }
        }

        let name_span = SourceLocation { start: name_start, end: name_end };

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
        let mut module = if self.peek_kind() == Some(TokenKind::Number) {
            // Numeric version like 5.036
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
                        if &*dot_token.text == "." {
                            self.consume_token()?; // consume dot
                            if self.peek_kind() != /* ~ changed by cargo-mutants ~ */ Some(TokenKind::Number)
                            {
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
            } else {
                return Err(ParseError::syntax(
                    format!("Expected module name or version, found {:?}", first_token.kind),
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

        // Parse optional import list
        let mut args = Vec::new();

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
        // Handle bare arguments (no parentheses)
        else if matches!(self.peek_kind(), Some(k) if matches!(k, TokenKind::String | TokenKind::Identifier | TokenKind::Minus | TokenKind::QuoteWords))
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
                            // Reformat as "qw(FOO BAR)" for consistency
                            let words: Vec<&str> = content.split_whitespace().collect();
                            let qw_str = format!("qw({})", words.join(" "));
                            args.push(qw_str);
                        } else {
                            // Fallback: just add the whole token as string
                            args.push(qw_token.text.to_string());
                        }
                    }
                    Some(TokenKind::Minus) => {
                        // Handle -strict and other flags
                        let minus = self.consume_token()?;
                        if self.peek_kind() == Some(TokenKind::Identifier) {
                            let flag = self.consume_token()?;
                            // Combine minus and identifier as a single flag
                            args.push(format!("-{}", flag.text));
                        } else {
                            // Just a minus sign (shouldn't happen in use statements)
                            args.push(minus.text.to_string());
                        }
                    }
                    Some(TokenKind::Identifier) => {
                        // Check if this might be a constant declaration
                        let ident = self.consume_token()?;
                        args.push(ident.text.to_string());

                        // Check for comma or fat arrow
                        match self.peek_kind() {
                            Some(TokenKind::Comma) => {
                                self.consume_token()?; // consume comma
                                // Continue to parse next argument
                            }
                            Some(TokenKind::FatArrow) => {
                                self.consume_token()?; // consume =>
                                // Parse the value as a simple expression
                                // But check if an identifier is followed by => (making it a key, not a value)
                                match self.peek_kind() {
                                    Some(TokenKind::Number | TokenKind::String) => {
                                        args.push(self.consume_token()?.text.to_string());
                                    }
                                    Some(TokenKind::Identifier) => {
                                        // Peek ahead to see if this identifier is followed by =>
                                        // If so, it's actually a key for the next pair, not a value
                                        if self.tokens.peek_second().map(|t| t.kind)
                                            == Ok(TokenKind::FatArrow)
                                        {
                                            // Don't consume - let the outer loop handle it as a key
                                        } else {
                                            args.push(self.consume_token()?.text.to_string());
                                        }
                                    }
                                    _ => {
                                        // For more complex expressions, just consume tokens until semicolon
                                        while !Self::is_statement_terminator(self.peek_kind())
                                            && self.peek_kind() != Some(TokenKind::Comma)
                                            && self.peek_kind() != Some(TokenKind::FatArrow)
                                        {
                                            args.push(self.consume_token()?.text.to_string());
                                        }
                                    }
                                }
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

            // Parse import list
            while self.peek_kind() != Some(TokenKind::RightParen) {
                if self.peek_kind() == Some(TokenKind::String) {
                    args.push(self.consume_token()?.text.to_string());
                } else if self.peek_kind() == Some(TokenKind::Identifier) {
                    args.push(self.consume_token()?.text.to_string());
                } else {
                    return Err(ParseError::syntax(
                        "Expected string or identifier in import list",
                        self.current_position(),
                    ));
                }

                if self.peek_kind() == Some(TokenKind::Comma) {
                    self.consume_token()?; // consume comma
                } else if self.peek_kind() != Some(TokenKind::RightParen) {
                    return Err(ParseError::syntax(
                        "Expected comma or closing parenthesis",
                        self.current_position(),
                    ));
                }
            }

            self.expect(TokenKind::RightParen)?;
        }

        // Don't consume semicolon here - let parse_statement handle it uniformly

        let end = self.previous_position();
        Ok(Node::new(NodeKind::Use { module, args }, SourceLocation { start, end }))
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

        // Parse module name (can include ::)
        let mut module = self.expect(TokenKind::Identifier)?.text.to_string();

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
                    }
                    Some(TokenKind::Identifier) => {
                        args.push(self.consume_token()?.text.to_string());
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

            // Parse argument list
            while self.peek_kind() != Some(TokenKind::RightParen) {
                if self.peek_kind() == Some(TokenKind::String) {
                    args.push(self.consume_token()?.text.to_string());
                } else if self.peek_kind() == Some(TokenKind::Identifier) {
                    args.push(self.consume_token()?.text.to_string());
                } else {
                    return Err(ParseError::syntax(
                        "Expected string or identifier in argument list",
                        self.current_position(),
                    ));
                }

                if self.peek_kind() == Some(TokenKind::Comma) {
                    self.consume_token()?; // consume comma
                } else if self.peek_kind() != Some(TokenKind::RightParen) {
                    return Err(ParseError::syntax(
                        "Expected comma or closing parenthesis",
                        self.current_position(),
                    ));
                }
            }

            self.expect(TokenKind::RightParen)?;
        }

        // Don't consume semicolon here - let parse_statement handle it uniformly

        let end = self.previous_position();
        Ok(Node::new(NodeKind::No { module, args }, SourceLocation { start, end }))
    }

}
