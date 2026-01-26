impl<'a> Parser<'a> {
    /// Parse quote operator (q, qq, qw, qr, qx)
    fn parse_quote_operator(&mut self) -> ParseResult<Node> {
        let op_token = self.consume_token()?; // consume q/qq/qw/qr/qx
        let start = op_token.start;
        let op = op_token.text.as_ref();

        // Get the delimiter - it might be a bracket token or other punctuation
        let delim_token = self.consume_token()?;
        let delim_char = match delim_token.kind {
            TokenKind::LeftBrace => '{',
            TokenKind::LeftBracket => '[',
            TokenKind::LeftParen => '(',
            TokenKind::Less => '<',
            _ => delim_token.text.chars().next().ok_or_else(|| {
                ParseError::syntax("Expected delimiter after quote operator", delim_token.start)
            })?,
        };

        // Determine closing delimiter
        let close_delim = match delim_char {
            '{' => '}',
            '[' => ']',
            '(' => ')',
            '<' => '>',
            _ => delim_char, // For other delimiters like / or |, use the same char
        };

        // Store delimiters for later use
        let opening_delim = delim_char;
        let closing_delim = close_delim;

        // Collect content until closing delimiter
        let mut content = String::new();
        
        // For regex operators (m, s), we need to preserve the exact pattern
        let preserve_exact_content = matches!(op, "m" | "s" | "qr");

        // Stack-based matching for balanced delimiters
        // For non-balanced, we just look for the closing delimiter
        if matches!(delim_char, '{' | '[' | '(' | '<') {
            let mut depth = 1;
            let max_depth = 50; // Limit nesting depth to prevent timeouts
            
            while depth > 0 && !self.tokens.is_eof() {
                let token_kind = self.peek_kind();
                
                // Check if we hit recursion limit
                if depth > max_depth {
                    return Err(ParseError::syntax(
                        format!("Quote delimiter nesting too deep (exceeded {})", max_depth), 
                        self.current_position()
                    ));
                }

                match (delim_char, token_kind) {
                    ('{', Some(TokenKind::LeftBrace)) => {
                        self.consume_token()?;
                        content.push('{');
                        depth += 1;
                    }
                    ('{', Some(TokenKind::RightBrace)) => {
                        self.consume_token()?;
                        depth -= 1;
                        if depth > 0 {
                            content.push('}');
                        }
                    }
                    ('[', Some(TokenKind::LeftBracket)) => {
                        self.consume_token()?;
                        content.push('[');
                        depth += 1;
                    }
                    ('[', Some(TokenKind::RightBracket)) => {
                        self.consume_token()?;
                        depth -= 1;
                        if depth > 0 {
                            content.push(']');
                        }
                    }
                    ('(', Some(TokenKind::LeftParen)) => {
                        self.consume_token()?;
                        content.push('(');
                        depth += 1;
                    }
                    ('(', Some(TokenKind::RightParen)) => {
                        self.consume_token()?;
                        depth -= 1;
                        if depth > 0 {
                            content.push(')');
                        }
                    }
                    ('<', Some(TokenKind::Less)) => {
                        self.consume_token()?;
                        content.push('<');
                        depth += 1;
                    }
                    ('<', Some(TokenKind::Greater)) => {
                        self.consume_token()?;
                        depth -= 1;
                        if depth > 0 {
                            content.push('>');
                        }
                    }
                    _ => {
                        // Regular token, add to content
                        let token = self.consume_token()?;
                        content.push_str(&token.text);
                        if !preserve_exact_content && !self.tokens.is_eof() && !content.is_empty() {
                            content.push(' ');
                        }
                    }
                }
            }
        } else {
            // For non-balanced delimiters, just scan for the closing char
            // This avoids recursion entirely
            while !self.tokens.is_eof() {
                let token = self.consume_token()?;
                if token.text.contains(close_delim) {
                    let pos = token.text.find(close_delim).ok_or_else(|| {
                        ParseError::syntax("Closing delimiter not found in token", token.start)
                    })?;
                    content.push_str(&token.text[..pos]);
                    break;
                } else {
                    content.push_str(&token.text);
                    if !preserve_exact_content && !self.tokens.is_eof() {
                        content.push(' ');
                    }
                }
            }
        }

        // Parse modifiers for regex operators
        let mut modifiers = String::new();
        if matches!(op, "m" | "qr") {
            // Check for modifiers (letters after closing delimiter)
            while let Ok(token) = self.tokens.peek() {
                if token.kind == TokenKind::Identifier && token.text.len() == 1 {
                    // Single letter identifier could be a modifier
                    let ch =
                        token.text.chars().next().ok_or_else(|| {
                            ParseError::syntax("Empty identifier token", token.start)
                        })?;
                    if ch.is_ascii_alphabetic() {
                        modifiers.push(ch);
                        self.tokens.next()?;
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            }
        }

        let mut end = self.previous_position();

        // Create appropriate node based on operator
        match op {
            "qq" => {
                // Double-quoted string with interpolation
                Ok(Node::new(
                    NodeKind::String { value: format!("\"{}\"", content), interpolated: true },
                    SourceLocation { start, end },
                ))
            }
            "q" => {
                // Single-quoted string without interpolation
                Ok(Node::new(
                    NodeKind::String { value: format!("'{}'", content), interpolated: false },
                    SourceLocation { start, end },
                ))
            }
            "qw" => {
                // Word list - split on whitespace
                let words: Vec<Node> = content
                    .split_whitespace()
                    .map(|word| {
                        Node::new(
                            NodeKind::String { value: format!("'{}'", word), interpolated: false },
                            SourceLocation { start, end },
                        )
                    })
                    .collect();

                Ok(Node::new(
                    NodeKind::ArrayLiteral { elements: words },
                    SourceLocation { start, end },
                ))
            }
            "qr" => {
                // Regular expression
                // Validate regex complexity and check for embedded code
                let validator = crate::engine::regex_validator::RegexValidator::new();
                validator.validate(&content, start)?;
                let has_embedded_code = validator.detects_code_execution(&content);

                Ok(Node::new(
                    NodeKind::Regex {
                        pattern: format!("{}{}{}", opening_delim, content, closing_delim),
                        replacement: None,
                        modifiers,
                        has_embedded_code,
                    },
                    SourceLocation { start, end },
                ))
            }
            "qx" => {
                // Backticks/command execution
                Ok(Node::new(
                    NodeKind::String { value: format!("`{}`", content), interpolated: true },
                    SourceLocation { start, end },
                ))
            }
            "m" => {
                // Match operator with pattern
                // Validate regex complexity and check for embedded code
                let validator = crate::engine::regex_validator::RegexValidator::new();
                validator.validate(&content, start)?;
                let has_embedded_code = validator.detects_code_execution(&content);

                let mut modifiers = String::new();
                while let Ok(token) = self.tokens.peek() {
                    if token.kind == TokenKind::Identifier && token.text.len() == 1 {
                        let ch = token.text.chars().next().ok_or_else(|| {
                            ParseError::syntax("Empty identifier token", token.start)
                        })?;
                        if ch.is_ascii_alphabetic() {
                            modifiers.push(ch);
                            self.tokens.next()?;
                        } else {
                            break;
                        }
                    } else {
                        break;
                    }
                }
                end = self.previous_position();
                Ok(Node::new(
                    NodeKind::Regex {
                        pattern: format!("{}{}{}", opening_delim, content, closing_delim),
                        replacement: None,
                        modifiers,
                        has_embedded_code,
                    },
                    SourceLocation { start, end },
                ))
            }
            "s" => {
                // Substitution operator shouldn't reach here - handled by TokenKind::Substitution
                // This is kept for defensive programming
                Err(ParseError::syntax(
                    "Substitution operator should be handled by TokenKind::Substitution",
                    start,
                ))
            }
            _ => Err(ParseError::syntax(format!("Unknown quote operator: {}", op), start)),
        }
    }

    /// After having consumed the `qw` identifier, parse `qw<delim>...<close>`
    fn parse_qw_words(&mut self) -> ParseResult<Vec<String>> {
        // Grab the opening delimiter as a single *token* (whatever it is).
        // This could be (, [, {, <, or any single character like |, !, #, etc.
        let open = self.tokens.next()?; // e.g., '(', '{', '|', '#', '!'
        let open_txt = &open.text;

        // Special case for # - it causes lexer issues as it starts comments
        // When we see qw#, we need to consume carefully
        if open_txt.as_ref() == "#" {
            let mut words = Vec::<String>::new();

            // The lexer will treat the closing # as starting a comment,
            // so we won't see it as a token. We need to consume words
            // until we hit something that indicates the qw list is done.
            // We'll stop when we see a keyword that starts a new statement.
            while !self.tokens.is_eof() {
                let peek = self.tokens.peek()?;

                // Stop if we see a keyword that starts a new statement
                if matches!(
                    peek.kind,
                    TokenKind::Use
                        | TokenKind::My
                        | TokenKind::Our
                        | TokenKind::Sub
                        | TokenKind::Package
                        | TokenKind::If
                        | TokenKind::While
                        | TokenKind::For
                        | TokenKind::Return
                ) {
                    break;
                }

                // Also stop on semicolon (though we likely won't see it after #)
                if matches!(peek.kind, TokenKind::Semicolon) {
                    break;
                }

                match peek.kind {
                    TokenKind::Identifier | TokenKind::Number => {
                        // Check if this is a keyword that likely isn't part of the qw list
                        if matches!(peek.text.as_ref(), "use" | "constant" | "my" | "our" | "sub") {
                            // Don't consume it, just stop here
                            break;
                        }
                        let t = self.tokens.next()?;
                        words.push(t.text.to_string());
                    }
                    _ => {
                        // Skip other tokens
                        self.tokens.next()?;
                    }
                }
            }
            return Ok(words);
        }

        let close_txt = if let Some(ct) = Self::closing_delim_for(open_txt) {
            ct
        } else {
            // If we can't determine closing delimiter, use the same as opening for symmetric
            open_txt.to_string()
        };

        let mut words = Vec::<String>::new();

        // naive word split: treat IDENT/STRING/NUMBER as word atoms; anything else
        // (including newlines and whitespace that your lexer doesn't surface) just
        // acts as a separator or gets skipped.
        while !self.tokens.is_eof() {
            let peek = self.tokens.peek()?;
            if &*peek.text == close_txt.as_str() {
                self.tokens.next()?; // consume closer
                break;
            }

            match self.peek_kind() {
                Some(TokenKind::Identifier) | Some(TokenKind::Number) => {
                    let t = self.tokens.next()?;
                    words.push(t.text.to_string());
                }
                Some(TokenKind::String) => {
                    let t = self.tokens.next()?;
                    // normalize quotes → word (qw() is non-interpolating as list of words)
                    let w = t.text.trim_matches(|c| c == '"' || c == '\'').to_string();
                    if !w.is_empty() {
                        words.push(w);
                    }
                }
                // Skip whitespace, newlines, and any other tokens
                _ => {
                    self.tokens.next()?;
                }
            }
        }
        Ok(words)
    }

    /// Parse qw() word list
    fn parse_qw_list(&mut self) -> ParseResult<Vec<Node>> {
        // Handle different delimiters for qw
        let delimiter_token = self.tokens.peek()?.clone();
        let close_delim = match delimiter_token.kind {
            TokenKind::LeftParen => {
                self.consume_token()?;
                TokenKind::RightParen
            }
            TokenKind::LeftBracket => {
                self.consume_token()?;
                TokenKind::RightBracket
            }
            TokenKind::LeftBrace => {
                self.consume_token()?;
                TokenKind::RightBrace
            }
            TokenKind::Less => {
                self.consume_token()?;
                TokenKind::Greater
            }
            // For other delimiters like |, !, #, ~, etc.
            _ => {
                // Try to consume whatever delimiter is there
                // For now, default to parentheses if we don't recognize it
                self.expect(TokenKind::LeftParen)?;
                TokenKind::RightParen
            }
        };

        let mut words = Vec::new();

        // Parse space-separated words until closing delimiter
        while self.peek_kind() != Some(close_delim) && !self.tokens.is_eof() {
            if let Some(TokenKind::Identifier) = self.peek_kind() {
                let token = self.tokens.next()?;
                words.push(Node::new(
                    NodeKind::String {
                        value: format!("'{}'", token.text), // qw produces single-quoted strings
                        interpolated: false,
                    },
                    SourceLocation { start: token.start, end: token.end },
                ));
            } else if self.peek_kind() == Some(TokenKind::String) {
                // Also allow string tokens in qw lists
                let token = self.tokens.next()?;
                words.push(Node::new(
                    NodeKind::String {
                        value: format!("'{}'", token.text.trim_matches(|c| c == '"' || c == '\'')),
                        interpolated: false,
                    },
                    SourceLocation { start: token.start, end: token.end },
                ));
            } else {
                // Skip other tokens (might be separators or special chars)
                self.tokens.next()?;
            }
        }

        self.expect(close_delim)?;
        Ok(words)
    }

}
