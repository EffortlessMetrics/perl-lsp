//! Token compatibility layer between perl-lexer and tree-sitter-perl-rs
//!
//! This module provides conversions and mappings between the token types
//! used by the external perl-lexer crate and the parser.

use std::sync::Arc;

/// Extended token types for the parser
#[derive(Debug, Clone, PartialEq)]
pub enum TokenType {
    // Basic operators
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    
    // Assignment operators
    Equal,
    PlusEqual,
    MinusEqual,
    StarEqual,
    SlashEqual,
    
    // Comparison operators
    EqualEqual,
    NotEqual,
    Less,
    Greater,
    LessEqual,
    GreaterEqual,
    
    // String comparison operators
    StringEq,
    StringNe,
    StringLt,
    StringGt,
    StringLe,
    StringGe,
    
    // Logical operators
    AndAnd,
    OrOr,
    Not,
    
    // Other operators
    Arrow,
    Dot,
    Range,
    Ellipsis,
    Question,
    ColonColon,
    Spaceship,    // <=>
    StringCmp,    // cmp
    StringRepeat, // x
    LeftShift,    // <<
    RightShift,   // >>
    BitwiseNot,   // ~
    Backslash,    // \
    Increment,    // ++
    Decrement,    // --
    
    // Delimiters
    LeftParen,
    RightParen,
    LeftBracket,
    RightBracket,
    LeftBrace,
    RightBrace,
    
    // Punctuation
    Semicolon,
    Comma,
    Colon,
    
    // Literals
    Number,
    SingleQuotedString,
    DoubleQuotedString,
    BacktickString,
    
    // Variables
    ScalarVariable,
    ArrayVariable,
    HashVariable,
    
    // Identifiers
    Identifier,
    
    // Special
    Whitespace,
    Comment,
    EOF,
    Error(String),
    
    // Heredoc
    HeredocStart,
    HeredocBody,
    
    // Regex
    RegexMatch,
    Substitution,
    Transliteration,
}

/// Convert from perl-lexer TokenType to our TokenType
pub fn from_perl_lexer_token(token: &crate::perl_lexer::Token) -> Token {
    use crate::perl_lexer::TokenType as PLTokenType;
    
    // Check for variables first by examining the text
    let token_type = if token.text.starts_with('$') && token.text.len() > 1 {
        TokenType::ScalarVariable
    } else if token.text.starts_with('@') && token.text.len() > 1 {
        TokenType::ArrayVariable
    } else if token.text.starts_with('%') && token.text.len() > 1 {
        TokenType::HashVariable
    } else {
        match &token.token_type {
        PLTokenType::Division => TokenType::Slash,
        PLTokenType::RegexMatch => TokenType::RegexMatch,
        PLTokenType::Substitution => TokenType::Substitution,
        PLTokenType::Transliteration => TokenType::Transliteration,
        PLTokenType::StringLiteral => {
            // Determine string type based on first character
            if token.text.starts_with('\'') {
                TokenType::SingleQuotedString
            } else if token.text.starts_with('"') {
                TokenType::DoubleQuotedString
            } else if token.text.starts_with('`') {
                TokenType::BacktickString
            } else {
                TokenType::DoubleQuotedString
            }
        }
        PLTokenType::Identifier(name) => {
            // Check if it's a variable
            if name.starts_with('$') {
                TokenType::ScalarVariable
            } else if name.starts_with('@') {
                TokenType::ArrayVariable
            } else if name.starts_with('%') {
                TokenType::HashVariable
            } else if name.starts_with('&') {
                // Subroutine reference
                TokenType::Identifier
            } else {
                TokenType::Identifier
            }
        }
        PLTokenType::Number(_) => TokenType::Number,
        PLTokenType::Operator(op) => match op.as_ref() {
            "+" => TokenType::Plus,
            "-" => TokenType::Minus,
            "*" => TokenType::Star,
            "/" => TokenType::Slash,
            "%" => TokenType::Percent,
            "=" => TokenType::Equal,
            "+=" => TokenType::PlusEqual,
            "-=" => TokenType::MinusEqual,
            "*=" => TokenType::StarEqual,
            "/=" => TokenType::SlashEqual,
            "==" => TokenType::EqualEqual,
            "!=" => TokenType::NotEqual,
            "<" => TokenType::Less,
            ">" => TokenType::Greater,
            "<=" => TokenType::LessEqual,
            ">=" => TokenType::GreaterEqual,
            "eq" => TokenType::StringEq,
            "ne" => TokenType::StringNe,
            "lt" => TokenType::StringLt,
            "gt" => TokenType::StringGt,
            "le" => TokenType::StringLe,
            "ge" => TokenType::StringGe,
            "&&" => TokenType::AndAnd,
            "||" => TokenType::OrOr,
            "!" => TokenType::Not,
            "->" => TokenType::Arrow,
            "." => TokenType::Dot,
            ".." => TokenType::Range,
            "..." => TokenType::Ellipsis,
            "=>" => TokenType::Arrow, // Fat comma
            "?" => TokenType::Question,
            "::" => TokenType::ColonColon,
            "<=>" => TokenType::Spaceship,
            "cmp" => TokenType::StringCmp,
            "x" => TokenType::StringRepeat,
            "<<" => TokenType::LeftShift,
            ">>" => TokenType::RightShift,
            "~" => TokenType::BitwiseNot,
            "\\" => TokenType::Backslash,
            "++" => TokenType::Increment,
            "--" => TokenType::Decrement,
            _ => TokenType::Error(format!("Unknown operator: {}", op)),
        },
        PLTokenType::LeftParen => TokenType::LeftParen,
        PLTokenType::RightParen => TokenType::RightParen,
        PLTokenType::LeftBracket => TokenType::LeftBracket,
        PLTokenType::RightBracket => TokenType::RightBracket,
        PLTokenType::LeftBrace => TokenType::LeftBrace,
        PLTokenType::RightBrace => TokenType::RightBrace,
        PLTokenType::Semicolon => TokenType::Semicolon,
        PLTokenType::Comma => TokenType::Comma,
        PLTokenType::Colon => TokenType::Colon,
        PLTokenType::Arrow => TokenType::Arrow,
        PLTokenType::FatComma => TokenType::Arrow, // => is like ->
        PLTokenType::Whitespace => TokenType::Whitespace,
        PLTokenType::Comment(_) => TokenType::Comment,
        PLTokenType::HeredocStart => TokenType::HeredocStart,
        PLTokenType::HeredocBody(_) => TokenType::HeredocBody,
        PLTokenType::Newline => TokenType::Whitespace,
        PLTokenType::EOF => TokenType::EOF,
        PLTokenType::Error(msg) => TokenType::Error(msg.to_string()),
        _ => TokenType::Error("Unhandled token type".to_string()),
        }
    };
    
    Token {
        token_type,
        text: token.text.clone(),
        start: token.start,
        end: token.end,
    }
}

/// Token with position information
#[derive(Debug, Clone)]
pub struct Token {
    pub token_type: TokenType,
    pub text: Arc<str>,
    pub start: usize,
    pub end: usize,
}