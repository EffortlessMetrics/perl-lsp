impl<'a> Parser<'a> {
    /// Parse qualified identifier (may contain ::)
    fn parse_qualified_identifier(&mut self) -> ParseResult<Node> {
        // Note: qualified identifier parsing is not recursive - no guard needed
        let start_token = self.consume_token()?;
        let start = start_token.start;
        let mut name = if start_token.kind == TokenKind::DoubleColon {
            // Handle absolute path like ::Foo::Bar
            "::".to_string()
        } else {
            start_token.text.to_string()
        };

        // Keep consuming :: and identifiers
        // Handle both DoubleColon tokens and separate Colon tokens (in case lexer sends :: as separate colons)
        while self.peek_kind() == Some(TokenKind::DoubleColon)
            || (self.peek_kind() == Some(TokenKind::Colon)
                && self.tokens.peek_second().map(|t| t.kind) == Ok(TokenKind::Colon))
        {
            if self.peek_kind() == Some(TokenKind::DoubleColon) {
                self.consume_token()?; // consume ::
                name.push_str("::");
            } else if self.peek_kind() == Some(TokenKind::Colon) {
                // Handle two separate Colon tokens as ::
                self.consume_token()?; // consume first :
                self.consume_token()?; // consume second :
                name.push_str("::");
            }

            // In Perl, trailing :: is valid (e.g., Foo::Bar::)
            // Only consume identifier if there is one
            if self.peek_kind() == Some(TokenKind::Identifier) {
                let next_part = self.consume_token()?;
                name.push_str(&next_part.text);
            }
            // No error for trailing :: - it's valid in Perl
        }

        let end = self.previous_position();
        Ok(Node::new(NodeKind::Identifier { name }, SourceLocation { start, end }))
    }

    /// Parse primary expression
    fn parse_primary(&mut self) -> ParseResult<Node> {
        self.with_recursion_guard(|s| s.parse_primary_inner())
    }

    /// Inner implementation of parse_primary (called under recursion guard)
    fn parse_primary_inner(&mut self) -> ParseResult<Node> {
        let token = self.tokens.peek()?;
        let token_kind = token.kind;

        match token_kind {
            TokenKind::Number => {
                let token = self.tokens.next()?;
                Ok(Node::new(
                    NodeKind::Number { value: token.text.to_string() },
                    SourceLocation { start: token.start, end: token.end },
                ))
            }

            TokenKind::String => {
                let token = self.tokens.next()?;
                // Check if it's a double-quoted string (interpolated)
                let interpolated = token.text.starts_with('"');
                Ok(Node::new(
                    NodeKind::String { value: token.text.to_string(), interpolated },
                    SourceLocation { start: token.start, end: token.end },
                ))
            }

            TokenKind::Regex => {
                let token = self.tokens.next()?;
                let (pattern, body, modifiers) = quote_parser::extract_regex_parts(&token.text);

                // Validate regex complexity and check for embedded code
                let validator = crate::engine::regex_validator::RegexValidator::new();
                validator.validate(&body, token.start)?;
                let has_embedded_code = validator.detects_code_execution(&body);

                Ok(Node::new(
                    NodeKind::Regex { pattern, replacement: None, modifiers, has_embedded_code },
                    SourceLocation { start: token.start, end: token.end },
                ))
            }

            TokenKind::QuoteSingle | TokenKind::QuoteDouble => {
                let token = self.tokens.next()?;
                // Quote operators produce strings
                let interpolated = matches!(token.kind, TokenKind::QuoteDouble);
                Ok(Node::new(
                    NodeKind::String { value: token.text.to_string(), interpolated },
                    SourceLocation { start: token.start, end: token.end },
                ))
            }

            TokenKind::QuoteWords => {
                let token = self.tokens.next()?;
                let start = token.start;
                let text = &token.text;

                // Parse qw(...) to extract words
                if let Some(content) = text.strip_prefix("qw") {
                    // Find the delimiter and extract content
                    let (content_str, _delimiter) = if let Some(rest) = content.strip_prefix('(') {
                        (rest.strip_suffix(')').unwrap_or(rest), '(')
                    } else if let Some(rest) = content.strip_prefix('[') {
                        (rest.strip_suffix(']').unwrap_or(rest), '[')
                    } else if let Some(rest) = content.strip_prefix('{') {
                        (rest.strip_suffix('}').unwrap_or(rest), '{')
                    } else if let Some(rest) = content.strip_prefix('<') {
                        (rest.strip_suffix('>').unwrap_or(rest), '<')
                    } else {
                        // Other delimiter - find matching pair
                        let delim = content.chars().next().unwrap_or(' ');
                        let inner = &content[delim.len_utf8()..];
                        let trimmed = inner.trim_end_matches(delim);
                        (trimmed, delim)
                    };

                    // Split into words
                    let words: Vec<Node> = content_str
                        .split_whitespace()
                        .map(|word| {
                            Node::new(
                                NodeKind::String { value: word.to_string(), interpolated: false },
                                SourceLocation { start, end: token.end },
                            )
                        })
                        .collect();

                    Ok(Node::new(
                        NodeKind::ArrayLiteral { elements: words },
                        SourceLocation { start, end: token.end },
                    ))
                } else {
                    // Fallback - shouldn't happen with proper lexer
                    Ok(Node::new(
                        NodeKind::String { value: token.text.to_string(), interpolated: false },
                        SourceLocation { start, end: token.end },
                    ))
                }
            }

            TokenKind::QuoteCommand => {
                let token = self.tokens.next()?;
                // qx/backticks - for now treat as a string
                Ok(Node::new(
                    NodeKind::String { value: token.text.to_string(), interpolated: true },
                    SourceLocation { start: token.start, end: token.end },
                ))
            }

            TokenKind::Substitution => {
                let token = self.tokens.next()?;
                // Use strict validation that rejects invalid modifiers
                let (pattern, replacement, modifiers) =
                    quote_parser::extract_substitution_parts_strict(&token.text).map_err(
                        |e| {
                            let message = match e {
                                quote_parser::SubstitutionError::InvalidModifier(c) => {
                                    format!(
                                        "Invalid substitution modifier '{}'. Valid modifiers are: g, i, m, s, x, o, e, r",
                                        c
                                    )
                                }
                                quote_parser::SubstitutionError::MissingDelimiter => {
                                    "Missing delimiter after 's'".to_string()
                                }
                                quote_parser::SubstitutionError::MissingPattern => {
                                    "Missing pattern in substitution".to_string()
                                }
                                quote_parser::SubstitutionError::MissingReplacement => {
                                    "Missing replacement in substitution".to_string()
                                }
                                quote_parser::SubstitutionError::MissingClosingDelimiter => {
                                    "Missing closing delimiter in substitution".to_string()
                                }
                            };
                            ParseError::SyntaxError {
                                message,
                                location: token.start,
                            }
                        },
                    )?;

                // Validate regex complexity and check for embedded code
                let validator = crate::engine::regex_validator::RegexValidator::new();
                validator.validate(&pattern, token.start)?;
                let has_embedded_code = validator.detects_code_execution(&pattern);

                // Substitution as a standalone expression (will be used with =~ later)
                Ok(Node::new(
                    NodeKind::Substitution {
                        expr: Box::new(Node::new(
                            NodeKind::Identifier { name: String::from("$_") },
                            SourceLocation { start: token.start, end: token.start },
                        )),
                        pattern,
                        replacement,
                        modifiers,
                        has_embedded_code,
                    },
                    SourceLocation { start: token.start, end: token.end },
                ))
            }

            TokenKind::Transliteration => {
                let token = self.tokens.next()?;
                let (search, replace, modifiers) =
                    quote_parser::extract_transliteration_parts(&token.text);

                // Transliteration as a standalone expression (will be used with =~ later)
                Ok(Node::new(
                    NodeKind::Transliteration {
                        expr: Box::new(Node::new(
                            NodeKind::Identifier { name: String::from("$_") },
                            SourceLocation { start: token.start, end: token.start },
                        )),
                        search,
                        replace,
                        modifiers,
                    },
                    SourceLocation { start: token.start, end: token.end },
                ))
            }

            TokenKind::HeredocStart => {
                let start_token = self.tokens.next()?;
                let text = &start_token.text;
                let start = start_token.start;
                let end = start_token.end;

                // Parse heredoc delimiter from the token text
                let (delimiter, interpolated, indented, command) = parse_heredoc_delimiter(text);

                // Map interpolation to QuoteKind (check original text for quote style)
                let quote = map_heredoc_quote_kind(text, interpolated);

                // Enqueue for later content collection
                self.push_heredoc_decl(delimiter.to_string(), indented, quote, start, end);
                self.byte_cursor = end;

                // Return declaration node (content attaches when draining pending heredocs)
                Ok(Node::new(
                    NodeKind::Heredoc {
                        delimiter: delimiter.to_string(),
                        content: String::new(), // Placeholder until drain_pending_heredocs
                        interpolated,
                        indented,
                        command,
                        body_span: None, // Populated by drain_pending_heredocs
                    },
                    SourceLocation { start, end },
                ))
            }

            TokenKind::HeredocDepthLimit => {
                let token = self.tokens.next()?;
                Err(ParseError::syntax(
                    format!("Heredoc depth limit exceeded (max {})", MAX_HEREDOC_DEPTH),
                    token.start,
                ))
            }

            TokenKind::Eval => self.parse_eval(),

            TokenKind::Do => self.parse_do(),

            // Note: TokenKind::Sub is handled in the keyword-as-identifier case below
            // This allows 'sub' to be used as a hash key or identifier in expressions
            TokenKind::Try => self.parse_try(),

            TokenKind::Less => {
                // Could be diamond operator <> or <FILEHANDLE>
                let start = self.consume_token()?.start; // consume <

                if self.peek_kind() == Some(TokenKind::Greater) {
                    // Diamond operator <>
                    self.consume_token()?; // consume >
                    let end = self.previous_position();
                    Ok(Node::new(NodeKind::Diamond, SourceLocation { start, end }))
                } else {
                    // Try to parse content until >
                    let mut pattern = String::new();
                    let mut has_glob_chars = false;

                    while self.peek_kind() != Some(TokenKind::Greater) && !self.tokens.is_eof() {
                        let token = self.consume_token()?;

                        // Check if this looks like a glob pattern
                        if token.text.contains('*')
                            || token.text.contains('?')
                            || token.text.contains('[')
                            || token.text.contains('.')
                        {
                            has_glob_chars = true;
                        }

                        pattern.push_str(&token.text);
                    }

                    if self.peek_kind() == Some(TokenKind::Greater) {
                        self.consume_token()?; // consume >
                        let end = self.previous_position();

                        if pattern.is_empty() {
                            // Empty <> is diamond operator
                            Ok(Node::new(NodeKind::Diamond, SourceLocation { start, end }))
                        } else if has_glob_chars || pattern.contains('/') {
                            // Looks like a glob pattern
                            Ok(Node::new(NodeKind::Glob { pattern }, SourceLocation { start, end }))
                        } else if pattern.chars().all(|c| c.is_uppercase() || c == '_') {
                            // Looks like a filehandle
                            Ok(Node::new(
                                NodeKind::Readline { filehandle: Some(pattern) },
                                SourceLocation { start, end },
                            ))
                        } else {
                            // Default to glob
                            Ok(Node::new(NodeKind::Glob { pattern }, SourceLocation { start, end }))
                        }
                    } else {
                        Err(ParseError::syntax(
                            "Expected '>' to close angle bracket construct",
                            self.current_position(),
                        ))
                    }
                }
            }

            TokenKind::Identifier => {
                // Check if it's a variable (starts with sigil)
                let token = self.tokens.peek()?;
                if token.text.starts_with('$')
                    || token.text.starts_with('@')
                    || token.text.starts_with('%')
                    || token.text.starts_with('&')
                {
                    self.parse_variable()
                } else if token.text.starts_with('*') && token.text.len() > 1 {
                    // Only treat * as a glob sigil if followed by identifier
                    self.parse_variable()
                } else {
                    // Check if it's a quote operator or tie/untie
                    match token.text.as_ref() {
                        "q" | "qq" | "qw" | "qr" | "qx" | "m" | "s" => self.parse_quote_operator(),
                        "tie" => {
                            let token = self.tokens.next()?;
                            let start = token.start;
                            let variable = if matches!(
                                self.peek_kind(),
                                Some(
                                    TokenKind::My
                                        | TokenKind::Our
                                        | TokenKind::Local
                                        | TokenKind::State
                                )
                            ) {
                                Box::new(self.parse_variable_declaration()?)
                            } else {
                                Box::new(self.parse_assignment()?)
                            };
                            self.expect(TokenKind::Comma)?;
                            let package = Box::new(self.parse_assignment()?);
                            let mut args = vec![];
                            while self.peek_kind() == Some(TokenKind::Comma) {
                                self.consume_token()?;
                                args.push(self.parse_assignment()?);
                            }
                            let end = self.previous_position();
                            Ok(Node::new(
                                NodeKind::Tie { variable, package, args },
                                SourceLocation { start, end },
                            ))
                        }
                        "untie" => {
                            let token = self.tokens.next()?;
                            let start = token.start;
                            let variable = Box::new(self.parse_assignment()?);
                            let end = self.previous_position();
                            Ok(Node::new(
                                NodeKind::Untie { variable },
                                SourceLocation { start, end },
                            ))
                        }
                        _ => {
                            // Regular identifier (possibly qualified with ::)
                            self.parse_qualified_identifier()
                        }
                    }
                }
            }

            // Handle sigil tokens (for when lexer sends them separately)
            TokenKind::ScalarSigil
            | TokenKind::ArraySigil
            | TokenKind::HashSigil
            | TokenKind::SubSigil
            | TokenKind::GlobSigil
            | TokenKind::Percent => self.parse_variable_from_sigil(),

            TokenKind::LeftParen => {
                let start_token = self.tokens.next()?; // consume (
                let start = start_token.start;

                // Check for empty list
                if self.peek_kind() == Some(TokenKind::RightParen) {
                    let end_token = self.tokens.next()?;
                    return Ok(Node::new(
                        NodeKind::ArrayLiteral { elements: vec![] },
                        SourceLocation { start, end: end_token.end },
                    ));
                }

                // Check if we might have a simple parenthesized expression
                // If there's no comma or fat arrow after the first element, parse the full expression
                // to handle operators like 'or', 'and' etc.
                let first = if self.peek_kind() == Some(TokenKind::RightParen) {
                    // Simple case - just one element
                    self.parse_assignment()?
                } else {
                    // Peek ahead to see if this is a list or a complex expression
                    let expr = self.parse_assignment()?;

                    // Check what comes after
                    match self.peek_kind() {
                        Some(TokenKind::Comma) | Some(TokenKind::FatArrow) => {
                            // It's a list, continue with list parsing
                            expr
                        }
                        Some(TokenKind::RightParen) => {
                            // End of simple expression
                            expr
                        }
                        _ => {
                            // Could be an operator like 'or', 'and', etc.
                            // We need to continue parsing the expression
                            self.parse_word_or_expr(expr)?
                        }
                    }
                };

                if self.peek_kind() == Some(TokenKind::Comma)
                    || self.peek_kind() == Some(TokenKind::FatArrow)
                {
                    // It's a list
                    let mut elements = vec![first];
                    let mut saw_fat_comma = false;

                    // Handle fat arrow after first element
                    if self.peek_kind() == Some(TokenKind::FatArrow) {
                        saw_fat_comma = true;
                        self.tokens.next()?; // consume =>
                        elements.push(self.parse_assignment()?);
                    }

                    while self.peek_kind() == Some(TokenKind::Comma)
                        || self.peek_kind() == Some(TokenKind::FatArrow)
                    {
                        if self.peek_kind() == Some(TokenKind::Comma) {
                            self.consume_token()?; // consume comma
                        }

                        if self.peek_kind() == Some(TokenKind::RightParen) {
                            break;
                        }

                        let elem = self.parse_assignment()?;

                        // Check for fat arrow after element
                        if self.peek_kind() == Some(TokenKind::FatArrow) {
                            saw_fat_comma = true;
                            self.consume_token()?; // consume =>
                            elements.push(elem);
                            if self.peek_kind() != Some(TokenKind::RightParen) {
                                elements.push(self.parse_assignment()?);
                            }
                        } else {
                            elements.push(elem);
                        }
                    }

                    self.expect(TokenKind::RightParen)?;
                    let end = self.previous_position();

                    // Only convert to hash if we saw a fat comma
                    Ok(Self::build_list_or_hash(elements, saw_fat_comma, start, end))
                } else {
                    // It's a parenthesized expression
                    self.expect(TokenKind::RightParen)?;
                    Ok(first)
                }
            }

            TokenKind::LeftBracket => {
                // Array literal
                let start_token = self.tokens.next()?; // consume [
                let start = start_token.start;

                let mut elements = Vec::new();

                while self.peek_kind() != Some(TokenKind::RightBracket) && !self.tokens.is_eof() {
                    elements.push(self.parse_expression()?);

                    if self.peek_kind() == Some(TokenKind::Comma) {
                        self.tokens.next()?;
                    } else {
                        break;
                    }
                }

                self.expect(TokenKind::RightBracket)?;
                let end = self.previous_position();

                Ok(Node::new(NodeKind::ArrayLiteral { elements }, SourceLocation { start, end }))
            }

            // Handle & as sigil when at primary position
            TokenKind::BitwiseAnd => {
                // This is a subroutine call or code dereference
                // Convert to SubSigil behavior
                self.parse_variable_from_sigil()
            }

            TokenKind::LeftBrace => {
                // Could be hash literal or block
                // Try to parse as hash literal first
                self.parse_hash_or_block()
            }

            TokenKind::Ellipsis => {
                let token = self.tokens.next()?;
                Ok(Node::new(
                    NodeKind::Ellipsis,
                    SourceLocation { start: token.start, end: token.end },
                ))
            }

            TokenKind::Undef => {
                let token = self.tokens.next()?;
                Ok(Node::new(
                    NodeKind::Undef,
                    SourceLocation { start: token.start, end: token.end },
                ))
            }

            // Handle 'sub' specially - it might be an anonymous subroutine
            TokenKind::Sub => {
                // Check if the token AFTER 'sub' is { or ( (anonymous subroutine)
                // We use peek_second() because peek() is still 'sub' (unconsumed)
                let next = self.tokens.peek_second().ok().map(|t| t.kind);
                if matches!(next, Some(k) if matches!(k, TokenKind::LeftBrace | TokenKind::LeftParen))
                {
                    // It's an anonymous subroutine
                    self.parse_subroutine()
                } else {
                    // It's used as an identifier
                    let token = self.tokens.next()?;
                    Ok(Node::new(
                        NodeKind::Identifier { name: token.text.to_string() },
                        SourceLocation { start: token.start, end: token.end },
                    ))
                }
            }

            // Handle keywords that can be used as identifiers in certain contexts
            // Note: Statement-level keywords (if, unless, while, return, etc.) should NOT be here
            TokenKind::My
            | TokenKind::Our
            | TokenKind::Local
            | TokenKind::State
            | TokenKind::Package
            | TokenKind::Use
            | TokenKind::No
            | TokenKind::Begin
            | TokenKind::End
            | TokenKind::Check
            | TokenKind::Init
            | TokenKind::Unitcheck
            | TokenKind::Given
            | TokenKind::When
            | TokenKind::Default
            | TokenKind::Catch
            | TokenKind::Finally
            | TokenKind::Continue
            | TokenKind::Class
            | TokenKind::Method
            | TokenKind::Format => {
                // In expression context, some keywords can be used as barewords/identifiers
                // This happens in hash keys, method names, etc.
                // But NOT for statement modifiers like if, unless, while, etc.
                let token = self.tokens.next()?;
                Ok(Node::new(
                    NodeKind::Identifier { name: token.text.to_string() },
                    SourceLocation { start: token.start, end: token.end },
                ))
            }

            TokenKind::DoubleColon => {
                // Absolute package path like ::Foo::Bar
                self.parse_qualified_identifier()
            }

            _ => {
                // Get position before consuming
                let pos = self.current_position();
                Err(ParseError::unexpected("expression", format!("{:?}", token_kind), pos))
            }
        }
    }

}
