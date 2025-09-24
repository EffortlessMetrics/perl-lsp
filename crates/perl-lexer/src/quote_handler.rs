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

/// Canonicalize modifier flags to a consistent order for stable comparisons
pub fn canon_run(run: &str, spec: &ModSpec) -> String {
    let mut out = String::new();
    for &c in spec.run {
        if run.contains(c) {
            out.push(c);
        }
    }
    out
}

/// Split a contiguous alphabetic tail into (`run_flags`, `charset_flag`) for the given spec
pub fn split_tail_for_spec(tail: &str, spec: &ModSpec) -> Option<(String, Option<&'static str>)> {
    // Must be all alphabetic
    if !tail.chars().all(|c| c.is_ascii_alphabetic()) {
        return None;
    }

    // If charset not allowed, all chars must be valid run flags
    if !spec.allow_charset {
        return if tail.chars().all(|c| spec.run.contains(&c)) {
            Some((canon_run(tail, spec), None))
        } else {
            None
        };
    }

    // Check for charset suffix (at most one, at the very end)
    let (run_part, charset): (&str, Option<&'static str>) =
        if let Some(stripped) = tail.strip_suffix("aa") {
            (stripped, Some("aa"))
        } else if let Some(stripped) = tail.strip_suffix('a') {
            (stripped, Some("a"))
        } else if let Some(stripped) = tail.strip_suffix('d') {
            (stripped, Some("d"))
        } else if let Some(stripped) = tail.strip_suffix('l') {
            (stripped, Some("l"))
        } else if let Some(stripped) = tail.strip_suffix('u') {
            (stripped, Some("u"))
        } else {
            (tail, None)
        };

    // Run-part must be in the allowed set
    if !run_part.chars().all(|c| spec.run.contains(&c)) {
        return None;
    }

    // All good: return canonicalized run + optional charset
    let run = canon_run(run_part, spec);
    Some((run, charset))
}

/// Information about a quote operator being parsed
#[derive(Debug, Clone)]
pub struct QuoteOperatorInfo {
    pub operator: String, // "qr", "m", "s", etc.
    pub delimiter: char,  // The opening delimiter
    pub start_pos: usize, // Where the operator started
}

/// Parse result for quote operators
#[derive(Debug)]
#[allow(dead_code)] // Future placeholder for quote parsing enhancements
pub struct QuoteResult {
    pub token_type: TokenType,
    pub text: Arc<str>,
    pub start: usize,
    pub end: usize,
}

/// Check if we're currently parsing a quote operator
#[allow(dead_code)]
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

/// Get the modifier specification for an operator
#[allow(dead_code)]
pub fn get_mod_spec(operator: &str) -> Option<&'static ModSpec> {
    match operator {
        "qr" => Some(&QR_SPEC),
        "m" => Some(&M_SPEC),
        "s" => Some(&S_SPEC),
        "tr" | "y" => Some(&TR_SPEC),
        _ => None, // q, qq, qw, qx don't take modifiers
    }
}
