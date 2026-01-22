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

    #[inline]
    fn is_postfix_op(kind: Option<TokenKind>) -> bool {
        matches!(kind, Some(TokenKind::Increment) | Some(TokenKind::Decrement))
    }

    #[inline]
    fn is_variable_sigil(kind: Option<TokenKind>) -> bool {
        matches!(
            kind,
            Some(TokenKind::ScalarSigil) | Some(TokenKind::ArraySigil) | Some(TokenKind::HashSigil)
        )
    }

    /// Check recursion depth with optimized hot path
    #[inline(always)]
    fn check_recursion(&mut self) -> ParseResult<()> {
        self.recursion_depth += 1;
        // Fast path: avoid expensive comparisons in the common case
        if self.recursion_depth > MAX_RECURSION_DEPTH {
            return Err(ParseError::RecursionLimit);
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

    /// Check if an identifier is a builtin function that can take arguments without parens
    fn is_builtin_function(name: &str) -> bool {
        matches!(
            name,
            "print"
                | "say"
                | "die"
                | "warn"
                | "return"
                | "defined"
                | "undef"
                | "ref"
                | "chomp"
                | "chop"
                | "split"
                | "join"
                | "push"
                | "pop"
                | "shift"
                | "unshift"
                | "sort"
                | "map"
                | "grep"
                | "keys"
                | "values"
                | "each"
                | "delete"
                | "exists"
                | "open"
                | "close"
                | "read"
                | "write"
                | "printf"
                | "sprintf"
                | "exit"
                | "next"
                | "last"
                | "redo"
                | "goto"
                | "dump"
                | "caller"
                | "import"
                | "unimport"
                | "require"
                | "bless"
                | "tie"
                | "tied"
                | "untie"
                | "scalar"
                | "wantarray"
                // Math functions
                | "abs"
                | "atan2"
                | "cos"
                | "sin"
                | "exp"
                | "log"
                | "sqrt"
                | "rand"
                | "srand"
                | "int"
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

    /// Check if the next token starts a variable
    fn is_variable_start(&mut self) -> bool {
        Self::is_variable_sigil(self.peek_kind())
    }

    /// Expect a specific token kind
    fn expect(&mut self, kind: TokenKind) -> ParseResult<Token> {
        let token = self.tokens.next()?;
        if token.kind != kind {
            return Err(ParseError::unexpected(
                format!("{:?}", kind),
                format!("{:?}", token.kind),
                token.start,
            ));
        }
        self.last_end_position = token.end;
        Ok(token)
    }

    /// Get current position
    fn current_position(&mut self) -> usize {
        self.tokens.peek().map(|t| t.start).unwrap_or(0)
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

    /// Record a parse error for later retrieval
    fn record_error(&mut self, error: ParseError) {
        self.errors.push(error);
    }

    /// Get all recorded errors
    pub fn get_errors(&self) -> &[ParseError] {
        &self.errors
    }

    /// Check if current token is a synchronization point for error recovery
    fn is_sync_point(&mut self) -> bool {
        match self.peek_kind() {
            Some(TokenKind::Semicolon) => true,
            Some(TokenKind::RightBrace) => true,
            Some(TokenKind::My) | Some(TokenKind::Our) | Some(TokenKind::Local) | Some(TokenKind::State) => true,
            Some(TokenKind::Sub) | Some(TokenKind::Package) | Some(TokenKind::Use) => true,
            Some(TokenKind::If) | Some(TokenKind::Unless) => true,
            Some(TokenKind::While) | Some(TokenKind::Until) => true,
            Some(TokenKind::For) | Some(TokenKind::Foreach) => true,
            None => true,  // EOF is a sync point
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
                // If we're at a semicolon, consume it
                if matches!(self.peek_kind(), Some(TokenKind::Semicolon)) {
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
    fn recover_from_error(&mut self, message: String, expected: String, found: String, location: usize) -> Node {
        // Record the error
        let error = ParseError::unexpected(expected, found.clone(), location);
        self.record_error(error);

        // Create error node
        let end = self.current_position();
        Node::new(
            NodeKind::Error { message },
            SourceLocation { start: location, end }
        )
    }

}
