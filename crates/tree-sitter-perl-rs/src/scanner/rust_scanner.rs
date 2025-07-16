//! Rust-native scanner implementation for Perl

use super::{PerlScanner, ScannerConfig, ScannerState, TokenType};
use crate::error::{ParseError, ParseResult};
use crate::unicode::UnicodeUtils;

/// Rust-native Perl scanner implementation
pub struct RustScanner {
    #[allow(dead_code)]
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
    #[allow(dead_code)]
    fn peek_char(&self) -> Option<char> {
        if self.position >= self.input.len() {
            return None;
        }
        char::from_u32(self.input[self.position] as u32)
    }

    /// Peek at the next character without advancing
    fn peek_next_char(&self) -> ParseResult<char> {
        if self.position + 1 >= self.input.len() {
            return Err(ParseError::ParseFailed);
        }

        let next_byte = self.input[self.position + 1];
        if next_byte & 0xC0 == 0x80 {
            // Continuation byte, not a valid UTF-8 start
            return Err(ParseError::ParseFailed);
        }

        let next_slice = &self.input[self.position + 1..];
        let next_str = std::str::from_utf8(next_slice).map_err(|_| ParseError::ParseFailed)?;

        next_str.chars().next().ok_or(ParseError::ParseFailed)
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

    /// Scan a regex pattern
    #[allow(dead_code)]
    fn scan_regex(&mut self) -> ParseResult<TokenType> {
        self.state.in_regex = true;

        // Skip the opening delimiter
        let delimiter = self.peek_char().ok_or(ParseError::UnexpectedEof)?;
        self.advance();

        // Track if we're in character class
        let mut in_char_class = false;
        let mut escaped = false;

        while !self.is_eof() {
            let ch = self.peek_char().ok_or(ParseError::UnexpectedEof)?;

            if escaped {
                escaped = false;
                self.advance();
                continue;
            }

            match ch {
                '\\' => {
                    escaped = true;
                    self.advance();
                }
                '[' => {
                    if !in_char_class {
                        in_char_class = true;
                    }
                    self.advance();
                }
                ']' => {
                    if in_char_class {
                        in_char_class = false;
                    }
                    self.advance();
                }
                '/' | 'm' | 's' | 'y' => {
                    // Check for closing delimiter
                    if !in_char_class && !escaped {
                        // Look ahead to see if this is the closing delimiter
                        let next_ch = self.peek_next_char()?;
                        if next_ch == delimiter {
                            self.advance(); // consume the delimiter
                            self.state.in_regex = false;
                            return Ok(TokenType::Regex);
                        }
                    }
                    self.advance();
                }
                't' => {
                    // Check for 'tr' transliteration
                    if !in_char_class && !escaped {
                        let next_ch = self.peek_next_char()?;
                        if next_ch == 'r' {
                            // This is a transliteration operator, not a regex
                            self.advance(); // consume 't'
                            self.advance(); // consume 'r'
                        // Continue scanning for the transliteration pattern
                        } else if next_ch == delimiter {
                            // This is a regex with 't' as delimiter
                            self.advance(); // consume the delimiter
                            self.state.in_regex = false;
                            return Ok(TokenType::Regex);
                        }
                    }
                    self.advance();
                }
                _ => {
                    self.advance();
                }
            }
        }

        // If we reach EOF without finding closing delimiter, it's an error
        self.state.in_regex = false;
        Err(ParseError::ParseFailed)
    }

    /// Scan a heredoc pattern
    fn scan_heredoc(&mut self) -> ParseResult<TokenType> {
        self.state.in_heredoc = true;

        // Look for << or <<~ followed by delimiter
        let first_ch = self.peek_char().ok_or(ParseError::UnexpectedEof)?;
        if first_ch != '<' {
            return Err(ParseError::ParseFailed);
        }
        self.advance();

        let second_ch = self.peek_char().ok_or(ParseError::UnexpectedEof)?;
        if second_ch != '<' {
            return Err(ParseError::ParseFailed);
        }
        self.advance();

        // Check for indented heredoc (~)
        let third_ch = self.peek_char().ok_or(ParseError::UnexpectedEof)?;
        let is_indented = third_ch == '~';
        if is_indented {
            self.advance();
        }

        // Read delimiter
        let mut delimiter = String::new();
        while !self.is_eof() {
            let ch = self.peek_char().ok_or(ParseError::UnexpectedEof)?;
            if ch.is_whitespace() || ch == ';' {
                break;
            }
            delimiter.push(ch);
            self.advance();
        }

        if delimiter.is_empty() {
            return Err(ParseError::ParseFailed);
        }

        self.state.heredoc_delimiter = Some(delimiter);

        // Skip to end of line
        while !self.is_eof() {
            let ch = self.peek_char().ok_or(ParseError::UnexpectedEof)?;
            if ch == '\n' {
                self.advance();
                break;
            }
            self.advance();
        }

        Ok(TokenType::HereDocument)
    }

    /// Scan POD (Plain Old Documentation)
    fn scan_pod(&mut self) -> ParseResult<TokenType> {
        self.state.in_pod = true;

        // Look for =pod, =head1, =head2, etc.
        let first_ch = self.peek_char().ok_or(ParseError::UnexpectedEof)?;
        if first_ch != '=' {
            return Err(ParseError::ParseFailed);
        }
        self.advance();

        // Read POD command
        let mut command = String::new();
        while !self.is_eof() {
            let ch = self.peek_char().ok_or(ParseError::UnexpectedEof)?;
            if ch.is_whitespace() || ch == '\n' {
                break;
            }
            command.push(ch);
            self.advance();
        }

        // Skip to end of line
        while !self.is_eof() {
            let ch = self.peek_char().ok_or(ParseError::UnexpectedEof)?;
            if ch == '\n' {
                self.advance();
                break;
            }
            self.advance();
        }

        // Check for =cut to end POD
        if command == "cut" {
            self.state.in_pod = false;
            return Ok(TokenType::Pod);
        }

        Ok(TokenType::Pod)
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
                if self.lookahead.is_some() {
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
            '$' => self.scan_variable(),
            '#' => self.scan_comment(),
            '\'' => self.scan_string('\''),
            '"' => self.scan_string('"'),
            '`' => self.scan_string('`'),
            '0'..='9' => self.scan_number(),
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
}

impl PerlScanner for RustScanner {
    fn scan(&mut self, input: &[u8]) -> ParseResult<Option<u16>> {
        self.input = input.to_vec();
        self.position = 0;
        self.lookahead = self.peek_char();

        // Skip whitespace
        self.skip_whitespace();

        if self.is_eof() {
            return Ok(None);
        }

        let token_type = match self.lookahead {
            Some(ch) => match ch {
                '#' => {
                    // Comment
                    self.scan_comment()?
                }
                '=' => {
                    // POD or assignment
                    if self
                        .peek_next_char()
                        .is_ok_and(|next| next.is_ascii_alphabetic())
                    {
                        self.scan_pod()?
                    } else {
                        self.scan_operator()?
                    }
                }
                '<' => {
                    // Heredoc or comparison
                    if self.peek_next_char() == Ok('<') {
                        self.scan_heredoc()?
                    } else {
                        self.scan_operator()?
                    }
                }
                '/' | 'm' | 's' | 'y' | 't' => {
                    // Regex or division
                    if self.state.in_regex || self.is_regex_context() {
                        self.scan_regex()?
                    } else {
                        self.scan_operator()?
                    }
                }
                '$' | '@' | '%' | '&' | '*' => {
                    // Variable
                    self.scan_variable()?
                }
                '\'' | '"' => {
                    // String literal
                    self.scan_string(ch)?
                }
                '0'..='9' => {
                    // Number literal
                    self.scan_number()?
                }
                'a'..='z' | 'A'..='Z' | '_' => {
                    // Identifier or keyword
                    self.scan_identifier()?
                }
                _ => {
                    // Operator or other
                    self.scan_operator()?
                }
            },
            None => TokenType::Eof,
        };

        Ok(Some(token_type as u16))
    }

    fn serialize(&self, buffer: &mut Vec<u8>) -> ParseResult<()> {
        // Serialize scanner state
        let state_bytes = bincode::serialize(&self.state).map_err(|e| {
            ParseError::scanner_error_simple(&format!("Serialization failed: {}", e))
        })?;
        buffer.extend_from_slice(&state_bytes);
        Ok(())
    }

    fn deserialize(&mut self, buffer: &[u8]) -> ParseResult<()> {
        // Deserialize scanner state
        self.state = bincode::deserialize(buffer).map_err(|e| {
            ParseError::scanner_error_simple(&format!("Deserialization failed: {}", e))
        })?;
        Ok(())
    }

    fn is_eof(&self) -> bool {
        self.lookahead.is_none()
    }

    fn position(&self) -> (usize, usize) {
        self.state.position()
    }

    /// Check if we're in a context where a regex is expected
    fn is_regex_context(&self) -> bool {
        // This is a simplified check - in a real implementation,
        // we'd need to track more context about the parsing state
        false
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

        let result = scanner.scan(b"my_variable");
        assert!(result.is_ok());
        let token = result.unwrap();
        assert!(token.is_some());
    }
}
