use std::sync::Arc;

/// Perl lexer mode to disambiguate slash tokens
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LexerMode {
    /// Expecting a term (value) - slash starts a regex
    ExpectTerm,
    /// Expecting an operator - slash is division
    ExpectOperator,
}

/// Token types for disambiguation
#[derive(Debug, Clone, PartialEq)]
pub enum TokenType {
    // Slash-derived tokens
    Division,
    RegexMatch,        // m// or //
    Substitution,      // s///
    Transliteration,   // tr/// or y///
    QuoteRegex,        // qr//
    
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
    Arrow,             // =>
    FatComma,          // ,
    Whitespace,
    Newline,
    Comment(Arc<str>),
    EOF,
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
}

impl<'a> PerlLexer<'a> {
    pub fn new(input: &'a str) -> Self {
        Self {
            input,
            position: 0,
            mode: LexerMode::ExpectTerm,
            _delimiter_stack: Vec::new(),
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
        matches!(ch, '/' | '!' | '#' | '%' | '&' | '*' | ',' | '.' | ':' | ';' | '=' | '?' | '@' | '^' | '|' | '~' | '\'' | '"' | '`')
    }
    
    /// Update mode based on the token type
    fn update_mode(&mut self, token: &TokenType) {
        use TokenType::*;
        self.mode = match token {
            // These produce a value, so next slash is division
            Identifier(_) | Number(_) | RightParen | RightBracket | RightBrace => LexerMode::ExpectOperator,
            
            // These expect a value next, so slash starts regex
            Operator(_) | LeftParen | LeftBracket | LeftBrace | Semicolon | Comma | Arrow | FatComma => LexerMode::ExpectTerm,
            
            // Keywords depend on which keyword
            Keyword(kw) => match kw.as_ref() {
                // These expect a value
                "if" | "unless" | "while" | "until" | "for" | "foreach" | "given" |
                "return" | "my" | "our" | "local" | "state" => LexerMode::ExpectTerm,
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
        
        // Check for explicit operators
        if self.peek_str("s/") || self.peek_str("s{") || self.peek_str("s(") || self.peek_str("s[") {
            return self.scan_substitution();
        }
        if self.peek_str("tr/") || self.peek_str("y/") {
            return self.scan_transliteration();
        }
        if self.peek_str("m/") || self.peek_str("m{") || self.peek_str("m(") || self.peek_str("m[") {
            return self.scan_match_regex();
        }
        if self.peek_str("qr/") || self.peek_str("qr{") || self.peek_str("qr(") || self.peek_str("qr[") {
            return self.scan_quote_regex();
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
        
        // Skip 'm' if present
        if self.peek_str("m") {
            self.position += 1;
        }
        
        // Get delimiter
        if self.position >= self.input.len() {
            return None;
        }
        let delimiter = self.input.as_bytes()[self.position] as char;
        if !Self::is_regex_delimiter(delimiter) {
            return None;
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
                b'i' | b'm' | b's' | b'x' | b'o' | b'g' | b'c' | b'e' | b'r' | b'a' | b'd' | b'l' | b'u' | b'n' | b'p' => {
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
    
    /// Scan a substitution (s///)
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
        
        // For bracketed delimiters, skip whitespace and find next delimiter
        if delimiter != closing {
            self.skip_whitespace();
            if self.position < self.input.len() && self.input.as_bytes()[self.position] as char == delimiter {
                self.position += 1;
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
            if self.position < self.input.len() && self.input.as_bytes()[self.position] as char == delimiter {
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
    
    /// Get the next token
    pub fn next_token(&mut self) -> Option<Token> {
        self.skip_whitespace();
        
        if self.position >= self.input.len() {
            return Some(Token {
                token_type: TokenType::EOF,
                text: Arc::from(""),
                start: self.position,
                end: self.position,
            });
        }
        
        let start = self.position;
        let ch = self.input.as_bytes()[self.position];
        
        // Check for regex-like constructs first
        if ch == b'/' || self.peek_str("s") || self.peek_str("m") || self.peek_str("tr") || self.peek_str("y") || self.peek_str("qr") {
            if let Some(token) = self.scan_regex_like() {
                self.update_mode(&token.token_type);
                return Some(token);
            }
        }
        
        // Handle other tokens
        match ch {
            b'#' => {
                // Comment
                self.skip_line();
                Some(Token {
                    token_type: TokenType::Comment(Arc::from(self.safe_slice(start, self.position))),
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
                if self.position + 1 < self.input.len() && self.input.as_bytes()[self.position + 1] == b'>' {
                    self.position += 2;
                    let token = Token {
                        token_type: TokenType::Arrow,
                        text: Arc::from("=>"),
                        start,
                        end: self.position,
                    };
                    self.update_mode(&token.token_type);
                    Some(token)
                } else if self.position + 1 < self.input.len() && self.input.as_bytes()[self.position + 1] == b'~' {
                    self.position += 2;
                    let token = Token {
                        token_type: TokenType::Operator(Arc::from("=~")),
                        text: Arc::from("=~"),
                        start,
                        end: self.position,
                    };
                    self.update_mode(&token.token_type);
                    Some(token)
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
                while self.position < self.input.len() {
                    match self.input.as_bytes()[self.position] {
                        b'0'..=b'9' | b'.' | b'e' | b'E' | b'_' => self.position += 1,
                        _ => break,
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
            b'a'..=b'z' | b'A'..=b'Z' | b'_' => {
                // Identifier or keyword
                while self.position < self.input.len() {
                    match self.input.as_bytes()[self.position] {
                        b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'_' => self.position += 1,
                        _ => break,
                    }
                }
                let text = self.safe_slice(start, self.position);
                let token = match text {
                    "if" | "unless" | "while" | "until" | "for" | "foreach" | "given" |
                    "return" | "my" | "our" | "local" | "state" | "sub" | "do" | "eval" |
                    "package" | "use" | "require" | "no" | "BEGIN" | "END" | "CHECK" | "INIT" => {
                        Token {
                            token_type: TokenType::Keyword(Arc::from(text)),
                            text: Arc::from(text),
                            start,
                            end: self.position,
                        }
                    }
                    _ => {
                        Token {
                            token_type: TokenType::Identifier(Arc::from(text)),
                            text: Arc::from(text),
                            start,
                            end: self.position,
                        }
                    }
                };
                self.update_mode(&token.token_type);
                Some(token)
            }
            b'+' | b'-' | b'*' | b'%' | b'&' | b'|' | b'^' | b'~' | b'!' | b'<' | b'>' | b'.' => {
                // Operators
                self.position += 1;
                // Check for compound operators
                if self.position < self.input.len() {
                    let next = self.input.as_bytes()[self.position];
                    match (ch, next) {
                        (b'+', b'+') | (b'-', b'-') | (b'*', b'*') | (b'<', b'<') | (b'>', b'>') |
                        (b'&', b'&') | (b'|', b'|') | (b'.', b'.') | (b'!', b'~') => {
                            self.position += 1;
                        }
                        (b'<', b'=') | (b'>', b'=') | (b'!', b'=') | (b'=', b'=') => {
                            self.position += 1;
                        }
                        _ => {}
                    }
                }
                let token = Token {
                    token_type: TokenType::Operator(Arc::from(self.safe_slice(start, self.position))),
                    text: Arc::from(self.safe_slice(start, self.position)),
                    start,
                    end: self.position,
                };
                self.update_mode(&token.token_type);
                Some(token)
            }
            _ => {
                // Unknown character, skip it
                self.position += 1;
                self.next_token()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slash_disambiguation() {
        // Test case 1: Division after identifier
        let mut lexer = PerlLexer::new("x / 2");
        assert_eq!(lexer.next_token().unwrap().token_type, TokenType::Identifier(Arc::from("x")));
        assert_eq!(lexer.next_token().unwrap().token_type, TokenType::Division);
        assert_eq!(lexer.next_token().unwrap().token_type, TokenType::Number(Arc::from("2")));
        
        // Test case 2: Regex after operator
        let mut lexer = PerlLexer::new("=~ /foo/");
        assert_eq!(lexer.next_token().unwrap().token_type, TokenType::Operator(Arc::from("=~")));
        assert_eq!(lexer.next_token().unwrap().token_type, TokenType::RegexMatch);
        
        // Test case 3: Division then regex
        let mut lexer = PerlLexer::new("1/ /abc/");
        assert_eq!(lexer.next_token().unwrap().token_type, TokenType::Number(Arc::from("1")));
        assert_eq!(lexer.next_token().unwrap().token_type, TokenType::Division);
        assert_eq!(lexer.next_token().unwrap().token_type, TokenType::RegexMatch);
        
        // Test case 4: Substitution
        let mut lexer = PerlLexer::new("s/foo/bar/g");
        let token = lexer.next_token().unwrap();
        assert_eq!(token.token_type, TokenType::Substitution);
        assert_eq!(token.text.as_ref(), "s/foo/bar/g");
    }
    
    #[test]
    fn test_complex_delimiters() {
        // Test s{}{} syntax
        let mut lexer = PerlLexer::new("s{foo}{bar}g");
        let token = lexer.next_token().unwrap();
        assert_eq!(token.token_type, TokenType::Substitution);
        
        // Test nested delimiters
        let mut lexer = PerlLexer::new("s{f{o}o}{bar}");
        let token = lexer.next_token().unwrap();
        assert_eq!(token.token_type, TokenType::Substitution);
    }
}