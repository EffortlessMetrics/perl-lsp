//! Scanner module (legacy) - Used only for C parser benchmarking
//!
//! Note: The Pure Rust Pest parser does not use this scanner module.
//! This is retained only for benchmarking comparisons with the legacy C implementation.

#[cfg(feature = "rust-scanner")]
mod rust_scanner;

#[cfg(feature = "c-scanner")]
mod c_scanner;

#[cfg(feature = "rust-scanner")]
pub use rust_scanner::*;

#[cfg(feature = "c-scanner")]
pub use c_scanner::*;

use crate::error::ParseResult;
use serde::{Deserialize, Serialize};

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

    /// Check if we're in a regex context
    fn is_regex_context(&self) -> bool;
}

/// Scanner configuration options
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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

    // Compatibility aliases for test suite
    StringLiteral = 1000, // Alias for String
    NumberLiteral = 1001, // Alias for Integer
    Operator = 1002,      // Alias for Plus
    Keyword = 1003,       // Alias for My
}

impl TokenType {
    /// Get the name of the token type
    pub fn name(&self) -> &'static str {
        match self {
            TokenType::Package => "package",
            TokenType::Use => "use",
            TokenType::Require => "require",
            TokenType::Sub => "sub",
            TokenType::My => "my",
            TokenType::Our => "our",
            TokenType::Local => "local",
            TokenType::Return => "return",
            TokenType::If => "if",
            TokenType::Unless => "unless",
            TokenType::Elsif => "elsif",
            TokenType::Else => "else",
            TokenType::While => "while",
            TokenType::Until => "until",
            TokenType::For => "for",
            TokenType::Foreach => "foreach",
            TokenType::Do => "do",
            TokenType::Last => "last",
            TokenType::Next => "next",
            TokenType::Redo => "redo",
            TokenType::Goto => "goto",
            TokenType::Die => "die",
            TokenType::Warn => "warn",
            TokenType::Print => "print",
            TokenType::Say => "say",
            TokenType::Defined => "defined",
            TokenType::Undef => "undef",
            TokenType::Blessed => "blessed",
            TokenType::Ref => "ref",
            TokenType::Scalar => "scalar",
            TokenType::Array => "array",
            TokenType::Hash => "hash",
            TokenType::Keys => "keys",
            TokenType::Values => "values",
            TokenType::Each => "each",
            TokenType::Delete => "delete",
            TokenType::Exists => "exists",
            TokenType::Push => "push",
            TokenType::Pop => "pop",
            TokenType::Shift => "shift",
            TokenType::Unshift => "unshift",
            TokenType::Splice => "splice",
            TokenType::Sort => "sort",
            TokenType::Reverse => "reverse",
            TokenType::Map => "map",
            TokenType::Grep => "grep",
            TokenType::Join => "join",
            TokenType::Split => "split",
            TokenType::Length => "length",
            TokenType::Substr => "substr",
            TokenType::Index => "index",
            TokenType::Rindex => "rindex",
            TokenType::Lc => "lc",
            TokenType::Uc => "uc",
            TokenType::Lcfirst => "lcfirst",
            TokenType::Ucfirst => "ucfirst",
            TokenType::Chomp => "chomp",
            TokenType::Chop => "chop",
            TokenType::Hex => "hex",
            TokenType::Oct => "oct",
            TokenType::Ord => "ord",
            TokenType::Chr => "chr",
            TokenType::Int => "int",
            TokenType::Abs => "abs",
            TokenType::Sqrt => "sqrt",
            TokenType::Log => "log",
            TokenType::Exp => "exp",
            TokenType::Sin => "sin",
            TokenType::Cos => "cos",
            TokenType::Tan => "tan",
            TokenType::Atan2 => "atan2",
            TokenType::Rand => "rand",
            TokenType::Srand => "srand",
            TokenType::Time => "time",
            TokenType::Localtime => "localtime",
            TokenType::Gmtime => "gmtime",
            TokenType::Sleep => "sleep",
            TokenType::Alarm => "alarm",
            TokenType::Fork => "fork",
            TokenType::Wait => "wait",
            TokenType::Waitpid => "waitpid",
            TokenType::System => "system",
            TokenType::Exec => "exec",
            TokenType::Open => "open",
            TokenType::Close => "close",
            TokenType::Read => "read",
            TokenType::Write => "write",
            TokenType::Seek => "seek",
            TokenType::Tell => "tell",
            TokenType::Truncate => "truncate",
            TokenType::Flock => "flock",
            TokenType::Link => "link",
            TokenType::Unlink => "unlink",
            TokenType::Symlink => "symlink",
            TokenType::Readlink => "readlink",
            TokenType::Mkdir => "mkdir",
            TokenType::Rmdir => "rmdir",
            TokenType::Chdir => "chdir",
            TokenType::Chmod => "chmod",
            TokenType::Chown => "chown",
            TokenType::Umask => "umask",
            TokenType::Rename => "rename",
            TokenType::Stat => "stat",
            TokenType::Lstat => "lstat",
            TokenType::Fcntl => "fcntl",
            TokenType::Ioctl => "ioctl",
            TokenType::Select => "select",
            TokenType::Pipe => "pipe",
            TokenType::Socket => "socket",
            TokenType::Bind => "bind",
            TokenType::Listen => "listen",
            TokenType::Accept => "accept",
            TokenType::Connect => "connect",
            TokenType::Shutdown => "shutdown",
            TokenType::Getsockopt => "getsockopt",
            TokenType::Setsockopt => "setsockopt",
            TokenType::Getsockname => "getsockname",
            TokenType::Getpeername => "getpeername",
            TokenType::Send => "send",
            TokenType::Recv => "recv",
            TokenType::Shmget => "shmget",
            TokenType::Shmctl => "shmctl",
            TokenType::Shmread => "shmread",
            TokenType::Shmwrite => "shmwrite",
            TokenType::Msgget => "msgget",
            TokenType::Msgctl => "msgctl",
            TokenType::Msgsnd => "msgsnd",
            TokenType::Msgrcv => "msgrcv",
            TokenType::Semget => "semget",
            TokenType::Semctl => "semctl",
            TokenType::Semop => "semop",
            TokenType::Semclose => "semclose",
            TokenType::Semremove => "semremove",
            TokenType::Shmclose => "shmclose",
            TokenType::Shmremove => "shmremove",
            TokenType::Msgclose => "msgclose",
            TokenType::Msgremove => "msgremove",
            TokenType::Plus => "+",
            TokenType::Minus => "-",
            TokenType::Multiply => "*",
            TokenType::Divide => "/",
            TokenType::Modulo => "%",
            TokenType::Power => "**",
            TokenType::Assign => "=",
            TokenType::PlusAssign => "+=",
            TokenType::MinusAssign => "-=",
            TokenType::MultiplyAssign => "*=",
            TokenType::DivideAssign => "/=",
            TokenType::ModuloAssign => "%=",
            TokenType::PowerAssign => "**=",
            TokenType::Increment => "++",
            TokenType::Decrement => "--",
            TokenType::Equal => "==",
            TokenType::NotEqual => "!=",
            TokenType::LessThan => "<",
            TokenType::GreaterThan => ">",
            TokenType::LessEqual => "<=",
            TokenType::GreaterEqual => ">=",
            TokenType::StringEqual => "eq",
            TokenType::StringNotEqual => "ne",
            TokenType::StringLessThan => "lt",
            TokenType::StringGreaterThan => "gt",
            TokenType::StringLessEqual => "le",
            TokenType::StringGreaterEqual => "ge",
            TokenType::LogicalAnd => "&&",
            TokenType::LogicalOr => "||",
            TokenType::LogicalNot => "!",
            TokenType::BitwiseAnd => "&",
            TokenType::BitwiseOr => "|",
            TokenType::BitwiseXor => "^",
            TokenType::BitwiseNot => "~",
            TokenType::LeftShift => "<<",
            TokenType::RightShift => ">>",
            TokenType::Range => "..",
            TokenType::RangeExclusive => "...",
            TokenType::Comma => ",",
            TokenType::FatComma => "=>",
            TokenType::Arrow => "->",
            TokenType::DoubleArrow => "=>",
            TokenType::Question => "?",
            TokenType::Colon => ":",
            TokenType::Semicolon => ";",
            TokenType::Dot => ".",
            TokenType::DoubleDot => "..",
            TokenType::TripleDot => "...",
            TokenType::Integer => "integer",
            TokenType::Float => "float",
            TokenType::String => "string",
            TokenType::SingleQuotedString => "single_quoted_string",
            TokenType::DoubleQuotedString => "double_quoted_string",
            TokenType::HereDocument => "here_document",
            TokenType::Regex => "regex",
            TokenType::RegexSubstitution => "regex_substitution",
            TokenType::RegexTransliteration => "regex_transliteration",
            TokenType::Identifier => "identifier",
            TokenType::Variable => "variable",
            TokenType::ArrayVariable => "array_variable",
            TokenType::HashVariable => "hash_variable",
            TokenType::ScalarVariable => "scalar_variable",
            TokenType::PackageVariable => "package_variable",
            TokenType::Comment => "comment",
            TokenType::Pod => "pod",
            TokenType::Data => "data",
            TokenType::End => "end",
            TokenType::Error => "error",
            TokenType::Whitespace => "whitespace",
            TokenType::Newline => "newline",
            TokenType::Tab => "tab",
            TokenType::CarriageReturn => "carriage_return",
            TokenType::FormFeed => "form_feed",
            TokenType::VerticalTab => "vertical_tab",
            TokenType::LeftParenthesis => "(",
            TokenType::RightParenthesis => ")",
            TokenType::LeftBracket => "[",
            TokenType::RightBracket => "]",
            TokenType::LeftBrace => "{",
            TokenType::RightBrace => "}",
            TokenType::LeftAngle => "<",
            TokenType::RightAngle => ">",
            TokenType::Eof => "eof",
            TokenType::Unknown => "unknown",

            // Compatibility aliases for test suite
            TokenType::StringLiteral => "string_literal",
            TokenType::NumberLiteral => "number_literal",
            TokenType::Operator => "operator",
            TokenType::Keyword => "keyword",
        }
    }
}

pub use crate::unicode::UnicodeUtils;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scanner_state() {
        let mut state = ScannerState::default();
        assert_eq!(state.line, 1);
        assert_eq!(state.column, 1);
        assert_eq!(state.offset, 0);

        state.advance('a');
        assert_eq!(state.line, 1);
        assert_eq!(state.column, 2);
        assert_eq!(state.offset, 1);

        state.advance('\n');
        assert_eq!(state.line, 2);
        assert_eq!(state.column, 1);
        assert_eq!(state.offset, 2);
    }

    #[test]
    fn test_token_type_names() {
        assert_eq!(TokenType::Package.name(), "package");
        assert_eq!(TokenType::Plus.name(), "+");
        assert_eq!(TokenType::Identifier.name(), "identifier");
    }
}
