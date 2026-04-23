impl<'a> Parser<'a> {
    #[inline]
    fn is_statement_terminator(kind: Option<TokenKind>) -> bool {
        matches!(kind, Some(TokenKind::Semicolon) | Some(TokenKind::Eof) | None)
    }

    #[inline]
    fn is_stmt_modifier_kind(kind: TokenKind) -> bool {
        matches!(
            kind,
            TokenKind::If
                | TokenKind::Unless
                | TokenKind::While
                | TokenKind::Until
                | TokenKind::For
                | TokenKind::When
                | TokenKind::Foreach
        )
    }

    #[inline]
    fn is_logical_or(kind: Option<TokenKind>) -> bool {
        matches!(kind, Some(TokenKind::Or) | Some(TokenKind::DefinedOr))
    }

    /// Returns true if the token kind is a postfix increment/decrement operator.
    ///
    /// These operators (`++` and `--`) can appear after an expression like `$x++`
    /// unlike prefix operators which appear before.
    #[inline]
    fn is_postfix_op(kind: Option<TokenKind>) -> bool {
        matches!(kind, Some(TokenKind::Increment) | Some(TokenKind::Decrement))
    }

    fn peek_compound_assign_op(&mut self) -> Option<&'static str> {
        match self.peek_kind()? {
            TokenKind::Assign => Some("="),
            TokenKind::PlusAssign => Some("+="),
            TokenKind::MinusAssign => Some("-="),
            TokenKind::StarAssign => Some("*="),
            TokenKind::SlashAssign => Some("/="),
            TokenKind::PercentAssign => Some("%="),
            TokenKind::DotAssign => Some(".="),
            TokenKind::AndAssign => Some("&="),
            TokenKind::OrAssign => Some("|="),
            TokenKind::XorAssign => Some("^="),
            TokenKind::PowerAssign => Some("**="),
            TokenKind::LeftShiftAssign => Some("<<="),
            TokenKind::RightShiftAssign => Some(">>="),
            TokenKind::LogicalAndAssign => Some("&&="),
            TokenKind::LogicalOrAssign => Some("||="),
            TokenKind::DefinedOrAssign => Some("//="),
            _ => None,
        }
    }

    #[inline]
    fn is_variable_sigil(kind: Option<TokenKind>) -> bool {
        matches!(
            kind,
            Some(TokenKind::ScalarSigil) | Some(TokenKind::ArraySigil) | Some(TokenKind::HashSigil)
        )
    }

    /// Returns `true` when `field` is followed by tokens that indicate a
    /// field *declaration* (Perl 5.38+ `field $var` syntax) rather than a
    /// function call or bareword identifier.
    ///
    /// A field declaration requires a variable sigil immediately after the
    /// keyword. Everything else
    /// (`field("str")`, `field()`, `field;`, etc.) is a regular
    /// expression / function call.
    fn is_field_declaration_context(&mut self) -> bool {
        let next = match self.tokens.peek_second() {
            Ok(t) => t.kind,
            Err(_) => return false,
        };

        // Direct sigil token after `field` — definitely a declaration
        if matches!(
            next,
            TokenKind::ScalarSigil
                | TokenKind::ArraySigil
                | TokenKind::HashSigil
                | TokenKind::SubSigil
                | TokenKind::GlobSigil
                | TokenKind::Percent
                | TokenKind::BitwiseAnd
                | TokenKind::Star
        ) {
            return true;
        }

        // Identifier that starts with a sigil char (e.g. `$name`, `@items`)
        if next == TokenKind::Identifier {
            if let Ok(t) = self.tokens.peek_second() {
                if let Some(ch) = t.text.chars().next() {
                    if matches!(ch, '$' | '@' | '%' | '*' | '&') {
                        return true;
                    }
                }
            }
        }

        false
    }

    /// After a leading block/hash argument in a bare call, decide whether the
    /// following token should be parsed as another implicit argument.
    ///
    /// Perl permits `func { ... } @list` and DSL-style named args such as
    /// `func { ... } foreach => $items` without a comma after the block. We
    /// still stop at real statement boundaries and at postfix modifiers unless
    /// the modifier token is being autoquoted before `=>`.
    fn should_continue_bare_call_after_block(&mut self) -> bool {
        match self.peek_kind() {
            Some(TokenKind::Semicolon)
            | Some(TokenKind::RightBrace)
            | Some(TokenKind::RightParen)
            | Some(TokenKind::RightBracket)
            | Some(TokenKind::Eof)
            | None => false,
            Some(TokenKind::WordOr | TokenKind::WordAnd | TokenKind::WordXor | TokenKind::WordNot) => {
                false
            }
            Some(kind) if Self::is_stmt_modifier_kind(kind) => self.is_keyword_before_fat_arrow(),
            _ => true,
        }
    }

    /// Check recursion depth with optimized hot path
    #[inline(always)]
    fn check_recursion(&mut self) -> ParseResult<()> {
        self.recursion_depth += 1;
        // Fast path: avoid expensive comparisons in the common case
        if self.recursion_depth > MAX_RECURSION_DEPTH {
            return Err(ParseError::NestingTooDeep {
                depth: self.recursion_depth,
                max_depth: MAX_RECURSION_DEPTH,
            });
        }
        Ok(())
    }

    fn exit_recursion(&mut self) {
        self.recursion_depth = self.recursion_depth.saturating_sub(1);
    }

    /// Run `f` under the recursion depth budget.
    ///
    /// - `check_recursion()` increments depth (and may error)
    /// - depth is decremented on scope exit (even on early return / panic)
    #[inline]
    fn with_recursion_guard<T>(
        &mut self,
        f: impl FnOnce(&mut Self) -> ParseResult<T>,
    ) -> ParseResult<T> {
        self.check_recursion()?;

        struct Guard<'p, 'src>(&'p mut Parser<'src>);
        impl<'p, 'src> Drop for Guard<'p, 'src> {
            fn drop(&mut self) {
                self.0.exit_recursion();
            }
        }

        let guard = Guard(self);
        f(guard.0)
    }

    /// Check if an identifier is a builtin function that can take arguments without parens.
    /// Delegates to the canonical builtin registry in `perl-builtins-phf`,
    /// excluding nullary builtins and keywords that have dedicated parser handlers.
    fn is_builtin_function(name: &str) -> bool {
        perl_lexer::builtins::builtin_signatures_phf::is_builtin(name)
            && !Self::is_nullary_builtin(name)
            && !Self::is_keyword_handled_builtin(name)
    }

    /// Strip `CORE::` prefix from a name and return the bare builtin name if it
    /// is a known builtin function.  Returns `None` if the prefix is absent or
    /// if the bare name is not a builtin.
    ///
    /// This allows `CORE::select`, `CORE::print`, etc. to be recognised as
    /// function calls with bare arguments just like their unqualified forms.
    fn core_qualified_builtin_name(name: &str) -> Option<&str> {
        let bare = name.strip_prefix("CORE::")?;
        if Self::is_builtin_function(bare) || Self::is_nullary_builtin(bare) {
            Some(bare)
        } else {
            None
        }
    }

    /// Builtins that have dedicated keyword token kinds or parser handlers and
    /// must NOT be matched by the generic `is_builtin_function` guard.
    fn is_keyword_handled_builtin(name: &str) -> bool {
        matches!(
            name,
            "my" | "our" | "local" | "state" | "field" // variable declarations
                | "use" // pragma handling
                | "tie" | "untie" // dedicated AST nodes (matched before guard)
                | "eval" | "do" // dedicated keyword tokens with own parsers
                | "goto" // dedicated keyword token with parse_goto handler
        )
    }

    /// Check if an identifier is a nullary builtin that can stand alone without arguments.
    /// These builtins work on implicit variables like @_ when called without arguments.
    fn is_nullary_builtin(name: &str) -> bool {
        matches!(
            name,
            "shift"
                | "pop"
                | "caller"
                | "wantarray"
                | "__FILE__"
                | "__LINE__"
                | "__PACKAGE__"
                | "time"
                | "times"
                | "localtime"
                | "gmtime"
                | "getlogin"
                | "getppid"
                | "getpwent"
                | "getgrent"
                | "gethostent"
                | "getnetent"
                | "getprotoent"
                | "getservent"
                | "setpwent"
                | "setgrent"
                | "endpwent"
                | "endgrent"
                | "endhostent"
                | "endnetent"
                | "endprotoent"
                | "endservent"
                | "fork"
                | "wait"
                | "dump"
        )
    }

    /// Check if an identifier is an optional-argument builtin that can operate
    /// on `$_` when called without an explicit argument.  When followed by a
    /// binary operator in expression context (e.g., `defined && ...`,
    /// `length > 0`, `ord >= 32`), these builtins should be treated as having
    /// no argument rather than trying to parse the operator as their operand.
    pub(crate) fn is_optional_arg_builtin(name: &str) -> bool {
        matches!(
            name,
            "defined"
                | "ref"
                | "ord"
                | "chr"
                | "length"
                | "chop"
                | "chomp"
                | "lc"
                | "lcfirst"
                | "uc"
                | "ucfirst"
                | "pos"
                | "hex"
                | "oct"
                | "abs"
                | "int"
                | "sqrt"
                | "cos"
                | "sin"
                | "log"
                | "exp"
        )
    }

    /// Check if a module is a source filter (security risk)
    fn is_filter_module(module: &str) -> bool {
        matches!(
            module,
            "Filter"
                | "Filter::Util::Call"
                | "Filter::Simple"
                | "Filter::cpp"
                | "Filter::exec"
                | "Filter::sh"
                | "Filter::tee"
                | "Filter::decrypt"
        )
    }

    /// Check if a token kind is a keyword that has a dedicated parser handler
    /// in `parse_statement_inner`.  These are the tokens that would normally be
    /// dispatched to keyword-specific parsers but should instead be treated as
    /// autoquoted barewords when followed by `=>`.
    #[inline]
    fn is_keyword_token(kind: TokenKind) -> bool {
        matches!(
            kind,
            // Variable declarations
            TokenKind::My
                | TokenKind::Our
                | TokenKind::State
                | TokenKind::Local
                | TokenKind::Field
                // Control flow
                | TokenKind::If
                | TokenKind::Unless
                | TokenKind::While
                | TokenKind::Until
                | TokenKind::For
                | TokenKind::Foreach
                | TokenKind::Given
                | TokenKind::Default
                | TokenKind::Try
                // Loop control
                | TokenKind::Next
                | TokenKind::Last
                | TokenKind::Redo
                | TokenKind::Goto
                // Subroutines and OOP
                | TokenKind::Sub
                | TokenKind::Class
                | TokenKind::Method
                // Package management
                | TokenKind::Package
                | TokenKind::Use
                | TokenKind::No
                // Format
                | TokenKind::Format
                // Phase blocks
                | TokenKind::Begin
                | TokenKind::End
                | TokenKind::Check
                | TokenKind::Init
                | TokenKind::Unitcheck
                // Return
                | TokenKind::Return
                // Other keywords handled in parse_primary
                | TokenKind::Eval
                | TokenKind::Do
                | TokenKind::Continue
                | TokenKind::Catch
                | TokenKind::Finally
                | TokenKind::When
                | TokenKind::Undef
                // else/elsif — handled in parse_statement_inner for orphan recovery
                | TokenKind::Else
                | TokenKind::Elsif
        )
    }

    /// Check if a token kind is a binary operator that couldn't start an expression argument.
    fn is_binary_operator(kind: TokenKind) -> bool {
        matches!(
            kind,
            TokenKind::Or
                | TokenKind::And
                | TokenKind::DefinedOr
                | TokenKind::WordOr
                | TokenKind::WordAnd
                | TokenKind::WordXor
                | TokenKind::Equal
                | TokenKind::NotEqual
                | TokenKind::Less
                | TokenKind::Greater
                | TokenKind::LessEqual
                | TokenKind::GreaterEqual
                | TokenKind::Spaceship
                | TokenKind::StringCompare
                | TokenKind::Match
                | TokenKind::NotMatch
                | TokenKind::SmartMatch
                | TokenKind::Dot
                | TokenKind::Range
                | TokenKind::LeftShift
                | TokenKind::RightShift
                | TokenKind::BitwiseAnd
                | TokenKind::BitwiseOr
                | TokenKind::BitwiseXor
                | TokenKind::Question
                | TokenKind::Colon
                | TokenKind::Assign
                | TokenKind::PlusAssign
                | TokenKind::MinusAssign
                | TokenKind::StarAssign
                | TokenKind::SlashAssign
                | TokenKind::PercentAssign
                | TokenKind::DotAssign
                | TokenKind::AndAssign
                | TokenKind::OrAssign
                | TokenKind::XorAssign
                | TokenKind::PowerAssign
                | TokenKind::LeftShiftAssign
                | TokenKind::RightShiftAssign
                | TokenKind::LogicalAndAssign
                | TokenKind::LogicalOrAssign
                | TokenKind::DefinedOrAssign
        )
    }

    /// Peek at the next token's kind
    fn peek_kind(&mut self) -> Option<TokenKind> {
        self.tokens.peek().ok().map(|t| t.kind)
    }

    /// Peek at the next token without consuming it
    #[allow(dead_code)]
    fn peek_token(&mut self) -> ParseResult<&Token> {
        self.tokens.peek()
    }

    /// Check if the next token starts a variable.
    ///
    /// This checks both dedicated sigil tokens (`ScalarSigil`, `ArraySigil`,
    /// `HashSigil`) and combined `Identifier` tokens whose text begins with
    /// a variable sigil character (`$`, `@`, `%`).
    fn is_variable_start(&mut self) -> bool {
        if Self::is_variable_sigil(self.peek_kind()) {
            return true;
        }
        // The lexer sometimes emits variables as Identifier("$x") tokens
        if self.peek_kind() == Some(TokenKind::Identifier) {
            if let Ok(tok) = self.peek_token() {
                return tok.text.starts_with('$')
                    || tok.text.starts_with('@')
                    || tok.text.starts_with('%');
            }
        }
        false
    }

    /// Expect a specific token kind
    fn expect(&mut self, kind: TokenKind) -> ParseResult<Token> {
        let token = self.tokens.next()?;
        if token.kind != kind {
            return Err(ParseError::unexpected(
                kind.display_name(),
                token.kind.display_name(),
                token.start,
            ));
        }
        self.last_end_position = token.end;
        Ok(token)
    }

    /// Get current position
    fn current_position(&mut self) -> usize {
        self.tokens.peek().map(|t| t.start).unwrap_or_else(|_| {
            // Default position when no token available
            0
        })
    }

    /// Get previous position
    fn previous_position(&self) -> usize {
        self.last_end_position
    }

    /// Consume next token and track position
    fn consume_token(&mut self) -> ParseResult<Token> {
        let token = self.tokens.next()?;
        self.last_end_position = token.end;
        Ok(token)
    }

    /// Get closing delimiter for a given opening delimiter
    #[inline]
    fn closing_delim_for(open_txt: &str) -> Option<String> {
        // prefer textual comparison so we don't need to enumerate TokenKind variants
        match open_txt {
            "(" => Some(")".to_string()),
            "[" => Some("]".to_string()),
            "{" => Some("}".to_string()),
            "<" => Some(">".to_string()),
            // symmetric delimiters (| ! # ~ / etc.) close with themselves
            s if s.len() == 1 => Some(open_txt.to_string()),
            _ => None,
        }
    }

    /// Utility to build either a HashLiteral or ArrayLiteral based on whether
    /// fat arrow (=>) was seen and we have an even number of elements
    fn build_list_or_hash(
        elements: Vec<Node>,
        saw_fat_arrow: bool,
        start: usize,
        end: usize,
    ) -> Node {
        if saw_fat_arrow && elements.len().is_multiple_of(2) {
            // Convert to HashLiteral
            let mut pairs = Vec::with_capacity(elements.len() / 2);
            for chunk in elements.chunks(2) {
                pairs.push((chunk[0].clone(), chunk[1].clone()));
            }
            Node::new(NodeKind::HashLiteral { pairs }, SourceLocation { start, end })
        } else {
            Node::new(NodeKind::ArrayLiteral { elements }, SourceLocation { start, end })
        }
    }

    /// Auto-quote a bare identifier when it appears on the left side of `=>`.
    fn autoquote_fat_arrow_key(node: &mut Node) {
        if let NodeKind::Identifier { ref name } = node.kind {
            *node = Node::new(
                NodeKind::String { value: name.clone(), interpolated: false },
                node.location,
            );
        }
    }

    /// Continue parsing a comma / fat-arrow separated list when the first
    /// element was already parsed by the caller.
    ///
    /// This is the shared low-precedence collector used by expression parsing
    /// after a leading element is known. It stops at hard delimiters and at
    /// statement modifiers unless the modifier token itself is being used as an
    /// autoquoted hash key before `=>`.
    fn collect_comma_fat_arrow_continuation(&mut self, first: Node) -> ParseResult<Node> {
        if self.peek_kind() != Some(TokenKind::Comma)
            && self.peek_kind() != Some(TokenKind::FatArrow)
        {
            return Ok(first);
        }

        let start = first.location.start;
        let mut expressions = vec![first];
        let mut saw_fat_arrow = false;

        if self.peek_kind() == Some(TokenKind::FatArrow) {
            saw_fat_arrow = true;
            if let Some(last) = expressions.last_mut() {
                Self::autoquote_fat_arrow_key(last);
            }
            self.consume_token()?; // consume =>
            if self.peek_kind() == Some(TokenKind::FatArrow) {
                self.consume_token()?; // consume redundant chained =>
            }
            expressions.push(self.parse_assignment()?);
        }

        while self.peek_kind() == Some(TokenKind::Comma)
            || self.peek_kind() == Some(TokenKind::FatArrow)
        {
            let was_comma = self.peek_kind() == Some(TokenKind::Comma);
            if was_comma {
                self.consume_token()?; // consume comma
            }

            if self.peek_kind() == Some(TokenKind::FatArrow) {
                saw_fat_arrow = true;
                if !was_comma {
                    if let Some(last) = expressions.last_mut() {
                        Self::autoquote_fat_arrow_key(last);
                    }
                }
                self.consume_token()?; // consume =>
            }

            match self.peek_kind() {
                Some(TokenKind::Semicolon)
                | Some(TokenKind::RightParen)
                | Some(TokenKind::RightBrace)
                | Some(TokenKind::RightBracket) => break,
                Some(k) if Self::is_stmt_modifier_kind(k) && !self.is_keyword_before_fat_arrow() => {
                    break;
                }
                _ => {}
            }

            let mut elem = self.parse_assignment()?;

            if self.peek_kind() == Some(TokenKind::FatArrow) {
                saw_fat_arrow = true;
                Self::autoquote_fat_arrow_key(&mut elem);
                self.consume_token()?; // consume =>
                expressions.push(elem);

                match self.peek_kind() {
                    Some(TokenKind::Semicolon)
                    | Some(TokenKind::RightParen)
                    | Some(TokenKind::RightBrace)
                    | Some(TokenKind::RightBracket) => break,
                    Some(k) if Self::is_stmt_modifier_kind(k) => break,
                    _ => expressions.push(self.parse_assignment()?),
                }
            } else {
                expressions.push(elem);
            }
        }

        let end = expressions
            .last()
            .map(|expr| expr.location.end)
            .unwrap_or(start);
        Ok(Self::build_list_or_hash(expressions, saw_fat_arrow, start, end))
    }

    /// Record a parse error for later retrieval
    fn record_error(&mut self, error: ParseError) {
        self.errors.push(error);
    }

    /// Get all recorded errors
    pub fn get_errors(&self) -> &[ParseError] {
        &self.errors
    }

    /// Check whether the current token is a strong follower that cannot be the
    /// start of an expression — meaning the RHS of an infix operator is absent.
    ///
    /// Strong followers are tokens that always terminate an expression context:
    /// `;`, `}`, `)`, `]`, or EOF.  Encountering one after consuming an infix
    /// operator is a clear sign that the operand is missing.
    fn is_infix_rhs_absent(&mut self) -> bool {
        matches!(
            self.peek_kind(),
            Some(TokenKind::Semicolon)
                | Some(TokenKind::RightBrace)
                | Some(TokenKind::RightParen)
                | Some(TokenKind::RightBracket)
                | Some(TokenKind::Eof)
                | None
        )
    }

    /// If the next token cannot start an expression (i.e., is a strong
    /// follower), emit a `ParseError::Recovered { InfixRhs, MissingOperand }`
    /// and return a `NodeKind::MissingExpression` placeholder node.
    ///
    /// Returns `None` when the next token IS a valid expression start —
    /// the caller should parse normally.
    ///
    /// # Usage
    ///
    /// Call this method immediately after consuming an infix operator but
    /// before calling the RHS parse function:
    ///
    /// ```ignore
    /// let op_token = self.tokens.next()?;
    /// if let Some(missing) = self.recover_missing_infix_rhs(op_token.start) {
    ///     // wrap (left_expr op missing) and continue
    /// }
    /// let right = self.parse_something()?;
    /// ```
    fn recover_missing_infix_rhs(&mut self, op_pos: usize) -> Option<Node> {
        if !self.is_infix_rhs_absent() {
            return None;
        }
        self.errors.push(ParseError::Recovered {
            site: RecoverySite::InfixRhs,
            kind: RecoveryKind::MissingOperand,
            location: op_pos,
        });
        let pos = op_pos;
        Some(Node::new(NodeKind::MissingExpression, SourceLocation { start: pos, end: pos }))
    }

    /// Expect a closing delimiter, recovering gracefully if missing.
    ///
    /// At strong followers (`;`, `}`, `{`, statement keywords, or EOF) the
    /// missing closer is inferred rather than returned as an error:
    /// - Emits `ParseError::Recovered { site, kind: InsertedCloser, location }` so
    ///   callers can count recoveries and gate LSP features by confidence.
    /// - Returns `Ok(())` to allow parsing to continue.
    ///
    /// At any other token, returns `Err(ParseError::UnexpectedToken)` so the
    /// caller can propagate or handle it.
    fn expect_closing_delimiter(&mut self, kind: TokenKind) -> ParseResult<()> {
        if self.peek_kind() == Some(kind) {
            self.consume_token()?;
            return Ok(());
        }
        if self.is_delimiter_recovery_point() {
            let pos = self.current_position();
            let site = Self::recovery_site_for_closer(kind);
            self.errors.push(ParseError::Recovered {
                site,
                kind: RecoveryKind::InsertedCloser,
                location: pos,
            });
            return Ok(());
        }
        let pos = self.current_position();
        Err(ParseError::unexpected(
            kind.display_name(),
            self.peek_kind().map(|k| k.display_name()).unwrap_or("end of input"),
            pos,
        ))
    }

    /// Map a closing-delimiter token to its [`RecoverySite`] for recovery annotations.
    #[inline]
    fn recovery_site_for_closer(kind: TokenKind) -> RecoverySite {
        match kind {
            TokenKind::RightParen => RecoverySite::ArgList,
            TokenKind::RightBracket => RecoverySite::ArraySubscript,
            TokenKind::RightBrace => RecoverySite::HashSubscript,
            _ => RecoverySite::ArgList,
        }
    }

    /// Check if current token is a strong follower at which a missing closer can
    /// be inferred.  The set covers hard statement boundaries and EOF.
    ///
    /// `peek_kind()` returns `Some(TokenKind::Eof)` at end-of-input (the EOF
    /// token is sticky) so we match it explicitly alongside `None` (which covers
    /// the case where the lexer itself returns an error).
    fn is_delimiter_recovery_point(&mut self) -> bool {
        matches!(
            self.peek_kind(),
            Some(TokenKind::Semicolon)
                | Some(TokenKind::RightBrace)
                | Some(TokenKind::LeftBrace)
                | Some(TokenKind::Eof)
                | Some(TokenKind::My)
                | Some(TokenKind::Our)
                | Some(TokenKind::Local)
                | Some(TokenKind::State)
                | Some(TokenKind::Sub)
                | Some(TokenKind::Package)
                | Some(TokenKind::Use)
                | Some(TokenKind::If)
                | Some(TokenKind::Unless)
                | Some(TokenKind::Elsif)
                | Some(TokenKind::Else)
                | Some(TokenKind::While)
                | Some(TokenKind::Until)
                | Some(TokenKind::For)
                | Some(TokenKind::Foreach)
                | None
        )
    }

    /// Check if current token is a synchronization point for error recovery
    fn is_sync_point(&mut self) -> bool {
        match self.peek_kind() {
            Some(TokenKind::Semicolon) => true,
            Some(TokenKind::RightBrace) => true,
            Some(TokenKind::My)
            | Some(TokenKind::Our)
            | Some(TokenKind::Local)
            | Some(TokenKind::State) => true,
            Some(TokenKind::Sub) | Some(TokenKind::Package) | Some(TokenKind::Use) => true,
            Some(TokenKind::If) | Some(TokenKind::Unless) => true,
            Some(TokenKind::Elsif) | Some(TokenKind::Else) => true,
            Some(TokenKind::While) | Some(TokenKind::Until) => true,
            Some(TokenKind::For) | Some(TokenKind::Foreach) => true,
            None => true, // EOF is a sync point
            _ => false,
        }
    }

    /// Synchronize to next statement boundary for error recovery
    /// Returns true if synchronization was successful
    fn synchronize(&mut self) -> bool {
        let mut skipped = 0;

        while !self.tokens.is_eof() && skipped < 100 {
            // Check if we're at a sync point
            if self.is_sync_point() {
                // Consume tokens that would cause infinite loops if left unconsumed.
                // Semicolon: standard statement terminator, safe to skip.
                // RightBrace: orphan closing brace at top level must be consumed
                //             to prevent parse_program from looping forever.
                // Both are valid sync points but need to be consumed for progress.
                if matches!(
                    self.peek_kind(),
                    Some(TokenKind::Semicolon) | Some(TokenKind::RightBrace)
                ) {
                    let _ = self.consume_token();
                }
                return true;
            }

            // Skip the current token
            let _ = self.consume_token();
            skipped += 1;
        }

        false
    }

    /// Create an error node and record the error
    fn recover_from_error(
        &mut self,
        message: String,
        expected: String,
        found: String,
        location: usize,
    ) -> Node {
        // Record the error
        let error = ParseError::unexpected(expected, found, location);
        self.record_error(error);

        // Create error node
        let end = self.current_position();
        let found_token = self.tokens.peek().ok().cloned();

        Node::new(
            NodeKind::Error { message, expected: vec![], found: found_token, partial: None },
            SourceLocation { start: location, end },
        )
    }

    // /// Enhanced error recovery with context-aware suggestions
    // fn recover_from_error_enhanced(&mut self, error: ParseError) -> Node {
    //     // Check resource limits
    //     if let Err(e) = self.enhanced_recovery.should_continue() {
    //         self.record_error(e);
    //         return self.create_error_node("Resource limit exceeded".to_string(), vec![]);
    //     }

    //     // Analyze error context
    //     let current_token = self.tokens.peek().ok().cloned();
    //     let previous_tokens = self.get_previous_tokens_for_context(5);
    //     let context = helpers::analyze_error_context(
    //         current_token,
    //         previous_tokens,
    //         self.at_stmt_start,
    //         self.recursion_depth,
    //     );

    //     // Try adaptive recovery first
    //     if self.apply_adaptive_recovery(&error, &context) {
    //         // Recovery succeeded, create a node to continue parsing
    //         return self.create_enhanced_error_node(error, context);
    //     }

    //     // Fall back to standard error node creation
    //     self.create_enhanced_error_node(error, context)
    // }

    // /// Get previous tokens for context analysis
    // fn get_previous_tokens_for_context(&self, _count: usize) -> Vec<Token> {
    //     // This is a simplified implementation - in practice, you'd need to track token history
    //     Vec::new()
    // }

    /// AC4: Error node creation with precise token info
    #[allow(dead_code)]
    fn create_error_node(&mut self, message: String, expected: Vec<TokenKind>) -> Node {
        let start = self.current_position();
        let found = self.tokens.peek().ok().cloned();

        Node::new(
            NodeKind::Error { message, expected, found, partial: None },
            SourceLocation { start, end: start },
        )
    }

    /// Return true if `name` is a function that takes a block as its first argument
    /// followed by a list (like `map { } @list`). This covers Perl builtins and
    /// commonly-used functions from List::Util, List::MoreUtils, Scalar::Util, etc.
    ///
    /// These functions need special handling so that `{ ... }` is parsed as a code
    /// block rather than a hash constructor.
    #[inline]
    pub(crate) fn is_block_list_func(name: &str) -> bool {
        matches!(
            name,
            // Perl builtins
            "map" | "grep" | "sort"
            // List::Util (most common block-taking exports)
            | "first" | "any" | "all" | "none" | "reduce" | "pairfirst" | "pairgrep" | "pairmap"
            // List::MoreUtils / List::Util extensions
            | "first_index" | "last_index" | "indexes" | "any_u" | "all_u" | "none_u"
        )
    }

    /// Detect whether an unknown identifier (not a builtin) is being used as a
    /// bare function call (list operator) with arguments.
    ///
    /// In Perl, any function can be called without parentheses: `foo $arg`
    /// or `foo bar(...)`. This heuristic detects the common CPAN patterns:
    ///
    /// - `func $var` / `func @arr` / `func %hash` (sigiled argument)
    /// - `func other_func(...)` (identifier followed by `(`)
    /// - `func other_func $arg` (chained bare calls)
    ///
    /// Returns `true` when `name` looks like a user-defined function that
    /// accepts a block argument: `capture { ... }`, `where { ... }`, etc.
    ///
    /// In Perl, hash element access ALWAYS requires a sigil (`\$hash{key}`).
    /// A bare lowercase identifier followed by `{` is therefore a function
    /// call with a block argument, not a hash subscript.
    #[inline]
    pub(crate) fn looks_like_block_call_name(name: &str) -> bool {
        // Sigiled names are variables, not function calls
        if name.starts_with('$')
            || name.starts_with('@')
            || name.starts_with('%')
            || name.starts_with('&')
            || name.starts_with('*')
        {
            return false;
        }
        // Qualified names like Tk::catch are always function calls
        if name.contains("::") {
            return true;
        }
        // Only lowercase/underscore-leading identifiers
        name.starts_with(|c: char| c.is_ascii_lowercase() || c == '_')
    }

    /// We are conservative: the identifier must be lowercase (uppercase bare
    /// identifiers are more likely to be constants or package names) and
    /// must NOT be a string comparison operator (`eq`, `ne`, `lt`, `gt`, etc.)
    /// or a keyword token.
    fn looks_like_bare_call(&mut self, name: &str) -> bool {
        if self
            .peek_kind()
            .is_some_and(|kind| matches!(kind, TokenKind::My | TokenKind::Our | TokenKind::Local | TokenKind::State))
        {
            return !name.is_empty()
                && (name.contains("::")
                    || name.starts_with(|c: char| {
                        c.is_ascii_alphabetic() || c == '_'
                    }));
        }

        // Only lowercase identifiers can be bare function calls.
        // Uppercase identifiers like `FIRST_FD` are constants.
        if name.is_empty() || !name.starts_with(|c: char| c.is_ascii_lowercase() || c == '_') {
            return false;
        }

        // Exclude string comparison operators that are TokenKind::Identifier
        if matches!(name, "eq" | "ne" | "lt" | "le" | "gt" | "ge" | "cmp" | "x" | "ISA") {
            return false;
        }

        // Must not already be at a statement end or followed by a binary operator
        if self.is_at_statement_end() || self.peek_kind().is_some_and(Self::is_binary_operator) {
            return false;
        }

        // Peek at the next token to see if it could be an argument
        let next = match self.tokens.peek() {
            Ok(t) => t,
            Err(_) => return false,
        };

        match next.kind {
            // Sigiled variables: `func $x`, `func @arr`, `func %hash`
            TokenKind::Identifier
                if next.text.starts_with('$')
                    || next.text.starts_with('@')
                    || next.text.starts_with('%') =>
            {
                true
            }
            TokenKind::ScalarSigil | TokenKind::ArraySigil | TokenKind::HashSigil => true,

            // `func "string"` or `func 'string'` — bare function call with a string literal arg.
            // Handles: `croak "error message"`, `_estr "fmt"`, `die "msg"`, etc.
            // Imported functions that behave like builtins often take string args without parens.
            TokenKind::String => true,

            // `func other_func(args)` — identifier followed by `(`
            // `func bareword => value` — identifier followed by fat arrow (auto-quoted arg)
            // Also: `func Qualified::Name->method(...)` — qualified name as arg (Sub-pattern A).
            // The `!starts_with_uppercase` guard is relaxed for names that contain `::` so that
            // `func File::Spec->catfile(...)` is recognised as a bare call.
            TokenKind::Identifier => {
                let next_text = next.text.clone();
                // Special tokens like __PACKAGE__, __FILE__, __LINE__, __SUB__ are
                // nullary builtins that produce values. They are valid bare-call arguments.
                // e.g. `croak __PACKAGE__, ": error"` (Encode/Encoder.pm)
                if matches!(
                    next_text.as_ref(),
                    "__PACKAGE__" | "__FILE__" | "__LINE__" | "__SUB__"
                ) {
                    return true;
                }
                // Allow qualified names (e.g. `File::Spec`, `Scalar::Util`) as arguments.
                // They start with uppercase but the `::` disambiguates them from constants.
                if next_text.contains("::") {
                    // Qualified name — check that the following token is `->` or `(`.
                    if let Ok(third) = self.tokens.peek_second() {
                        return third.kind == TokenKind::Arrow
                            || third.kind == TokenKind::LeftParen;
                    }
                    return false;
                }
                if next_text.starts_with(|c: char| c.is_ascii_uppercase()) {
                    // Plain uppercase identifier (e.g. constant) — not an argument.
                    return false;
                }
                // Block-list functions (map/grep/sort/etc.) as argument: `uniq map { ... } @list`
                if Self::is_block_list_func(&next_text) {
                    if let Ok(third) = self.tokens.peek_second() {
                        return third.kind == TokenKind::LeftBrace
                            || third.kind == TokenKind::LeftParen;
                    }
                }
                // Builtin functions that take a single sigiled argument as argument:
                // `max values %hash`, `func keys %h`, `func reverse @list`
                // These are builtins whose first/only argument is a sigiled expression.
                if Self::is_builtin_function(&next_text) {
                    if let Ok(third) = self.tokens.peek_second() {
                        let third_text: &str = &third.text;
                        return third_text.starts_with('$')
                            || third_text.starts_with('@')
                            || third_text.starts_with('%')
                            || third.kind == TokenKind::ScalarSigil
                            || third.kind == TokenKind::ArraySigil
                            || third.kind == TokenKind::HashSigil;
                    }
                }
                // Check if the next-next token is `(` — that signals a function call
                // or `=>` (fat arrow after bareword) — that signals an auto-quoted arg
                self.tokens.peek_second().ok().is_some_and(|t| {
                    t.kind == TokenKind::LeftParen || t.kind == TokenKind::FatArrow
                })
            }

            _ => false,
        }
    }

    /// Check if a token kind can appear as a subroutine name after `sub`.
    ///
    /// In Perl, any bareword can be a subroutine name, including reserved
    /// keywords and word-operators:
    /// ```perl
    /// sub return { ... }   # autodie/exception.pm
    /// sub eval { ... }     # perl5db.pl
    /// sub next { ... }     # Net/NNTP.pm
    /// sub cmp { ... }      # IO/Compress/Base/Common.pm
    /// ```
    ///
    /// Returns `false` for tokens that signal anonymous sub syntax: `{`, `(`,
    /// `:` (attribute), `;` (forward decl), sigils, operators, etc.
    #[inline]
    pub(crate) fn can_be_sub_name(kind: TokenKind) -> bool {
        matches!(
            kind,
            // Regular identifiers
            TokenKind::Identifier
            // All keyword tokens (valid sub names in Perl)
            | TokenKind::If
            | TokenKind::Unless
            | TokenKind::While
            | TokenKind::Until
            | TokenKind::For
            | TokenKind::Foreach
            | TokenKind::My
            | TokenKind::Our
            | TokenKind::State
            | TokenKind::Local
            | TokenKind::Field
            | TokenKind::Sub
            | TokenKind::Return
            | TokenKind::Next
            | TokenKind::Last
            | TokenKind::Redo
            | TokenKind::Goto
            | TokenKind::Eval
            | TokenKind::Do
            | TokenKind::Use
            | TokenKind::No
            | TokenKind::Package
            | TokenKind::Class
            | TokenKind::Method
            | TokenKind::Try
            | TokenKind::Catch
            | TokenKind::Finally
            | TokenKind::Given
            | TokenKind::When
            | TokenKind::Default
            | TokenKind::Continue
            | TokenKind::Format
            | TokenKind::Begin
            | TokenKind::End
            | TokenKind::Check
            | TokenKind::Init
            | TokenKind::Unitcheck
            | TokenKind::Undef
            // Word-operators (also valid bareword sub names)
            | TokenKind::WordAnd    // sub and { ... }
            | TokenKind::WordOr     // sub or { ... }
            | TokenKind::WordNot    // sub not { ... }
            | TokenKind::WordXor    // sub xor { ... }
            | TokenKind::StringCompare // sub cmp { ... }
        )
    }
}
