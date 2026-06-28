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
        //
        // Exception: print/say/printf/exec/system with a block filehandle `{ $fh }`
        // or a `$var VAR` (two consecutive sigiled tokens, no comma) are unambiguous
        // enough to allow outside statement-start context.
        let is_filehandle_builtin = matches!(name, "print" | "say" | "printf" | "exec" | "system");
        if !self.at_stmt_start && !is_filehandle_builtin && name != "new" {
            return false;
        }
        // In non-stmt-start context for print/etc., only allow the unambiguous forms:
        // 1. `{ $fh }` block-form filehandle
        // 2. `$var $something` (two sigiled tokens, no comma) — indirect $fh
        if !self.at_stmt_start && is_filehandle_builtin {
            // Clone the needed info to avoid multiple borrows of self.tokens.
            let next_kind = self.tokens.peek_second().ok().map(|t| t.kind);
            let next_text = self.tokens.peek_second().ok().map(|t| t.text.to_string());
            let third_kind = self.tokens.peek_third().ok().map(|t| t.kind);
            let third_text = self.tokens.peek_third().ok().map(|t| t.text.to_string());

            if let Some(nk) = next_kind {
                // Form 1: block filehandle `print { $fh } ...`
                if nk == TokenKind::LeftBrace {
                    if let Some(ref txt) = third_text {
                        if txt.starts_with('$') || txt.starts_with('*') {
                            return true;
                        }
                    }
                }
                // Form 2: variable filehandle `print $fh $msg` (no comma after $fh)
                if next_text.as_deref().unwrap_or("").starts_with('$') {
                    // If next-next starts with $ or @ or is a string, it's likely
                    // `print $fh $msg` — no comma between filehandle and message.
                    // A comma means it's a regular `print $var, $other` list.
                    if third_kind != Some(TokenKind::Comma) {
                        if let Some(ref txt) = third_text {
                            if txt.starts_with('$')
                                || txt.starts_with('@')
                                || third_kind == Some(TokenKind::String)
                            {
                                return true;
                            }
                        }
                    }
                }
            }
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
                // Word operators bind below list operators, so they terminate
                // a zero-arg builtin call instead of starting an indirect object.
                kind if kind.is_low_precedence_word_operator() => return false,
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
                    // NOTE: LeftBracket and LeftBrace are intentionally excluded here.
                    // When the second token is a $-sigiled variable, a following `[` or `{`
                    // is ALWAYS a subscript of that variable ($var[i] or $var{key}), never
                    // the start of a separate indirect-object argument. Perl treats both
                    // `print $hash{key}` and `print $hash {key}` (with a space) identically
                    // — both are subscripts of $hash, never filehandle + argument.
                    // Verified via `perl -MO=Terse`. See issue #974.
                    let third_text = &third.text;
                    return matches!(
                        third.kind,
                        TokenKind::String       // print $fh "x"
                        | TokenKind::LeftParen    // print $fh ($x)
                    ) || third_text.starts_with('$')    // print $fh $x
                      || third_text.starts_with('@')    // print $fh @array
                      || third_text.starts_with('%'); // print $fh %hash
                }
                return false; // Can't see more; be conservative
            }

            // print STDOUT ... (uppercase bareword filehandle)
            // But NOT if followed by comma — that's a regular call: open FILE, "..."
            // And NOT if followed by arrow — that's a class method chain:
            //   print Data::Dumper->new([$self])->Dump()
            // And NOT if followed by fat arrow — that's a hash-style list, NOT indirect:
            //   print STDERR => "msg"  means  print(STDERR => "msg"), not print to STDERR
            if next_kind == TokenKind::Identifier {
                if next_text.chars().next().is_some_and(|c| c.is_uppercase()) {
                    if let Ok(third) = self.tokens.peek_third() {
                        if matches!(
                            third.kind,
                            TokenKind::Comma | TokenKind::Arrow | TokenKind::FatArrow
                        ) {
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
        //
        // Block-list functions (sort, map, grep, etc.) are excluded because
        // `sort @list or die` is a regular function call with word operator,
        // not an indirect method call.
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
                    // Block-list functions: `sort @list or die` is a regular call
                    | "sort"
                    | "map"
                    | "grep"
                    | "reverse"
                    | "join"
                    // Common builtins whose first arg is never an indirect object
                    | "die"
                    | "warn"
                    | "carp"
                    | "croak"
                    | "confess"
                    | "cluck"
                    | "exit"
                    | "eval"
                    | "require"
                    | "return"
                    | "delete"
                    | "exists"
                    | "values"
                    | "keys"
                    | "each"
                    | "local"
                    | "wantarray"
                    | "caller"
                    | "mkdir"
                    | "rmdir"
                    | "chdir"
                    | "chmod"
                    | "chown"
                    | "unlink"
                    | "rename"
                    | "symlink"
                    | "link"
                    | "readlink"
                    | "stat"
                    | "lstat"
                    | "chroot"
                    | "utime"
                    | "umask"
                    // `pos` is an lvalue-capable optional-arg builtin; `pos $s = value`
                    // must not be treated as an indirect method call.
                    | "pos"
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
                        // Short-circuit operators after $var mean a regular call with
                        // the operator outside the argument: func $arg || die,
                        // func @list or die, func $arg and next, etc.
                        // These are NOT indirect method calls.
                        if Self::is_symbolic_short_circuit_operator(Some(third.kind))
                            || matches!(
                                third.kind,
                                TokenKind::WordOr
                                    | TokenKind::WordAnd
                                    | TokenKind::WordXor
                                    | TokenKind::WordNot
                                    | TokenKind::Question
                                    | TokenKind::Semicolon
                            )
                        {
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

        // In Perl, `=>` (fat arrow) is equivalent to `,` everywhere, including
        // in indirect-call argument lists.  Consume a leading fat arrow, or the
        // comma form for the double-sigil object shape that otherwise reaches
        // this parser as an indirect call:
        //   print STDERR => "msg";  — STDERR is object, "msg" is first arg
        //   imported $$obj, $arg    — double-sigil object plus argument list
        //   imported_fn $$obj, $arg — same but after unbraced-deref fix, object is Unary ${}
        let comma_after_double_sigil_object = self.peek_kind() == Some(TokenKind::Comma)
            && (matches!(
                &object.kind,
                NodeKind::Variable { sigil, name } if sigil == "$" && name.starts_with('$')
            ) || matches!(
                &object.kind,
                NodeKind::Unary { op, .. } if op == "${}"
            ));
        if self.peek_kind() == Some(TokenKind::FatArrow) || comma_after_double_sigil_object {
            self.tokens.next()?; // consume , or =>
        }

        // Continue parsing arguments until we hit a statement terminator
        // Word operators (or, and, not, xor) bind less tightly than list operators,
        // so they terminate argument collection for indirect calls.
        while !Self::is_statement_terminator(self.peek_kind())
            && !self.is_statement_modifier_keyword()
            && !Self::is_symbolic_short_circuit_operator(self.peek_kind())
            && !matches!(
                self.peek_kind(),
                Some(
                    TokenKind::WordOr
                        | TokenKind::WordAnd
                        | TokenKind::WordXor
                        | TokenKind::WordNot
                        | TokenKind::Question
                        | TokenKind::RightBrace
                        | TokenKind::RightParen
                        | TokenKind::RightBracket
                )
            )
        {
            // Use parse_assignment instead of parse_expression to avoid grouping by comma operator
            args.push(self.parse_assignment()?);

            // Check if we should continue (comma or fat arrow as separator in indirect syntax)
            if matches!(
                self.peek_kind(),
                Some(TokenKind::Comma | TokenKind::FatArrow)
            ) {
                self.tokens.next()?; // consume , or =>
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
    ///
    /// After a declaration **without** an `=` initializer, binary operators
    /// such as `&&`, `||`, `=~` etc. that follow are consumed using
    /// `parse_below_assignment_with()`.  This handles patterns like:
    ///
    ///   `if (our $CAN_HAZ_XS && $ok) { … }`   — Pattern D
    ///   `(our $AUTOLOAD =~ /([^:]+)$/)`          — Pattern E
    fn parse_assignment_or_declaration(&mut self) -> ParseResult<Node> {
        if matches!(
            self.peek_kind(),
            Some(TokenKind::My | TokenKind::Our | TokenKind::Local | TokenKind::State)
        ) && !self.is_keyword_before_fat_arrow()
        {
            self.parse_declaration_expression()
        } else {
            self.parse_assignment()
        }
    }

    /// Parse a declaration used as an expression and consume any binary operator
    /// that binds to the declaration itself.
    ///
    /// This covers both argument-list contexts and RHS expression contexts:
    ///
    ///   `(our $AUTOLOAD =~ /pattern/)`
    ///   `(my $field = our $AUTOLOAD) =~ s/.*://`
    fn parse_declaration_expression(&mut self) -> ParseResult<Node> {
        let decl = self.parse_declaration_arg()?;

        // After a declaration that has no initializer (no `=` was consumed),
        // binary operators like `&&`, `||`, `=~`, `?:`, etc. may follow.
        // Apply the full binary-operator chain so that the declaration node
        // is correctly used as the left operand.
        //
        // We detect the "no initializer" case by checking whether the
        // declaration node itself contains an initializer. If parse_declaration_arg
        // already consumed an `=` and its rhs, parse_assignment() handled all
        // operators inside the initializer, so no further processing is needed.
        let has_initializer = match &decl.kind {
            NodeKind::VariableDeclaration { initializer, .. } => initializer.is_some(),
            NodeKind::VariableListDeclaration { initializer, .. } => initializer.is_some(),
            _ => true, // not a declaration node; treat as already complete
        };

        if has_initializer {
            Ok(decl)
        } else {
            self.parse_below_assignment_with(decl)
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

        // `local(...)` can contain arbitrary lvalue expressions (e.g. local($ENV{PATH})).
        // Only use the single-lvalue path when the content is a complex lvalue
        // (the token after the first variable sigil is a subscript operator like `{`, `[`, `->`)
        // so that plain-variable lists like `local($x, $y)` fall through to the
        // VariableListDeclaration path which correctly handles the comma-separated form.
        let local_has_complex_lvalue = declarator == "local"
            && self.peek_kind() == Some(TokenKind::LeftParen)
            && matches!(
                self.tokens.peek_third().ok().map(|t| t.kind),
                Some(TokenKind::LeftBrace | TokenKind::LeftBracket | TokenKind::Arrow)
            );
        if local_has_complex_lvalue {
            self.consume_token()?; // consume (
            let variable = self.parse_assignment()?;
            self.expect_closing_delimiter(TokenKind::RightParen)?;

            let initializer = if self.peek_kind() == Some(TokenKind::Assign) {
                let op_token = self.tokens.next()?; // consume =
                let rhs = if let Some(missing) = self.recover_missing_infix_rhs(op_token.start) {
                    missing
                } else {
                    self.parse_assignment()?
                };
                Some(Box::new(rhs))
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
        // Check if we have a list declaration like `my ($x, $y)`
        } else if self.peek_kind() == Some(TokenKind::LeftParen) {
            self.consume_token()?; // consume (

            let mut variables = Vec::new();

            while self.peek_kind() != Some(TokenKind::RightParen) && !self.tokens.is_eof() {
                let var = self.parse_variable_list_item()?;
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
                let op_token = self.tokens.next()?; // consume =
                let rhs = if let Some(missing) = self.recover_missing_infix_rhs(op_token.start) {
                    missing
                } else {
                    self.parse_assignment()?
                };
                Some(Box::new(rhs))
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
                let op_token = self.tokens.next()?; // consume =
                let rhs = if let Some(missing) = self.recover_missing_infix_rhs(op_token.start) {
                    missing
                } else {
                    self.parse_assignment()?
                };
                Some(Box::new(rhs))
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

    /// Parse arguments for `print(…)`, `say(…)`, `printf(…)`, `send(…)` with
    /// explicit parentheses, detecting the optional filehandle (indirect-object)
    /// as the first argument.
    ///
    /// Perl allows `print( $fh EXPR )` where `$fh` is a scalar variable acting
    /// as the filehandle, with the message list following without a comma.
    /// `parse_args` treats every argument as comma-separated, so the above
    /// would fail when `EXPR` is not preceded by a comma.  This function
    /// detects the pattern and falls back to `parse_args` when the first
    /// argument is not a scalar filehandle.
    fn parse_print_parens_args(&mut self) -> ParseResult<Vec<Node>> {
        self.with_recursion_guard(|s| {
            s.expect(TokenKind::LeftParen)?;

            if s.peek_kind() == Some(TokenKind::RightParen) {
                s.expect_closing_delimiter(TokenKind::RightParen)?;
                return Ok(vec![]);
            }

            // Peek ahead: if the first token is a scalar variable ($fh) and the
            // token after it is NOT a comma, fat-arrow, or `)`, treat it as the
            // indirect filehandle (no comma before the message list).
            let first_is_scalar = s.tokens.peek().is_ok_and(|t| {
                t.text.starts_with('$') && t.text.len() > 1
            });
            let first_is_filehandle_block = s
                .tokens
                .peek()
                .is_ok_and(|t| t.kind == TokenKind::LeftBrace)
                && s.tokens.peek_second().is_ok_and(|t| {
                    t.text.starts_with('$') || t.text.starts_with('*')
                });
            let second_is_not_separator = s.tokens.peek_second().is_ok_and(|t| {
                !matches!(
                    t.kind,
                    TokenKind::Comma | TokenKind::FatArrow | TokenKind::RightParen
                )
            });

            if (first_is_scalar && second_is_not_separator) || first_is_filehandle_block {
                // Filehandle form: parse `$fh` or `{ *FH }` as first argument,
                // then remaining args without requiring a comma separator.
                let mut args = Vec::new();
                let filehandle = s.parse_assignment_or_declaration()?;
                args.push(filehandle);

                // Collect remaining arguments (no comma required after filehandle)
                while s.peek_kind() != Some(TokenKind::RightParen) && !s.tokens.is_eof() {
                    if matches!(s.peek_kind(), Some(TokenKind::Comma) | Some(TokenKind::FatArrow))
                    {
                        s.tokens.next()?;
                    }
                    if s.peek_kind() == Some(TokenKind::RightParen) {
                        break;
                    }
                    args.push(s.parse_assignment_or_declaration()?);
                }

                s.expect_closing_delimiter(TokenKind::RightParen)?;
                Ok(args)
            } else {
                // Regular argument list — parse comma-separated args then close paren.
                let mut args = Vec::new();

                while s.peek_kind() != Some(TokenKind::RightParen) && !s.tokens.is_eof() {
                    let arg = s.parse_assignment_or_declaration()?;
                    args.push(arg);
                    match s.peek_kind() {
                        Some(TokenKind::Comma) | Some(TokenKind::FatArrow) => {
                            s.tokens.next()?;
                        }
                        _ => break,
                    }
                }

                s.expect_closing_delimiter(TokenKind::RightParen)?;
                Ok(args)
            }
        })
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
                    if s.peek_kind() == Some(TokenKind::FatArrow) {
                        s.tokens.next()?; // consume redundant chained =>
                    }
                    // Continue to parse more arguments (the value after =>)
                    continue;
                }

                args.push(arg);

                // Accept both comma and fat arrow as separators.
                // Special case: a Block or HashLiteral argument (from `exec({ ... } @args)`,
                // `map({ ... } @list)`, or `map({k => v} keys ...)`) may be followed by the
                // list without a comma. If we just pushed a Block or HashLiteral and next is
                // not `)` or a separator, treat the rest as implicit comma-separated arguments.
                let last_is_block = matches!(
                    args.last(),
                    Some(n) if matches!(n.kind, NodeKind::Block { .. } | NodeKind::HashLiteral { .. })
                );
                if last_is_block
                    && s.peek_kind() != Some(TokenKind::RightParen)
                    && !matches!(s.peek_kind(), Some(TokenKind::Comma) | Some(TokenKind::FatArrow))
                {
                    // Continue the while loop to parse more args (implicit comma after block)
                    continue;
                }

                match s.peek_kind() {
                    Some(TokenKind::Comma) | Some(TokenKind::FatArrow) => {
                        let separator = s.consume_token()?;
                        if separator.kind == TokenKind::Comma {
                            s.consume_redundant_commas()?;
                        }
                        // Handle `, =>` (comma then fat arrow) — consume the
                        // redundant separator.
                        if s.peek_kind() == Some(TokenKind::FatArrow) {
                            s.consume_token()?;
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
