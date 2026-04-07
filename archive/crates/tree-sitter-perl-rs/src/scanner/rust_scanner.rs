//! Rust-native scanner implementation for tree-sitter Perl parser
//!
//! This module provides a high-performance Rust implementation of the Perl scanner
//! that can be used as an alternative to the C scanner implementation.

use crate::error::ParseResult;
use crate::scanner::{PerlScanner, ScannerConfig, ScannerState, TokenType};
use std::collections::HashMap;

/// Rust-native scanner implementation
pub struct RustScanner {
    #[allow(dead_code)]
    config: ScannerConfig,
    state: ScannerState,
    // Add some Rust-specific optimizations
    keyword_cache: HashMap<&'static str, TokenType>,
    #[allow(dead_code)]
    identifier_cache: HashMap<String, bool>,
}

impl RustScanner {
    /// Create a new Rust scanner with default configuration
    pub fn new() -> Self {
        let mut scanner = Self {
            config: ScannerConfig::default(),
            state: ScannerState::default(),
            keyword_cache: HashMap::new(),
            identifier_cache: HashMap::new(),
        };

        // Pre-populate keyword cache for faster lookups
        scanner.init_keyword_cache();

        scanner
    }

    /// Create a new Rust scanner with custom configuration
    pub fn with_config(config: ScannerConfig) -> Self {
        let mut scanner = Self {
            config,
            state: ScannerState::default(),
            keyword_cache: HashMap::new(),
            identifier_cache: HashMap::new(),
        };

        scanner.init_keyword_cache();

        scanner
    }

    fn init_keyword_cache(&mut self) {
        // Populate with Perl keywords for fast lookup
        let keywords = [
            ("package", TokenType::Package),
            ("use", TokenType::Use),
            ("require", TokenType::Require),
            ("sub", TokenType::Sub),
            ("my", TokenType::My),
            ("our", TokenType::Our),
            ("local", TokenType::Local),
            ("return", TokenType::Return),
            ("if", TokenType::If),
            ("unless", TokenType::Unless),
            ("elsif", TokenType::Elsif),
            ("else", TokenType::Else),
            ("while", TokenType::While),
            ("until", TokenType::Until),
            ("for", TokenType::For),
            ("foreach", TokenType::Foreach),
            ("do", TokenType::Do),
            ("last", TokenType::Last),
            ("next", TokenType::Next),
            ("redo", TokenType::Redo),
            ("goto", TokenType::Goto),
            ("die", TokenType::Die),
            ("warn", TokenType::Warn),
            ("print", TokenType::Print),
            ("say", TokenType::Say),
            ("defined", TokenType::Defined),
            ("undef", TokenType::Undef),
            ("blessed", TokenType::Blessed),
            ("ref", TokenType::Ref),
            ("scalar", TokenType::Scalar),
            ("array", TokenType::Array),
            ("hash", TokenType::Hash),
            ("keys", TokenType::Keys),
            ("values", TokenType::Values),
            ("each", TokenType::Each),
            ("delete", TokenType::Delete),
            ("exists", TokenType::Exists),
            ("push", TokenType::Push),
            ("pop", TokenType::Pop),
            ("shift", TokenType::Shift),
            ("unshift", TokenType::Unshift),
            ("splice", TokenType::Splice),
            ("sort", TokenType::Sort),
            ("reverse", TokenType::Reverse),
            ("map", TokenType::Map),
            ("grep", TokenType::Grep),
            ("join", TokenType::Join),
            ("split", TokenType::Split),
            ("length", TokenType::Length),
            ("substr", TokenType::Substr),
            ("index", TokenType::Index),
            ("rindex", TokenType::Rindex),
            ("lc", TokenType::Lc),
            ("uc", TokenType::Uc),
            ("lcfirst", TokenType::Lcfirst),
            ("ucfirst", TokenType::Ucfirst),
            ("chomp", TokenType::Chomp),
            ("chop", TokenType::Chop),
            ("hex", TokenType::Hex),
            ("oct", TokenType::Oct),
            ("ord", TokenType::Ord),
            ("chr", TokenType::Chr),
            ("int", TokenType::Int),
            ("abs", TokenType::Abs),
            ("sqrt", TokenType::Sqrt),
            ("log", TokenType::Log),
            ("exp", TokenType::Exp),
            ("sin", TokenType::Sin),
            ("cos", TokenType::Cos),
            ("tan", TokenType::Tan),
            ("atan2", TokenType::Atan2),
            ("rand", TokenType::Rand),
            ("srand", TokenType::Srand),
            ("time", TokenType::Time),
            ("localtime", TokenType::Localtime),
            ("gmtime", TokenType::Gmtime),
            ("sleep", TokenType::Sleep),
            ("alarm", TokenType::Alarm),
            ("fork", TokenType::Fork),
            ("wait", TokenType::Wait),
            ("waitpid", TokenType::Waitpid),
            ("system", TokenType::System),
            ("exec", TokenType::Exec),
            ("open", TokenType::Open),
            ("close", TokenType::Close),
            ("read", TokenType::Read),
            ("write", TokenType::Write),
            ("seek", TokenType::Seek),
            ("tell", TokenType::Tell),
            ("truncate", TokenType::Truncate),
            ("flock", TokenType::Flock),
            ("link", TokenType::Link),
            ("unlink", TokenType::Unlink),
            ("symlink", TokenType::Symlink),
            ("readlink", TokenType::Readlink),
            ("mkdir", TokenType::Mkdir),
            ("rmdir", TokenType::Rmdir),
            ("chdir", TokenType::Chdir),
            ("chmod", TokenType::Chmod),
            ("chown", TokenType::Chown),
            ("umask", TokenType::Umask),
            ("rename", TokenType::Rename),
            ("stat", TokenType::Stat),
            ("lstat", TokenType::Lstat),
            ("fcntl", TokenType::Fcntl),
            ("ioctl", TokenType::Ioctl),
            ("select", TokenType::Select),
            ("pipe", TokenType::Pipe),
            ("socket", TokenType::Socket),
            ("bind", TokenType::Bind),
            ("listen", TokenType::Listen),
            ("accept", TokenType::Accept),
            ("connect", TokenType::Connect),
            ("shutdown", TokenType::Shutdown),
            ("getsockopt", TokenType::Getsockopt),
            ("setsockopt", TokenType::Setsockopt),
            ("getsockname", TokenType::Getsockname),
            ("getpeername", TokenType::Getpeername),
            ("send", TokenType::Send),
            ("recv", TokenType::Recv),
            ("shmget", TokenType::Shmget),
            ("shmctl", TokenType::Shmctl),
            ("shmread", TokenType::Shmread),
            ("shmwrite", TokenType::Shmwrite),
            ("msgget", TokenType::Msgget),
            ("msgctl", TokenType::Msgctl),
            ("msgsnd", TokenType::Msgsnd),
            ("msgrcv", TokenType::Msgrcv),
            ("semget", TokenType::Semget),
            ("semctl", TokenType::Semctl),
            ("semop", TokenType::Semop),
            ("semclose", TokenType::Semclose),
            ("semremove", TokenType::Semremove),
            ("shmclose", TokenType::Shmclose),
            ("shmremove", TokenType::Shmremove),
            ("msgclose", TokenType::Msgclose),
            ("msgremove", TokenType::Msgremove),
        ];

        for (keyword, token_type) in keywords {
            self.keyword_cache.insert(keyword, token_type);
        }
    }

    /// Scan the next token using Rust-native implementation
    fn scan_rust_token(&mut self, input: &[u8]) -> ParseResult<Option<TokenType>> {
        if input.is_empty() {
            return Ok(None);
        }

        let mut pos = 0;
        let mut current_char = None;

        // Skip whitespace
        while pos < input.len() {
            let c = input[pos] as char;
            if c.is_whitespace() {
                pos += 1;
                self.state.advance(c);
                current_char = Some(c);
            } else {
                current_char = Some(c);
                break;
            }
        }

        if pos >= input.len() {
            return Ok(None);
        }

        let c = current_char.ok_or("unexpected missing character")?;

        // Parse based on current character
        match c {
            '$' => self.scan_variable(&input[pos..]),
            '@' => self.scan_array_variable(&input[pos..]),
            '%' => self.scan_hash_variable(&input[pos..]),
            '#' => self.scan_comment(&input[pos..]),
            '"' | '\'' => self.scan_string(&input[pos..]),
            '0'..='9' => self.scan_number(&input[pos..]),
            'a'..='z' | 'A'..='Z' | '_' => self.scan_identifier(&input[pos..]),
            '+' | '-' | '*' | '/' | '=' | '<' | '>' | '!' | '&' | '|' | '^' | '~' => {
                self.scan_operator(&input[pos..])
            }
            '(' | ')' | '[' | ']' | '{' | '}' | ',' | ';' | ':' | '.' => {
                self.scan_delimiter(&input[pos..])
            }
            _ => Ok(Some(TokenType::Unknown)),
        }
    }

    fn scan_variable(&mut self, _input: &[u8]) -> ParseResult<Option<TokenType>> {
        // Simplified variable scanning
        Ok(Some(TokenType::Variable))
    }

    fn scan_array_variable(&mut self, _input: &[u8]) -> ParseResult<Option<TokenType>> {
        Ok(Some(TokenType::ArrayVariable))
    }

    fn scan_hash_variable(&mut self, _input: &[u8]) -> ParseResult<Option<TokenType>> {
        Ok(Some(TokenType::HashVariable))
    }

    fn scan_comment(&mut self, _input: &[u8]) -> ParseResult<Option<TokenType>> {
        Ok(Some(TokenType::Comment))
    }

    fn scan_string(&mut self, _input: &[u8]) -> ParseResult<Option<TokenType>> {
        Ok(Some(TokenType::String))
    }

    fn scan_number(&mut self, _input: &[u8]) -> ParseResult<Option<TokenType>> {
        Ok(Some(TokenType::Integer))
    }

    fn scan_identifier(&mut self, input: &[u8]) -> ParseResult<Option<TokenType>> {
        // Check if it's a keyword first
        if let Ok(s) = std::str::from_utf8(input) {
            if let Some(token_type) = self.keyword_cache.get(s) {
                return Ok(Some(token_type.clone()));
            }
        }
        Ok(Some(TokenType::Identifier))
    }

    fn scan_operator(&mut self, _input: &[u8]) -> ParseResult<Option<TokenType>> {
        Ok(Some(TokenType::Plus)) // Simplified
    }

    fn scan_delimiter(&mut self, _input: &[u8]) -> ParseResult<Option<TokenType>> {
        Ok(Some(TokenType::LeftParenthesis)) // Simplified
    }
}

impl PerlScanner for RustScanner {
    fn scan(&mut self, input: &[u8]) -> ParseResult<Option<u16>> {
        // Use the Rust-native implementation
        match self.scan_rust_token(input)? {
            Some(token_type) => Ok(Some(token_type as u16)),
            None => Ok(None),
        }
    }

    fn serialize(&self, buffer: &mut Vec<u8>) -> ParseResult<()> {
        // Serialize scanner state
        let serialized = postcard::to_allocvec(&self.state)
            .map_err(|_| crate::error::ParseError::SerializationFailed)?;
        buffer.extend_from_slice(&serialized);
        Ok(())
    }

    fn deserialize(&mut self, buffer: &[u8]) -> ParseResult<()> {
        // Deserialize scanner state
        let decoded: ScannerState = postcard::from_bytes(buffer)
            .map_err(|_| crate::error::ParseError::DeserializationFailed)?;
        self.state = decoded;
        Ok(())
    }

    fn is_eof(&self) -> bool {
        self.state.offset == 0 // Simplified
    }

    fn position(&self) -> (usize, usize) {
        self.state.position()
    }

    fn is_regex_context(&self) -> bool {
        self.state.in_regex
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
    fn test_rust_scanner_creation() {
        let scanner = RustScanner::new();
        assert_eq!(scanner.state.line, 1);
        assert_eq!(scanner.state.column, 1);
    }

    #[test]
    fn test_keyword_cache() {
        let scanner = RustScanner::new();
        assert!(scanner.keyword_cache.contains_key("my"));
        assert!(scanner.keyword_cache.contains_key("sub"));
        assert!(scanner.keyword_cache.contains_key("if"));
    }

    #[test]
    fn test_scan_basic() {
        let mut scanner = RustScanner::new();
        let input = b"my $var = 42;";

        // This is a simplified test - the actual implementation would be more complex
        let result = scanner.scan(input);
        assert!(result.is_ok());
    }
}
