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
        if !self.at_stmt_start && name != "new" {
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

            // Check for print { $fh } pattern (block-form filehandle)
            // e.g. print { $self->{fh} } "data\n"
            //      print { *STDERR } "error\n"
            // A LeftBrace followed by a sigiled variable or glob is a filehandle block,
            // not a hash constructor or code block.
            if next_kind == TokenKind::LeftBrace
                && matches!(
                    name,
                    "print" | "say" | "printf"
                )
            {
                if let Ok(third) = self.tokens.peek_third() {
                    let third_text = &third.text;
                    // $var or *GLOB inside { } is a filehandle
                    if third_text.starts_with('$') || third_text.starts_with('*') {
                        return true;
                    }
                }
                return false;
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
        //
        // Unary operators that can never take an indirect object are excluded so
        // that `defined $obj->{field}` and `ref $obj->{list}` at statement start
        // do not fire the indirect-call path (which would use parse_primary() and
        // stop before the `->`, producing an error).
        //
        // Array/list manipulation builtins are also excluded because
        // `push $aref->@*, $x` has `$aref->@*` as the first argument, not an
        // indirect object.
        if name.chars().next().is_some_and(|c| c.is_lowercase())
            && !matches!(
                name,
                "tie"
                    | "untie"
                    | "bless"
                    | "push"
                    | "pop"
                    | "shift"
                    | "unshift"
                    | "splice"
                    | "defined"
                    | "ref"
                    | "scalar"
                    | "not"
                    | "abs"
                    | "chr"
                    | "chop"
                    | "chomp"
                    | "lc"
                    | "lcfirst"
                    | "length"
                    | "ord"
                    | "uc"
                    | "ucfirst"
                    | "int"
                    | "hex"
                    | "oct"
                    | "sqrt"
                    | "cos"
                    | "sin"
                    | "exp"
                    | "log"
            )
        {
            if let Ok(next) = self.tokens.peek_second() {
                let next_text = &next.text;
                if next_text.starts_with('$') || next_text.starts_with('@') || next_text.starts_with('%') {
                    // Bare sigil followed by { or [ is a dereference expression
                    // like @{$ref}, %{$hash}, not an indirect object
                    if next_text.len() <= 1 {
                        return false;
                    }
                    // Check if another typical arg or terminator follows to confirm it's not a regular call
                    if let Ok(third) = self.tokens.peek_third() {
                        // Comma or fat arrow means regular call: func $arg, ...
                        // e.g. push @array, $val  or  push @array => $val
                        if matches!(third.kind, TokenKind::Comma | TokenKind::FatArrow) {
                            return false;
                        }
                        // A closing brace/paren/bracket means the call is the
                        // last expression inside a block or parenthesised list,
                        // e.g. `grep { defined $v }` — not an indirect call.
                        if matches!(
                            third.kind,
                            TokenKind::RightBrace | TokenKind::RightParen | TokenKind::RightBracket
                        ) {
                            return false;
                        }
                        // Arrow after $var means method/deref chain: func $obj->method(...)
                        // That's a regular call with a complex first argument, not indirect object.
                        if third.kind == TokenKind::Arrow {
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

        // Some builtins take a full postfix expression as their argument so that
        // arrow-dereference chains are included in the operand:
        //   delete $self->{key}    — $self->{key} is one postfix expr
        //   exists $ref->[0]       — $ref->[0] is one postfix expr
        //   scalar $dh->read       — $dh->read is one postfix expr
        // Other indirect-call builtins (print, say, etc.) only consume
        // the object/filehandle here; remaining args are parsed in the loop below.
        let object = if matches!(
            method.as_str(),
            "delete" | "exists" | "scalar" | "ref" | "defined"
        ) {
            self.parse_postfix()?
        } else {
            self.parse_primary()?
        };

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

    /// Parse an assignment expression or a variable declaration.
    ///
    /// When the current token is `my`, `our`, `local`, or `state`, this
    /// delegates to [`parse_declaration_arg`] so that the declaration is
    /// properly constructed and its initializer uses `parse_assignment()`
    /// (not `parse_expression()`), preventing commas from being absorbed
    /// into the initializer.  Otherwise, falls back to `parse_assignment()`.
    ///
    /// Exception: when the declaration keyword is followed by `=>` (fat
    /// arrow), it is a bareword hash key, not a declaration.  For example:
    /// `(my => "value")` should autoquote `my` as a string.
    fn parse_assignment_or_declaration(&mut self) -> ParseResult<Node> {
        if matches!(
            self.peek_kind(),
            Some(TokenKind::My | TokenKind::Our | TokenKind::Local | TokenKind::State)
        ) && !self.is_keyword_before_fat_arrow()
        {
            self.parse_declaration_arg()
        } else {
            self.parse_assignment()
        }
    }

    /// Parse a variable declaration as a function argument.
    ///
    /// Handles `my $x`, `our @arr`, `local $var`, `state $count` inside
    /// parenthesized argument lists (e.g. `foo(my $x, $y)`).
    ///
    /// Uses `parse_assignment()` for any initializer so that commas are
    /// treated as argument separators rather than being consumed by the
    /// comma operator.
    fn parse_declaration_arg(&mut self) -> ParseResult<Node> {
        let start = self.current_position();
        let declarator_token = self.consume_token()?;
        let declarator = declarator_token.text.to_string();

        // Check if we have a list declaration like `my ($x, $y)`
        if self.peek_kind() == Some(TokenKind::LeftParen) {
            self.consume_token()?; // consume (

            let mut variables = Vec::new();

            while self.peek_kind() != Some(TokenKind::RightParen) && !self.tokens.is_eof() {
                let var = self.parse_variable()?;
                variables.push(var);

                if self.peek_kind() == Some(TokenKind::Comma) {
                    self.consume_token()?; // consume comma
                } else if self.peek_kind() != Some(TokenKind::RightParen) {
                    return Err(ParseError::syntax(
                        "Expected comma or closing parenthesis in variable list",
                        self.current_position(),
                    ));
                }
            }

            self.expect_closing_delimiter(TokenKind::RightParen)?; // consume )

            let initializer = if self.peek_kind() == Some(TokenKind::Assign) {
                self.tokens.next()?; // consume =
                Some(Box::new(self.parse_assignment()?))
            } else {
                None
            };

            let end = self.previous_position();
            Ok(Node::new(
                NodeKind::VariableListDeclaration {
                    declarator,
                    variables,
                    attributes: Vec::new(),
                    initializer,
                },
                SourceLocation { start, end },
            ))
        } else {
            // Single variable declaration
            let variable = if declarator == "local" {
                self.parse_assignment()?
            } else {
                self.parse_variable()?
            };

            let initializer = if self.peek_kind() == Some(TokenKind::Assign) {
                self.tokens.next()?; // consume =
                Some(Box::new(self.parse_assignment()?))
            } else {
                None
            };

            let end = self.previous_position();
            Ok(Node::new(
                NodeKind::VariableDeclaration {
                    declarator,
                    variable: Box::new(variable),
                    attributes: Vec::new(),
                    initializer,
                },
                SourceLocation { start, end },
            ))
        }
    }

    /// Parse function arguments
    /// Handles both comma-separated and fat-comma-separated arguments.
    /// Fat comma (=>) auto-quotes bareword identifiers on its left side.
    fn parse_args(&mut self) -> ParseResult<Vec<Node>> {
        self.with_recursion_guard(|s| {
            s.expect(TokenKind::LeftParen)?;
            let mut args = Vec::new();

            while s.peek_kind() != Some(TokenKind::RightParen) && !s.tokens.is_eof() {
                // Handle variable declarations (my/our/local/state) inside argument lists,
                // otherwise parse as a normal assignment expression.
                let mut arg = s.parse_assignment_or_declaration()?;

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
                        // Handle `, =>` (comma then fat arrow) — consume the
                        // redundant separator.
                        if s.peek_kind() == Some(TokenKind::FatArrow) {
                            s.tokens.next()?;
                        }
                    }
                    _ => break,
                }
            }

            s.expect_closing_delimiter(TokenKind::RightParen)?;
            Ok(args)
        })
    }

}
