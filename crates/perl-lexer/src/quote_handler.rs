use crate::TokenType;
/// Quote operator handling with uniform delimiter processing and modifier attachment
///
/// This module provides consistent handling for all Perl quote-like operators:
/// - q/qq/qw/qr/qx for quote operators
/// - m for match operators  
/// - s for substitution operators
/// - tr/y for transliteration operators
use std::sync::Arc;

/// Specification for which modifiers are allowed for each operator
///
/// Note: These specs are currently defined for documentation and potential future use.
/// Modifier validation has been moved to the parser layer (MUT_005 fix) to provide
/// better error messages when invalid modifiers are encountered.
#[allow(dead_code)]
pub struct ModSpec {
    pub run: &'static [char], // allowed single-letter flags
    pub allow_charset: bool,  // whether one charset suffix is allowed
}

pub const QR_SPEC: ModSpec = ModSpec { run: &['i', 'm', 's', 'x', 'p', 'n'], allow_charset: true };

pub const M_SPEC: ModSpec =
    ModSpec { run: &['i', 'm', 's', 'x', 'p', 'n', 'g', 'c'], allow_charset: true };

pub const S_SPEC: ModSpec =
    ModSpec { run: &['i', 'm', 's', 'x', 'p', 'n', 'e', 'r'], allow_charset: true };

pub const TR_SPEC: ModSpec = ModSpec { run: &['c', 'd', 's', 'r'], allow_charset: false };

/// Get the paired closing delimiter for an opening delimiter
pub fn paired_close(open: char) -> Option<char> {
    match open {
        '(' => Some(')'),
        '[' => Some(']'),
        '{' => Some('}'),
        '<' => Some('>'),
        _ => None,
    }
}

/// Information about a quote operator being parsed
#[derive(Debug, Clone)]
pub struct QuoteOperatorInfo {
    pub operator: String, // "qr", "m", "s", etc.
    pub delimiter: char,  // The opening delimiter
    pub start_pos: usize, // Where the operator started
}

/// Check if we're currently parsing a quote operator
pub fn is_quote_operator(word: &str) -> bool {
    matches!(word, "q" | "qq" | "qw" | "qr" | "qx" | "m" | "s" | "tr" | "y")
}

/// Get the token type for a completed quote operator
pub fn get_quote_token_type(operator: &str) -> TokenType {
    match operator {
        "q" => TokenType::QuoteSingle,
        "qq" => TokenType::QuoteDouble,
        "qw" => TokenType::QuoteWords,
        "qr" => TokenType::QuoteRegex,
        "qx" => TokenType::QuoteCommand,
        "m" => TokenType::RegexMatch,
        "s" => TokenType::Substitution,
        "tr" | "y" => TokenType::Transliteration,
        _ => TokenType::Error(Arc::from(format!("Unknown quote operator: {}", operator))),
    }
}

}
