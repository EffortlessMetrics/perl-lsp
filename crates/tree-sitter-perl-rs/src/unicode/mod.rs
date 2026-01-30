//! Unicode handling for Perl parser
//!
//! This module provides Unicode normalization, validation, and processing
//! functionality required for proper Perl parsing.

use crate::error::{ParseError, ParseResult};
use unicode_ident::{is_xid_continue, is_xid_start};
use unicode_normalization::UnicodeNormalization;

/// Unicode normalization options
#[derive(Debug, Clone, PartialEq, Default)]
pub enum NormalizationForm {
    /// NFC normalization (default)
    #[default]
    NFC,
    /// NFD normalization
    NFD,
    /// NFKC normalization
    NFKC,
    /// NFKD normalization
    NFKD,
}

/// Unicode utilities for Perl parsing
pub struct UnicodeUtils;

impl UnicodeUtils {
    /// Normalize a string according to the specified form
    pub fn normalize(input: &str, form: NormalizationForm) -> ParseResult<String> {
        let normalized = match form {
            NormalizationForm::NFC => input.nfc().collect::<String>(),
            NormalizationForm::NFD => input.nfd().collect::<String>(),
            NormalizationForm::NFKC => input.nfkc().collect::<String>(),
            NormalizationForm::NFKD => input.nfkd().collect::<String>(),
        };
        Ok(normalized)
    }

    /// Normalize an identifier using NFC normalization
    pub fn normalize_identifier(input: &str) -> String {
        input.nfc().collect::<String>()
    }

    /// Check if a character is a valid Perl identifier start
    pub fn is_identifier_start(ch: char) -> bool {
        is_xid_start(ch) || ch == '_'
    }

    /// Check if a character is a valid Perl identifier continue
    pub fn is_identifier_continue(ch: char) -> bool {
        is_xid_continue(ch) || ch == '_'
    }

    /// Validate Unicode surrogate pairs
    pub fn validate_surrogate_pair(high: u16, low: u16) -> ParseResult<char> {
        if (0xD800..=0xDBFF).contains(&high) && (0xDC00..=0xDFFF).contains(&low) {
            let code_point = ((high as u32 - 0xD800) << 10) + (low as u32 - 0xDC00) + 0x10000;
            char::from_u32(code_point)
                .ok_or_else(|| ParseError::unicode_error("Invalid surrogate pair"))
        } else {
            Err(ParseError::unicode_error("Invalid surrogate pair"))
        }
    }

    /// Check if a string is a valid Perl identifier
    pub fn is_valid_identifier(input: &str) -> bool {
        let mut chars = input.chars();
        match chars.next() {
            Some(first) if Self::is_identifier_start(first) => {
                chars.all(Self::is_identifier_continue)
            }
            _ => false,
        }
    }

    /// Check if a character is a Unicode combining mark
    pub fn is_combining_mark(ch: char) -> bool {
        matches!(ch as u32,
            0x0300..=0x036F | // Combining Diacritical Marks
            0x1AB0..=0x1AFF | // Combining Diacritical Marks Extended
            0x20D0..=0x20FF | // Combining Diacritical Marks for Symbols
            0xFE20..=0xFE2F   // Combining Half Marks
        )
    }

    /// Check if a character is a Unicode whitespace character
    pub fn is_unicode_whitespace(ch: char) -> bool {
        ch.is_whitespace()
            || matches!(
                ch as u32,
                0x00A0 | // NO-BREAK SPACE
            0x1680 | // OGHAM SPACE MARK
            0x2000
                    ..=0x200A | // Various space characters
            0x202F | // NARROW NO-BREAK SPACE
            0x205F | // MEDIUM MATHEMATICAL SPACE
            0x3000 // IDEOGRAPHIC SPACE
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unicode_normalization() {
        let input = "café";
        let normalized = UnicodeUtils::normalize(input, NormalizationForm::NFC).unwrap();
        assert_eq!(normalized, "café");
    }

    #[test]
    fn test_identifier_validation() {
        assert!(UnicodeUtils::is_identifier_start('a'));
        assert!(UnicodeUtils::is_identifier_start('_'));
        assert!(UnicodeUtils::is_identifier_start('変'));
        assert!(!UnicodeUtils::is_identifier_start('1'));

        assert!(UnicodeUtils::is_identifier_continue('a'));
        assert!(UnicodeUtils::is_identifier_continue('1'));
        assert!(UnicodeUtils::is_identifier_continue('_'));
    }
}
