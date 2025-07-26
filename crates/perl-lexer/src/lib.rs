//! High-performance Perl lexer with context-aware tokenization
//!
//! This crate provides a lexer for Perl that handles the context-sensitive
//! nature of the language, particularly the ambiguity of the `/` character
//! which can be either division or the start of a regex.

use std::sync::Arc;

pub mod token;
pub mod mode;
pub mod error;
pub mod position;
mod unicode;

pub use token::{Token, TokenType, StringPart};
pub use mode::LexerMode;
pub use error::{LexerError, Result};
pub use position::Position;

use unicode::{is_perl_identifier_start, is_perl_identifier_continue};

/// Configuration for the lexer
#[derive(Debug, Clone)]
pub struct LexerConfig {
    /// Enable interpolation parsing in strings
    pub parse_interpolation: bool,
    /// Track token positions for error reporting
    pub track_positions: bool,
    /// Maximum lookahead for disambiguation
    pub max_lookahead: usize,
}

impl Default for LexerConfig {
    fn default() -> Self {
        Self {
            parse_interpolation: true,
            track_positions: true,
            max_lookahead: 1024,
        }
    }
}

/// Mode-aware Perl lexer
pub struct PerlLexer<'a> {
    input: &'a str,
    /// Cached input bytes for faster access
    input_bytes: &'a [u8],
    position: usize,
    mode: LexerMode,
    config: LexerConfig,
    /// Stack for nested delimiters in s{}{} constructs
    delimiter_stack: Vec<char>,
    /// Track if we're inside prototype parens after 'sub'
    in_prototype: bool,
    /// Paren depth to track when we exit prototype
    prototype_depth: usize,
    /// Current position with line/column tracking
    #[allow(dead_code)]
    current_pos: Position,
}

impl<'a> PerlLexer<'a> {
    /// Create a new lexer for the given input
    pub fn new(input: &'a str) -> Self {
        Self::with_config(input, LexerConfig::default())
    }

    /// Create a new lexer with custom configuration
    pub fn with_config(input: &'a str, config: LexerConfig) -> Self {
        Self {
            input,
            input_bytes: input.as_bytes(),
            position: 0,
            mode: LexerMode::ExpectTerm,
            config,
            delimiter_stack: Vec::new(),
            in_prototype: false,
            prototype_depth: 0,
            current_pos: Position::start(),
        }
    }

    /// Get the next token from the input
    pub fn next_token(&mut self) -> Option<Token> {
        // Handle format body parsing if we're in that mode
        if matches!(self.mode, LexerMode::InFormatBody) {
            return self.parse_format_body();
        }
        
        self.skip_whitespace_and_comments()?;
        
        if self.position >= self.input.len() {
            return Some(Token {
                token_type: TokenType::EOF,
                text: Arc::from(""),
                start: self.position,
                end: self.position,
            });
        }

        let start = self.position;
        
        // Check for special tokens first
        if let Some(token) = self.try_heredoc() {
            return Some(token);
        }
        
        if let Some(token) = self.try_string() {
            return Some(token);
        }
        
        if let Some(token) = self.try_variable() {
            return Some(token);
        }
        
        if let Some(token) = self.try_number() {
            return Some(token);
        }
        
        if let Some(token) = self.try_identifier_or_keyword() {
            return Some(token);
        }
        
        if let Some(token) = self.try_operator() {
            return Some(token);
        }
        
        if let Some(token) = self.try_delimiter() {
            return Some(token);
        }
        
        // If nothing else matches, return an error token
        let ch = self.current_char()?;
        self.advance();
        
        Some(Token {
            token_type: TokenType::Error(Arc::from(format!("Unexpected character: {}", ch))),
            text: Arc::from(ch.to_string()),
            start,
            end: self.position,
        })
    }

    /// Peek at the next token without consuming it
    pub fn peek_token(&mut self) -> Option<Token> {
        let saved_pos = self.position;
        let saved_mode = self.mode;
        let saved_prototype = self.in_prototype;
        let saved_depth = self.prototype_depth;
        
        let token = self.next_token();
        
        self.position = saved_pos;
        self.mode = saved_mode;
        self.in_prototype = saved_prototype;
        self.prototype_depth = saved_depth;
        
        token
    }

    /// Get all remaining tokens
    pub fn collect_tokens(&mut self) -> Vec<Token> {
        let mut tokens = Vec::new();
        while let Some(token) = self.next_token() {
            if token.token_type == TokenType::EOF {
                tokens.push(token);
                break;
            }
            tokens.push(token);
        }
        tokens
    }

    /// Reset the lexer to the beginning
    pub fn reset(&mut self) {
        self.position = 0;
        self.mode = LexerMode::ExpectTerm;
        self.delimiter_stack.clear();
        self.in_prototype = false;
        self.prototype_depth = 0;
    }
    
    /// Switch lexer to format body parsing mode
    pub fn enter_format_mode(&mut self) {
        self.mode = LexerMode::InFormatBody;
    }

    // Internal helper methods
    
    #[inline]
    fn current_char(&self) -> Option<char> {
        if self.position < self.input_bytes.len() {
            // For ASCII, direct access is safe
            let byte = self.input_bytes[self.position];
            if byte < 128 {
                Some(byte as char)
            } else {
                // For non-ASCII, fall back to proper UTF-8 parsing
                self.input[self.position..].chars().next()
            }
        } else {
            None
        }
    }
    
    #[inline]
    fn peek_char(&self, offset: usize) -> Option<char> {
        let pos = self.position + offset;
        if pos < self.input_bytes.len() {
            // For ASCII, direct access is safe
            let byte = self.input_bytes[pos];
            if byte < 128 {
                Some(byte as char)
            } else {
                // For non-ASCII, use chars iterator
                self.input[self.position..].chars().nth(offset)
            }
        } else {
            None
        }
    }
    
    #[inline]
    fn advance(&mut self) {
        if self.position < self.input_bytes.len() {
            let byte = self.input_bytes[self.position];
            if byte < 128 {
                // ASCII fast path
                self.position += 1;
            } else if let Some(ch) = self.input[self.position..].chars().next() {
                self.position += ch.len_utf8();
            }
        }
    }
    
    /// Fast byte-level check for ASCII characters
    #[inline]
    fn peek_byte(&self, offset: usize) -> Option<u8> {
        let pos = self.position + offset;
        if pos < self.input_bytes.len() {
            Some(self.input_bytes[pos])
        } else {
            None
        }
    }
    
    /// Check if the next bytes match a pattern (ASCII only)
    #[inline]
    #[allow(dead_code)]
    fn matches_bytes(&self, pattern: &[u8]) -> bool {
        let end = self.position + pattern.len();
        if end <= self.input_bytes.len() {
            &self.input_bytes[self.position..end] == pattern
        } else {
            false
        }
    }
    
    fn skip_whitespace_and_comments(&mut self) -> Option<()> {
        while self.position < self.input_bytes.len() {
            let byte = self.input_bytes[self.position];
            match byte {
                b' ' | b'\t' | b'\r' => self.position += 1,
                b'\n' => {
                    self.advance();
                    // Newlines can affect parsing context
                }
                b'#' => {
                    // In ExpectDelimiter mode, '#' is a delimiter, not a comment
                    if matches!(self.mode, LexerMode::ExpectDelimiter) {
                        break;
                    }
                    
                    // Skip line comment using byte-level operations
                    self.advance();
                    while self.position < self.input_bytes.len() {
                        if self.input_bytes[self.position] == b'\n' {
                            break;
                        }
                        self.advance();
                    }
                }
                _ => {
                    // For non-ASCII whitespace, use char check
                    if byte >= 128 {
                        if let Some(ch) = self.current_char() {
                            if ch.is_whitespace() {
                                self.advance();
                                continue;
                            }
                        }
                    }
                    break;
                }
            }
        }
        Some(())
    }
    
    fn try_heredoc(&mut self) -> Option<Token> {
        // Check for heredoc start
        if self.peek_byte(0) != Some(b'<') || self.peek_byte(1) != Some(b'<') {
            return None;
        }
        
        let start = self.position;
        self.position += 2; // Skip <<
        
        // Check for indented heredoc (~)
        let _indented = if self.current_char() == Some('~') {
            self.advance();
            true
        } else {
            false
        };
        
        // Skip whitespace
        while let Some(ch) = self.current_char() {
            if ch == ' ' || ch == '\t' {
                self.advance();
            } else {
                break;
            }
        }
        
        // Parse delimiter
        let delimiter_start = self.position;
        let _delimiter = if self.position < self.input.len() {
            match self.current_char() {
                Some('"') => {
                    // Double-quoted delimiter
                    self.advance();
                    let delim_start = self.position;
                    while self.position < self.input.len() {
                        if self.current_char() == Some('"') {
                            let _delim = self.input[delim_start..self.position].to_string();
                            self.advance();
                            break;
                        }
                        self.advance();
                    }
                    self.input[delim_start..self.position-1].to_string()
                }
                Some('\'') => {
                    // Single-quoted delimiter
                    self.advance();
                    let delim_start = self.position;
                    while self.position < self.input.len() {
                        if self.current_char() == Some('\'') {
                            let _delim = self.input[delim_start..self.position].to_string();
                            self.advance();
                            break;
                        }
                        self.advance();
                    }
                    self.input[delim_start..self.position-1].to_string()
                }
                Some(c) if is_perl_identifier_start(c) => {
                    // Bare word delimiter
                    while self.position < self.input.len() {
                        if let Some(c) = self.current_char() {
                            if is_perl_identifier_continue(c) {
                                self.advance();
                            } else {
                                break;
                            }
                        } else {
                            break;
                        }
                    }
                    self.input[delimiter_start..self.position].to_string()
                }
                _ => return None,
            }
        } else {
            return None;
        };
        
        // For now, return a placeholder token
        // The actual heredoc body would be parsed later when we encounter it
        let text = &self.input[start..self.position];
        self.mode = LexerMode::ExpectOperator;
        
        Some(Token {
            token_type: TokenType::HeredocStart,
            text: Arc::from(text),
            start,
            end: self.position,
        })
    }
    
    fn try_string(&mut self) -> Option<Token> {
        let start = self.position;
        let quote = self.current_char()?;
        
        match quote {
            '"' => self.parse_double_quoted_string(start),
            '\'' => self.parse_single_quoted_string(start),
            '`' => self.parse_backtick_string(start),
            'q' if self.peek_char(1) == Some('{') => self.parse_q_string(start),
            _ => None,
        }
    }
    
    fn try_number(&mut self) -> Option<Token> {
        let start = self.position;
        
        // Fast byte check for digits
        if self.position < self.input_bytes.len() && self.input_bytes[self.position].is_ascii_digit() {
            // Consume initial digits
            while self.position < self.input_bytes.len() {
                match self.input_bytes[self.position] {
                    b'0'..=b'9' | b'_' => self.position += 1,
                    _ => break,
                }
            }
            
            // Check for decimal point
            if self.position < self.input_bytes.len() && self.input_bytes[self.position] == b'.' {
                // Peek ahead to see what follows the dot
                let followed_by_digit = self.position + 1 < self.input_bytes.len() && 
                    self.input_bytes[self.position + 1].is_ascii_digit();
                
                // In Perl, "5." is a valid decimal number (5.0)
                // We consume the dot if:
                // 1. It's followed by a digit (5.123)
                // 2. OR it's followed by whitespace, operator, or end of input (5.)
                let should_consume_dot = if followed_by_digit {
                    true
                } else if self.position + 1 >= self.input_bytes.len() {
                    // End of input after dot
                    true
                } else {
                    // Check if followed by something that would end a number
                    let next_byte = self.input_bytes[self.position + 1];
                    // Also check for 'e' or 'E' for scientific notation
                    matches!(next_byte, 
                        b' ' | b'\t' | b'\n' | b'\r' |  // whitespace
                        b';' | b',' | b')' | b'}' | b']' |  // delimiters
                        b'+' | b'-' | b'*' | b'/' | b'%' |  // operators
                        b'=' | b'<' | b'>' | b'!' | b'&' | b'|' | b'^' | b'~' |
                        b'e' | b'E'  // scientific notation
                    )
                };
                
                if should_consume_dot {
                    self.position += 1; // consume the dot
                    // Consume fractional digits if any
                    while self.position < self.input_bytes.len() {
                        match self.input_bytes[self.position] {
                            b'0'..=b'9' | b'_' => self.position += 1,
                            _ => break,
                        }
                    }
                }
            }
            
            // Check for exponent
            if self.position < self.input_bytes.len() {
                let byte = self.input_bytes[self.position];
                if byte == b'e' || byte == b'E' {
                    let exp_start = self.position;
                    self.position += 1; // consume 'e' or 'E'
                    
                    // Check for optional sign
                    if self.position < self.input_bytes.len() {
                        let next = self.input_bytes[self.position];
                        if next == b'+' || next == b'-' {
                            self.position += 1;
                        }
                    }
                    
                    // Must have at least one digit after exponent
                    let digit_start = self.position;
                    while self.position < self.input_bytes.len() && self.input_bytes[self.position].is_ascii_digit() {
                        self.position += 1;
                    }
                    
                    // If no digits after exponent, backtrack
                    if self.position == digit_start {
                        self.position = exp_start;
                    }
                }
            }
            
            let text = &self.input[start..self.position];
            self.mode = LexerMode::ExpectOperator;
            
            Some(Token {
                token_type: TokenType::Number(Arc::from(text)),
                text: Arc::from(text),
                start,
                end: self.position,
            })
        } else {
            None
        }
    }
    
    fn parse_decimal_number(&mut self, start: usize) -> Option<Token> {
        // We're at the dot, consume it
        self.advance();
        
        // Parse the fractional part
        while self.position < self.input_bytes.len() {
            let byte = self.input_bytes[self.position];
            match byte {
                b'0'..=b'9' | b'_' => self.position += 1,
                b'e' | b'E' => {
                    // Handle scientific notation
                    self.advance();
                    if self.position < self.input_bytes.len() {
                        let next = self.input_bytes[self.position];
                        if next == b'+' || next == b'-' {
                            self.advance();
                        }
                    }
                    // Parse exponent digits
                    while self.position < self.input_bytes.len() && self.input_bytes[self.position].is_ascii_digit() {
                        self.position += 1;
                    }
                    break;
                }
                _ => break,
            }
        }
        
        let text = &self.input[start..self.position];
        self.mode = LexerMode::ExpectOperator;
        
        Some(Token {
            token_type: TokenType::Number(Arc::from(text)),
            text: Arc::from(text),
            start,
            end: self.position,
        })
    }
    
    fn try_variable(&mut self) -> Option<Token> {
        let start = self.position;
        let sigil = self.current_char()?;
        
        match sigil {
            '$' | '@' | '%' | '*' => {
                // In ExpectOperator mode, * should be treated as multiplication, not a glob sigil
                if sigil == '*' && self.mode == LexerMode::ExpectOperator {
                    return None;
                }
                self.advance();
                
                // Special case: After ->, sigils followed by { or [ should be tokenized separately
                // This is for postfix dereference like ->@*, ->%{}, ->@[]
                if self.position >= 3 && &self.input[self.position.saturating_sub(3)..self.position.saturating_sub(1)] == "->" {
                    if matches!(self.current_char(), Some('{') | Some('[') | Some('*')) {
                        // Just return the sigil
                        let text = &self.input[start..self.position];
                        self.mode = LexerMode::ExpectOperator;
                        
                        return Some(Token {
                            token_type: TokenType::Identifier(Arc::from(text)),
                            text: Arc::from(text),
                            start,
                            end: self.position,
                        });
                    }
                }
                
                // Check for $# (array length operator)
                if sigil == '$' && self.current_char() == Some('#') {
                    self.advance(); // consume #
                    // Now parse the array name
                    while let Some(ch) = self.current_char() {
                        if is_perl_identifier_continue(ch) {
                            self.advance();
                        } else if ch == ':' && self.peek_char(1) == Some(':') {
                            // Package-qualified array name
                            self.advance();
                            self.advance();
                        } else {
                            break;
                        }
                    }
                    
                    let text = &self.input[start..self.position];
                    self.mode = LexerMode::ExpectOperator;
                    
                    return Some(Token {
                        token_type: TokenType::Identifier(Arc::from(text)),
                        text: Arc::from(text),
                        start,
                        end: self.position,
                    });
                }
                
                // Check for special cases like ${^MATCH} or ${::{foo}} or *{$glob}
                if self.current_char() == Some('{') {
                    // Peek ahead to decide if we should consume the brace
                    let next_char = self.peek_char(1);
                    
                    // Check if this is a dereference like @{$ref} or @{[...]}
                    // If the next char suggests dereference, don't consume the brace
                    if sigil != '*' && matches!(next_char, 
                        Some('$') | Some('@') | Some('%') | Some('*') | Some('&') |
                        Some('[') | Some(' ') | Some('\t') | Some('\n') | Some('\r')) {
                        // This is a dereference, don't consume the brace
                        let text = &self.input[start..self.position];
                        self.mode = LexerMode::ExpectOperator;
                        
                        return Some(Token {
                            token_type: TokenType::Identifier(Arc::from(text)),
                            text: Arc::from(text),
                            start,
                            end: self.position,
                        });
                    }
                    
                    self.advance(); // consume {
                    
                    // Handle special variables with caret
                    if self.current_char() == Some('^') {
                        self.advance(); // consume ^
                        // Parse the special variable name
                        while let Some(ch) = self.current_char() {
                            if ch == '}' {
                                self.advance(); // consume }
                                break;
                            } else if is_perl_identifier_continue(ch) {
                                self.advance();
                            } else {
                                break;
                            }
                        }
                    } 
                    // Handle stash access like $::{foo}
                    else if self.current_char() == Some(':') && self.peek_char(1) == Some(':') {
                        self.advance(); // consume first :
                        self.advance(); // consume second :
                        // Skip optional { and }
                        if self.current_char() == Some('{') {
                            self.advance();
                        }
                        // Parse the name
                        while let Some(ch) = self.current_char() {
                            if ch == '}' {
                                self.advance();
                                if self.current_char() == Some('}') {
                                    self.advance(); // consume closing } of ${...}
                                }
                                break;
                            } else if is_perl_identifier_continue(ch) {
                                self.advance();
                            } else {
                                break;
                            }
                        }
                    }
                    // Regular braced variable like ${foo} or glob like *{$glob}
                    else {
                        // Check if this is a dereference like ${$ref} or @{$ref} or @{[...]}
                        // If the next char is a sigil or other expression starter, we should stop here and let the parser handle it
                        // EXCEPT for globs - *{$glob} should be parsed as one token
                        if sigil != '*' && matches!(self.current_char(), 
                            Some('$') | Some('@') | Some('%') | Some('*') | Some('&') |
                            Some('[') | Some(' ') | Some('\t') | Some('\n') | Some('\r')) {
                            // This is a dereference, backtrack
                            self.position = start + 1; // Just past the sigil
                            let text = &self.input[start..self.position];
                            self.mode = LexerMode::ExpectOperator;
                            
                            return Some(Token {
                                token_type: TokenType::Identifier(Arc::from(text)),
                                text: Arc::from(text),
                                start,
                                end: self.position,
                            });
                        }
                        
                        // For glob access, we need to consume everything inside braces
                        if sigil == '*' {
                            let mut brace_depth = 1;
                            while let Some(ch) = self.current_char() {
                                if ch == '{' {
                                    brace_depth += 1;
                                } else if ch == '}' {
                                    brace_depth -= 1;
                                    if brace_depth == 0 {
                                        self.advance(); // consume final }
                                        break;
                                    }
                                }
                                self.advance();
                            }
                        } else {
                            // Regular variable
                            while let Some(ch) = self.current_char() {
                                if ch == '}' {
                                    self.advance(); // consume }
                                    break;
                                } else if is_perl_identifier_continue(ch) {
                                    self.advance();
                                } else {
                                    break;
                                }
                            }
                        }
                    }
                }
                // Parse regular variable name
                else if let Some(ch) = self.current_char() {
                    if is_perl_identifier_start(ch) {
                        while let Some(ch) = self.current_char() {
                            if is_perl_identifier_continue(ch) {
                                self.advance();
                            } else {
                                break;
                            }
                        }
                    }
                    // Handle special punctuation variables
                    else if sigil == '$' && matches!(ch, '?' | '!' | '@' | '&' | '`' | '\'' | '.' | '/' | '\\' | '|' | '+' | '-' | '[' | ']' | '$') {
                        self.advance(); // consume the special character
                    }
                    // Handle special array/hash punctuation variables
                    else if (sigil == '@' || sigil == '%') && matches!(ch, '+' | '-') {
                        self.advance(); // consume the + or -
                    }
                    // Handle :: for package-qualified variables or stash access
                    else if ch == ':' && self.peek_char(1) == Some(':') {
                        self.advance(); // consume first :
                        self.advance(); // consume second :
                        // Now parse the rest like a normal identifier
                        while let Some(ch) = self.current_char() {
                            if is_perl_identifier_continue(ch) {
                                self.advance();
                            } else if ch == ':' && self.peek_char(1) == Some(':') {
                                self.advance();
                                self.advance();
                            } else {
                                break;
                            }
                        }
                    }
                }
                
                let text = &self.input[start..self.position];
                self.mode = LexerMode::ExpectOperator;
                
                Some(Token {
                    token_type: TokenType::Identifier(Arc::from(text)),
                    text: Arc::from(text),
                    start,
                    end: self.position,
                })
            }
            _ => None,
        }
    }
    
    fn try_identifier_or_keyword(&mut self) -> Option<Token> {
        let start = self.position;
        let ch = self.current_char()?;
        
        if is_perl_identifier_start(ch) {
            while let Some(ch) = self.current_char() {
                if is_perl_identifier_continue(ch) {
                    self.advance();
                } else {
                    break;
                }
            }
            
            let text = &self.input[start..self.position];
            
            // Check for substitution/transliteration operators
            if matches!(text, "s" | "tr" | "y") {
                if let Some(next) = self.current_char() {
                    // Check if followed by a delimiter
                    if matches!(next, '/' | '|' | '{' | '[' | '(' | '<' | '!' | '#' | '@' | '$' | '%' | '^' | '&' | '*' | '+' | '=' | '~' | '`') {
                        match text {
                            "s" => {
                                return self.parse_substitution(start);
                            }
                            "tr" | "y" => {
                                return self.parse_transliteration(start);
                            }
                            _ => unreachable!()
                        }
                    }
                }
            }
            
            let token_type = if is_keyword(text) {
                // Check for special keywords that affect lexer mode
                match text {
                    "if" | "unless" | "while" | "until" | "for" | "foreach" => {
                        self.mode = LexerMode::ExpectTerm;
                    }
                    "sub" => {
                        self.in_prototype = true;
                    }
                    // Quote operators expect a delimiter next
                    "q" | "qq" | "qw" | "qr" | "qx" | "m" => {
                        self.mode = LexerMode::ExpectDelimiter;
                    }
                    // Format declarations need special handling
                    "format" => {
                        // We'll need to check for the = after the format name
                        // For now, just mark that we saw format
                    }
                    _ => {}
                }
                TokenType::Keyword(Arc::from(text))
            } else {
                self.mode = LexerMode::ExpectOperator;
                TokenType::Identifier(Arc::from(text))
            };
            
            Some(Token {
                token_type,
                text: Arc::from(text),
                start,
                end: self.position,
            })
        } else {
            None
        }
    }
    
    /// Parse format body - consumes until a line with just a dot
    fn parse_format_body(&mut self) -> Option<Token> {
        let start = self.position;
        let mut body = String::new();
        let mut line_start = true;
        
        while self.position < self.input.len() {
            // Check if we're at the start of a line and the next char is a dot
            if line_start && self.current_char() == Some('.') {
                // Check if this line contains only a dot
                let mut peek_pos = self.position + 1;
                let mut found_terminator = true;
                
                // Skip any trailing whitespace on the dot line
                while peek_pos < self.input.len() {
                    match self.input_bytes[peek_pos] {
                        b' ' | b'\t' | b'\r' => peek_pos += 1,
                        b'\n' => break,
                        _ => {
                            found_terminator = false;
                            break;
                        }
                    }
                }
                
                if found_terminator {
                    // We found the terminating dot, consume it
                    self.position = peek_pos;
                    if self.position < self.input.len() && self.input_bytes[self.position] == b'\n' {
                        self.position += 1;
                    }
                    
                    // Switch back to normal mode
                    self.mode = LexerMode::ExpectTerm;
                    
                    return Some(Token {
                        token_type: TokenType::FormatBody(Arc::from(body.clone())),
                        text: Arc::from(body),
                        start,
                        end: self.position,
                    });
                }
            }
            
            // Not a terminator, consume the character
            match self.current_char() {
                Some(ch) => {
                    body.push(ch);
                    self.advance();
                    
                    // Track if we're at the start of a line
                    line_start = ch == '\n';
                }
                None => {
                    // Reached EOF without finding terminator
                    break;
                }
            }
        }
        
        // If we reach here, we didn't find a terminator
        self.mode = LexerMode::ExpectTerm;
        Some(Token {
            token_type: TokenType::Error(Arc::from("Unterminated format body")),
            text: Arc::from(body),
            start,
            end: self.position,
        })
    }
    
    fn try_operator(&mut self) -> Option<Token> {
        let start = self.position;
        let ch = self.current_char()?;
        
        // Handle slash disambiguation
        if ch == '/' {
            if self.mode == LexerMode::ExpectTerm {
                // It's a regex
                return self.parse_regex(start);
            } else {
                // It's division or defined-or operator
                self.advance();
                // Check for // or //=
                if self.current_char() == Some('/') {
                    self.advance(); // consume second /
                    if self.current_char() == Some('=') {
                        self.advance(); // consume =
                        let text = &self.input[start..self.position];
                        self.mode = LexerMode::ExpectTerm;
                        return Some(Token {
                            token_type: TokenType::Operator(Arc::from(text)),
                            text: Arc::from(text),
                            start,
                            end: self.position,
                        });
                    } else {
                        let text = &self.input[start..self.position];
                        self.mode = LexerMode::ExpectTerm;
                        return Some(Token {
                            token_type: TokenType::Operator(Arc::from(text)),
                            text: Arc::from(text),
                            start,
                            end: self.position,
                        });
                    }
                } else {
                    self.mode = LexerMode::ExpectTerm;
                    return Some(Token {
                        token_type: TokenType::Division,
                        text: Arc::from("/"),
                        start,
                        end: self.position,
                    });
                }
            }
        }
        
        // Handle other operators - simplified
        match ch {
            '.' => {
                // Check if it's a decimal number like .5
                if self.peek_char(1).map_or(false, |c| c.is_ascii_digit()) {
                    return self.parse_decimal_number(start);
                }
                self.advance();
                // Check for compound operators
                if let Some(next) = self.current_char() {
                    if is_compound_operator(ch, next) {
                        self.advance();
                        
                        // Check for three-character operators like **=, <<=, >>=
                        if self.position < self.input.len() {
                            let third = self.current_char();
                            if ch == '*' && next == '*' && third == Some('=') {
                                self.advance(); // consume the =
                            } else if ch == '<' && next == '<' && third == Some('=') {
                                self.advance(); // consume the =
                            } else if ch == '<' && next == '=' && third == Some('>') {
                                self.advance(); // consume the >
                                // Special case: <=> spaceship operator
                            } else if ch == '>' && next == '>' && third == Some('=') {
                                self.advance(); // consume the =
                            } else if ch == '&' && next == '&' && third == Some('=') {
                                self.advance(); // consume the =
                            } else if ch == '|' && next == '|' && third == Some('=') {
                                self.advance(); // consume the =
                            } else if ch == '/' && next == '/' && third == Some('=') {
                                self.advance(); // consume the =
                            } else if ch == '.' && next == '.' && third == Some('.') {
                                self.advance(); // consume the third .
                            }
                        }
                    }
                }
            }
            '+' | '-' | '*' | '%' | '&' | '|' | '^' | '~' | '!' | '=' | '<' | '>' | ':' | '?' | '\\' => {
                self.advance();
                // Check for compound operators
                if let Some(next) = self.current_char() {
                    if is_compound_operator(ch, next) {
                        self.advance();
                        
                        // Check for three-character operators like **=, <<=, >>=
                        if self.position < self.input.len() {
                            let third = self.current_char();
                            if ch == '*' && next == '*' && third == Some('=') {
                                self.advance(); // consume the =
                            } else if ch == '<' && next == '<' && third == Some('=') {
                                self.advance(); // consume the =
                            } else if ch == '<' && next == '=' && third == Some('>') {
                                self.advance(); // consume the >
                                // Special case: <=> spaceship operator
                            } else if ch == '>' && next == '>' && third == Some('=') {
                                self.advance(); // consume the =
                            } else if ch == '&' && next == '&' && third == Some('=') {
                                self.advance(); // consume the =
                            } else if ch == '|' && next == '|' && third == Some('=') {
                                self.advance(); // consume the =
                            } else if ch == '/' && next == '/' && third == Some('=') {
                                self.advance(); // consume the =
                            }
                        }
                    }
                }
            }
            _ => return None,
        }
        
        let text = &self.input[start..self.position];
        self.mode = LexerMode::ExpectTerm;
        
        Some(Token {
            token_type: TokenType::Operator(Arc::from(text)),
            text: Arc::from(text),
            start,
            end: self.position,
        })
    }
    
    fn try_delimiter(&mut self) -> Option<Token> {
        let start = self.position;
        let ch = self.current_char()?;
        
        match ch {
            '(' => {
                self.advance();
                if self.in_prototype {
                    self.prototype_depth += 1;
                }
                self.mode = LexerMode::ExpectTerm;
                Some(Token {
                    token_type: TokenType::LeftParen,
                    text: Arc::from("("),
                    start,
                    end: self.position,
                })
            }
            ')' => {
                self.advance();
                if self.in_prototype && self.prototype_depth > 0 {
                    self.prototype_depth -= 1;
                    if self.prototype_depth == 0 {
                        self.in_prototype = false;
                    }
                }
                self.mode = LexerMode::ExpectOperator;
                Some(Token {
                    token_type: TokenType::RightParen,
                    text: Arc::from(")"),
                    start,
                    end: self.position,
                })
            }
            ';' => {
                self.advance();
                self.mode = LexerMode::ExpectTerm;
                Some(Token {
                    token_type: TokenType::Semicolon,
                    text: Arc::from(";"),
                    start,
                    end: self.position,
                })
            }
            ',' => {
                self.advance();
                self.mode = LexerMode::ExpectTerm;
                Some(Token {
                    token_type: TokenType::Comma,
                    text: Arc::from(","),
                    start,
                    end: self.position,
                })
            }
            '[' => {
                self.advance();
                self.mode = LexerMode::ExpectTerm;
                Some(Token {
                    token_type: TokenType::LeftBracket,
                    text: Arc::from("["),
                    start,
                    end: self.position,
                })
            }
            ']' => {
                self.advance();
                self.mode = LexerMode::ExpectOperator;
                Some(Token {
                    token_type: TokenType::RightBracket,
                    text: Arc::from("]"),
                    start,
                    end: self.position,
                })
            }
            '{' => {
                self.advance();
                self.mode = LexerMode::ExpectTerm;
                Some(Token {
                    token_type: TokenType::LeftBrace,
                    text: Arc::from("{"),
                    start,
                    end: self.position,
                })
            }
            '}' => {
                self.advance();
                self.mode = LexerMode::ExpectOperator;
                Some(Token {
                    token_type: TokenType::RightBrace,
                    text: Arc::from("}"),
                    start,
                    end: self.position,
                })
            }
            '#' => {
                // Only treat as delimiter in ExpectDelimiter mode
                if matches!(self.mode, LexerMode::ExpectDelimiter) {
                    self.advance();
                    // Reset mode after consuming delimiter
                    self.mode = LexerMode::ExpectTerm;
                    Some(Token {
                        token_type: TokenType::Operator(Arc::from("#")),
                        text: Arc::from("#"),
                        start,
                        end: self.position,
                    })
                } else {
                    None
                }
            }
            _ => None,
        }
    }
    
    fn parse_double_quoted_string(&mut self, start: usize) -> Option<Token> {
        self.advance(); // Skip opening quote
        let mut parts = Vec::new();
        let mut current_literal = String::new();
        
        while let Some(ch) = self.current_char() {
            match ch {
                '"' => {
                    self.advance();
                    if !current_literal.is_empty() {
                        parts.push(StringPart::Literal(Arc::from(current_literal)));
                    }
                    
                    let text = &self.input[start..self.position];
                    self.mode = LexerMode::ExpectOperator;
                    
                    return Some(Token {
                        token_type: if parts.is_empty() {
                            TokenType::StringLiteral
                        } else {
                            TokenType::InterpolatedString(parts)
                        },
                        text: Arc::from(text),
                        start,
                        end: self.position,
                    });
                }
                '\\' => {
                    self.advance();
                    if let Some(escaped) = self.current_char() {
                        current_literal.push('\\');
                        current_literal.push(escaped);
                        self.advance();
                    }
                }
                '$' if self.config.parse_interpolation => {
                    // Handle variable interpolation
                    if !current_literal.is_empty() {
                        parts.push(StringPart::Literal(Arc::from(current_literal.clone())));
                        current_literal.clear();
                    }
                    
                    // Parse variable - simplified
                    self.advance();
                    let var_start = self.position;
                    while let Some(ch) = self.current_char() {
                        if is_perl_identifier_continue(ch) {
                            self.advance();
                        } else {
                            break;
                        }
                    }
                    
                    if self.position > var_start {
                        let var_name = &self.input[var_start - 1..self.position];
                        parts.push(StringPart::Variable(Arc::from(var_name)));
                    }
                }
                _ => {
                    current_literal.push(ch);
                    self.advance();
                }
            }
        }
        
        // Unterminated string
        None
    }
    
    fn parse_single_quoted_string(&mut self, start: usize) -> Option<Token> {
        self.advance(); // Skip opening quote
        
        while let Some(ch) = self.current_char() {
            match ch {
                '\'' => {
                    self.advance();
                    let text = &self.input[start..self.position];
                    self.mode = LexerMode::ExpectOperator;
                    
                    return Some(Token {
                        token_type: TokenType::StringLiteral,
                        text: Arc::from(text),
                        start,
                        end: self.position,
                    });
                }
                '\\' => {
                    self.advance();
                    if self.current_char() == Some('\'') || self.current_char() == Some('\\') {
                        self.advance();
                    }
                }
                _ => self.advance(),
            }
        }
        
        // Unterminated string
        None
    }
    
    fn parse_backtick_string(&mut self, start: usize) -> Option<Token> {
        self.advance(); // Skip opening backtick
        
        while let Some(ch) = self.current_char() {
            match ch {
                '`' => {
                    self.advance();
                    let text = &self.input[start..self.position];
                    self.mode = LexerMode::ExpectOperator;
                    
                    return Some(Token {
                        token_type: TokenType::QuoteCommand,
                        text: Arc::from(text),
                        start,
                        end: self.position,
                    });
                }
                '\\' => {
                    self.advance();
                    if self.current_char().is_some() {
                        self.advance();
                    }
                }
                _ => self.advance(),
            }
        }
        
        // Unterminated string
        None
    }
    
    fn parse_q_string(&mut self, _start: usize) -> Option<Token> {
        // Simplified q-string parsing
        None
    }
    
    fn parse_substitution(&mut self, start: usize) -> Option<Token> {
        // We've already consumed 's'
        let delimiter = self.current_char()?;
        self.advance(); // Skip delimiter
        
        // Parse pattern
        let mut depth = 1;
        let is_paired = matches!(delimiter, '{' | '[' | '(' | '<');
        let closing = match delimiter {
            '{' => '}',
            '[' => ']',
            '(' => ')',
            '<' => '>',
            _ => delimiter,
        };
        
        while let Some(ch) = self.current_char() {
            match ch {
                '\\' => {
                    self.advance();
                    if self.current_char().is_some() {
                        self.advance();
                    }
                }
                _ if ch == delimiter && is_paired => {
                    depth += 1;
                    self.advance();
                }
                _ if ch == closing => {
                    self.advance();
                    if is_paired {
                        depth -= 1;
                        if depth == 0 {
                            break;
                        }
                    } else {
                        break;
                    }
                }
                _ => self.advance(),
            }
        }
        
        // Parse replacement - same delimiter handling
        if is_paired {
            // Skip whitespace between pattern and replacement for paired delimiters
            while let Some(ch) = self.current_char() {
                if ch.is_whitespace() {
                    self.advance();
                } else {
                    break;
                }
            }
            
            // Expect opening delimiter for replacement
            if self.current_char() == Some(delimiter) {
                self.advance();
                depth = 1;
            }
        }
        
        while let Some(ch) = self.current_char() {
            match ch {
                '\\' => {
                    self.advance();
                    if self.current_char().is_some() {
                        self.advance();
                    }
                }
                _ if ch == delimiter && is_paired => {
                    depth += 1;
                    self.advance();
                }
                _ if ch == closing => {
                    self.advance();
                    if is_paired {
                        depth -= 1;
                        if depth == 0 {
                            break;
                        }
                    } else {
                        break;
                    }
                }
                _ => self.advance(),
            }
        }
        
        // Parse modifiers
        while let Some(ch) = self.current_char() {
            if ch.is_alphabetic() {
                self.advance();
            } else {
                break;
            }
        }
        
        let text = &self.input[start..self.position];
        self.mode = LexerMode::ExpectOperator;
        
        Some(Token {
            token_type: TokenType::Substitution,
            text: Arc::from(text),
            start,
            end: self.position,
        })
    }
    
    fn parse_transliteration(&mut self, start: usize) -> Option<Token> {
        // We've already consumed 'tr' or 'y'
        let delimiter = self.current_char()?;
        self.advance(); // Skip delimiter
        
        // Parse search list
        let mut depth = 1;
        let is_paired = matches!(delimiter, '{' | '[' | '(' | '<');
        let closing = match delimiter {
            '{' => '}',
            '[' => ']',
            '(' => ')',
            '<' => '>',
            _ => delimiter,
        };
        
        while let Some(ch) = self.current_char() {
            match ch {
                '\\' => {
                    self.advance();
                    if self.current_char().is_some() {
                        self.advance();
                    }
                }
                _ if ch == delimiter && is_paired => {
                    depth += 1;
                    self.advance();
                }
                _ if ch == closing => {
                    self.advance();
                    if is_paired {
                        depth -= 1;
                        if depth == 0 {
                            break;
                        }
                    } else {
                        break;
                    }
                }
                _ => self.advance(),
            }
        }
        
        // Parse replacement list - same delimiter handling
        if is_paired {
            // Skip whitespace between search and replace for paired delimiters
            while let Some(ch) = self.current_char() {
                if ch.is_whitespace() {
                    self.advance();
                } else {
                    break;
                }
            }
            
            // Expect opening delimiter for replacement
            if self.current_char() == Some(delimiter) {
                self.advance();
                depth = 1;
            }
        }
        
        while let Some(ch) = self.current_char() {
            match ch {
                '\\' => {
                    self.advance();
                    if self.current_char().is_some() {
                        self.advance();
                    }
                }
                _ if ch == delimiter && is_paired => {
                    depth += 1;
                    self.advance();
                }
                _ if ch == closing => {
                    self.advance();
                    if is_paired {
                        depth -= 1;
                        if depth == 0 {
                            break;
                        }
                    } else {
                        break;
                    }
                }
                _ => self.advance(),
            }
        }
        
        // Parse modifiers
        while let Some(ch) = self.current_char() {
            if ch.is_alphabetic() {
                self.advance();
            } else {
                break;
            }
        }
        
        let text = &self.input[start..self.position];
        self.mode = LexerMode::ExpectOperator;
        
        Some(Token {
            token_type: TokenType::Transliteration,
            text: Arc::from(text),
            start,
            end: self.position,
        })
    }
    
    fn parse_regex(&mut self, start: usize) -> Option<Token> {
        self.advance(); // Skip opening /
        
        while let Some(ch) = self.current_char() {
            match ch {
                '/' => {
                    self.advance();
                    // Parse flags
                    while let Some(ch) = self.current_char() {
                        if ch.is_alphabetic() {
                            self.advance();
                        } else {
                            break;
                        }
                    }
                    
                    let text = &self.input[start..self.position];
                    self.mode = LexerMode::ExpectOperator;
                    
                    return Some(Token {
                        token_type: TokenType::RegexMatch,
                        text: Arc::from(text),
                        start,
                        end: self.position,
                    });
                }
                '\\' => {
                    self.advance();
                    if self.current_char().is_some() {
                        self.advance();
                    }
                }
                _ => self.advance(),
            }
        }
        
        // Unterminated regex
        None
    }
}

/// Perl keywords sorted by length for faster rejection
#[allow(dead_code)]
const KEYWORDS: &[&str] = &[
    // 2 letters
    "if", "do", "my", "or",
    // 3 letters
    "sub", "our", "use", "and", "not", "xor", "die", "say", "for", "try", "END", "cmp",
    // 4 letters
    "else", "when", "next", "last", "redo", "goto", "eval", "warn", "INIT",
    // 5 letters
    "elsif", "while", "until", "local", "state", "given", "break", "print", "catch", "BEGIN", "CHECK", "class", "undef",
    // 6+ letters
    "unless", "return", "require", "package", "default", "foreach", "finally", "continue", "UNITCHECK", "method", "format",
];

#[inline]
fn is_keyword(word: &str) -> bool {
    // Fast length check first
    match word.len() {
        1 => matches!(word, "q" | "m"),
        2 => matches!(word, "if" | "do" | "my" | "or" | "qq" | "qw" | "qr" | "qx" | "tr"),
        3 => matches!(word, "sub" | "our" | "use" | "and" | "not" | "xor" | "die" | "say" | "for" | "try" | "END" | "cmp"),
        4 => matches!(word, "else" | "when" | "next" | "last" | "redo" | "goto" | "eval" | "warn" | "INIT"),
        5 => matches!(word, "elsif" | "while" | "until" | "local" | "state" | "given" | "break" | "print" | "catch" | "BEGIN" | "CHECK" | "class" | "undef"),
        6 => matches!(word, "unless" | "return" | "method" | "format"),
        7 => matches!(word, "require" | "package" | "default" | "foreach" | "finally"),
        8 => word == "continue",
        9 => word == "UNITCHECK",
        _ => false,
    }
}

/// Fast lookup table for compound operator second characters
#[allow(dead_code)]
const COMPOUND_SECOND_CHARS: &[u8] = b"=<>&|+->.~*";

#[inline]
fn is_compound_operator(first: char, second: char) -> bool {
    // Fast path for ASCII
    if first.is_ascii() && second.is_ascii() {
        let first_byte = first as u8;
        let second_byte = second as u8;
        
        match first_byte {
            b'+' => second_byte == b'=' || second_byte == b'+',
            b'-' => second_byte == b'=' || second_byte == b'-' || second_byte == b'>',
            b'*' => second_byte == b'=' || second_byte == b'*',
            b'/' => second_byte == b'=' || second_byte == b'/',
            b'%' | b'^' => second_byte == b'=',
            b'&' => second_byte == b'=' || second_byte == b'&',
            b'|' => second_byte == b'=' || second_byte == b'|',
            b'<' => second_byte == b'=' || second_byte == b'<',
            b'>' => second_byte == b'=' || second_byte == b'>',
            b'=' => second_byte == b'=' || second_byte == b'~' || second_byte == b'>',
            b'!' => second_byte == b'=' || second_byte == b'~',
            b'.' => second_byte == b'.' || second_byte == b'=',
            b'~' => second_byte == b'~',
            b':' => second_byte == b':',
            _ => false,
        }
    } else {
        // Fallback for non-ASCII
        matches!((first, second),
            ('+', '=') | ('-', '=') | ('*', '=') | ('*', '*') | ('/', '=') | ('/', '/') | ('%', '=') |
            ('&', '=') | ('|', '=') | ('^', '=') | ('<', '<') | ('>', '>') |
            ('<', '=') | ('>', '=') | ('=', '=') | ('!', '=') | ('=', '~') |
            ('!', '~') | ('+', '+') | ('-', '-') | ('&', '&') | ('|', '|') |
            ('-', '>') | ('=', '>') | ('.', '.') | ('.', '=') | ('~', '~')
        )
    }
}

#[cfg(test)]
mod test_format_debug;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_tokens() {
        let mut lexer = PerlLexer::new("my $x = 42;");
        
        let token = lexer.next_token().unwrap();
        assert_eq!(token.token_type, TokenType::Keyword(Arc::from("my")));
        
        let token = lexer.next_token().unwrap();
        assert!(matches!(token.token_type, TokenType::Identifier(_)));
        
        let token = lexer.next_token().unwrap();
        assert!(matches!(token.token_type, TokenType::Operator(_)));
        
        let token = lexer.next_token().unwrap();
        assert!(matches!(token.token_type, TokenType::Number(_)));
        
        let token = lexer.next_token().unwrap();
        assert_eq!(token.token_type, TokenType::Semicolon);
    }

    #[test]
    fn test_slash_disambiguation() {
        // Division
        let mut lexer = PerlLexer::new("10 / 2");
        lexer.next_token(); // 10
        let token = lexer.next_token().unwrap();
        assert_eq!(token.token_type, TokenType::Division);
        
        // Regex
        let mut lexer = PerlLexer::new("if (/pattern/)");
        lexer.next_token(); // if
        lexer.next_token(); // (
        let token = lexer.next_token().unwrap();
        assert_eq!(token.token_type, TokenType::RegexMatch);
    }
}