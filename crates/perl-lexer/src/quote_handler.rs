//! Quote-like operator helpers for the lexer.
//!
//! This module centralizes metadata and utility functions used while tokenizing
//! Perl quote-like operators:
//! - quote operators (`q`, `qq`, `qw`, `qr`, `qx`)
//! - regex operators (`m`, `s`)
//! - transliteration operators (`tr`, `y`)
//!
//! Modifier validation now lives in the parser, but canonicalization and
//! operator-to-token mapping remain here for lexing and shared normalization.
use crate::TokenType;
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

/// Canonicalize modifier flags to a consistent order for stable comparisons
///
/// Note: Currently unused as modifier validation moved to parser layer (MUT_005 fix).
/// Retained for potential future use in normalization or code generation.
#[allow(dead_code)]
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
///
/// Note: Currently unused as modifier validation moved to parser layer (MUT_005 fix).
/// Retained for potential future use in advanced modifier analysis.
#[allow(dead_code)]
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

// ============================================================================
// paired_delimiter_conformance — inline tests for paired_close
//
// This test block is one of THREE that together form the conformance matrix
// for paired-delimiter implementations across the workspace (#1320).
//
// The same input set is tested in:
//   - crates/perl-parser-core/src/syntax/quote.rs        (get_closing_delimiter)
//   - crates/perl-dap/src/inline_values/code_mask.rs     (matching_delimiter)
//
// Normalized contract: each impl maps to (close_char: char, is_paired: bool).
//   - This impl: paired_close(c) → Some(cl) ⇒ (cl, true); None ⇒ (c, false)
// ============================================================================
#[cfg(test)]
mod tests {
    use super::*;

    /// The shared conformance matrix.
    /// Each entry is `(open_char, expected_close, expected_is_paired)`.
    const MATRIX: &[(char, char, bool)] = &[
        // --- Paired openers -------------------------------------------
        ('(', ')', true),
        ('[', ']', true),
        ('{', '}', true),
        ('<', '>', true),
        // --- Self-delimiting: common punctuation ----------------------
        ('/', '/', false),
        ('#', '#', false),
        ('|', '|', false),
        ('!', '!', false),
        (',', ',', false),
        ('%', '%', false),
        ('~', '~', false),
        ('.', '.', false),
        (':', ':', false),
        (';', ';', false),
        // --- Self-delimiting: quote-adjacent chars --------------------
        ('\'', '\'', false),
        ('"', '"', false),
        // --- Self-delimiting: less-common punctuation ----------------
        ('@', '@', false),
        ('$', '$', false),
        ('^', '^', false),
        ('&', '&', false),
        ('*', '*', false),
        ('+', '+', false),
        ('-', '-', false),
        ('=', '=', false),
        ('?', '?', false),
        // --- Closing chars used as openers (not paired) --------------
        // Note: Perl does NOT auto-pair ) ] } > as openers.
        (')', ')', false),
        (']', ']', false),
        ('}', '}', false),
        ('>', '>', false),
    ];

    /// Normalize `paired_close` to the shared `(close_char, is_paired)` shape.
    fn normalize(open: char) -> (char, bool) {
        match paired_close(open) {
            Some(close) => (close, true),
            None => (open, false),
        }
    }

    #[test]
    fn paired_close_agrees_with_conformance_matrix() {
        for &(open, expected_close, expected_paired) in MATRIX {
            let (got_close, got_paired) = normalize(open);
            assert_eq!(
                got_close, expected_close,
                "perl-lexer paired_close({open:?}): close char mismatch \
                 (got {got_close:?}, expected {expected_close:?})"
            );
            assert_eq!(
                got_paired, expected_paired,
                "perl-lexer paired_close({open:?}): is_paired mismatch \
                 (got {got_paired}, expected {expected_paired})"
            );
        }
    }

    #[test]
    fn paired_close_paired_openers_return_some() {
        // The four Perl auto-paired delimiters must all return Some.
        for open in ['(', '[', '{', '<'] {
            assert!(
                paired_close(open).is_some(),
                "paired_close({open:?}) should be Some for a paired opener"
            );
        }
    }

    #[test]
    fn paired_close_self_delimiters_return_none() {
        // Self-delimiting chars that appear in Perl quote-like operators.
        let self_delims = ['/', '#', '|', '!', ',', '%', '~', '.', ':', ';', '\'', '"', '@'];
        for open in self_delims {
            assert!(
                paired_close(open).is_none(),
                "paired_close({open:?}) should be None for a self-delimiting char"
            );
        }
    }

    #[test]
    fn paired_close_closing_chars_used_as_openers_return_none() {
        // Perl only auto-pairs the opener chars; the closer chars are NOT themselves
        // auto-paired when used as an opener.
        for close in [')', ']', '}', '>'] {
            assert!(
                paired_close(close).is_none(),
                "paired_close({close:?}) should be None — closing chars are not themselves paired openers"
            );
        }
    }

    #[test]
    fn paired_close_handles_balanced_and_unbalanced_delimiters() {
        assert_eq!(paired_close('('), Some(')'));
        assert_eq!(paired_close('['), Some(']'));
        assert_eq!(paired_close('{'), Some('}'));
        assert_eq!(paired_close('<'), Some('>'));
        assert_eq!(paired_close('/'), None);
    }

    #[test]
    fn split_tail_for_spec_rejects_invalid_input() {
        assert_eq!(split_tail_for_spec("im1", &QR_SPEC), None);
        assert_eq!(split_tail_for_spec("z", &QR_SPEC), None);
        assert_eq!(split_tail_for_spec("ca", &TR_SPEC), None);
    }

    #[test]
    fn split_tail_for_spec_supports_charset_suffix_and_canonical_run_order() {
        assert_eq!(split_tail_for_spec("mixa", &QR_SPEC), Some(("imx".to_string(), Some("a"))));
        assert_eq!(split_tail_for_spec("ximaa", &QR_SPEC), Some(("imx".to_string(), Some("aa"))));
        assert_eq!(split_tail_for_spec("d", &QR_SPEC), Some(("".to_string(), Some("d"))));
    }

    #[test]
    fn split_tail_for_spec_without_charset_only_accepts_run_flags() {
        assert_eq!(split_tail_for_spec("rsc", &TR_SPEC), Some(("csr".to_string(), None)));
        assert_eq!(split_tail_for_spec("rsu", &TR_SPEC), None);
    }

    #[test]
    fn quote_operator_helpers_cover_known_and_unknown_operators() {
        assert!(is_quote_operator("qq"));
        assert!(is_quote_operator("tr"));
        assert!(!is_quote_operator("foo"));

        assert_eq!(get_quote_token_type("q"), TokenType::QuoteSingle);
        assert_eq!(get_quote_token_type("y"), TokenType::Transliteration);

        let unknown = get_quote_token_type("unknown");
        assert!(matches!(unknown, TokenType::Error(_)));
        if let TokenType::Error(message) = unknown {
            assert!(message.contains("unknown"));
        }
    }
}
