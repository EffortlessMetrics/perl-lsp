//! Rust-native scanner implementation for Perl

use super::{PerlScanner, ScannerConfig, ScannerState, TokenType};
use crate::error::{ParseError, ParseResult};
use crate::unicode::UnicodeUtils;

/// Rust-native Perl scanner implementation
pub struct RustScanner {
    config: ScannerConfig,
    state: ScannerState,
    input: Vec<u8>,
    position: usize,
    lookahead: Option<char>,
}

impl RustScanner {
    /// Create a new Rust scanner with default configuration
    pub fn new() -> Self {
        Self::with_config(ScannerConfig::default())
    }

    /// Create a new Rust scanner with custom configuration
    pub fn with_config(config: ScannerConfig) -> Self {
        Self {
            config,
            state: ScannerState::default(),
            input: Vec::new(),
            position: 0,
            lookahead: None,
        }
    }

    /// Set the input source for scanning
    pub fn set_input(&mut self, input: &[u8]) {
        self.input = input.to_vec();
        self.position = 0;
        self.state.reset();
        self.lookahead = self.next_char();
    }

    /// Get the next character from input
    fn next_char(&mut self) -> Option<char> {
        if self.position >= self.input.len() {
            return None;
        }

        let ch = char::from_u32(self.input[self.position] as u32)?;
        self.position += ch.len_utf8();
        Some(ch)
    }

    /// Peek at the next character without consuming it
    fn peek_char(&self) -> Option<char> {
        if self.position >= self.input.len() {
            return None;
        }
        char::from_u32(self.input[self.position] as u32)
    }

    /// Advance the scanner by one character
    fn advance(&mut self) {
        if let Some(ch) = self.lookahead {
            self.state.advance(ch);
            self.lookahead = self.next_char();
        }
    }

    /// Skip whitespace characters
    fn skip_whitespace(&mut self) {
        while let Some(ch) = self.lookahead {
            if UnicodeUtils::is_unicode_whitespace(ch) {
                self.advance();
            } else {
                break;
            }
        }
    }

    /// Scan a comment
    fn scan_comment(&mut self) -> ParseResult<TokenType> {
        self.state.in_comment = true;

        // Skip the # character
        self.advance();

        // Consume until newline or EOF
        while let Some(ch) = self.lookahead {
            if ch == '\n' {
                break;
            }
            self.advance();
        }

        self.state.in_comment = false;
        Ok(TokenType::Comment)
    }

    /// Scan POD (Plain Old Documentation)
    fn scan_pod(&mut self) -> ParseResult<TokenType> {
        self.state.in_pod = true;

        // Skip the = character
        self.advance();

        // Consume until =cut or EOF
        while let Some(ch) = self.lookahead {
            if ch == '=' {
                // Check for =cut
                let mut temp_pos = self.position;
                let mut temp_input = self.input.clone();
                if temp_pos + 3 < temp_input.len() {
                    let cut_bytes = &temp_input[temp_pos..temp_pos + 4];
                    if let Ok(cut_str) = std::str::from_utf8(cut_bytes) {
                        if cut_str == "=cut" {
                            // Skip =cut
                            for _ in 0..4 {
                                self.advance();
                            }
                            self.state.in_pod = false;
                            return Ok(TokenType::Pod);
                        }
                    }
                }
            }
            self.advance();
        }

        // Unterminated POD
        self.state.in_pod = false;
        Err(ParseError::unterminated_string(self.state.position()))
    }

    /// Scan a string literal
    fn scan_string(&mut self, delimiter: char) -> ParseResult<TokenType> {
        self.state.in_string = true;
        self.state.string_delimiter = Some(delimiter);

        // Skip the opening delimiter
        self.advance();

        while let Some(ch) = self.lookahead {
            if ch == delimiter {
                self.advance();
                self.state.in_string = false;
                self.state.string_delimiter = None;
                return Ok(if delimiter == '\'' {
                    TokenType::SingleQuotedString
                } else {
                    TokenType::DoubleQuotedString
                });
            } else if ch == '\\' {
                self.advance();
                // Skip the escaped character
                if let Some(_) = self.lookahead {
                    self.advance();
                }
            } else {
                self.advance();
            }
        }

        // Unterminated string
        self.state.in_string = false;
        self.state.string_delimiter = None;
        Err(ParseError::unterminated_string(self.state.position()))
    }

    /// Scan a here document
    fn scan_heredoc(&mut self) -> ParseResult<TokenType> {
        self.state.in_heredoc = true;

        // Skip the <<
        self.advance();
        self.advance();

        // Skip whitespace
        self.skip_whitespace();

        // Read delimiter
        let mut delimiter = String::new();
        while let Some(ch) = self.lookahead {
            if ch == '\n' || UnicodeUtils::is_unicode_whitespace(ch) {
                break;
            }
            delimiter.push(ch);
            self.advance();
        }

        self.state.heredoc_delimiter = Some(delimiter.clone());

        // Skip to newline
        while let Some(ch) = self.lookahead {
            if ch == '\n' {
                self.advance();
                break;
            }
            self.advance();
        }

        // Consume content until delimiter
        while let Some(ch) = self.lookahead {
            if ch == '\n' {
                // Check if next line starts with delimiter
                let mut temp_pos = self.position;
                let mut temp_input = self.input.clone();
                if temp_pos + delimiter.len() < temp_input.len() {
                    let delim_bytes = &temp_input[temp_pos..temp_pos + delimiter.len()];
                    if let Ok(delim_str) = std::str::from_utf8(delim_bytes) {
                        if delim_str == delimiter {
                            // Skip delimiter
                            for _ in 0..delimiter.len() {
                                self.advance();
                            }
                            self.state.in_heredoc = false;
                            self.state.heredoc_delimiter = None;
                            return Ok(TokenType::HereDocument);
                        }
                    }
                }
            }
            self.advance();
        }

        // Unterminated heredoc
        self.state.in_heredoc = false;
        self.state.heredoc_delimiter = None;
        Err(ParseError::unterminated_string(self.state.position()))
    }

    /// Scan a regex pattern
    fn scan_regex(&mut self) -> ParseResult<TokenType> {
        self.state.in_regex = true;

        // Skip the opening delimiter
        let delimiter = self.lookahead.unwrap_or('/');
        self.advance();

        while let Some(ch) = self.lookahead {
            if ch == delimiter {
                self.advance();
                self.state.in_regex = false;
                return Ok(TokenType::Regex);
            } else if ch == '\\' {
                self.advance();
                // Skip the escaped character
                if let Some(_) = self.lookahead {
                    self.advance();
                }
            } else {
                self.advance();
            }
        }

        // Unterminated regex
        self.state.in_regex = false;
        Err(ParseError::unterminated_string(self.state.position()))
    }

    /// Scan a variable
    fn scan_variable(&mut self) -> ParseResult<TokenType> {
        // Skip the $ character
        self.advance();

        if let Some(ch) = self.lookahead {
            match ch {
                '@' => {
                    self.advance();
                    Ok(TokenType::ArrayVariable)
                }
                '%' => {
                    self.advance();
                    Ok(TokenType::HashVariable)
                }
                '$' => {
                    self.advance();
                    Ok(TokenType::ScalarVariable)
                }
                '&' => {
                    self.advance();
                    Ok(TokenType::Variable)
                }
                '*' => {
                    self.advance();
                    Ok(TokenType::Variable)
                }
                _ => {
                    // Scan identifier part
                    if UnicodeUtils::is_identifier_start(ch) || ch.is_ascii_digit() {
                        while let Some(next_ch) = self.lookahead {
                            if UnicodeUtils::is_identifier_continue(next_ch) {
                                self.advance();
                            } else {
                                break;
                            }
                        }
                    }
                    Ok(TokenType::Variable)
                }
            }
        } else {
            Ok(TokenType::Variable)
        }
    }

    /// Scan an identifier or keyword
    fn scan_identifier(&mut self) -> ParseResult<TokenType> {
        let mut identifier = String::new();

        // First character must be identifier start
        if let Some(ch) = self.lookahead {
            if UnicodeUtils::is_identifier_start(ch) {
                identifier.push(ch);
                self.advance();
            } else {
                return Err(ParseError::invalid_token(
                    ch.to_string(),
                    self.state.position(),
                ));
            }
        }

        // Continue with identifier continue characters
        while let Some(ch) = self.lookahead {
            if UnicodeUtils::is_identifier_continue(ch) {
                identifier.push(ch);
                self.advance();
            } else {
                break;
            }
        }

        // Check if it's a keyword
        match identifier.as_str() {
            "package" => Ok(TokenType::Package),
            "use" => Ok(TokenType::Use),
            "require" => Ok(TokenType::Require),
            "sub" => Ok(TokenType::Sub),
            "my" => Ok(TokenType::My),
            "our" => Ok(TokenType::Our),
            "local" => Ok(TokenType::Local),
            "return" => Ok(TokenType::Return),
            "if" => Ok(TokenType::If),
            "unless" => Ok(TokenType::Unless),
            "elsif" => Ok(TokenType::Elsif),
            "else" => Ok(TokenType::Else),
            "while" => Ok(TokenType::While),
            "until" => Ok(TokenType::Until),
            "for" => Ok(TokenType::For),
            "foreach" => Ok(TokenType::Foreach),
            "do" => Ok(TokenType::Do),
            "last" => Ok(TokenType::Last),
            "next" => Ok(TokenType::Next),
            "redo" => Ok(TokenType::Redo),
            "goto" => Ok(TokenType::Goto),
            "die" => Ok(TokenType::Die),
            "warn" => Ok(TokenType::Warn),
            "print" => Ok(TokenType::Print),
            "say" => Ok(TokenType::Say),
            "defined" => Ok(TokenType::Defined),
            "undef" => Ok(TokenType::Undef),
            "blessed" => Ok(TokenType::Blessed),
            "ref" => Ok(TokenType::Ref),
            "scalar" => Ok(TokenType::Scalar),
            "array" => Ok(TokenType::Array),
            "hash" => Ok(TokenType::Hash),
            "keys" => Ok(TokenType::Keys),
            "values" => Ok(TokenType::Values),
            "each" => Ok(TokenType::Each),
            "delete" => Ok(TokenType::Delete),
            "exists" => Ok(TokenType::Exists),
            "push" => Ok(TokenType::Push),
            "pop" => Ok(TokenType::Pop),
            "shift" => Ok(TokenType::Shift),
            "unshift" => Ok(TokenType::Unshift),
            "splice" => Ok(TokenType::Splice),
            "sort" => Ok(TokenType::Sort),
            "reverse" => Ok(TokenType::Reverse),
            "map" => Ok(TokenType::Map),
            "grep" => Ok(TokenType::Grep),
            "join" => Ok(TokenType::Join),
            "split" => Ok(TokenType::Split),
            "length" => Ok(TokenType::Length),
            "substr" => Ok(TokenType::Substr),
            "index" => Ok(TokenType::Index),
            "rindex" => Ok(TokenType::Rindex),
            "lc" => Ok(TokenType::Lc),
            "uc" => Ok(TokenType::Uc),
            "lcfirst" => Ok(TokenType::Lcfirst),
            "ucfirst" => Ok(TokenType::Ucfirst),
            "chomp" => Ok(TokenType::Chomp),
            "chop" => Ok(TokenType::Chop),
            "hex" => Ok(TokenType::Hex),
            "oct" => Ok(TokenType::Oct),
            "ord" => Ok(TokenType::Ord),
            "chr" => Ok(TokenType::Chr),
            "int" => Ok(TokenType::Int),
            "abs" => Ok(TokenType::Abs),
            "sqrt" => Ok(TokenType::Sqrt),
            "log" => Ok(TokenType::Log),
            "exp" => Ok(TokenType::Exp),
            "sin" => Ok(TokenType::Sin),
            "cos" => Ok(TokenType::Cos),
            "tan" => Ok(TokenType::Tan),
            "atan2" => Ok(TokenType::Atan2),
            "rand" => Ok(TokenType::Rand),
            "srand" => Ok(TokenType::Srand),
            "time" => Ok(TokenType::Time),
            "localtime" => Ok(TokenType::Localtime),
            "gmtime" => Ok(TokenType::Gmtime),
            "sleep" => Ok(TokenType::Sleep),
            "alarm" => Ok(TokenType::Alarm),
            "fork" => Ok(TokenType::Fork),
            "wait" => Ok(TokenType::Wait),
            "waitpid" => Ok(TokenType::Waitpid),
            "system" => Ok(TokenType::System),
            "exec" => Ok(TokenType::Exec),
            "open" => Ok(TokenType::Open),
            "close" => Ok(TokenType::Close),
            "read" => Ok(TokenType::Read),
            "write" => Ok(TokenType::Write),
            "seek" => Ok(TokenType::Seek),
            "tell" => Ok(TokenType::Tell),
            "truncate" => Ok(TokenType::Truncate),
            "flock" => Ok(TokenType::Flock),
            "link" => Ok(TokenType::Link),
            "unlink" => Ok(TokenType::Unlink),
            "symlink" => Ok(TokenType::Symlink),
            "readlink" => Ok(TokenType::Readlink),
            "mkdir" => Ok(TokenType::Mkdir),
            "rmdir" => Ok(TokenType::Rmdir),
            "chdir" => Ok(TokenType::Chdir),
            "chmod" => Ok(TokenType::Chmod),
            "chown" => Ok(TokenType::Chown),
            "umask" => Ok(TokenType::Umask),
            "rename" => Ok(TokenType::Rename),
            "stat" => Ok(TokenType::Stat),
            "lstat" => Ok(TokenType::Lstat),
            "fcntl" => Ok(TokenType::Fcntl),
            "ioctl" => Ok(TokenType::Ioctl),
            "select" => Ok(TokenType::Select),
            "pipe" => Ok(TokenType::Pipe),
            "socket" => Ok(TokenType::Socket),
            "listen" => Ok(TokenType::Listen),
            "accept" => Ok(TokenType::Accept),
            "connect" => Ok(TokenType::Connect),
            "shutdown" => Ok(TokenType::Shutdown),
            "getsockopt" => Ok(TokenType::Getsockopt),
            "setsockopt" => Ok(TokenType::Setsockopt),
            "getsockname" => Ok(TokenType::Getsockname),
            "getpeername" => Ok(TokenType::Getpeername),
            "send" => Ok(TokenType::Send),
            "recv" => Ok(TokenType::Recv),
            "shmget" => Ok(TokenType::Shmget),
            "shmctl" => Ok(TokenType::Shmctl),
            "shmread" => Ok(TokenType::Shmread),
            "shmwrite" => Ok(TokenType::Shmwrite),
            "msgget" => Ok(TokenType::Msgget),
            "msgctl" => Ok(TokenType::Msgctl),
            "msgsnd" => Ok(TokenType::Msgsnd),
            "msgrcv" => Ok(TokenType::Msgrcv),
            "semget" => Ok(TokenType::Semget),
            "semctl" => Ok(TokenType::Semctl),
            "semop" => Ok(TokenType::Semop),
            "semclose" => Ok(TokenType::Semclose),
            "semremove" => Ok(TokenType::Semremove),
            "shmclose" => Ok(TokenType::Shmclose),
            "shmremove" => Ok(TokenType::Shmremove),
            "msgclose" => Ok(TokenType::Msgclose),
            "msgremove" => Ok(TokenType::Msgremove),
            _ => Ok(TokenType::Identifier),
        }
    }

    /// Scan a number literal
    fn scan_number(&mut self) -> ParseResult<TokenType> {
        let mut has_decimal = false;
        let mut has_exponent = false;

        // First digit
        if let Some(ch) = self.lookahead {
            if ch.is_ascii_digit() {
                self.advance();
            } else {
                return Err(ParseError::invalid_token(
                    ch.to_string(),
                    self.state.position(),
                ));
            }
        }

        // Continue scanning digits, decimal points, and exponents
        while let Some(ch) = self.lookahead {
            match ch {
                '0'..='9' => {
                    self.advance();
                }
                '.' if !has_decimal && !has_exponent => {
                    has_decimal = true;
                    self.advance();
                }
                'e' | 'E' if !has_exponent => {
                    has_exponent = true;
                    self.advance();

                    // Optional sign after exponent
                    if let Some(sign) = self.lookahead {
                        if sign == '+' || sign == '-' {
                            self.advance();
                        }
                    }
                }
                _ => break,
            }
        }

        Ok(if has_decimal || has_exponent {
            TokenType::Float
        } else {
            TokenType::Integer
        })
    }

    /// Scan operators and punctuation
    fn scan_operator(&mut self) -> ParseResult<TokenType> {
        let ch = self.lookahead.unwrap_or('\0');
        
        match ch {
            '+' => {
                self.advance();
                if let Some(next) = self.lookahead {
                    if next == '=' {
                    self.advance();
                        Ok(TokenType::PlusAssign)
                    } else if next == '+' {
                    self.advance();
                        Ok(TokenType::Increment)
                    } else {
                        Ok(TokenType::Plus)
                    }
                } else {
                    Ok(TokenType::Plus)
                }
            }
            '-' => {
                self.advance();
                if let Some(next) = self.lookahead {
                    if next == '=' {
                    self.advance();
                        Ok(TokenType::MinusAssign)
                    } else if next == '-' {
                    self.advance();
                        Ok(TokenType::Decrement)
                    } else {
                        Ok(TokenType::Minus)
                    }
                } else {
                    Ok(TokenType::Minus)
                }
            }
            '*' => {
                self.advance();
                if let Some(next) = self.lookahead {
                    if next == '=' {
                    self.advance();
                        Ok(TokenType::MultiplyAssign)
                    } else if next == '*' {
                        self.advance();
                        Ok(TokenType::Power)
                    } else {
                        Ok(TokenType::Multiply)
                    }
                } else {
                    Ok(TokenType::Multiply)
                }
            }
            '/' => {
                self.advance();
                if let Some(next) = self.lookahead {
                    if next == '=' {
                    self.advance();
                        Ok(TokenType::DivideAssign)
                    } else {
                        Ok(TokenType::Divide)
                    }
                } else {
                    Ok(TokenType::Divide)
                }
            }
            '%' => {
                self.advance();
                if let Some(next) = self.lookahead {
                    if next == '=' {
                    self.advance();
                        Ok(TokenType::ModuloAssign)
                    } else {
                        Ok(TokenType::Modulo)
                    }
                } else {
                    Ok(TokenType::Modulo)
                }
            }
            '=' => {
                self.advance();
                if let Some(next) = self.lookahead {
                    if next == '=' {
                        self.advance();
                        Ok(TokenType::Equal)
                    } else if next == '~' {
                        self.advance();
                        Ok(TokenType::StringEqual)
                    } else if next == '>' {
                    self.advance();
                        Ok(TokenType::DoubleArrow)
                    } else {
                        Ok(TokenType::Assign)
                    }
                } else {
                    Ok(TokenType::Assign)
                }
            }
            '!' => {
                self.advance();
                if let Some(next) = self.lookahead {
                    if next == '=' {
                    self.advance();
                        Ok(TokenType::NotEqual)
                    } else {
                        Ok(TokenType::LogicalNot)
                    }
                } else {
                    Ok(TokenType::LogicalNot)
                }
            }
            '<' => {
                self.advance();
                if let Some(next) = self.lookahead {
                    if next == '=' {
                        self.advance();
                        Ok(TokenType::LessEqual)
                    } else if next == '<' {
                        self.advance();
                        Ok(TokenType::LeftShift)
                    } else if next == '>' {
                    self.advance();
                        Ok(TokenType::NotEqual)
                    } else {
                        Ok(TokenType::LessThan)
                    }
                } else {
                    Ok(TokenType::LessThan)
                }
            }
            '>' => {
                self.advance();
                if let Some(next) = self.lookahead {
                    if next == '=' {
                    self.advance();
                        Ok(TokenType::GreaterEqual)
                    } else if next == '>' {
                    self.advance();
                        Ok(TokenType::RightShift)
                    } else {
                        Ok(TokenType::GreaterThan)
                    }
                } else {
                    Ok(TokenType::GreaterThan)
                }
            }
            '&' => {
                self.advance();
                if let Some(next) = self.lookahead {
                    if next == '&' {
                    self.advance();
                        Ok(TokenType::LogicalAnd)
                    } else {
                        Ok(TokenType::BitwiseAnd)
                    }
                } else {
                    Ok(TokenType::BitwiseAnd)
                }
            }
            '|' => {
                self.advance();
                if let Some(next) = self.lookahead {
                    if next == '|' {
                    self.advance();
                        Ok(TokenType::LogicalOr)
                    } else {
                        Ok(TokenType::BitwiseOr)
                    }
                } else {
                    Ok(TokenType::BitwiseOr)
                }
            }
            '^' => {
                self.advance();
                if let Some(next) = self.lookahead {
                    if next == '^' {
                    self.advance();
                        Ok(TokenType::LogicalOr) // xor operator
                    } else {
                        Ok(TokenType::BitwiseXor)
                    }
                } else {
                    Ok(TokenType::BitwiseXor)
                }
            }
            '~' => {
                self.advance();
                Ok(TokenType::BitwiseNot)
            }
            '?' => {
                self.advance();
                Ok(TokenType::Question)
            }
            ':' => {
                self.advance();
                Ok(TokenType::Colon)
            }
            ';' => {
                self.advance();
                Ok(TokenType::Semicolon)
            }
            ',' => {
                self.advance();
                Ok(TokenType::Comma)
            }
            '.' => {
                    self.advance();
                if let Some(next) = self.lookahead {
                    if next == '.' {
                        self.advance();
                        if let Some(next_next) = self.lookahead {
                            if next_next == '.' {
                                self.advance();
                                Ok(TokenType::TripleDot)
                            } else {
                                Ok(TokenType::DoubleDot)
                            }
                        } else {
                            Ok(TokenType::DoubleDot)
                        }
                    } else {
                        Ok(TokenType::Dot)
                    }
                } else {
                    Ok(TokenType::Dot)
                }
            }
            '(' => {
                self.advance();
                Ok(TokenType::LeftParenthesis)
            }
            ')' => {
                self.advance();
                Ok(TokenType::RightParenthesis)
            }
            '[' => {
                self.advance();
                Ok(TokenType::LeftBracket)
            }
            ']' => {
                self.advance();
                Ok(TokenType::RightBracket)
            }
            '{' => {
                self.advance();
                Ok(TokenType::LeftBrace)
            }
            '}' => {
                self.advance();
                Ok(TokenType::RightBrace)
            }
            '=' => {
                self.advance();
                Ok(TokenType::Assign)
            }
            '$' => {
                self.scan_variable()
            }
            '#' => {
                self.scan_comment()
            }
            '\'' => {
                self.scan_string('\'')
            }
            '"' => {
                self.scan_string('"')
            }
            '`' => {
                self.scan_string('`')
            }
            '/' => {
                self.scan_regex()
            }
            '0'..='9' => {
                self.scan_number()
            }
            _ => {
                if UnicodeUtils::is_identifier_start(ch) {
                    self.scan_identifier()
                } else {
                    Err(ParseError::invalid_token(
                    ch.to_string(),
                    self.state.position(),
                    ))
                }
            }
        }
    }
}

impl PerlScanner for RustScanner {
    fn scan(&mut self, input: &[u8]) -> ParseResult<Option<u16>> {
        // Set input if provided
        if !input.is_empty() {
            self.set_input(input);
        }

        // Skip whitespace
        self.skip_whitespace();

        // Check for EOF
        if self.lookahead.is_none() {
            return Ok(None);
        }

        // Scan next token
        let token_type = self.scan_operator()?;

        // Convert token type to u16 (tree-sitter token ID)
        let token_id = match token_type {
            TokenType::Package => 1,
            TokenType::Use => 2,
            TokenType::Require => 3,
            TokenType::Sub => 4,
            TokenType::My => 5,
            TokenType::Our => 6,
            TokenType::Local => 7,
            TokenType::Return => 8,
            TokenType::If => 9,
            TokenType::Unless => 10,
            TokenType::Elsif => 11,
            TokenType::Else => 12,
            TokenType::While => 13,
            TokenType::Until => 14,
            TokenType::For => 15,
            TokenType::Foreach => 16,
            TokenType::Do => 17,
            TokenType::Last => 18,
            TokenType::Next => 19,
            TokenType::Redo => 20,
            TokenType::Goto => 21,
            TokenType::Die => 22,
            TokenType::Warn => 23,
            TokenType::Print => 24,
            TokenType::Say => 25,
            TokenType::Defined => 26,
            TokenType::Undef => 27,
            TokenType::Identifier => 28,
            TokenType::Variable => 29,
            TokenType::ArrayVariable => 30,
            TokenType::HashVariable => 31,
            TokenType::ScalarVariable => 32,
            TokenType::Integer => 33,
            TokenType::Float => 34,
            TokenType::String => 35,
            TokenType::SingleQuotedString => 36,
            TokenType::DoubleQuotedString => 37,
            TokenType::HereDocument => 38,
            TokenType::Regex => 39,
            TokenType::Comment => 40,
            TokenType::Pod => 41,
            TokenType::Plus => 42,
            TokenType::Minus => 43,
            TokenType::Multiply => 44,
            TokenType::Divide => 45,
            TokenType::Modulo => 46,
            TokenType::Power => 47,
            TokenType::Assign => 48,
            TokenType::Equal => 49,
            TokenType::NotEqual => 50,
            TokenType::LessThan => 51,
            TokenType::GreaterThan => 52,
            TokenType::LessEqual => 53,
            TokenType::GreaterEqual => 54,
            TokenType::LogicalAnd => 55,
            TokenType::LogicalOr => 56,
            TokenType::LogicalNot => 57,
            TokenType::BitwiseAnd => 58,
            TokenType::BitwiseOr => 59,
            TokenType::BitwiseXor => 60,
            TokenType::BitwiseNot => 61,
            TokenType::LeftShift => 62,
            TokenType::RightShift => 63,
            TokenType::Increment => 64,
            TokenType::Decrement => 65,
            TokenType::PlusAssign => 66,
            TokenType::MinusAssign => 67,
            TokenType::MultiplyAssign => 68,
            TokenType::DivideAssign => 69,
            TokenType::ModuloAssign => 70,
            TokenType::PowerAssign => 71,
            TokenType::StringEqual => 72,
            TokenType::StringNotEqual => 73,
            TokenType::StringLessThan => 74,
            TokenType::StringGreaterThan => 75,
            TokenType::StringLessEqual => 76,
            TokenType::StringGreaterEqual => 77,
            TokenType::Range => 78,
            TokenType::RangeExclusive => 79,
            TokenType::Comma => 80,
            TokenType::FatComma => 81,
            TokenType::Arrow => 82,
            TokenType::DoubleArrow => 83,
            TokenType::Question => 84,
            TokenType::Colon => 85,
            TokenType::Semicolon => 86,
            TokenType::Dot => 87,
            TokenType::DoubleDot => 88,
            TokenType::TripleDot => 89,
            TokenType::LeftParenthesis => 90,
            TokenType::RightParenthesis => 91,
            TokenType::LeftBracket => 92,
            TokenType::RightBracket => 93,
            TokenType::LeftBrace => 94,
            TokenType::RightBrace => 95,
            TokenType::LeftAngle => 96,
            TokenType::RightAngle => 97,
            TokenType::Whitespace => 98,
            TokenType::Newline => 99,
            TokenType::Tab => 100,
            TokenType::CarriageReturn => 101,
            TokenType::FormFeed => 102,
            TokenType::VerticalTab => 103,
            TokenType::Eof => 104,
            TokenType::Error => 105,
            TokenType::Unknown => 106,
            // Add more mappings as needed
            _ => 106, // Unknown
        };

        Ok(Some(token_id))
    }

    fn serialize(&self, buffer: &mut Vec<u8>) -> ParseResult<()> {
        // Serialize scanner state
        let state_bytes = bincode::serialize(&self.state)
            .map_err(|e| ParseError::scanner_error_simple(&format!("Serialization failed: {}", e)))?;
        buffer.extend_from_slice(&state_bytes);
        Ok(())
    }

    fn deserialize(&mut self, buffer: &[u8]) -> ParseResult<()> {
        // Deserialize scanner state
        self.state = bincode::deserialize(buffer)
            .map_err(|e| ParseError::scanner_error_simple(&format!("Deserialization failed: {}", e)))?;
        Ok(())
    }

    fn is_eof(&self) -> bool {
        self.lookahead.is_none()
    }

    fn position(&self) -> (usize, usize) {
        self.state.position()
    }
}

impl Default for RustScanner {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scanner_creation() {
        let scanner = RustScanner::new();
        assert_eq!(scanner.state.line, 1);
        assert_eq!(scanner.state.column, 1);
    }

    #[test]
    fn test_identifier_scanning() {
        let mut scanner = RustScanner::new();
        scanner.set_input(b"my_variable");
        
        let result = scanner.scan(b"");
        assert!(result.is_ok());
        let token = result.unwrap();
        assert!(token.is_some());
    }
}
