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

    fn record_unclosed_interpolation_delimiter(&mut self, text: &str, token_start: usize) {
        if let Some(delim) = Self::find_unclosed_interpolation_delimiter(text) {
            self.record_error(ParseError::syntax(
                format!(
                    "Unclosed {} delimiter in interpolated string before closing quote",
                    delim
                ),
                token_start,
            ));
        }
    }

    fn find_unclosed_interpolation_delimiter(text: &str) -> Option<char> {
        let bytes = text.as_bytes();
        if bytes.len() < 2 || bytes.first() != Some(&b'"') {
            return None;
        }

        let mut i = 1usize;
        let quote_end = bytes.len() - 1;
        while i < quote_end {
            let ch = bytes[i] as char;
            if ch == '\\' {
                i = i.saturating_add(2);
                continue;
            }

            if ch == '$' {
                i += 1;
                if i >= quote_end {
                    break;
                }

                if bytes[i] == b'{' {
                    if !Self::consume_balanced_in_interpolated_string(bytes, i, b'{', b'}', quote_end)
                    {
                        return Some('{');
                    }
                    continue;
                }

                if Self::is_identifier_start(bytes[i]) {
                    i += 1;
                    while i < quote_end && Self::is_identifier_continue(bytes[i]) {
                        i += 1;
                    }

                    if i + 1 < quote_end && bytes[i] == b'-' && bytes[i + 1] == b'>' {
                        i += 2;
                        if i < quote_end && bytes[i] == b'{' {
                            if !Self::consume_balanced_in_interpolated_string(
                                bytes, i, b'{', b'}', quote_end,
                            ) {
                                return Some('{');
                            }
                            continue;
                        }
                        if i < quote_end && bytes[i] == b'[' {
                            if !Self::consume_balanced_in_interpolated_string(
                                bytes, i, b'[', b']', quote_end,
                            ) {
                                return Some('[');
                            }
                            continue;
                        }
                        if i < quote_end && bytes[i] == b'(' {
                            if !Self::consume_balanced_in_interpolated_string(
                                bytes, i, b'(', b')', quote_end,
                            ) {
                                return Some('(');
                            }
                            continue;
                        }
                        // bare method name: $obj->method(...) — scan the name, then check for (
                        if i < quote_end && Self::is_identifier_start(bytes[i]) {
                            while i < quote_end && Self::is_identifier_continue(bytes[i]) {
                                i += 1;
                            }
                            if i < quote_end && bytes[i] == b'(' {
                                if !Self::consume_balanced_in_interpolated_string(
                                    bytes, i, b'(', b')', quote_end,
                                ) {
                                    return Some('(');
                                }
                                continue;
                            }
                        }
                    }

                    if i < quote_end && bytes[i] == b'{' {
                        if !Self::consume_balanced_in_interpolated_string(
                            bytes, i, b'{', b'}', quote_end,
                        ) {
                            return Some('{');
                        }
                        continue;
                    }

                    if i < quote_end && bytes[i] == b'[' {
                        if !Self::consume_balanced_in_interpolated_string(
                            bytes, i, b'[', b']', quote_end,
                        ) {
                            return Some('[');
                        }
                        continue;
                    }

                    continue;
                }
            }

            i += 1;
        }

        None
    }

    fn consume_balanced_in_interpolated_string(
        bytes: &[u8],
        start: usize,
        open: u8,
        close: u8,
        quote_end: usize,
    ) -> bool {
        let mut i = start + 1;
        let mut depth = 1usize;

        while i < quote_end {
            let b = bytes[i];
            if b == b'\\' {
                i = i.saturating_add(2);
                continue;
            }
            if b == open {
                depth += 1;
            } else if b == close {
                depth -= 1;
                if depth == 0 {
                    return true;
                }
            }
            i += 1;
        }

        false
    }

    fn is_identifier_start(byte: u8) -> bool {
        byte.is_ascii_alphabetic() || byte == b'_'
    }

    fn is_identifier_continue(byte: u8) -> bool {
        byte.is_ascii_alphanumeric() || byte == b'_'
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

            TokenKind::VString => {
                let token = self.tokens.next()?;
                Ok(Node::new(
                    NodeKind::String { value: token.text.to_string(), interpolated: false },
                    SourceLocation { start: token.start, end: token.end },
                ))
            }

            TokenKind::String => {
                let token = self.tokens.next()?;
                // Check if it's a double-quoted string (interpolated)
                let interpolated = token.text.starts_with('"');
                if interpolated {
                    self.record_unclosed_interpolation_delimiter(&token.text, token.start);
                }
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
                // Nested quantifiers are recorded as a non-fatal diagnostic
                // rather than a hard error to avoid false positives on valid
                // Perl patterns like (?:/\.)+, (\w+)*, (?:pattern)+
                if validator.detect_nested_quantifiers(&body) {
                    self.record_error(ParseError::syntax(
                        "Nested quantifiers detected (possible backtracking risk)",
                        token.start,
                    ));
                }
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
                let text = token.text.as_ref();

                // Detect unclosed bracket-style delimiters in operator strings
                // (e.g. q{...}, qq[...], q(...), q<...>).  Normal 'x' / "x" strings
                // are already handled by the lexer's own unterminated-string detection.
                let op_len = if text.starts_with("qq") {
                    2
                } else if text.starts_with('q') {
                    1
                } else {
                    0
                };
                if op_len > 0 {
                    let after_op = &text[op_len..];
                    if let Some(open) = after_op.chars().next() {
                        let close = match open {
                            '(' => ')',
                            '[' => ']',
                            '{' => '}',
                            '<' => '>',
                            c => c, // symmetric delimiter closes with itself
                        };
                        if !after_op.ends_with(close) {
                            self.record_error(ParseError::syntax(
                                format!(
                                    "Unclosed {} delimiter in string operator before end of file",
                                    open
                                ),
                                token.start,
                            ));
                        }
                    }
                }

                Ok(Node::new(
                    NodeKind::String { value: text.to_string(), interpolated },
                    SourceLocation { start: token.start, end: token.end },
                ))
            }

            TokenKind::QuoteWords => {
                let token = self.tokens.next()?;
                let start = token.start;
                let text = &token.text;

                // Parse qw(...) to extract words
                if let Some(content) = text.strip_prefix("qw") {
                    // Find the delimiter and extract content.
                    // Track whether the closing delimiter was present so we can record an error
                    // when the qw() is unclosed (e.g. qw(one two three at EOF).
                    let (content_str, _delimiter, delim_closed) =
                        if let Some(rest) = content.strip_prefix('(') {
                            let closed = rest.strip_suffix(')').is_some();
                            (rest.strip_suffix(')').unwrap_or(rest), '(', closed)
                        } else if let Some(rest) = content.strip_prefix('[') {
                            let closed = rest.strip_suffix(']').is_some();
                            (rest.strip_suffix(']').unwrap_or(rest), '[', closed)
                        } else if let Some(rest) = content.strip_prefix('{') {
                            let closed = rest.strip_suffix('}').is_some();
                            (rest.strip_suffix('}').unwrap_or(rest), '{', closed)
                        } else if let Some(rest) = content.strip_prefix('<') {
                            let closed = rest.strip_suffix('>').is_some();
                            (rest.strip_suffix('>').unwrap_or(rest), '<', closed)
                        } else {
                            // Other delimiter - find matching pair
                            let delim = content.chars().next().unwrap_or(' ');
                            let inner = &content[delim.len_utf8()..];
                            let trimmed = inner.trim_end_matches(delim);
                            let closed = trimmed.len() < inner.len();
                            (trimmed, delim, closed)
                        };

                    if !delim_closed {
                        self.record_error(ParseError::syntax(
                            "Unclosed qw() delimiter: missing closing delimiter before end of file",
                            start,
                        ));
                    }

                    // Split into words, stripping # line comments first (perlop).
                    let cleaned = strip_qw_comments(content_str);
                    let words: Vec<Node> = cleaned
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
                if validator.detect_nested_quantifiers(&pattern) {
                    self.record_error(ParseError::syntax(
                        "Nested quantifiers detected (possible backtracking risk)",
                        token.start,
                    ));
                }
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
                        negated: false,
                    },
                    SourceLocation { start: token.start, end: token.end },
                ))
            }

            TokenKind::Transliteration => {
                let token = self.tokens.next()?;
                let (search, replace, modifiers) =
                    quote_parser::extract_transliteration_parts_strict(&token.text).map_err(
                        |e| {
                            let message = match e {
                                quote_parser::TransliterationError::InvalidModifier(c) => {
                                    format!(
                                        "Invalid transliteration modifier '{}'. Valid modifiers are: c, d, s, r",
                                        c
                                    )
                                }
                                quote_parser::TransliterationError::InvalidDelimiter(c) => {
                                    format!(
                                        "Invalid transliteration delimiter '{}'. Delimiter must be a non-alphanumeric, non-whitespace character",
                                        c
                                    )
                                }
                                quote_parser::TransliterationError::MissingDelimiter => {
                                    "Missing delimiter after transliteration operator".to_string()
                                }
                                quote_parser::TransliterationError::MissingSearch => {
                                    "Missing search list in transliteration".to_string()
                                }
                                quote_parser::TransliterationError::MissingReplacement => {
                                    "Missing replacement list in transliteration".to_string()
                                }
                                quote_parser::TransliterationError::MissingClosingDelimiter => {
                                    "Missing closing delimiter in transliteration".to_string()
                                }
                            };
                            ParseError::SyntaxError {
                                message,
                                location: token.start,
                            }
                        },
                    )?;

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
                        negated: false,
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

            TokenKind::Eval => {
                // Check for autoquoting: `eval => value`
                if self.is_keyword_before_fat_arrow() {
                    let token = self.tokens.next()?;
                    Ok(Node::new(
                        NodeKind::Identifier { name: token.text.to_string() },
                        SourceLocation { start: token.start, end: token.end },
                    ))
                } else {
                    self.parse_eval()
                }
            }

            TokenKind::Do => {
                // Check for autoquoting: `do => value`
                if self.is_keyword_before_fat_arrow() {
                    let token = self.tokens.next()?;
                    Ok(Node::new(
                        NodeKind::Identifier { name: token.text.to_string() },
                        SourceLocation { start: token.start, end: token.end },
                    ))
                } else {
                    self.parse_do()
                }
            }

            // Note: TokenKind::Sub is handled in the keyword-as-identifier case below
            // This allows 'sub' to be used as a hash key or identifier in expressions
            TokenKind::Try => {
                // Check for autoquoting: `try => value`
                if self.is_keyword_before_fat_arrow() {
                    let token = self.tokens.next()?;
                    Ok(Node::new(
                        NodeKind::Identifier { name: token.text.to_string() },
                        SourceLocation { start: token.start, end: token.end },
                    ))
                } else {
                    self.parse_try()
                }
            }

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
                        "q" | "qq" | "qw" | "qr" | "qx" | "m" | "s" => {
                            // When the quote-op name is immediately before `=>` or `}`,
                            // treat it as a bareword string, not a quote/regex operator.
                            // Cases:
                            //   my %h = (m => 1)    — fat-arrow autoquoting
                            //   $ref->{m}           — hash subscript key (} follows)
                            let next_token = self.tokens.peek_second();
                            let next_is_fat_arrow = matches!(
                                next_token,
                                Ok(t) if t.kind == TokenKind::FatArrow
                            );
                            let next_is_right_brace = matches!(
                                next_token,
                                Ok(t) if t.kind == TokenKind::RightBrace
                            );
                            if next_is_fat_arrow || next_is_right_brace {
                                let tok = self.tokens.next()?;
                                Ok(Node::new(
                                    NodeKind::String {
                                        value: tok.text.to_string(),
                                        interpolated: false,
                                    },
                                    SourceLocation { start: tok.start, end: tok.end },
                                ))
                            } else {
                                self.parse_quote_operator()
                            }
                        }
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
                        "new" => {
                            // When `new` is immediately before `}`, `=>`, or `,`, treat it as a
                            // bareword identifier, not an indirect constructor call.
                            // Cases:
                            //   $h{new} = 1          — hash subscript key (} follows)
                            //   $ref->{new}          — arrow hash subscript key
                            //   delete $h->{new}     — delete with arrow subscript
                            //   (new => 1)           — fat-arrow autoquoting
                            //   @h{new, other}       — hash slice (comma follows)
                            let next_token = self.tokens.peek_second();
                            let next_is_right_brace = matches!(
                                next_token,
                                Ok(t) if t.kind == TokenKind::RightBrace
                            );
                            let next_is_fat_arrow = matches!(
                                next_token,
                                Ok(t) if t.kind == TokenKind::FatArrow
                            );
                            let next_is_comma = matches!(
                                next_token,
                                Ok(t) if t.kind == TokenKind::Comma
                            );
                            if next_is_right_brace || next_is_fat_arrow || next_is_comma {
                                let tok = self.tokens.next()?;
                                return Ok(Node::new(
                                    NodeKind::Identifier { name: tok.text.to_string() },
                                    SourceLocation { start: tok.start, end: tok.end },
                                ));
                            }

                            let new_token = self.tokens.next()?;
                            let start = new_token.start;

                            // If `new` is followed immediately by `(`, treat it as a
                            // plain function call rather than an indirect constructor.
                            // e.g. `new($rtsig, $val, $flags)` inside a sub body (POSIX.pm).
                            // The class name comes from `(` being the next token, not an
                            // identifier, so there is no target class — it resolves at runtime.
                            if self.peek_kind() == Some(TokenKind::LeftParen) {
                                let args = self.parse_args()?;
                                let end = self.previous_position();
                                return Ok(Node::new(
                                    NodeKind::FunctionCall {
                                        name: String::from("new"),
                                        args,
                                    },
                                    SourceLocation { start, end },
                                ));
                            }

                            // Constructor target can be qualified (e.g. IO::Handle)
                            let object = Box::new(self.parse_qualified_identifier()?);
                            let mut args = Vec::new();

                            // In expression context, stop at common delimiters to avoid
                            // consuming surrounding list/argument separators.
                            while !self.tokens.is_eof()
                                && !matches!(
                                    self.peek_kind(),
                                    Some(
                                        TokenKind::Semicolon
                                            | TokenKind::RightParen
                                            | TokenKind::RightBracket
                                            | TokenKind::RightBrace
                                            | TokenKind::Comma
                                            | TokenKind::FatArrow
                                    )
                                )
                            {
                                args.push(self.parse_assignment()?);
                            }

                            let end = self.previous_position();
                            Ok(Node::new(
                                NodeKind::IndirectCall {
                                    method: String::from("new"),
                                    object,
                                    args,
                                },
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

                // Inside parentheses we are no longer at statement start.
                // This prevents the indirect-call heuristic from firing on
                // builtins like `shift`/`pop` inside `(shift @arr)->method()`.
                self.mark_not_stmt_start();

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
                    self.parse_assignment_or_declaration()?
                } else {
                    // Peek ahead to see if this is a list or a complex expression
                    let expr = self.parse_assignment_or_declaration()?;

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

                    // Handle fat arrow after first element — auto-quote bare identifiers
                    if self.peek_kind() == Some(TokenKind::FatArrow) {
                        saw_fat_comma = true;
                        // Auto-quote the key if it is a bare identifier
                        let last_idx = elements.len() - 1;
                        if let NodeKind::Identifier { ref name } = elements[last_idx].kind {
                            let loc = elements[last_idx].location;
                            elements[last_idx] = Node::new(
                                NodeKind::String { value: name.clone(), interpolated: false },
                                loc,
                            );
                        }
                        self.tokens.next()?; // consume =>
                        if self.peek_kind() == Some(TokenKind::FatArrow) {
                            self.tokens.next()?; // consume redundant chained =>
                        }
                        // The value after => may be followed by a word operator inside the list:
                        // e.g. `(key => $val or "default")`.
                        let val = self.parse_assignment_or_declaration()?;
                        elements.push(self.parse_word_or_expr(val)?);
                    }

                    while self.peek_kind() == Some(TokenKind::Comma)
                        || self.peek_kind() == Some(TokenKind::FatArrow)
                    {
                        let was_comma = self.peek_kind() == Some(TokenKind::Comma);
                        if was_comma {
                            self.consume_token()?; // consume comma
                        }

                        // Handle `, =>` (comma then fat arrow) and chained `=>`
                        // where the previous value is now a key.  Auto-quote the
                        // last element when `=>` follows without a preceding comma.
                        if self.peek_kind() == Some(TokenKind::FatArrow) {
                            saw_fat_comma = true;
                            if !was_comma {
                                if let Some(last) = elements.last_mut() {
                                    if let NodeKind::Identifier { ref name } = last.kind {
                                        *last = Node::new(
                                            NodeKind::String { value: name.clone(), interpolated: false },
                                            last.location,
                                        );
                                    }
                                }
                            }
                            self.consume_token()?; // consume =>
                            if self.peek_kind() == Some(TokenKind::FatArrow) {
                                self.consume_token()?; // consume redundant chained =>
                            }
                        }

                        if self.peek_kind() == Some(TokenKind::RightParen) {
                            break;
                        }

                        let mut elem = self.parse_assignment_or_declaration()?;

                        // Check for fat arrow after element — auto-quote bare identifiers
                        if self.peek_kind() == Some(TokenKind::FatArrow) {
                            saw_fat_comma = true;
                            if let NodeKind::Identifier { ref name } = elem.kind {
                                elem = Node::new(
                                    NodeKind::String { value: name.clone(), interpolated: false },
                                    elem.location,
                                );
                            }
                            self.consume_token()?; // consume =>
                            if self.peek_kind() == Some(TokenKind::FatArrow) {
                                self.consume_token()?; // consume redundant chained =>
                            }
                            elements.push(elem);
                            if self.peek_kind() != Some(TokenKind::RightParen) {
                                // The value after => may be followed by a word operator.
                                let val = self.parse_assignment_or_declaration()?;
                                elements.push(self.parse_word_or_expr(val)?);
                            }
                        } else {
                            // Each list element may be followed by a word operator:
                            // e.g. `($val or "default")`.
                            let elem = self.parse_word_or_expr(elem)?;
                            elements.push(elem);
                        }
                    }

                    self.expect_closing_delimiter(TokenKind::RightParen)?;
                    let end = self.previous_position();

                    // Only convert to hash if we saw a fat comma
                    Ok(Self::build_list_or_hash(elements, saw_fat_comma, start, end))
                } else {
                    // It's a parenthesized expression
                    self.expect_closing_delimiter(TokenKind::RightParen)?;
                    Ok(first)
                }
            }

            TokenKind::LeftBracket => {
                // Array reference constructor: [ LIST ]
                //
                // Inside [...] the content is always list context. Fat arrow (=>)
                // acts as a comma with auto-quoting of the left-hand bareword — it
                // does NOT introduce a hash literal. We parse element-by-element
                // using parse_assignment so that comma / fat-arrow separators are
                // consumed at this level rather than being swallowed into a single
                // inner expression by parse_expression -> parse_comma.
                let start_token = self.tokens.next()?; // consume [
                let start = start_token.start;

                let mut elements = Vec::new();

                while self.peek_kind() != Some(TokenKind::RightBracket) && !self.tokens.is_eof() {
                    let mut elem = self.parse_assignment()?;

                    // Fat arrow: auto-quote bare identifiers and consume the =>
                    if self.peek_kind() == Some(TokenKind::FatArrow) {
                        Self::autoquote_fat_arrow_key(&mut elem);
                        self.consume_token()?; // consume =>
                        elements.push(elem);
                        // Parse the value that follows =>
                        if self.peek_kind() != Some(TokenKind::RightBracket) {
                            elements.push(self.parse_assignment()?);
                        }
                    } else {
                        elements.push(elem);
                    }

                    // Consume comma separator; a fat-arrow separator is left
                    // for the top of the next iteration to handle as a key.
                    // e.g. `[a => b => c]` — after pushing `a` and `b`, the
                    // next peek is `=>`, so we do NOT break; we let the loop
                    // re-enter and treat `b` (already pushed) as the key for
                    // the implicit next pair.  Actually `b` is already in
                    // elements — the chained `=>` makes `c` a new element too.
                    // We consume `=>` here so the loop-top `parse_assignment`
                    // picks up `c` as the value.
                    if self.peek_kind() == Some(TokenKind::Comma) {
                        self.consume_token()?; // consume ,
                    } else if self.peek_kind() == Some(TokenKind::FatArrow) {
                        // Chained fat arrow: the value we just pushed becomes
                        // the auto-quoted key for the next pair.  Autoquote the
                        // last element and consume the `=>`.
                        if let Some(last) = elements.last_mut() {
                            Self::autoquote_fat_arrow_key(last);
                        }
                        self.consume_token()?; // consume chained =>
                        // Parse the value that follows the chained =>
                        if self.peek_kind() != Some(TokenKind::RightBracket) && !self.tokens.is_eof() {
                            elements.push(self.parse_assignment()?);
                        }
                        // Continue loop — there may be more separators
                    } else {
                        break;
                    }
                }

                self.expect_closing_delimiter(TokenKind::RightBracket)?;
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
                // Check if the token AFTER 'sub' starts an anonymous subroutine:
                //   sub { ... }          — block body
                //   sub ( ... ) { ... }  — with prototype/signature
                //   sub :attr { ... }    — with attribute(s), e.g. :lvalue, :shared
                // We use peek_second() because peek() is still 'sub' (unconsumed)
                let next = self.tokens.peek_second().ok().map(|t| t.kind);
                if matches!(
                    next,
                    Some(TokenKind::LeftBrace | TokenKind::LeftParen | TokenKind::Colon)
                ) {
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

            // Handle keywords that can be used as identifiers in certain contexts.
            // In expression context, keywords can appear as barewords / hash keys
            // (especially before `=>`).  Control-flow keywords are only safe here
            // because `parse_statement_inner` already handled the real keyword case.
            TokenKind::My
            | TokenKind::Our
            | TokenKind::Local
            | TokenKind::State
            | TokenKind::Field
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
            | TokenKind::Format
            // Control-flow keywords — allowed as barewords in expression context
            // (e.g. `if => 1` or `(for => 2)`)
            | TokenKind::If
            | TokenKind::Elsif
            | TokenKind::Else
            | TokenKind::Unless
            | TokenKind::While
            | TokenKind::Until
            | TokenKind::For
            | TokenKind::Foreach
            | TokenKind::Goto => {
                // In expression context, keywords are used as barewords/identifiers
                // This happens in hash keys, method names, etc.
                let token = self.tokens.next()?;
                Ok(Node::new(
                    NodeKind::Identifier { name: token.text.to_string() },
                    SourceLocation { start: token.start, end: token.end },
                ))
            }

            // return / next / last / redo — when followed by `=>` they are
            // autoquoted hash keys (e.g. `return => 1`).  Otherwise they are
            // real control-flow expressions that can appear inside ternary
            // branches, short-circuit operators, etc.
            TokenKind::Return => {
                if self.is_keyword_before_fat_arrow() {
                    let token = self.tokens.next()?;
                    Ok(Node::new(
                        NodeKind::Identifier { name: token.text.to_string() },
                        SourceLocation { start: token.start, end: token.end },
                    ))
                } else {
                    self.parse_return_expr()
                }
            }

            TokenKind::Next | TokenKind::Last | TokenKind::Redo => {
                if self.is_keyword_before_fat_arrow() {
                    let token = self.tokens.next()?;
                    Ok(Node::new(
                        NodeKind::Identifier { name: token.text.to_string() },
                        SourceLocation { start: token.start, end: token.end },
                    ))
                } else {
                    self.parse_loop_control()
                }
            }

            TokenKind::DoubleColon => {
                // Absolute package path like ::Foo::Bar
                self.parse_qualified_identifier()
            }

            TokenKind::DataMarker => {
                // __END__ / __DATA__ reached in expression context (e.g. after a
                // no-semicolon statement like `__PACKAGE__\n__END__`).  Delegate to
                // the statement-level handler so the data section is parsed correctly
                // rather than emitting an "expected expression" error.
                self.parse_data_section()
            }

            _ => {
                // Get position before consuming
                let pos = self.current_position();
                Err(ParseError::unexpected("expression", token_kind.display_name(), pos))
            }
        }
    }
}
