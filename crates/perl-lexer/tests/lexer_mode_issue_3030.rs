//! Tests for lexer mode tracking — closes #3030
//!
//! Covers `LexerMode` enum properties, `is_expect_term()`, `is_expect_operator()`,
//! copy/clone/equality semantics, and integration with the PerlLexer `set_mode` API.
//! Fills gaps left by the existing comprehensive unit tests, specifically
//! `is_expect_operator()` for the non-operator variants.

use perl_lexer::{LexerMode, PerlLexer, TokenType};

type R = Result<(), Box<dyn std::error::Error>>;

// ============================================================================
// is_expect_operator — exhaustive variant coverage
// ============================================================================

#[test]
fn is_expect_operator_false_for_expect_delimiter() {
    assert!(
        !LexerMode::ExpectDelimiter.is_expect_operator(),
        "ExpectDelimiter must not satisfy is_expect_operator"
    );
}

#[test]
fn is_expect_operator_false_for_in_format_body() {
    assert!(
        !LexerMode::InFormatBody.is_expect_operator(),
        "InFormatBody must not satisfy is_expect_operator"
    );
}

#[test]
fn is_expect_operator_false_for_in_data_section() {
    assert!(
        !LexerMode::InDataSection.is_expect_operator(),
        "InDataSection must not satisfy is_expect_operator"
    );
}

#[test]
fn is_expect_operator_true_only_for_expect_operator() {
    let variants = [
        (LexerMode::ExpectTerm, false),
        (LexerMode::ExpectOperator, true),
        (LexerMode::ExpectDelimiter, false),
        (LexerMode::InFormatBody, false),
        (LexerMode::InDataSection, false),
    ];
    for (mode, expected) in &variants {
        assert_eq!(
            mode.is_expect_operator(),
            *expected,
            "is_expect_operator() for {:?} should be {}",
            mode,
            expected
        );
    }
}

#[test]
fn is_expect_term_false_for_expect_operator() {
    assert!(
        !LexerMode::ExpectOperator.is_expect_term(),
        "ExpectOperator must not satisfy is_expect_term"
    );
}

#[test]
fn is_expect_term_false_for_expect_delimiter() {
    assert!(
        !LexerMode::ExpectDelimiter.is_expect_term(),
        "ExpectDelimiter must not satisfy is_expect_term"
    );
}

#[test]
fn is_expect_term_true_only_for_expect_term() {
    let variants = [
        (LexerMode::ExpectTerm, true),
        (LexerMode::ExpectOperator, false),
        (LexerMode::ExpectDelimiter, false),
        (LexerMode::InFormatBody, false),
        (LexerMode::InDataSection, false),
    ];
    for (mode, expected) in &variants {
        assert_eq!(
            mode.is_expect_term(),
            *expected,
            "is_expect_term() for {:?} should be {}",
            mode,
            expected
        );
    }
}

// ============================================================================
// Mutually exclusive — is_expect_term and is_expect_operator never both true
// ============================================================================

#[test]
fn modes_term_and_operator_are_mutually_exclusive() {
    let all_modes = [
        LexerMode::ExpectTerm,
        LexerMode::ExpectOperator,
        LexerMode::ExpectDelimiter,
        LexerMode::InFormatBody,
        LexerMode::InDataSection,
    ];
    for mode in &all_modes {
        assert!(
            !(mode.is_expect_term() && mode.is_expect_operator()),
            "mode {:?} must not be both expect_term and expect_operator",
            mode
        );
    }
}

// ============================================================================
// Copy / Clone / PartialEq / Eq semantics
// ============================================================================

#[test]
fn lexer_mode_copy_semantics() {
    // LexerMode is Copy, so assignment creates an independent copy
    let original = LexerMode::ExpectOperator;
    let copy = original;
    assert_eq!(original, copy);
    // Both are still usable after copy
    assert!(original.is_expect_operator());
    assert!(copy.is_expect_operator());
}

#[test]
fn lexer_mode_clone_is_equal() {
    for mode in &[
        LexerMode::ExpectTerm,
        LexerMode::ExpectOperator,
        LexerMode::ExpectDelimiter,
        LexerMode::InFormatBody,
        LexerMode::InDataSection,
    ] {
        let cloned = *mode;
        assert_eq!(*mode, cloned, "clone of {:?} must equal original", mode);
    }
}

#[test]
fn lexer_mode_different_variants_not_equal() {
    assert_ne!(LexerMode::ExpectTerm, LexerMode::ExpectOperator);
    assert_ne!(LexerMode::ExpectTerm, LexerMode::ExpectDelimiter);
    assert_ne!(LexerMode::ExpectTerm, LexerMode::InFormatBody);
    assert_ne!(LexerMode::ExpectTerm, LexerMode::InDataSection);
    assert_ne!(LexerMode::ExpectOperator, LexerMode::ExpectDelimiter);
    assert_ne!(LexerMode::ExpectOperator, LexerMode::InFormatBody);
    assert_ne!(LexerMode::ExpectOperator, LexerMode::InDataSection);
    assert_ne!(LexerMode::ExpectDelimiter, LexerMode::InFormatBody);
    assert_ne!(LexerMode::ExpectDelimiter, LexerMode::InDataSection);
    assert_ne!(LexerMode::InFormatBody, LexerMode::InDataSection);
}

// ============================================================================
// Default variant is ExpectTerm
// ============================================================================

#[test]
fn default_mode_is_expect_term() {
    let mode = LexerMode::default();
    assert_eq!(mode, LexerMode::ExpectTerm);
    assert!(mode.is_expect_term());
    assert!(!mode.is_expect_operator());
}

// ============================================================================
// Debug format is non-empty for all variants
// ============================================================================

#[test]
fn all_modes_have_non_empty_debug_output() {
    let modes = [
        LexerMode::ExpectTerm,
        LexerMode::ExpectOperator,
        LexerMode::ExpectDelimiter,
        LexerMode::InFormatBody,
        LexerMode::InDataSection,
    ];
    for mode in &modes {
        let dbg = format!("{:?}", mode);
        assert!(!dbg.is_empty(), "debug output for {:?} must be non-empty", mode);
    }
}

// ============================================================================
// Integration: set_mode controls slash disambiguation
// ============================================================================

#[test]
fn set_mode_expect_operator_makes_slash_division() -> R {
    // When mode is ExpectOperator, a leading slash is division, not regex start
    let mut lexer = PerlLexer::new("/ 2");
    lexer.set_mode(LexerMode::ExpectOperator);
    let tok = lexer.next_token().ok_or("expected a token")?;
    // Should be a Division or similar operator token, not a RegexMatch token
    assert!(
        !matches!(tok.token_type, TokenType::RegexMatch),
        "with ExpectOperator mode, '/' must not start a regex; got {:?}",
        tok.token_type
    );
    Ok(())
}

#[test]
fn set_mode_expect_term_makes_slash_regex() -> R {
    // When mode is ExpectTerm (default), a leading slash starts a regex
    let mut lexer = PerlLexer::new("/pattern/");
    lexer.set_mode(LexerMode::ExpectTerm);
    let tok = lexer.next_token().ok_or("expected a token")?;
    assert!(
        matches!(tok.token_type, TokenType::RegexMatch),
        "with ExpectTerm mode, '/' must start a regex; got {:?}",
        tok.token_type
    );
    Ok(())
}

#[test]
fn mode_transitions_after_identifier() -> R {
    // After an identifier the lexer should enter ExpectOperator mode
    // so that the subsequent '/' is division
    let mut lexer = PerlLexer::new("$x / 2");
    let tokens: Vec<_> = lexer
        .collect_tokens()
        .into_iter()
        .filter(|t| {
            !matches!(t.token_type, TokenType::Whitespace | TokenType::Newline | TokenType::EOF)
        })
        .collect();
    // We expect: Variable($x), Division(/), IntLiteral(2)
    let has_divide = tokens.iter().any(|t| matches!(t.token_type, TokenType::Division));
    let has_regex = tokens.iter().any(|t| matches!(t.token_type, TokenType::RegexMatch));
    assert!(has_divide, "after $x the slash must be division, not regex");
    assert!(!has_regex, "must not produce a Regex token after an identifier");
    Ok(())
}

#[test]
fn mode_transitions_after_keyword_to_expect_term() -> R {
    // After 'if' the lexer should enter ExpectTerm mode
    // so that the following '/' starts a regex
    let mut lexer = PerlLexer::new("if /pattern/");
    let tokens: Vec<_> = lexer
        .collect_tokens()
        .into_iter()
        .filter(|t| {
            !matches!(t.token_type, TokenType::Whitespace | TokenType::Newline | TokenType::EOF)
        })
        .collect();
    let has_regex = tokens.iter().any(|t| matches!(t.token_type, TokenType::RegexMatch));
    assert!(has_regex, "after 'if' keyword the slash must start a regex");
    Ok(())
}
