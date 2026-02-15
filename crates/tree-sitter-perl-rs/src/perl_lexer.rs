use crate::heredoc_recovery::{HeredocRecovery, RecoveryConfig};
use std::sync::Arc;

/// Perl lexer mode to disambiguate slash tokens
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LexerMode {
    /// Expecting a term (value) - slash starts a regex
    ExpectTerm,
    /// Expecting an operator - slash is division
    ExpectOperator,
}

/// Parts of an interpolated string
#[derive(Debug, Clone, PartialEq)]
pub enum StringPart {
    Literal(Arc<str>),
    Variable(Arc<str>),   // $var, @array, %hash
    Expression(Arc<str>), // ${expr}, @{expr}
    MethodCall(Arc<str>), // ->method()
    ArraySlice(Arc<str>), // [1..3]
}

/// Token types for disambiguation
#[derive(Debug, Clone, PartialEq)]
pub enum TokenType {
    // Slash-derived tokens
    Division,
    RegexMatch,      // m// or //
    Substitution,    // s///
    Transliteration, // tr/// or y///
    QuoteRegex,      // qr//

    // String and quote tokens
    StringLiteral, // "string" or 'string'
    QuoteSingle,   // q//
    QuoteDouble,   // qq//
    QuoteWords,    // qw//
    QuoteCommand,  // qx// or `backticks`

    // String interpolation tokens
    InterpolatedString(Vec<StringPart>), // String with interpolated parts

    // Heredoc tokens
    HeredocStart, // <<EOF or <<'EOF'
    HeredocBody(Arc<str>),

    // Version strings
    Version(Arc<str>), // v5.32.0

    // POD documentation
    Pod,

    // Other tokens that affect mode
    Identifier(Arc<str>),
    Number(Arc<str>),
    Operator(Arc<str>),
    Keyword(Arc<str>),
    LeftParen,
    RightParen,
    LeftBracket,
    RightBracket,
    LeftBrace,
    RightBrace,
    Semicolon,
    Comma,
    Colon,
    Arrow,    // =>
    FatComma, // ,
    Whitespace,
    Newline,
    Comment(Arc<str>),
    EOF,

    // Error token for unknown/invalid input
    Error(Arc<str>),
}

/// Token with position information
#[derive(Debug, Clone)]
pub struct Token {
    pub token_type: TokenType,
    pub text: Arc<str>,
    pub start: usize,
    pub end: usize,
}

/// Mode-aware Perl lexer
pub struct PerlLexer<'a> {
    input: &'a str,
    position: usize,
    mode: LexerMode,
    /// Stack for nested delimiters in s{}{} constructs
    _delimiter_stack: Vec<char>,
    /// Heredoc recovery system
    heredoc_recovery: HeredocRecovery,
    /// Collected tokens for static analysis
    tokens: Vec<Token>,
    /// Track if we're inside prototype parens after 'sub'
    in_prototype: bool,
    /// Paren depth to track when we exit prototype
    paren_depth: usize,
    /// Track if last token was 'sub'
    last_was_sub: bool,
    /// Track if we've seen sub NAME and expecting prototype
    expect_prototype: bool,
}

impl<'a> PerlLexer<'a> {
    pub fn new(input: &'a str) -> Self {
        Self {
            input,
            position: 0,
            mode: LexerMode::ExpectTerm,
            _delimiter_stack: Vec::new(),
            heredoc_recovery: HeredocRecovery::new(RecoveryConfig::default()),
            tokens: Vec::new(),
            in_prototype: false,
            paren_depth: 0,
            last_was_sub: false,
            expect_prototype: false,
        }
    }

    /// Safely slice the input string ensuring UTF-8 boundaries
    fn safe_slice(&self, start: usize, end: usize) -> &str {
        // Find the nearest valid UTF-8 boundaries
        let safe_start = if self.input.is_char_boundary(start) {
            start
        } else {
            // Find the previous valid boundary
            let mut s = start;
            while s > 0 && !self.input.is_char_boundary(s) {
                s -= 1;
            }
            s
        };

        let safe_end = if self.input.is_char_boundary(end) {
            end
        } else {
            // Find the next valid boundary
            let mut e = end;
            while e < self.input.len() && !self.input.is_char_boundary(e) {
                e += 1;
            }
            e.min(self.input.len())
        };

        &self.input[safe_start..safe_end]
    }

    fn is_unicode_identifier_start(&self, ch: char) -> bool {
        ch.is_alphabetic() || ch == '_'
    }

    fn is_unicode_identifier_continue(&self, ch: char) -> bool {
        ch.is_alphanumeric() || ch == '_'
    }

    /// Advance position by one character (handling UTF-8)
    fn _advance_char(&mut self) {
        if self.position < self.input.len() {
            let current_char = self.input[self.position..].chars().next();
            if let Some(ch) = current_char {
                self.position += ch.len_utf8();
            }
        }
    }

    /// Get current character without advancing
    fn _current_char(&self) -> Option<char> {
        self.input[self.position..].chars().next()
    }

    /// Skip whitespace and return the count
    fn skip_whitespace(&mut self) -> usize {
        let start = self.position;
        while self.position < self.input.len() {
            match self.input.as_bytes()[self.position] {
                b' ' | b'\t' | b'\r' => self.position += 1,
                _ => break,
            }
        }
        self.position - start
    }

    /// Skip to end of line
    fn skip_line(&mut self) {
        while self.position < self.input.len() && self.input.as_bytes()[self.position] != b'\n' {
            self.position += 1;
        }
        if self.position < self.input.len() {
            self.position += 1; // Skip the newline
        }
    }

    /// Peek at the next non-whitespace character
    fn _peek_next_non_ws(&self) -> Option<char> {
        let mut pos = self.position;
        while pos < self.input.len() {
            let ch = self.input.as_bytes()[pos];
            match ch {
                b' ' | b'\t' | b'\r' | b'\n' => pos += 1,
                _ => return Some(ch as char),
            }
        }
        None
    }

    /// Check if the next characters match a pattern
    fn peek_str(&self, s: &str) -> bool {
        // Ensure we're at a valid UTF-8 boundary
        if !self.input.is_char_boundary(self.position) {
            return false;
        }
        self.input[self.position..].starts_with(s)
    }

    /// Check if character can be a regex delimiter
    fn is_regex_delimiter(ch: char) -> bool {
        matches!(
            ch,
            '/' | '!'
                | '#'
                | '%'
                | '&'
                | '*'
                | ','
                | '.'
                | ':'
                | ';'
                | '='
                | '?'
                | '@'
                | '^'
                | '|'
                | '~'
                | '\''
                | '"'
                | '`'
                | '{'
                | '['
                | '('
                | '<'
        )
    }

    /// Scan an interpolated double-quoted string
    fn scan_interpolated_string(&mut self) -> Option<Token> {
        let start = self.position;
        self.position += 1; // Skip opening quote

        let mut parts = Vec::new();
        let mut current_literal = String::new();

        while self.position < self.input.len() {
            match self.input.as_bytes()[self.position] {
                b'"' => {
                    // End of string
                    if !current_literal.is_empty() {
                        parts.push(StringPart::Literal(Arc::from(current_literal)));
                    }
                    self.position += 1;
                    break;
                }
                b'\\' if self.position + 1 < self.input.len() => {
                    // Escape sequence
                    let next = self.input.as_bytes()[self.position + 1];
                    match next {
                        b'n' => current_literal.push('\n'),
                        b'r' => current_literal.push('\r'),
                        b't' => current_literal.push('\t'),
                        b'\\' => current_literal.push('\\'),
                        b'"' => current_literal.push('"'),
                        b'$' => current_literal.push('$'),
                        b'@' => current_literal.push('@'),
                        _ => {
                            current_literal.push('\\');
                            current_literal.push(next as char);
                        }
                    }
                    self.position += 2;
                }
                b'$' if self.position + 1 < self.input.len() => {
                    // Possible variable interpolation
                    if !current_literal.is_empty() {
                        parts.push(StringPart::Literal(Arc::from(current_literal.clone())));
                        current_literal.clear();
                    }

                    let var_start = self.position;
                    self.position += 1;

                    // Check for ${...} syntax
                    if self.position < self.input.len()
                        && self.input.as_bytes()[self.position] == b'{'
                    {
                        self.position += 1;
                        let expr_start = self.position;
                        let mut brace_depth = 1;

                        while self.position < self.input.len() && brace_depth > 0 {
                            match self.input.as_bytes()[self.position] {
                                b'{' => brace_depth += 1,
                                b'}' => brace_depth -= 1,
                                _ => {}
                            }
                            self.position += 1;
                        }

                        let expr = self.safe_slice(expr_start, self.position - 1);
                        parts.push(StringPart::Expression(Arc::from(format!("${{{}}}", expr))));
                    } else {
                        // Simple variable name

                        // Check for special variables
                        if self.position < self.input.len() {
                            let ch = self.input.as_bytes()[self.position];
                            if matches!(
                                ch,
                                b'!' | b'"'
                                    | b'#'
                                    | b'$'
                                    | b'%'
                                    | b'&'
                                    | b'\''
                                    | b'('
                                    | b')'
                                    | b'*'
                                    | b'+'
                                    | b','
                                    | b'-'
                                    | b'.'
                                    | b'/'
                                    | b'0'
                                    ..=b'9'
                                        | b':'
                                        | b';'
                                        | b'<'
                                        | b'='
                                        | b'>'
                                        | b'?'
                                        | b'@'
                                        | b'['
                                        | b'\\'
                                        | b']'
                                        | b'^'
                                        | b'_'
                                        | b'`'
                                        | b'{'
                                        | b'|'
                                        | b'}'
                                        | b'~'
                            ) {
                                self.position += 1;
                                let var = self.safe_slice(var_start, self.position);
                                parts.push(StringPart::Variable(Arc::from(var)));
                                continue;
                            }
                        }

                        // Regular identifier
                        while self.position < self.input.len() {
                            let ch = self.input.as_bytes()[self.position];
                            if ch.is_ascii_alphanumeric() || ch == b'_' {
                                self.position += 1;
                            } else {
                                break;
                            }
                        }

                        let var = self.safe_slice(var_start, self.position);
                        parts.push(StringPart::Variable(Arc::from(var)));

                        // Check for method calls
                        if self.position + 2 < self.input.len()
                            && self.input.as_bytes()[self.position] == b'-'
                            && self.input.as_bytes()[self.position + 1] == b'>'
                        {
                            self.position += 2;
                            let method_start = self.position;

                            // Scan method name
                            while self.position < self.input.len() {
                                let ch = self.input.as_bytes()[self.position];
                                if ch.is_ascii_alphanumeric() || ch == b'_' {
                                    self.position += 1;
                                } else {
                                    break;
                                }
                            }

                            let method = self.safe_slice(method_start, self.position);
                            let method_str = method.to_string(); // Clone to avoid borrow issues
                            parts.push(StringPart::MethodCall(Arc::from(format!(
                                "->{}",
                                method_str
                            ))));

                            // Check for method arguments
                            if self.position < self.input.len()
                                && self.input.as_bytes()[self.position] == b'('
                            {
                                // For now, just include the parens in the method call
                                let paren_start = self.position;
                                self.position += 1;
                                let mut paren_depth = 1;

                                while self.position < self.input.len() && paren_depth > 0 {
                                    match self.input.as_bytes()[self.position] {
                                        b'(' => paren_depth += 1,
                                        b')' => paren_depth -= 1,
                                        b'"' => break, // Don't go past string end
                                        _ => {}
                                    }
                                    self.position += 1;
                                }

                                if paren_depth == 0 {
                                    let args = self.safe_slice(paren_start, self.position);
                                    parts.pop(); // Remove the method call we just added
                                    parts.push(StringPart::MethodCall(Arc::from(format!(
                                        "->{}{}",
                                        method_str, args
                                    ))));
                                }
                            }
                        }

                        // Check for array/hash element access
                        if self.position < self.input.len()
                            && self.input.as_bytes()[self.position] == b'['
                        {
                            let bracket_start = self.position;
                            self.position += 1;
                            let mut bracket_depth = 1;

                            while self.position < self.input.len() && bracket_depth > 0 {
                                match self.input.as_bytes()[self.position] {
                                    b'[' => bracket_depth += 1,
                                    b']' => bracket_depth -= 1,
                                    b'"' => break, // Don't go past string end
                                    _ => {}
                                }
                                self.position += 1;
                            }

                            if bracket_depth == 0 {
                                let slice = self.safe_slice(bracket_start, self.position);
                                parts.push(StringPart::ArraySlice(Arc::from(slice)));
                            }
                        }
                    }
                }
                b'@' if self.position + 1 < self.input.len() => {
                    // Array interpolation
                    if !current_literal.is_empty() {
                        parts.push(StringPart::Literal(Arc::from(current_literal.clone())));
                        current_literal.clear();
                    }

                    let var_start = self.position;
                    self.position += 1;

                    // Check for @{...} syntax
                    if self.position < self.input.len()
                        && self.input.as_bytes()[self.position] == b'{'
                    {
                        self.position += 1;
                        let expr_start = self.position;
                        let mut brace_depth = 1;

                        while self.position < self.input.len() && brace_depth > 0 {
                            match self.input.as_bytes()[self.position] {
                                b'{' => brace_depth += 1,
                                b'}' => brace_depth -= 1,
                                _ => {}
                            }
                            self.position += 1;
                        }

                        let expr = self.safe_slice(expr_start, self.position - 1);
                        parts.push(StringPart::Expression(Arc::from(format!("@{{{}}}", expr))));
                    } else {
                        // Simple array name
                        while self.position < self.input.len() {
                            let ch = self.input.as_bytes()[self.position];
                            if ch.is_ascii_alphanumeric() || ch == b'_' {
                                self.position += 1;
                            } else {
                                break;
                            }
                        }

                        let var = self.safe_slice(var_start, self.position);
                        parts.push(StringPart::Variable(Arc::from(var)));
                    }
                }
                _ => {
                    current_literal.push(self.input.as_bytes()[self.position] as char);
                    self.position += 1;
                }
            }
        }

        // If we have multiple parts or any interpolation, it's an interpolated string
        let token = if parts.is_empty()
            || (parts.len() == 1 && matches!(parts[0], StringPart::Literal(_)))
        {
            // Just a literal string
            Token {
                token_type: TokenType::StringLiteral,
                text: Arc::from(self.safe_slice(start, self.position)),
                start,
                end: self.position,
            }
        } else {
            // Interpolated string
            Token {
                token_type: TokenType::InterpolatedString(parts),
                text: Arc::from(self.safe_slice(start, self.position)),
                start,
                end: self.position,
            }
        };

        self.update_mode(&token.token_type);
        Some(token)
    }

    /// Update mode based on the token type
    fn update_mode(&mut self, token: &TokenType) {
        use TokenType::*;

        // Track sub NAME ( pattern
        match token {
            Keyword(kw) if kw.as_ref() == "sub" => {
                self.last_was_sub = true;
                self.expect_prototype = false;
            }
            Identifier(_) if self.last_was_sub => {
                self.expect_prototype = true;
                self.last_was_sub = false;
            }
            LeftParen if self.expect_prototype => {
                self.in_prototype = true;
                self.paren_depth = 1;
                self.expect_prototype = false;
            }
            _ => {
                self.last_was_sub = false;
                self.expect_prototype = false;
            }
        }

        self.mode = match token {
            // These produce a value, so next slash is division
            Identifier(_)
            | Number(_)
            | RightParen
            | RightBracket
            | RightBrace
            | RegexMatch
            | Substitution
            | Transliteration
            | QuoteRegex
            | StringLiteral
            | InterpolatedString(_)
            | QuoteSingle
            | QuoteDouble
            | QuoteWords
            | QuoteCommand => LexerMode::ExpectOperator,

            // These expect a value next, so slash starts regex
            Operator(_) | LeftParen | LeftBracket | LeftBrace | Semicolon | Comma | Arrow
            | FatComma | Division => LexerMode::ExpectTerm,

            // Keywords depend on which keyword
            Keyword(kw) => match kw.as_ref() {
                // These expect a value
                "if" | "unless" | "while" | "until" | "for" | "foreach" | "given" | "return"
                | "my" | "our" | "local" | "state" | "print" | "say" | "printf" | "split"
                | "grep" | "map" | "sort" => LexerMode::ExpectTerm,
                // These produce a value
                "sub" => LexerMode::ExpectOperator,
                _ => self.mode, // Keep current mode
            },

            // Keep current mode for others
            _ => self.mode,
        }
    }

    /// Try to scan a regex-like construct (m//, s///, tr///, etc.)
    fn scan_regex_like(&mut self) -> Option<Token> {
        let start = self.position;

        // Check for explicit operators with delimiters
        if self.position + 1 < self.input.len() {
            let ch = self.input.as_bytes()[self.position] as char;
            let next = self.input.as_bytes()[self.position + 1] as char;

            // Check for s///, tr///, y///, m//, qr// patterns
            match ch {
                's' if Self::is_regex_delimiter(next) => return self.scan_substitution(),
                't' if self.position + 2 < self.input.len()
                    && self.input.as_bytes()[self.position + 1] == b'r'
                    && Self::is_regex_delimiter(
                        self.input.as_bytes()[self.position + 2] as char,
                    ) =>
                {
                    return self.scan_transliteration();
                }
                'y' if Self::is_regex_delimiter(next) => return self.scan_transliteration(),
                'm' if Self::is_regex_delimiter(next) => return self.scan_match_regex(),
                'q' if self.position + 2 < self.input.len()
                    && self.input.as_bytes()[self.position + 1] == b'r'
                    && Self::is_regex_delimiter(
                        self.input.as_bytes()[self.position + 2] as char,
                    ) =>
                {
                    return self.scan_quote_regex();
                }
                _ => {}
            }
        }

        // Bare slash - could be regex or division based on mode
        if self.position < self.input.len() && self.input.as_bytes()[self.position] == b'/' {
            match self.mode {
                LexerMode::ExpectTerm => {
                    // This is a regex match
                    self.scan_match_regex()
                }
                LexerMode::ExpectOperator => {
                    // This is division
                    self.position += 1;
                    Some(Token {
                        token_type: TokenType::Division,
                        text: Arc::from("/"),
                        start,
                        end: self.position,
                    })
                }
            }
        } else {
            None
        }
    }

    /// Scan a match regex (m// or //)
    fn scan_match_regex(&mut self) -> Option<Token> {
        let start = self.position;

        // Check if we have 'm' followed by a delimiter without advancing position yet
        let has_m_prefix = self.peek_str("m");
        let delimiter_pos = if has_m_prefix { self.position + 1 } else { self.position };

        // Get delimiter
        if delimiter_pos >= self.input.len() {
            return None;
        }
        let delimiter = self.input.as_bytes()[delimiter_pos] as char;
        if !Self::is_regex_delimiter(delimiter) {
            return None;
        }

        // Now that we've confirmed this is a valid regex pattern, advance position
        if has_m_prefix {
            self.position += 1;
        }
        self.position += 1;

        // Find closing delimiter
        let closing = match delimiter {
            '{' => '}',
            '[' => ']',
            '(' => ')',
            '<' => '>',
            _ => delimiter,
        };

        // Scan pattern
        while self.position < self.input.len() {
            let ch = self.input.as_bytes()[self.position];
            if ch as char == closing {
                self.position += 1;
                break;
            }
            if ch == b'\\' && self.position + 1 < self.input.len() {
                self.position += 2; // Skip escaped character
            } else {
                self.position += 1;
            }
        }

        // Scan flags (optional)
        while self.position < self.input.len() {
            match self.input.as_bytes()[self.position] {
                b'i' | b'm' | b's' | b'x' | b'o' | b'g' | b'c' | b'e' | b'r' | b'a' | b'd'
                | b'l' | b'u' | b'n' | b'p' => {
                    self.position += 1;
                }
                _ => break,
            }
        }

        Some(Token {
            token_type: TokenType::RegexMatch,
            text: Arc::from(self.safe_slice(start, self.position)),
            start,
            end: self.position,
        })
    }

    /// Scan a substitution operator (s///)
    ///
    /// Handles various delimiter forms:
    /// - s/pattern/replacement/flags (standard form)
    /// - s{pattern}{replacement}flags (braced form)
    /// - s(pattern)(replacement)flags (parenthesized form)
    /// - s[pattern][replacement]flags (bracketed form)
    /// - s<pattern><replacement>flags (angle bracket form)
    /// - s#pattern#replacement#flags (custom delimiter form)
    ///
    /// For paired delimiters (like braces), properly handles nesting depth
    /// to correctly parse patterns like s{a{b}c}{d{e}f}.
    fn scan_substitution(&mut self) -> Option<Token> {
        let start = self.position;

        // Skip 's'
        self.position += 1;

        // Get delimiter
        if self.position >= self.input.len() {
            return None;
        }
        let delimiter = self.input.as_bytes()[self.position] as char;
        if !Self::is_regex_delimiter(delimiter) {
            return None;
        }
        self.position += 1;

        let closing = match delimiter {
            '{' => '}',
            '[' => ']',
            '(' => ')',
            '<' => '>',
            _ => delimiter,
        };

        // Scan pattern
        let mut depth = 1;
        while self.position < self.input.len() && depth > 0 {
            let ch = self.input.as_bytes()[self.position];
            if ch as char == closing {
                depth -= 1;
                if depth == 0 {
                    self.position += 1;
                    break;
                }
            } else if ch as char == delimiter && delimiter != closing {
                depth += 1;
            }
            if ch == b'\\' && self.position + 1 < self.input.len() {
                self.position += 2;
            } else {
                self.position += 1;
            }
        }

        // For bracketed delimiters, skip whitespace and find next opening delimiter
        if delimiter != closing {
            self.skip_whitespace();
            // Expect the second opening delimiter (e.g., the second '{' in s{pattern}{replacement})
            if self.position < self.input.len()
                && self.input.as_bytes()[self.position] as char == delimiter
            {
                self.position += 1;
            } else {
                // If we don't find the expected second delimiter, this is an error
                return Some(Token {
                    token_type: TokenType::Error(Arc::from(
                        "Expected opening delimiter for replacement in substitution",
                    )),
                    text: Arc::from(self.safe_slice(start, self.position)),
                    start,
                    end: self.position,
                });
            }
        }

        // Scan replacement
        depth = 1;
        while self.position < self.input.len() && depth > 0 {
            let ch = self.input.as_bytes()[self.position];
            if ch as char == closing {
                depth -= 1;
                if depth == 0 {
                    self.position += 1;
                    break;
                }
            } else if ch as char == delimiter && delimiter != closing {
                depth += 1;
            }
            if ch == b'\\' && self.position + 1 < self.input.len() {
                self.position += 2;
            } else {
                self.position += 1;
            }
        }

        // Scan flags
        while self.position < self.input.len() {
            match self.input.as_bytes()[self.position] {
                b'i' | b'm' | b's' | b'x' | b'o' | b'g' | b'c' | b'e' | b'r' => {
                    self.position += 1;
                }
                _ => break,
            }
        }

        Some(Token {
            token_type: TokenType::Substitution,
            text: Arc::from(self.safe_slice(start, self.position)),
            start,
            end: self.position,
        })
    }

    /// Scan a transliteration (tr/// or y///)
    fn scan_transliteration(&mut self) -> Option<Token> {
        let start = self.position;

        // Skip 'tr' or 'y'
        if self.peek_str("tr") {
            self.position += 2;
        } else if self.peek_str("y") {
            self.position += 1;
        } else {
            return None;
        }

        // Similar to substitution but simpler (no regex escapes)
        let delimiter = self.input.as_bytes()[self.position] as char;
        if !Self::is_regex_delimiter(delimiter) {
            return None;
        }
        self.position += 1;

        let closing = match delimiter {
            '{' => '}',
            '[' => ']',
            '(' => ')',
            '<' => '>',
            _ => delimiter,
        };

        // Scan search list
        while self.position < self.input.len() {
            let ch = self.input.as_bytes()[self.position];
            if ch as char == closing {
                self.position += 1;
                break;
            }
            if ch == b'\\' && self.position + 1 < self.input.len() {
                self.position += 2;
            } else {
                self.position += 1;
            }
        }

        // For bracketed delimiters, skip whitespace
        if delimiter != closing {
            self.skip_whitespace();
            if self.position < self.input.len()
                && self.input.as_bytes()[self.position] as char == delimiter
            {
                self.position += 1;
            }
        }

        // Scan replacement list
        while self.position < self.input.len() {
            let ch = self.input.as_bytes()[self.position];
            if ch as char == closing {
                self.position += 1;
                break;
            }
            if ch == b'\\' && self.position + 1 < self.input.len() {
                self.position += 2;
            } else {
                self.position += 1;
            }
        }

        // Scan flags
        while self.position < self.input.len() {
            match self.input.as_bytes()[self.position] {
                b'c' | b'd' | b's' | b'r' => {
                    self.position += 1;
                }
                _ => break,
            }
        }

        Some(Token {
            token_type: TokenType::Transliteration,
            text: Arc::from(self.safe_slice(start, self.position)),
            start,
            end: self.position,
        })
    }

    /// Scan quote operators (q//, qq//, qw//, qx//)
    fn scan_quote_operator(&mut self) -> Option<Token> {
        let start = self.position;

        // Determine quote type
        let (quote_type, prefix_len) = if self.peek_str("qq") {
            (TokenType::QuoteDouble, 2)
        } else if self.peek_str("qw") {
            (TokenType::QuoteWords, 2)
        } else if self.peek_str("qx") {
            (TokenType::QuoteCommand, 2)
        } else if self.peek_str("qr") {
            // Handle qr// separately
            return self.scan_quote_regex();
        } else if self.peek_str("q") {
            (TokenType::QuoteSingle, 1)
        } else {
            return None;
        };

        // Skip prefix
        self.position += prefix_len;

        // Get delimiter
        if self.position >= self.input.len() {
            return None;
        }
        let delimiter = self.input.as_bytes()[self.position] as char;
        if !Self::is_regex_delimiter(delimiter) {
            return None;
        }
        self.position += 1;

        let closing = match delimiter {
            '{' => '}',
            '[' => ']',
            '(' => ')',
            '<' => '>',
            _ => delimiter,
        };

        // Scan content
        while self.position < self.input.len() {
            let ch = self.input.as_bytes()[self.position];
            if ch as char == closing {
                self.position += 1;
                break;
            }
            if ch == b'\\' && self.position + 1 < self.input.len() {
                self.position += 2;
            } else {
                self.position += 1;
            }
        }

        let token = Token {
            token_type: quote_type,
            text: Arc::from(self.safe_slice(start, self.position)),
            start,
            end: self.position,
        };
        self.update_mode(&token.token_type);
        Some(token)
    }

    /// Scan a qr// regex
    fn scan_quote_regex(&mut self) -> Option<Token> {
        let start = self.position;

        // Skip 'qr'
        self.position += 2;

        // Get delimiter
        if self.position >= self.input.len() {
            return None;
        }
        let delimiter = self.input.as_bytes()[self.position] as char;
        if !Self::is_regex_delimiter(delimiter) {
            return None;
        }
        self.position += 1;

        let closing = match delimiter {
            '{' => '}',
            '[' => ']',
            '(' => ')',
            '<' => '>',
            _ => delimiter,
        };

        // Scan pattern
        while self.position < self.input.len() {
            let ch = self.input.as_bytes()[self.position];
            if ch as char == closing {
                self.position += 1;
                break;
            }
            if ch == b'\\' && self.position + 1 < self.input.len() {
                self.position += 2;
            } else {
                self.position += 1;
            }
        }

        // Scan flags
        while self.position < self.input.len() {
            match self.input.as_bytes()[self.position] {
                b'i' | b'm' | b's' | b'x' | b'o' => {
                    self.position += 1;
                }
                _ => break,
            }
        }

        Some(Token {
            token_type: TokenType::QuoteRegex,
            text: Arc::from(self.safe_slice(start, self.position)),
            start,
            end: self.position,
        })
    }

    /// Scan a heredoc start token (<<EOF or <<'EOF' or <<$var)
    fn scan_heredoc_start(&mut self) -> Option<Token> {
        let start = self.position;

        // Skip <<
        self.position += 2;

        // Check for indented heredoc (<<~)
        let _indented =
            if self.position < self.input.len() && self.input.as_bytes()[self.position] == b'~' {
                self.position += 1;
                true
            } else {
                false
            };

        // Skip optional whitespace
        while self.position < self.input.len() && self.input.as_bytes()[self.position] == b' ' {
            self.position += 1;
        }

        // Check for dynamic delimiter (starts with $ or contains special chars)
        if self.position < self.input.len() {
            let ch = self.input.as_bytes()[self.position];
            if ch == b'$' || ch == b'@' || ch == b'%' || ch == b'{' {
                // This might be a dynamic heredoc delimiter
                // First, parse the complete expression
                if let Some((_expression, expr_end)) =
                    self.heredoc_recovery.parse_delimiter_expression(self.input, self.position)
                {
                    // Now try to recover the delimiter
                    let recovery_result = self.heredoc_recovery.recover_dynamic_heredoc(
                        self.input,
                        start,
                        &self.tokens,
                    );

                    // Update position to end of expression
                    self.position = expr_end;

                    if recovery_result.error_node {
                        // Generate error token
                        return Some(self.heredoc_recovery.generate_error_token(
                            self.input,
                            start,
                            &recovery_result,
                        ));
                    } else if let Some(delimiter) = recovery_result.delimiter {
                        // Successfully recovered - generate a special token
                        let token = Token {
                            token_type: TokenType::HeredocStart,
                            text: Arc::from(format!("<<{}", delimiter)),
                            start,
                            end: self.position,
                        };
                        self.update_mode(&token.token_type);
                        return Some(token);
                    }
                }
            }
        }

        // Check for quoted delimiter
        let quoted = if self.position < self.input.len() {
            match self.input.as_bytes()[self.position] {
                b'\'' | b'"' | b'`' => {
                    self.position += 1;
                    true
                }
                _ => false,
            }
        } else {
            false
        };

        // Scan delimiter
        while self.position < self.input.len() {
            let ch = self.input.as_bytes()[self.position];
            if quoted && (ch == b'\'' || ch == b'"' || ch == b'`') {
                self.position += 1;
                break;
            } else if !quoted && (ch == b' ' || ch == b'\t' || ch == b'\n' || ch == b';') {
                break;
            }
            self.position += 1;
        }

        let token = Token {
            token_type: TokenType::HeredocStart,
            text: Arc::from(self.safe_slice(start, self.position)),
            start,
            end: self.position,
        };
        self.update_mode(&token.token_type);
        Some(token)
    }

    /// Scan POD documentation
    fn scan_pod(&mut self) -> Option<Token> {
        let start = self.position;

        // Scan until =cut
        while self.position < self.input.len() {
            if self.peek_str("\n=cut") {
                self.position += 5; // Skip "\n=cut"
                // Skip to end of line
                self.skip_line();
                break;
            }
            self.position += 1;
        }

        Some(Token {
            token_type: TokenType::Pod,
            text: Arc::from(self.safe_slice(start, self.position)),
            start,
            end: self.position,
        })
    }

    /// Scan a version string (v5.32.0)
    fn scan_version(&mut self) -> Option<Token> {
        let start = self.position;

        // Skip 'v'
        self.position += 1;

        // Scan version parts
        while self.position < self.input.len() {
            let ch = self.input.as_bytes()[self.position];
            if ch.is_ascii_digit() || ch == b'.' || ch == b'_' {
                self.position += 1;
            } else {
                break;
            }
        }

        let text = self.safe_slice(start, self.position);
        let token = Token {
            token_type: TokenType::Version(Arc::from(text)),
            text: Arc::from(text),
            start,
            end: self.position,
        };
        self.update_mode(&token.token_type);
        Some(token)
    }

    /// Store token for recovery analysis
    fn store_token(&mut self, token: &Token) {
        // Keep a reasonable window of tokens for analysis
        if self.tokens.len() > 100 {
            self.tokens.remove(0);
        }
        self.tokens.push(token.clone());
    }

    /// Get the next token
    pub fn next_token(&mut self) -> Option<Token> {
        self.skip_whitespace();

        if self.position >= self.input.len() {
            return None;
        }

        // Store and return the token
        let token = self.next_token_internal()?;
        self.store_token(&token);
        Some(token)
    }

    /// Internal token generation (without storing)
    fn next_token_internal(&mut self) -> Option<Token> {
        let start = self.position;
        let ch = self.input.as_bytes()[self.position];

        // If this is not an ASCII character (high bit set), handle it as Unicode
        if ch > 127 {
            if let Some(unicode_ch) = self.input[self.position..].chars().next() {
                if self.is_unicode_identifier_start(unicode_ch) {
                    // Parse Unicode identifier
                    let char_len = unicode_ch.len_utf8();
                    self.position += char_len;

                    // Continue scanning identifier
                    while self.position < self.input.len() {
                        if let Some(ch) = self.input[self.position..].chars().next() {
                            if self.is_unicode_identifier_continue(ch) {
                                self.position += ch.len_utf8();
                            } else if ch == ':' && self.position + ch.len_utf8() < self.input.len()
                            {
                                // Check for :: in package names
                                let next_pos = self.position + ch.len_utf8();
                                if next_pos < self.input.len()
                                    && self.input.as_bytes()[next_pos] == b':'
                                {
                                    self.position += 2;
                                } else {
                                    break;
                                }
                            } else {
                                break;
                            }
                        } else {
                            break;
                        }
                    }

                    let text = self.safe_slice(start, self.position);
                    let token = Token {
                        token_type: TokenType::Identifier(Arc::from(text)),
                        text: Arc::from(text),
                        start,
                        end: self.position,
                    };
                    self.update_mode(&token.token_type);
                    return Some(token);
                } else {
                    // If not a valid identifier start, generate error token
                    let char_len = unicode_ch.len_utf8();
                    self.position += char_len;
                    let token = Token {
                        token_type: TokenType::Error(Arc::from(format!(
                            "Unknown character: '{}'",
                            unicode_ch
                        ))),
                        text: Arc::from(self.safe_slice(start, self.position)),
                        start,
                        end: self.position,
                    };
                    return Some(token);
                }
            } else {
                self.position += 1;
                return self.next_token();
            }
        }

        // Check for regex-like constructs first
        if (ch == b'/'
            || (self.mode == LexerMode::ExpectTerm
                && (self.peek_str("s/")
                    || self.peek_str("s{")
                    || self.peek_str("m/")
                    || self.peek_str("m{")
                    || self.peek_str("tr/")
                    || self.peek_str("y/")
                    || self.peek_str("qr/")
                    || self.peek_str("qr{"))))
            && let Some(token) = self.scan_regex_like()
        {
            self.update_mode(&token.token_type);
            return Some(token);
        }

        // Check for quote operators
        if self.peek_str("q/")
            || self.peek_str("q{")
            || self.peek_str("q(")
            || self.peek_str("q[")
            || self.peek_str("q!")
            || self.peek_str("q#")
            || self.peek_str("q|")
            || self.peek_str("q<")
        {
            return self.scan_quote_operator();
        }
        if self.peek_str("qq/")
            || self.peek_str("qq{")
            || self.peek_str("qq(")
            || self.peek_str("qq[")
            || self.peek_str("qq!")
            || self.peek_str("qq#")
            || self.peek_str("qq|")
            || self.peek_str("qq<")
        {
            return self.scan_quote_operator();
        }
        if self.peek_str("qw/")
            || self.peek_str("qw{")
            || self.peek_str("qw(")
            || self.peek_str("qw[")
            || self.peek_str("qw!")
            || self.peek_str("qw#")
            || self.peek_str("qw|")
            || self.peek_str("qw<")
        {
            return self.scan_quote_operator();
        }
        if self.peek_str("qx/")
            || self.peek_str("qx{")
            || self.peek_str("qx(")
            || self.peek_str("qx[")
            || self.peek_str("qx!")
            || self.peek_str("qx#")
            || self.peek_str("qx|")
            || self.peek_str("qx<")
        {
            return self.scan_quote_operator();
        }

        // Check for heredocs
        if ch == b'<'
            && self.position + 1 < self.input.len()
            && self.input.as_bytes()[self.position + 1] == b'<'
        {
            return self.scan_heredoc_start();
        }

        // Check for POD - must be at start of line and followed by a POD directive
        if ch == b'='
            && (self.position == 0
                || (self.position > 0 && self.input.as_bytes()[self.position - 1] == b'\n'))
        {
            // Check if it's a POD directive
            if self.position + 2 < self.input.len() {
                let next_chars = &self.input.as_bytes()[self.position..];
                if next_chars.starts_with(b"=pod")
                    || next_chars.starts_with(b"=head")
                    || next_chars.starts_with(b"=over")
                    || next_chars.starts_with(b"=item")
                    || next_chars.starts_with(b"=back")
                    || next_chars.starts_with(b"=cut")
                    || next_chars.starts_with(b"=for")
                    || next_chars.starts_with(b"=begin")
                    || next_chars.starts_with(b"=end")
                    || next_chars.starts_with(b"=encoding")
                {
                    return self.scan_pod();
                }
            }
        }

        // Check for version strings
        if ch == b'v'
            && self.position + 1 < self.input.len()
            && self.input.as_bytes()[self.position + 1].is_ascii_digit()
        {
            return self.scan_version();
        }

        // Handle other tokens
        match ch {
            b'#' => {
                // Comment
                self.skip_line();
                Some(Token {
                    token_type: TokenType::Comment(Arc::from(
                        self.safe_slice(start, self.position),
                    )),
                    text: Arc::from(self.safe_slice(start, self.position)),
                    start,
                    end: self.position,
                })
            }
            b'\n' => {
                self.position += 1;
                Some(Token {
                    token_type: TokenType::Newline,
                    text: Arc::from("\n"),
                    start,
                    end: self.position,
                })
            }
            b'(' => {
                self.position += 1;
                if self.in_prototype {
                    self.paren_depth += 1;
                }
                let token = Token {
                    token_type: TokenType::LeftParen,
                    text: Arc::from("("),
                    start,
                    end: self.position,
                };
                self.update_mode(&token.token_type);
                Some(token)
            }
            b')' => {
                self.position += 1;
                if self.in_prototype && self.paren_depth > 0 {
                    self.paren_depth -= 1;
                    if self.paren_depth == 0 {
                        self.in_prototype = false;
                    }
                }
                let token = Token {
                    token_type: TokenType::RightParen,
                    text: Arc::from(")"),
                    start,
                    end: self.position,
                };
                self.update_mode(&token.token_type);
                Some(token)
            }
            b'[' => {
                self.position += 1;
                let token = Token {
                    token_type: TokenType::LeftBracket,
                    text: Arc::from("["),
                    start,
                    end: self.position,
                };
                self.update_mode(&token.token_type);
                Some(token)
            }
            b']' => {
                self.position += 1;
                let token = Token {
                    token_type: TokenType::RightBracket,
                    text: Arc::from("]"),
                    start,
                    end: self.position,
                };
                self.update_mode(&token.token_type);
                Some(token)
            }
            b'{' => {
                self.position += 1;
                let token = Token {
                    token_type: TokenType::LeftBrace,
                    text: Arc::from("{"),
                    start,
                    end: self.position,
                };
                self.update_mode(&token.token_type);
                Some(token)
            }
            b'}' => {
                self.position += 1;
                let token = Token {
                    token_type: TokenType::RightBrace,
                    text: Arc::from("}"),
                    start,
                    end: self.position,
                };
                self.update_mode(&token.token_type);
                Some(token)
            }
            b';' => {
                self.position += 1;
                let token = Token {
                    token_type: TokenType::Semicolon,
                    text: Arc::from(";"),
                    start,
                    end: self.position,
                };
                self.update_mode(&token.token_type);
                Some(token)
            }
            b',' => {
                self.position += 1;
                let token = Token {
                    token_type: TokenType::Comma,
                    text: Arc::from(","),
                    start,
                    end: self.position,
                };
                self.update_mode(&token.token_type);
                Some(token)
            }
            b'=' => {
                if self.position + 1 < self.input.len() {
                    match self.input.as_bytes()[self.position + 1] {
                        b'>' => {
                            self.position += 2;
                            let token = Token {
                                token_type: TokenType::Arrow,
                                text: Arc::from("=>"),
                                start,
                                end: self.position,
                            };
                            self.update_mode(&token.token_type);
                            Some(token)
                        }
                        b'~' => {
                            self.position += 2;
                            let token = Token {
                                token_type: TokenType::Operator(Arc::from("=~")),
                                text: Arc::from("=~"),
                                start,
                                end: self.position,
                            };
                            self.update_mode(&token.token_type);
                            Some(token)
                        }
                        b'=' => {
                            self.position += 2;
                            let token = Token {
                                token_type: TokenType::Operator(Arc::from("==")),
                                text: Arc::from("=="),
                                start,
                                end: self.position,
                            };
                            self.update_mode(&token.token_type);
                            Some(token)
                        }
                        _ => {
                            self.position += 1;
                            let token = Token {
                                token_type: TokenType::Operator(Arc::from("=")),
                                text: Arc::from("="),
                                start,
                                end: self.position,
                            };
                            self.update_mode(&token.token_type);
                            Some(token)
                        }
                    }
                } else {
                    self.position += 1;
                    let token = Token {
                        token_type: TokenType::Operator(Arc::from("=")),
                        text: Arc::from("="),
                        start,
                        end: self.position,
                    };
                    self.update_mode(&token.token_type);
                    Some(token)
                }
            }
            b'0'..=b'9' => {
                // Number
                if ch == b'0' && self.position + 1 < self.input.len() {
                    match self.input.as_bytes()[self.position + 1] {
                        b'x' | b'X' => {
                            // Hex number
                            self.position += 2;
                            while self.position < self.input.len() {
                                match self.input.as_bytes()[self.position] {
                                    b'0'..=b'9' | b'a'..=b'f' | b'A'..=b'F' | b'_' => {
                                        self.position += 1
                                    }
                                    _ => break,
                                }
                            }
                        }
                        b'b' | b'B' => {
                            // Binary number
                            self.position += 2;
                            while self.position < self.input.len() {
                                match self.input.as_bytes()[self.position] {
                                    b'0' | b'1' | b'_' => self.position += 1,
                                    _ => break,
                                }
                            }
                        }
                        b'o' | b'O' => {
                            // Octal number (0o755 style)
                            self.position += 2;
                            while self.position < self.input.len() {
                                match self.input.as_bytes()[self.position] {
                                    b'0'..=b'7' | b'_' => self.position += 1,
                                    _ => break,
                                }
                            }
                        }
                        b'0'..=b'7' => {
                            // Octal number
                            self.position += 1;
                            while self.position < self.input.len() {
                                match self.input.as_bytes()[self.position] {
                                    b'0'..=b'7' | b'_' => self.position += 1,
                                    _ => break,
                                }
                            }
                        }
                        _ => {
                            // Regular number starting with 0
                            self.position += 1;
                            while self.position < self.input.len() {
                                match self.input.as_bytes()[self.position] {
                                    b'0'..=b'9' | b'.' | b'e' | b'E' | b'_' => self.position += 1,
                                    _ => break,
                                }
                            }
                        }
                    }
                } else {
                    // Regular decimal number
                    while self.position < self.input.len() {
                        match self.input.as_bytes()[self.position] {
                            b'0'..=b'9' | b'.' | b'e' | b'E' | b'_' => self.position += 1,
                            _ => break,
                        }
                    }
                }
                let token = Token {
                    token_type: TokenType::Number(Arc::from(self.safe_slice(start, self.position))),
                    text: Arc::from(self.safe_slice(start, self.position)),
                    start,
                    end: self.position,
                };
                self.update_mode(&token.token_type);
                Some(token)
            }
            b'$' | b'@' | b'%' | b'*' => {
                // In prototype context, these are prototype specifiers, not variables
                if self.in_prototype && (ch == b'$' || ch == b'@' || ch == b'%' || ch == b'*') {
                    self.position += 1;
                    let token = Token {
                        token_type: TokenType::Operator(Arc::from(
                            self.safe_slice(start, self.position),
                        )),
                        text: Arc::from(self.safe_slice(start, self.position)),
                        start,
                        end: self.position,
                    };
                    self.update_mode(&token.token_type);
                    return Some(token);
                }

                // Variable sigil
                let sigil = ch;
                if self.position + 1 < self.input.len() {
                    let next = self.input.as_bytes()[self.position + 1];

                    // Handle special variables
                    if sigil == b'$' {
                        match next {
                            // Special single-char variables
                            b'_' | b'.' | b'@' | b'!' | b'?' | b'&' | b'`' | b'\'' | b'+'
                            | b'$' | b'<' | b'>' | b'(' | b')' | b'[' | b']' | b'|' | b'~'
                            | b'%' => {
                                self.position += 2;
                                let text = self.safe_slice(start, self.position);
                                let token = Token {
                                    token_type: TokenType::Identifier(Arc::from(text)),
                                    text: Arc::from(text),
                                    start,
                                    end: self.position,
                                };
                                self.update_mode(&token.token_type);
                                return Some(token);
                            }
                            // Numeric special variables like $1, $2, $10, etc.
                            b'0'..=b'9' => {
                                self.position += 2;
                                // Continue scanning digits for multi-digit variables like $10
                                while self.position < self.input.len()
                                    && self.input.as_bytes()[self.position].is_ascii_digit()
                                {
                                    self.position += 1;
                                }
                                let text = self.safe_slice(start, self.position);
                                let token = Token {
                                    token_type: TokenType::Identifier(Arc::from(text)),
                                    text: Arc::from(text),
                                    start,
                                    end: self.position,
                                };
                                self.update_mode(&token.token_type);
                                return Some(token);
                            }
                            b'^' => {
                                // Handle ${^VARNAME} special variables
                                self.position += 2;
                                // Check if it's a single char like $^A or extended like ${^TAINT}
                                if self.position < self.input.len() {
                                    let ch = self.input.as_bytes()[self.position];
                                    if ch.is_ascii_uppercase() {
                                        self.position += 1;
                                    }
                                }
                                let text = self.safe_slice(start, self.position);
                                let token = Token {
                                    token_type: TokenType::Identifier(Arc::from(text)),
                                    text: Arc::from(text),
                                    start,
                                    end: self.position,
                                };
                                self.update_mode(&token.token_type);
                                return Some(token);
                            }
                            b'{' => {
                                // Handle ${identifier} or ${^SPECIAL}
                                self.position += 2;
                                if self.position < self.input.len()
                                    && self.input.as_bytes()[self.position] == b'^'
                                {
                                    self.position += 1;
                                }
                                while self.position < self.input.len() {
                                    match self.input.as_bytes()[self.position] {
                                        b'}' => {
                                            self.position += 1;
                                            break;
                                        }
                                        b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'_' => {
                                            self.position += 1
                                        }
                                        _ => break,
                                    }
                                }
                                let text = self.safe_slice(start, self.position);
                                let token = Token {
                                    token_type: TokenType::Identifier(Arc::from(text)),
                                    text: Arc::from(text),
                                    start,
                                    end: self.position,
                                };
                                self.update_mode(&token.token_type);
                                return Some(token);
                            }
                            _ => {}
                        }
                    }

                    // Regular variables
                    if matches!(next, b'a'..=b'z' | b'A'..=b'Z' | b'_' | b'0'..=b'9') {
                        self.position += 1;
                        // Scan the variable name
                        while self.position < self.input.len() {
                            if let Some(ch) = self.input[self.position..].chars().next() {
                                if ch.is_ascii() {
                                    match self.input.as_bytes()[self.position] {
                                        b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'_' => {
                                            self.position += 1
                                        }
                                        _ => break,
                                    }
                                } else if self.is_unicode_identifier_continue(ch) {
                                    self.position += ch.len_utf8();
                                } else {
                                    break;
                                }
                            } else {
                                break;
                            }
                        }
                        let text = self.safe_slice(start, self.position);
                        let token = Token {
                            token_type: TokenType::Identifier(Arc::from(text)),
                            text: Arc::from(text),
                            start,
                            end: self.position,
                        };
                        self.update_mode(&token.token_type);
                        return Some(token);
                    } else if let Some(ch) = self.input[(self.position + 1)..].chars().next() {
                        // Check for Unicode identifier start after sigil
                        if self.is_unicode_identifier_start(ch) {
                            self.position += 1; // Skip sigil
                            self.position += ch.len_utf8();

                            // Continue scanning identifier
                            while self.position < self.input.len() {
                                if let Some(ch) = self.input[self.position..].chars().next() {
                                    if self.is_unicode_identifier_continue(ch) {
                                        self.position += ch.len_utf8();
                                    } else {
                                        break;
                                    }
                                } else {
                                    break;
                                }
                            }
                            let text = self.safe_slice(start, self.position);
                            let token = Token {
                                token_type: TokenType::Identifier(Arc::from(text)),
                                text: Arc::from(text),
                                start,
                                end: self.position,
                            };
                            self.update_mode(&token.token_type);
                            return Some(token);
                        }
                    }
                }
                // Otherwise treat as operator (for % modulo)
                self.position += 1;
                let token = Token {
                    token_type: TokenType::Operator(Arc::from(
                        self.safe_slice(start, self.position),
                    )),
                    text: Arc::from(self.safe_slice(start, self.position)),
                    start,
                    end: self.position,
                };
                self.update_mode(&token.token_type);
                Some(token)
            }
            b'a'..=b'z' | b'A'..=b'Z' | b'_' => {
                // Check for regex operators first
                if self.position < self.input.len() {
                    let ch = self.input.as_bytes()[self.position] as char;
                    if (matches!(ch, 's' | 'm' | 'y') && self.position + 1 < self.input.len()) {
                        let next = self.input.as_bytes()[self.position + 1] as char;
                        if Self::is_regex_delimiter(next)
                            && let Some(token) = self.scan_regex_like()
                        {
                            self.update_mode(&token.token_type);
                            return Some(token);
                        }
                    } else if (ch == 't' || ch == 'q')
                        && self.position + 2 < self.input.len()
                        && self.input.as_bytes()[self.position + 1] == b'r'
                    {
                        let next = self.input.as_bytes()[self.position + 2] as char;
                        if Self::is_regex_delimiter(next)
                            && let Some(token) = self.scan_regex_like()
                        {
                            self.update_mode(&token.token_type);
                            return Some(token);
                        }
                    }
                }

                // Regular identifier or keyword (including package names with ::)
                while self.position < self.input.len() {
                    match self.input.as_bytes()[self.position] {
                        b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'_' => self.position += 1,
                        b':' if self.position + 1 < self.input.len()
                            && self.input.as_bytes()[self.position + 1] == b':' =>
                        {
                            // Include :: in identifier for package names
                            self.position += 2;
                        }
                        _ => break,
                    }
                }
                let text = self.safe_slice(start, self.position);
                let token = match text {
                    "if" | "unless" | "while" | "until" | "for" | "foreach" | "given"
                    | "return" | "my" | "our" | "local" | "state" | "sub" | "do" | "eval"
                    | "package" | "use" | "require" | "no" | "BEGIN" | "END" | "CHECK" | "INIT"
                    | "UNITCHECK" | "print" | "say" | "printf" | "split" | "grep" | "map"
                    | "sort" | "die" | "warn" | "open" | "close" | "read" | "write" | "tie"
                    | "format" => Token {
                        token_type: TokenType::Keyword(Arc::from(text)),
                        text: Arc::from(text),
                        start,
                        end: self.position,
                    },
                    _ => Token {
                        token_type: TokenType::Identifier(Arc::from(text)),
                        text: Arc::from(text),
                        start,
                        end: self.position,
                    },
                };
                self.update_mode(&token.token_type);
                Some(token)
            }
            b'<' => {
                // Could be <FH> readline, <*.txt> glob, or < operator
                if self.position + 1 < self.input.len() {
                    let mut end_pos = self.position + 1;
                    // Look for closing >
                    while end_pos < self.input.len() && self.input.as_bytes()[end_pos] != b'>' {
                        end_pos += 1;
                    }
                    if end_pos < self.input.len() && self.input.as_bytes()[end_pos] == b'>' {
                        // Found closing >, check if it's readline/glob
                        let content = &self.input[(self.position + 1)..end_pos];
                        if content.is_empty() || // <> diamond
                           content.chars().all(|c| c.is_ascii_uppercase() || c == '_') || // <FH>
                           content.contains('*') || content.contains('?') || content.contains('[')
                        {
                            // glob
                            self.position = end_pos + 1;
                            let token = Token {
                                token_type: TokenType::Operator(Arc::from(
                                    self.safe_slice(start, self.position),
                                )),
                                text: Arc::from(self.safe_slice(start, self.position)),
                                start,
                                end: self.position,
                            };
                            self.update_mode(&token.token_type);
                            return Some(token);
                        }
                    }
                }
                // Fall through to regular < operator handling
                self.position += 1;
                // Check for compound operators
                if self.position < self.input.len() {
                    let next = self.input.as_bytes()[self.position];
                    match next {
                        b'<' => self.position += 1, // <<
                        b'=' => {
                            self.position += 1; // <=
                            // Check for <=> (spaceship operator)
                            if self.position < self.input.len()
                                && self.input.as_bytes()[self.position] == b'>'
                            {
                                self.position += 1;
                            }
                        }
                        b'>' => self.position += 1, // <>
                        _ => {}
                    }
                }
                let token = Token {
                    token_type: TokenType::Operator(Arc::from(
                        self.safe_slice(start, self.position),
                    )),
                    text: Arc::from(self.safe_slice(start, self.position)),
                    start,
                    end: self.position,
                };
                self.update_mode(&token.token_type);
                Some(token)
            }
            b'+' | b'-' | b'&' | b'|' | b'^' | b'~' | b'!' | b'>' | b'.' | b'\\' => {
                // Check for number starting with decimal point
                if ch == b'.'
                    && self.position + 1 < self.input.len()
                    && self.input.as_bytes()[self.position + 1].is_ascii_digit()
                {
                    self.position += 1;
                    while self.position < self.input.len() {
                        match self.input.as_bytes()[self.position] {
                            b'0'..=b'9' | b'e' | b'E' | b'_' => self.position += 1,
                            _ => break,
                        }
                    }
                    let token = Token {
                        token_type: TokenType::Number(Arc::from(
                            self.safe_slice(start, self.position),
                        )),
                        text: Arc::from(self.safe_slice(start, self.position)),
                        start,
                        end: self.position,
                    };
                    self.update_mode(&token.token_type);
                    return Some(token);
                }

                // Operators
                self.position += 1;
                // Check for compound operators
                if self.position < self.input.len() {
                    let next = self.input.as_bytes()[self.position];
                    match (ch, next) {
                        (b'+', b'+')
                        | (b'-', b'-')
                        | (b'*', b'*')
                        | (b'<', b'<')
                        | (b'>', b'>')
                        | (b'&', b'&')
                        | (b'|', b'|')
                        | (b'!', b'~') => {
                            self.position += 1;
                        }
                        (b'-', b'>') => {
                            // -> method call operator
                            self.position += 1;
                            let token = Token {
                                token_type: TokenType::Arrow,
                                text: Arc::from("->"),
                                start,
                                end: self.position,
                            };
                            self.update_mode(&token.token_type);
                            return Some(token);
                        }
                        (b'~', b'~') => {
                            // ~~ smart match operator
                            self.position += 1;
                        }
                        (b'.', b'.') => {
                            // Check for ... ellipsis operator
                            if self.position + 1 < self.input.len()
                                && self.input.as_bytes()[self.position + 1] == b'.'
                            {
                                self.position += 2; // skip the next two dots
                            } else {
                                // Just ..
                                self.position += 1;
                            }
                        }
                        // File test operators
                        (
                            b'-',
                            b'r' | b'w' | b'x' | b'o' | b'R' | b'W' | b'X' | b'O' | b'e' | b'z'
                            | b's' | b'f' | b'd' | b'l' | b'p' | b'S' | b'b' | b'c' | b't' | b'u'
                            | b'g' | b'k' | b'T' | b'B' | b'M' | b'A' | b'C',
                        ) => {
                            self.position += 1;
                        }
                        (b'<', b'=') | (b'>', b'=') | (b'!', b'=') | (b'=', b'=') => {
                            self.position += 1;
                            // Check for <=> (spaceship operator)
                            if ch == b'<'
                                && self.position < self.input.len()
                                && self.input.as_bytes()[self.position] == b'>'
                            {
                                self.position += 1;
                            }
                        }
                        (b'<', b'>') => {
                            // <> diamond operator
                            self.position += 1;
                        }
                        _ => {}
                    }
                }
                let token = Token {
                    token_type: TokenType::Operator(Arc::from(
                        self.safe_slice(start, self.position),
                    )),
                    text: Arc::from(self.safe_slice(start, self.position)),
                    start,
                    end: self.position,
                };
                self.update_mode(&token.token_type);
                Some(token)
            }
            b'"' => {
                // Double-quoted string with interpolation support
                self.scan_interpolated_string()
            }
            b'\'' => {
                // Single-quoted string
                self.position += 1;
                while self.position < self.input.len() {
                    match self.input.as_bytes()[self.position] {
                        b'\'' => {
                            self.position += 1;
                            break;
                        }
                        b'\\' if self.position + 1 < self.input.len() => {
                            self.position += 2;
                        }
                        _ => self.position += 1,
                    }
                }
                let token = Token {
                    token_type: TokenType::StringLiteral,
                    text: Arc::from(self.safe_slice(start, self.position)),
                    start,
                    end: self.position,
                };
                self.update_mode(&token.token_type);
                Some(token)
            }
            b'`' => {
                // Backtick command execution
                self.position += 1;
                while self.position < self.input.len() {
                    match self.input.as_bytes()[self.position] {
                        b'`' => {
                            self.position += 1;
                            break;
                        }
                        b'\\' if self.position + 1 < self.input.len() => {
                            self.position += 2;
                        }
                        _ => self.position += 1,
                    }
                }
                let token = Token {
                    token_type: TokenType::QuoteCommand,
                    text: Arc::from(self.safe_slice(start, self.position)),
                    start,
                    end: self.position,
                };
                self.update_mode(&token.token_type);
                Some(token)
            }
            b':' => {
                self.position += 1;
                let token = Token {
                    token_type: TokenType::Colon,
                    text: Arc::from(":"),
                    start,
                    end: self.position,
                };
                self.update_mode(&token.token_type);
                Some(token)
            }
            _ => {
                // Unknown character, generate error token
                self.position += 1;
                let token = Token {
                    token_type: TokenType::Error(Arc::from(format!(
                        "Unknown character: '{}'",
                        self.safe_slice(start, self.position)
                    ))),
                    text: Arc::from(self.safe_slice(start, self.position)),
                    start,
                    end: self.position,
                };
                Some(token)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use perl_tdd_support::must_some;

    #[test]
    fn test_slash_disambiguation() {
        // Test case 1: Division after identifier
        let mut lexer = PerlLexer::new("x / 2");
        assert_eq!(must_some(lexer.next_token()).token_type, TokenType::Identifier(Arc::from("x")));
        assert_eq!(must_some(lexer.next_token()).token_type, TokenType::Division);
        assert_eq!(must_some(lexer.next_token()).token_type, TokenType::Number(Arc::from("2")));

        // Test case 2: Regex after operator
        let mut lexer = PerlLexer::new("=~ /foo/");
        assert_eq!(must_some(lexer.next_token()).token_type, TokenType::Operator(Arc::from("=~")));
        assert_eq!(must_some(lexer.next_token()).token_type, TokenType::RegexMatch);

        // Test case 3: Division then regex
        let mut lexer = PerlLexer::new("1/ /abc/");
        assert_eq!(must_some(lexer.next_token()).token_type, TokenType::Number(Arc::from("1")));
        assert_eq!(must_some(lexer.next_token()).token_type, TokenType::Division);
        assert_eq!(must_some(lexer.next_token()).token_type, TokenType::RegexMatch);

        // Test case 4: Substitution
        let mut lexer = PerlLexer::new("s/foo/bar/g");
        let token = must_some(lexer.next_token());
        assert_eq!(token.token_type, TokenType::Substitution);
        assert_eq!(token.text.as_ref(), "s/foo/bar/g");
    }

    #[test]
    fn test_complex_delimiters() {
        // Test s{}{} syntax
        let mut lexer = PerlLexer::new("s{foo}{bar}g");
        let token = must_some(lexer.next_token());
        assert_eq!(token.token_type, TokenType::Substitution);

        // Test nested delimiters
        let mut lexer = PerlLexer::new("s{f{o}o}{bar}");
        let token = must_some(lexer.next_token());
        assert_eq!(token.token_type, TokenType::Substitution);
    }

    #[test]
    fn test_string_interpolation() {
        // Test simple variable interpolation
        let mut lexer = PerlLexer::new(r#""Hello $name""#);
        let token = must_some(lexer.next_token());
        match &token.token_type {
            TokenType::InterpolatedString(parts) => {
                assert_eq!(parts.len(), 2);
                assert!(matches!(&parts[0], StringPart::Literal(s) if s.as_ref() == "Hello "));
                assert!(matches!(&parts[1], StringPart::Variable(s) if s.as_ref() == "$name"));
            }
            _ => unreachable!("Expected InterpolatedString"),
        }

        // Test method call interpolation
        let mut lexer = PerlLexer::new(r#""The value is $obj->method()""#);
        let token = must_some(lexer.next_token());
        match &token.token_type {
            TokenType::InterpolatedString(parts) => {
                assert_eq!(parts.len(), 3);
                assert!(
                    matches!(&parts[0], StringPart::Literal(s) if s.as_ref() == "The value is ")
                );
                assert!(matches!(&parts[1], StringPart::Variable(s) if s.as_ref() == "$obj"));
                assert!(
                    matches!(&parts[2], StringPart::MethodCall(s) if s.as_ref() == "->method()")
                );
            }
            _ => unreachable!("Expected InterpolatedString"),
        }

        // Test ${expr} interpolation
        let mut lexer = PerlLexer::new(r#""Total: ${count + 1}""#);
        let token = must_some(lexer.next_token());
        match &token.token_type {
            TokenType::InterpolatedString(parts) => {
                assert_eq!(parts.len(), 2);
                assert!(matches!(&parts[0], StringPart::Literal(s) if s.as_ref() == "Total: "));
                assert!(
                    matches!(&parts[1], StringPart::Expression(s) if s.as_ref() == "${count + 1}")
                );
            }
            _ => unreachable!("Expected InterpolatedString"),
        }

        // Test array interpolation
        let mut lexer = PerlLexer::new(r#""Items: @array""#);
        let token = must_some(lexer.next_token());
        match &token.token_type {
            TokenType::InterpolatedString(parts) => {
                assert_eq!(parts.len(), 2);
                assert!(matches!(&parts[0], StringPart::Literal(s) if s.as_ref() == "Items: "));
                assert!(matches!(&parts[1], StringPart::Variable(s) if s.as_ref() == "@array"));
            }
            _ => unreachable!("Expected InterpolatedString"),
        }

        // Test array slice interpolation
        let mut lexer = PerlLexer::new(r#""Value: $array[0]""#);
        let token = must_some(lexer.next_token());
        match &token.token_type {
            TokenType::InterpolatedString(parts) => {
                assert_eq!(parts.len(), 3);
                assert!(matches!(&parts[0], StringPart::Literal(s) if s.as_ref() == "Value: "));
                assert!(matches!(&parts[1], StringPart::Variable(s) if s.as_ref() == "$array"));
                assert!(matches!(&parts[2], StringPart::ArraySlice(s) if s.as_ref() == "[0]"));
            }
            _ => unreachable!("Expected InterpolatedString"),
        }

        // Test special variables
        let mut lexer = PerlLexer::new(r#""PID: $$, Error: $!""#);
        let token = must_some(lexer.next_token());
        match &token.token_type {
            TokenType::InterpolatedString(parts) => {
                assert_eq!(parts.len(), 4);
                assert!(matches!(&parts[0], StringPart::Literal(s) if s.as_ref() == "PID: "));
                assert!(matches!(&parts[1], StringPart::Variable(s) if s.as_ref() == "$$"));
                assert!(matches!(&parts[2], StringPart::Literal(s) if s.as_ref() == ", Error: "));
                assert!(matches!(&parts[3], StringPart::Variable(s) if s.as_ref() == "$!"));
            }
            _ => unreachable!("Expected InterpolatedString"),
        }

        // Test escaped characters (should be StringLiteral, not InterpolatedString)
        let mut lexer = PerlLexer::new(r#""Line 1\nLine 2\t\$escaped""#);
        let token = must_some(lexer.next_token());
        assert!(matches!(token.token_type, TokenType::StringLiteral));

        // Test plain string (no interpolation)
        let mut lexer = PerlLexer::new(r#""Just a plain string""#);
        let token = must_some(lexer.next_token());
        assert!(matches!(token.token_type, TokenType::StringLiteral));
    }

    #[test]
    fn test_error_tokens() {
        // Test unknown character
        let mut lexer = PerlLexer::new("x § y");

        let token1 = must_some(lexer.next_token());
        assert!(matches!(token1.token_type, TokenType::Identifier(_)));

        let token2 = must_some(lexer.next_token());
        match &token2.token_type {
            TokenType::Error(msg) => {
                assert!(msg.contains("Unknown character"));
                assert!(msg.contains("§"));
            }
            _ => unreachable!("Expected Error token, got {:?}", token2.token_type),
        }

        let token3 = must_some(lexer.next_token());
        assert!(matches!(token3.token_type, TokenType::Identifier(_)));
    }
}
