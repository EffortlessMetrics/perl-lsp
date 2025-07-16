//! Scanner module for tree-sitter Perl parser
//!
//! This module provides the lexical analysis functionality for parsing Perl code.
//! It supports both Rust-native and C scanner implementations through feature flags.

#[cfg(feature = "rust-scanner")]
mod rust_scanner;

#[cfg(feature = "c-scanner")]
mod c_scanner;

#[cfg(feature = "rust-scanner")]
pub use rust_scanner::*;

#[cfg(feature = "c-scanner")]
pub use c_scanner::*;

use crate::error::ParseResult;

/// Scanner trait that defines the interface for lexical analysis
pub trait PerlScanner {
    /// Scan the next token from the input
    fn scan(&mut self, input: &[u8]) -> ParseResult<Option<u16>>;

    /// Serialize the scanner state
    fn serialize(&self, buffer: &mut Vec<u8>) -> ParseResult<()>;

    /// Deserialize the scanner state
    fn deserialize(&mut self, buffer: &[u8]) -> ParseResult<()>;

    /// Check if the scanner is at the end of input
    fn is_eof(&self) -> bool;

    /// Get the current position in the input
    fn position(&self) -> (usize, usize);
}

/// Scanner configuration options
#[derive(Debug, Clone, PartialEq)]
pub struct ScannerConfig {
    /// Enable strict mode for better error reporting
    pub strict_mode: bool,
    /// Enable Unicode normalization
    pub unicode_normalization: bool,
    /// Maximum token length to prevent DoS
    pub max_token_length: usize,
    /// Enable debug logging
    pub debug: bool,
}

impl Default for ScannerConfig {
    fn default() -> Self {
        Self {
            strict_mode: false,
            unicode_normalization: true,
            max_token_length: 1024 * 1024, // 1MB
            debug: false,
        }
    }
}

/// Scanner state for tracking parsing context
#[derive(Debug, Clone, PartialEq)]
pub struct ScannerState {
    /// Current line number (1-based)
    pub line: usize,
    /// Current column number (1-based)
    pub column: usize,
    /// Current byte offset in input
    pub offset: usize,
    /// Whether we're inside a string literal
    pub in_string: bool,
    /// Whether we're inside a regex
    pub in_regex: bool,
    /// Whether we're inside a heredoc
    pub in_heredoc: bool,
    /// Whether we're inside a comment
    pub in_comment: bool,
    /// Whether we're inside POD
    pub in_pod: bool,
    /// Current heredoc delimiter
    pub heredoc_delimiter: Option<String>,
    /// Current string delimiter
    pub string_delimiter: Option<char>,
    /// Current regex delimiter
    pub regex_delimiter: Option<char>,
}

impl Default for ScannerState {
    fn default() -> Self {
        Self {
            line: 1,
            column: 1,
            offset: 0,
            in_string: false,
            in_regex: false,
            in_heredoc: false,
            in_comment: false,
            in_pod: false,
            heredoc_delimiter: None,
            string_delimiter: None,
            regex_delimiter: None,
        }
    }
}

impl ScannerState {
    /// Advance the scanner position by one character
    pub fn advance(&mut self, ch: char) {
        if ch == '\n' {
            self.line += 1;
            self.column = 1;
        } else {
            self.column += 1;
        }
        self.offset += ch.len_utf8();
    }

    /// Advance the scanner position by a byte slice
    pub fn advance_bytes(&mut self, bytes: &[u8]) {
        let s = std::str::from_utf8(bytes).unwrap_or("");
        for ch in s.chars() {
            self.advance(ch);
        }
    }

    /// Get current position as tuple
    pub fn position(&self) -> (usize, usize) {
        (self.line, self.column)
    }

    /// Reset the scanner state
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

/// Token types for Perl lexical analysis
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TokenType {
    // Keywords
    Package,
    Use,
    Require,
    Sub,
    My,
    Our,
    Local,
    Return,
    If,
    Unless,
    Elsif,
    Else,
    While,
    Until,
    For,
    Foreach,
    Do,
    Last,
    Next,
    Redo,
    Goto,
    Die,
    Warn,
    Print,
    Say,
    Defined,
    Undef,
    Blessed,
    Ref,
    Scalar,
    Array,
    Hash,
    Keys,
    Values,
    Each,
    Delete,
    Exists,
    Push,
    Pop,
    Shift,
    Unshift,
    Splice,
    Sort,
    Reverse,
    Map,
    Grep,
    Join,
    Split,
    Length,
    Substr,
    Index,
    Rindex,
    Lc,
    Uc,
    Lcfirst,
    Ucfirst,
    Chomp,
    Chop,
    Hex,
    Oct,
    Ord,
    Chr,
    Int,
    Abs,
    Sqrt,
    Log,
    Exp,
    Sin,
    Cos,
    Tan,
    Atan2,
    Rand,
    Srand,
    Time,
    Localtime,
    Gmtime,
    Sleep,
    Alarm,
    Fork,
    Wait,
    Waitpid,
    System,
    Exec,
    Open,
    Close,
    Read,
    Write,
    Seek,
    Tell,
    Truncate,
    Flock,
    Link,
    Unlink,
    Symlink,
    Readlink,
    Mkdir,
    Rmdir,
    Chdir,
    Chmod,
    Chown,
    Umask,
    Rename,
    Stat,
    Lstat,
    Fcntl,
    Ioctl,
    Select,
    Pipe,
    Socket,
    Bind,
    Listen,
    Accept,
    Connect,
    Shutdown,
    Getsockopt,
    Setsockopt,
    Getsockname,
    Getpeername,
    Send,
    Recv,
    Shmget,
    Shmctl,
    Shmread,
    Shmwrite,
    Msgget,
    Msgctl,
    Msgsnd,
    Msgrcv,
    Semget,
    Semctl,
    Semop,
    Semclose,
    Semremove,
    Shmclose,
    Shmremove,
    Msgclose,
    Msgremove,

    // Operators
    Plus,
    Minus,
    Multiply,
    Divide,
    Modulo,
    Power,
    Assign,
    PlusAssign,
    MinusAssign,
    MultiplyAssign,
    DivideAssign,
    ModuloAssign,
    PowerAssign,
    Increment,
    Decrement,
    Equal,
    NotEqual,
    LessThan,
    GreaterThan,
    LessEqual,
    GreaterEqual,
    StringEqual,
    StringNotEqual,
    StringLessThan,
    StringGreaterThan,
    StringLessEqual,
    StringGreaterEqual,
    LogicalAnd,
    LogicalOr,
    LogicalNot,
    BitwiseAnd,
    BitwiseOr,
    BitwiseXor,
    BitwiseNot,
    LeftShift,
    RightShift,
    Range,
    RangeExclusive,
    Comma,
    FatComma,
    Arrow,
    DoubleArrow,
    Question,
    Colon,
    Semicolon,
    Dot,
    DoubleDot,
    TripleDot,

    // Literals
    Integer,
    Float,
    String,
    SingleQuotedString,
    DoubleQuotedString,
    HereDocument,
    Regex,
    RegexSubstitution,
    RegexTransliteration,

    // Identifiers
    Identifier,
    Variable,
    ArrayVariable,
    HashVariable,
    ScalarVariable,
    PackageVariable,

    // Special tokens
    Comment,
    Pod,
    Data,
    End,
    Error,
    Whitespace,
    Newline,
    Tab,
    CarriageReturn,
    FormFeed,
    VerticalTab,

    // Delimiters
    LeftParenthesis,
    RightParenthesis,
    LeftBracket,
    RightBracket,
    LeftBrace,
    RightBrace,
    LeftAngle,
    RightAngle,

    // Other
    Eof,
    Unknown,
}

impl TokenType {
    /// Get the token name as a string
    pub fn name(&self) -> &'static str {
        match self {
            Self::Package => "package",
            Self::Use => "use",
            Self::Require => "require",
            Self::Sub => "sub",
            Self::My => "my",
            Self::Our => "our",
            Self::Local => "local",
            Self::Return => "return",
            Self::If => "if",
            Self::Unless => "unless",
            Self::Elsif => "elsif",
            Self::Else => "else",
            Self::While => "while",
            Self::Until => "until",
            Self::For => "for",
            Self::Foreach => "foreach",
            Self::Do => "do",
            Self::Last => "last",
            Self::Next => "next",
            Self::Redo => "redo",
            Self::Goto => "goto",
            Self::Die => "die",
            Self::Warn => "warn",
            Self::Print => "print",
            Self::Say => "say",
            Self::Defined => "defined",
            Self::Undef => "undef",
            Self::Blessed => "blessed",
            Self::Ref => "ref",
            Self::Scalar => "scalar",
            Self::Array => "array",
            Self::Hash => "hash",
            Self::Keys => "keys",
            Self::Values => "values",
            Self::Each => "each",
            Self::Delete => "delete",
            Self::Exists => "exists",
            Self::Push => "push",
            Self::Pop => "pop",
            Self::Shift => "shift",
            Self::Unshift => "unshift",
            Self::Splice => "splice",
            Self::Sort => "sort",
            Self::Reverse => "reverse",
            Self::Map => "map",
            Self::Grep => "grep",
            Self::Join => "join",
            Self::Split => "split",
            Self::Length => "length",
            Self::Substr => "substr",
            Self::Index => "index",
            Self::Rindex => "rindex",
            Self::Lc => "lc",
            Self::Uc => "uc",
            Self::Lcfirst => "lcfirst",
            Self::Ucfirst => "ucfirst",
            Self::Chomp => "chomp",
            Self::Chop => "chop",
            Self::Hex => "hex",
            Self::Oct => "oct",
            Self::Ord => "ord",
            Self::Chr => "chr",
            Self::Int => "int",
            Self::Abs => "abs",
            Self::Sqrt => "sqrt",
            Self::Log => "log",
            Self::Exp => "exp",
            Self::Sin => "sin",
            Self::Cos => "cos",
            Self::Tan => "tan",
            Self::Atan2 => "atan2",
            Self::Rand => "rand",
            Self::Srand => "srand",
            Self::Time => "time",
            Self::Localtime => "localtime",
            Self::Gmtime => "gmtime",
            Self::Sleep => "sleep",
            Self::Alarm => "alarm",
            Self::Fork => "fork",
            Self::Wait => "wait",
            Self::Waitpid => "waitpid",
            Self::System => "system",
            Self::Exec => "exec",
            Self::Open => "open",
            Self::Close => "close",
            Self::Read => "read",
            Self::Write => "write",
            Self::Seek => "seek",
            Self::Tell => "tell",
            Self::Truncate => "truncate",
            Self::Flock => "flock",
            Self::Link => "link",
            Self::Unlink => "unlink",
            Self::Symlink => "symlink",
            Self::Readlink => "readlink",
            Self::Mkdir => "mkdir",
            Self::Rmdir => "rmdir",
            Self::Chdir => "chdir",
            Self::Chmod => "chmod",
            Self::Chown => "chown",
            Self::Umask => "umask",
            Self::Rename => "rename",
            Self::Stat => "stat",
            Self::Lstat => "lstat",
            Self::Fcntl => "fcntl",
            Self::Ioctl => "ioctl",
            Self::Select => "select",
            Self::Pipe => "pipe",
            Self::Socket => "socket",
            Self::Bind => "bind",
            Self::Listen => "listen",
            Self::Accept => "accept",
            Self::Connect => "connect",
            Self::Shutdown => "shutdown",
            Self::Getsockopt => "getsockopt",
            Self::Setsockopt => "setsockopt",
            Self::Getsockname => "getsockname",
            Self::Getpeername => "getpeername",
            Self::Send => "send",
            Self::Recv => "recv",
            Self::Shmget => "shmget",
            Self::Shmctl => "shmctl",
            Self::Shmread => "shmread",
            Self::Shmwrite => "shmwrite",
            Self::Msgget => "msgget",
            Self::Msgctl => "msgctl",
            Self::Msgsnd => "msgsnd",
            Self::Msgrcv => "msgrcv",
            Self::Semget => "semget",
            Self::Semctl => "semctl",
            Self::Semop => "semop",
            Self::Semclose => "semclose",
            Self::Semremove => "semremove",
            Self::Shmclose => "shmclose",
            Self::Shmremove => "shmremove",
            Self::Msgclose => "msgclose",
            Self::Msgremove => "msgremove",
            Self::Plus => "+",
            Self::Minus => "-",
            Self::Multiply => "*",
            Self::Divide => "/",
            Self::Modulo => "%",
            Self::Power => "**",
            Self::Assign => "=",
            Self::PlusAssign => "+=",
            Self::MinusAssign => "-=",
            Self::MultiplyAssign => "*=",
            Self::DivideAssign => "/=",
            Self::ModuloAssign => "%=",
            Self::PowerAssign => "**=",
            Self::Increment => "++",
            Self::Decrement => "--",
            Self::Equal => "==",
            Self::NotEqual => "!=",
            Self::LessThan => "<",
            Self::GreaterThan => ">",
            Self::LessEqual => "<=",
            Self::GreaterEqual => ">=",
            Self::StringEqual => "eq",
            Self::StringNotEqual => "ne",
            Self::StringLessThan => "lt",
            Self::StringGreaterThan => "gt",
            Self::StringLessEqual => "le",
            Self::StringGreaterEqual => "ge",
            Self::LogicalAnd => "&&",
            Self::LogicalOr => "||",
            Self::LogicalNot => "!",
            Self::BitwiseAnd => "&",
            Self::BitwiseOr => "|",
            Self::BitwiseXor => "^",
            Self::BitwiseNot => "~",
            Self::LeftShift => "<<",
            Self::RightShift => ">>",
            Self::Range => "..",
            Self::RangeExclusive => "...",
            Self::Comma => ",",
            Self::FatComma => "=>",
            Self::Arrow => "->",
            Self::DoubleArrow => "=>",
            Self::Question => "?",
            Self::Colon => ":",
            Self::Semicolon => ";",
            Self::Dot => ".",
            Self::DoubleDot => "..",
            Self::TripleDot => "...",
            Self::Integer => "integer",
            Self::Float => "float",
            Self::String => "string",
            Self::SingleQuotedString => "single_quoted_string",
            Self::DoubleQuotedString => "double_quoted_string",
            Self::HereDocument => "here_document",
            Self::Regex => "regex",
            Self::RegexSubstitution => "regex_substitution",
            Self::RegexTransliteration => "regex_transliteration",
            Self::Identifier => "identifier",
            Self::Variable => "variable",
            Self::ArrayVariable => "array_variable",
            Self::HashVariable => "hash_variable",
            Self::ScalarVariable => "scalar_variable",
            Self::PackageVariable => "package_variable",
            Self::Comment => "comment",
            Self::Pod => "pod",
            Self::Data => "data",
            Self::End => "end",
            Self::Error => "error",
            Self::Whitespace => "whitespace",
            Self::Newline => "newline",
            Self::Tab => "tab",
            Self::CarriageReturn => "carriage_return",
            Self::FormFeed => "form_feed",
            Self::VerticalTab => "vertical_tab",
            Self::LeftParenthesis => "(",
            Self::RightParenthesis => ")",
            Self::LeftBracket => "[",
            Self::RightBracket => "]",
            Self::LeftBrace => "{",
            Self::RightBrace => "}",
            Self::LeftAngle => "<",
            Self::RightAngle => ">",
            Self::Eof => "EOF",
            Self::Unknown => "unknown",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scanner_state() {
        let mut state = ScannerState::default();
        assert_eq!(state.position(), (1, 1));

        state.advance('a');
        assert_eq!(state.position(), (1, 2));

        state.advance('\n');
        assert_eq!(state.position(), (2, 1));
    }

    #[test]
    fn test_token_type_names() {
        assert_eq!(TokenType::Package.name(), "package");
        assert_eq!(TokenType::Plus.name(), "+");
        assert_eq!(TokenType::Integer.name(), "integer");
    }
}
