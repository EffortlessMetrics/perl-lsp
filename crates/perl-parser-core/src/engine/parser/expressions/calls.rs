impl<'a> Parser<'a> {
    /// Check if this might be an indirect call pattern
    /// We only consider this at statement start to avoid ambiguous mid-expression cases.
    ///
    /// Note: When this is called, the parser has peeked at the function name (e.g., "print")
    /// but not consumed it. So:
    /// - peek() returns the function name (current position)
    /// - peek_second() returns the token after the function name
    /// - peek_third() returns two tokens after the function name
    fn is_indirect_call_pattern(&mut self, name: &str) -> bool {
        // Only check for indirect objects at statement start to avoid false positives
        // in contexts like: my $x = 1; if (1) { print $x; }
        if !self.at_stmt_start {
            return false;
        }

        // print "string" should not be treated as indirect object syntax
        // Note: peek_second() gets the token after "print" since peek() is "print"
        if name == "print" {
            if let Ok(next) = self.tokens.peek_second() {
                if next.kind == TokenKind::String {
                    return false;
                }
            }
        }

        // Known builtins that commonly use indirect object syntax
        let indirect_builtins = [
            "print", "printf", "say", "open", "close", "pipe", "sysopen", "sysread", "syswrite",
            "truncate", "fcntl", "ioctl", "flock", "seek", "tell", "select", "binmode", "exec",
            "system",
        ];

        // Check if it's a known builtin
        if indirect_builtins.contains(&name) {
            // Peek at the token AFTER the function name (use peek_second since peek is the function name)
            let next_token = if let Ok(next) = self.tokens.peek_second() {
                next
            } else {
                return false;
            };
            let next_kind = next_token.kind;
            let next_text = &next_token.text;

            // These tokens *cannot* start an indirect object
            match next_kind {
                TokenKind::Semicolon
                | TokenKind::RightBrace
                | TokenKind::RightParen
                | TokenKind::Comma
                | TokenKind::Eof => return false,
                _ => {}
            }

            // Check for print $fh $x pattern first (variable followed by another arg)
            // This must be checked before the STDOUT pattern because $fh is also an Identifier
            if next_text.starts_with('$') {
                // Only treat $var as an indirect object if a typical argument follows
                // without a comma. A comma means it's a regular argument list.
                // This prevents misclassifying `print $x, $y` as indirect object.
                // Use peek_third() to look at the token after $fh
                if let Ok(third) = self.tokens.peek_third() {
                    // A comma after $fh means regular argument list, NOT indirect object
                    // e.g., print $x, $y; is print both to STDOUT
                    if third.kind == TokenKind::Comma {
                        return false;
                    }

                    // Allow classic argument starts and sigiled variables ($x, @arr, %hash)
                    let third_text = &third.text;
                    return matches!(
                        third.kind,
                        TokenKind::String       // print $fh "x"
                        | TokenKind::LeftParen    // print $fh ($x)
                        | TokenKind::LeftBracket  // print $fh [$x]
                        | TokenKind::LeftBrace    // print $fh { ... }
                    ) || third_text.starts_with('$')    // print $fh $x
                      || third_text.starts_with('@')    // print $fh @array
                      || third_text.starts_with('%'); // print $fh %hash
                }
                return false; // Can't see more; be conservative
            }

            // print STDOUT ... (uppercase bareword filehandle)
            // But NOT if followed by comma — that's a regular call: open FILE, "..."
            if next_kind == TokenKind::Identifier {
                if next_text.chars().next().is_some_and(|c| c.is_uppercase()) {
                    if let Ok(third) = self.tokens.peek_third() {
                        if third.kind == TokenKind::Comma {
                            return false;
                        }
                    }
                    return true;
                }
            }
        }

        // Check for "new ClassName" pattern
        if name == "new" {
            // peek_second() gets the token after "new"
            if let Ok(next) = self.tokens.peek_second() {
                if let TokenKind::Identifier = next.kind {
                    // Uppercase identifier after "new" suggests constructor
                    if next.text.chars().next().is_some_and(|c| c.is_uppercase()) {
                        return true;
                    }
                }
            }
        }

        // AC1: General indirect method call heuristic: method $object
        // Lowercase identifier followed by a sigiled variable ($x, @arr, %hash)
        if name.chars().next().is_some_and(|c| c.is_lowercase()) 
           && !matches!(name, "tie" | "untie") 
        {
            if let Ok(next) = self.tokens.peek_second() {
                let next_text = &next.text;
                if next_text.starts_with('$') || next_text.starts_with('@') || next_text.starts_with('%') {
                    // Check if another typical arg or terminator follows to confirm it's not a regular call
                    if let Ok(third) = self.tokens.peek_third() {
                        // Comma means regular call: func $arg, ...
                        if third.kind == TokenKind::Comma {
                            return false;
                        }
                        return true;
                    }
                    return true;
                }
            }
        }

        false
    }

    /// Parse indirect object/method call
    fn parse_indirect_call(&mut self) -> ParseResult<Node> {
        // Use recursion guard to prevent stack overflow on deep nesting
        // Indirect calls can be nested: new Class(new Class(new Class()))
        self.check_recursion()?;
        
        let start = self.current_position();
        let method_token = self.consume_token()?; // consume method name
        let method = method_token.text.to_string();

        // We're consuming the function name, no longer at statement start
        self.mark_not_stmt_start();

        // Parse the object/filehandle
        let object = self.parse_primary()?;

        // Parse remaining arguments
        let mut args = vec![];

        // Continue parsing arguments until we hit a statement terminator
        // Word operators (or, and, not, xor) bind less tightly than list operators,
        // so they terminate argument collection for indirect calls.
        while !Self::is_statement_terminator(self.peek_kind())
            && !self.is_statement_modifier_keyword()
            && !matches!(
                self.peek_kind(),
                Some(TokenKind::WordOr | TokenKind::WordAnd | TokenKind::WordXor | TokenKind::WordNot)
            )
        {
            // Use parse_assignment instead of parse_expression to avoid grouping by comma operator
            args.push(self.parse_assignment()?);

            // Check if we should continue (comma is optional in indirect syntax)
            if self.peek_kind() == Some(TokenKind::Comma) {
                self.tokens.next()?; // consume comma
            } else if Self::is_statement_terminator(self.peek_kind())
                || self.is_statement_modifier_keyword()
            {
                break;
            }
        }

        let end = self.previous_position();
        
        self.exit_recursion();

        // Return as an indirect call node (using MethodCall with a flag or separate node)
        Ok(Node::new(
            NodeKind::IndirectCall { method, object: Box::new(object), args },
            SourceLocation { start, end },
        ))
    }

    /// Parse function arguments
    /// Handles both comma-separated and fat-comma-separated arguments.
    /// Fat comma (=>) auto-quotes bareword identifiers on its left side.
    fn parse_args(&mut self) -> ParseResult<Vec<Node>> {
        self.with_recursion_guard(|s| {
            s.expect(TokenKind::LeftParen)?;
            let mut args = Vec::new();

            while s.peek_kind() != Some(TokenKind::RightParen) && !s.tokens.is_eof() {
                // Use parse_assignment instead of parse_expression to avoid comma operator handling
                let mut arg = s.parse_assignment()?;

                // Check for fat arrow after the argument
                // If we see =>, the argument should be auto-quoted if it's a bare identifier
                if s.peek_kind() == Some(TokenKind::FatArrow) {
                    // Auto-quote bare identifiers before =>
                    if let NodeKind::Identifier { ref name } = arg.kind {
                        // Convert identifier to string (auto-quoting)
                        arg = Node::new(
                            NodeKind::String { value: name.clone(), interpolated: false },
                            arg.location,
                        );
                    }
                    args.push(arg);
                    s.tokens.next()?; // consume =>
                    // Continue to parse more arguments (the value after =>)
                    continue;
                }

                args.push(arg);

                // Accept both comma and fat arrow as separators
                match s.peek_kind() {
                    Some(TokenKind::Comma) | Some(TokenKind::FatArrow) => {
                        s.tokens.next()?;
                    }
                    _ => break,
                }
            }

            s.expect(TokenKind::RightParen)?;
            Ok(args)
        })
    }

}
