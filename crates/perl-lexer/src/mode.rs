//! Lexer modes for context-sensitive parsing
//!
//! # Slash Disambiguation (Issue #422)
//!
//! Perl's context-sensitive syntax requires tracking parser state to correctly
//! interpret ambiguous tokens like `/` (division vs regex) and `%` (modulo vs hash).
//!
//! ## Mode-Based Disambiguation Strategy
//!
//! The lexer uses a simple state machine with two primary modes:
//!
//! - **ExpectTerm**: Expecting a term/value → `/` starts regex, `%` starts hash
//! - **ExpectOperator**: Expecting an operator → `/` is division, `%` is modulo
//!
//! ## Context Heuristics (Implicit)
//!
//! Mode is automatically updated based on the previous token:
//!
//! | Previous Token          | Next Mode      | Example            |
//! |------------------------|----------------|-------------------|
//! | identifier             | ExpectOperator | `$x / 2`          |
//! | number                 | ExpectOperator | `10 / 3`          |
//! | closing paren/bracket  | ExpectOperator | `) / 2`           |
//! | keyword/word-operator  | ExpectTerm     | `if /p/`, `x and /p/` |
//! | operator               | ExpectTerm     | `=~ /test/`       |
//! | opening paren/bracket  | ExpectTerm     | `( /regex/`       |
//!
//! ## Timeout Protection
//!
//! - Budget guards prevent infinite loops on pathological input
//! - MAX_REGEX_BYTES (64KB) limit for regex literals
//! - Graceful degradation via UnknownRest token emission
//!
//! See `try_operator()` and `parse_regex()` in lib.rs for implementation.

/// Perl lexer mode to disambiguate slash tokens and other context-sensitive syntax
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LexerMode {
    /// Expecting a term (value) - slash starts a regex
    ///
    /// Examples: `if (/pattern/)`, `=~ /test/`, `while (/match/)`
    #[default]
    ExpectTerm,
    /// Expecting an operator - slash is division
    ///
    /// Examples: `$x / 2`, `$x // $y`, `10 / 3`, `) / 2`
    ExpectOperator,
    /// Expecting a delimiter for quote-like operators - # is not a comment
    ///
    /// Examples: `s/old/new/`, `tr/a-z/A-Z/`, `m{pattern}`
    ExpectDelimiter,
    /// Inside a format declaration body - consume until single dot on a line
    ///
    /// Example: `format STDOUT =\n...\n.\n`
    InFormatBody,
    /// Inside a data section (__DATA__ or __END__) - consume everything to EOF
    ///
    /// Example: `__DATA__\neverything after this is data`
    InDataSection,
}

impl LexerMode {
    /// Check if we're expecting a term
    pub fn is_expect_term(&self) -> bool {
        matches!(self, LexerMode::ExpectTerm)
    }

    /// Check if we're expecting an operator
    pub fn is_expect_operator(&self) -> bool {
        matches!(self, LexerMode::ExpectOperator)
    }

    /// Check if we're expecting a quote-like delimiter
    pub fn is_expect_delimiter(&self) -> bool {
        matches!(self, LexerMode::ExpectDelimiter)
    }
}

#[cfg(test)]
mod tests {
    use super::LexerMode;

    // --- is_expect_term ---

    #[test]
    fn lexer_mode_expect_term_is_term() -> Result<(), Box<dyn std::error::Error>> {
        assert!(LexerMode::ExpectTerm.is_expect_term());
        Ok(())
    }

    #[test]
    fn lexer_mode_expect_operator_is_not_term() -> Result<(), Box<dyn std::error::Error>> {
        assert!(!LexerMode::ExpectOperator.is_expect_term());
        Ok(())
    }

    #[test]
    fn lexer_mode_expect_delimiter_is_not_term() -> Result<(), Box<dyn std::error::Error>> {
        assert!(!LexerMode::ExpectDelimiter.is_expect_term());
        Ok(())
    }

    #[test]
    fn lexer_mode_in_format_body_is_not_term() -> Result<(), Box<dyn std::error::Error>> {
        assert!(!LexerMode::InFormatBody.is_expect_term());
        Ok(())
    }

    #[test]
    fn lexer_mode_in_data_section_is_not_term() -> Result<(), Box<dyn std::error::Error>> {
        assert!(!LexerMode::InDataSection.is_expect_term());
        Ok(())
    }

    // --- is_expect_operator ---

    #[test]
    fn lexer_mode_expect_operator_is_operator() -> Result<(), Box<dyn std::error::Error>> {
        assert!(LexerMode::ExpectOperator.is_expect_operator());
        Ok(())
    }

    #[test]
    fn lexer_mode_expect_term_is_not_operator() -> Result<(), Box<dyn std::error::Error>> {
        assert!(!LexerMode::ExpectTerm.is_expect_operator());
        Ok(())
    }

    #[test]
    fn lexer_mode_expect_delimiter_is_not_operator() -> Result<(), Box<dyn std::error::Error>> {
        assert!(!LexerMode::ExpectDelimiter.is_expect_operator());
        Ok(())
    }

    #[test]
    fn lexer_mode_in_format_body_is_not_operator() -> Result<(), Box<dyn std::error::Error>> {
        assert!(!LexerMode::InFormatBody.is_expect_operator());
        Ok(())
    }

    #[test]
    fn lexer_mode_in_data_section_is_not_operator() -> Result<(), Box<dyn std::error::Error>> {
        assert!(!LexerMode::InDataSection.is_expect_operator());
        Ok(())
    }

    // --- is_expect_delimiter ---

    #[test]
    fn lexer_mode_expect_delimiter_is_delimiter() -> Result<(), Box<dyn std::error::Error>> {
        assert!(LexerMode::ExpectDelimiter.is_expect_delimiter());
        Ok(())
    }

    #[test]
    fn lexer_mode_expect_term_is_not_delimiter() -> Result<(), Box<dyn std::error::Error>> {
        assert!(!LexerMode::ExpectTerm.is_expect_delimiter());
        Ok(())
    }

    #[test]
    fn lexer_mode_expect_operator_is_not_delimiter() -> Result<(), Box<dyn std::error::Error>> {
        assert!(!LexerMode::ExpectOperator.is_expect_delimiter());
        Ok(())
    }

    #[test]
    fn lexer_mode_in_format_body_is_not_delimiter() -> Result<(), Box<dyn std::error::Error>> {
        assert!(!LexerMode::InFormatBody.is_expect_delimiter());
        Ok(())
    }

    #[test]
    fn lexer_mode_in_data_section_is_not_delimiter() -> Result<(), Box<dyn std::error::Error>> {
        assert!(!LexerMode::InDataSection.is_expect_delimiter());
        Ok(())
    }

    // --- mutual exclusion ---
    // No mode is simultaneously term-expecting AND operator-expecting.

    #[test]
    fn lexer_mode_no_variant_is_both_term_and_operator() -> Result<(), Box<dyn std::error::Error>> {
        let modes = [
            LexerMode::ExpectTerm,
            LexerMode::ExpectOperator,
            LexerMode::ExpectDelimiter,
            LexerMode::InFormatBody,
            LexerMode::InDataSection,
        ];
        for mode in modes {
            assert!(
                !(mode.is_expect_term() && mode.is_expect_operator()),
                "{mode:?} should not be both term-expecting and operator-expecting",
            );
        }
        Ok(())
    }

    // --- default ---

    #[test]
    fn lexer_mode_default_is_expect_term() -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(LexerMode::default(), LexerMode::ExpectTerm);
        Ok(())
    }
}
