//! Tests for slash disambiguation after `given`/`when` keywords.
//!
//! Regression for issue #818: `given`/`when` (feature 'switch', deprecated
//! but valid Perl) take an expression in parens; a `/` immediately after
//! the keyword should start a regex match, not a division operator.
//!
//! The bug: `given`/`when` fall through to the default keyword arm which
//! sets `LexerMode::ExpectOperator`, causing a following `/` to lex as
//! Division instead of RegexMatch.
//!
//! `when /foo/ { ... }` — the `/foo/` must lex as RegexMatch, not Division.

use perl_lexer::{PerlLexer, TokenType};

type TestResult = Result<(), Box<dyn std::error::Error>>;

// ─── Primary failure cases (these are RED before the fix) ────────────────────

/// `when /foo/ { ... }` — slash directly after `when` (no parens) must be RegexMatch.
///
/// Without the fix, `when` leaves the lexer in ExpectOperator mode and the
/// following `/` is lexed as Division (the bug).
#[test]
fn lexer_given_when_regex_slash_after_when_no_parens() -> TestResult {
    let code = "when /foo/ { 1 }";
    let mut lexer = PerlLexer::new(code);

    // Skip `when`
    let when_tok = lexer.next_token().ok_or("Expected 'when' keyword")?;
    assert!(
        matches!(when_tok.token_type, TokenType::Keyword(_)),
        "Expected 'when' to be a keyword, got {:?}",
        when_tok.token_type
    );

    // The very next token (the `/`) must be a RegexMatch, not Division
    let slash_tok = lexer.next_token().ok_or("Expected regex or division token")?;
    assert!(
        matches!(slash_tok.token_type, TokenType::RegexMatch),
        "Expected RegexMatch after 'when', got {:?} (text: {:?}) — issue #818",
        slash_tok.token_type,
        slash_tok.text
    );

    Ok(())
}

/// `given /foo/ { ... }` — slash directly after `given` (no parens) must be RegexMatch.
///
/// Same mode-setting gap as `when`.
#[test]
fn lexer_given_when_regex_slash_after_given_no_parens() -> TestResult {
    let code = "given /foo/ { 1 }";
    let mut lexer = PerlLexer::new(code);

    // Skip `given`
    let given_tok = lexer.next_token().ok_or("Expected 'given' keyword")?;
    assert!(
        matches!(given_tok.token_type, TokenType::Keyword(_)),
        "Expected 'given' to be a keyword, got {:?}",
        given_tok.token_type
    );

    // The very next token must be a RegexMatch, not Division
    let slash_tok = lexer.next_token().ok_or("Expected regex or division token")?;
    assert!(
        matches!(slash_tok.token_type, TokenType::RegexMatch),
        "Expected RegexMatch after 'given', got {:?} (text: {:?}) — issue #818",
        slash_tok.token_type,
        slash_tok.text
    );

    Ok(())
}

/// Full scan: `when /^\d+$/ { ... }` — anchored pattern, no false Division found.
#[test]
fn lexer_given_when_regex_slash_after_when_anchored_no_parens() -> TestResult {
    let code = r#"when /^\d+$/ { 1 }"#;
    let mut lexer = PerlLexer::new(code);

    let mut saw_regex = false;
    while let Some(tok) = lexer.next_token() {
        if matches!(tok.token_type, TokenType::RegexMatch) {
            saw_regex = true;
            break;
        }
        if matches!(tok.token_type, TokenType::Division) {
            return Err(format!(
                "Got Division where RegexMatch was expected after 'when' — issue #818; text: {:?}",
                tok.text
            )
            .into());
        }
        if matches!(tok.token_type, TokenType::EOF) {
            break;
        }
    }

    assert!(saw_regex, "Expected RegexMatch token in 'when /^\\d+$/' — issue #818");
    Ok(())
}

// ─── These cases work even without the fix (parens save the mode) ────────────
// They're included as regression guards.

/// `when (/foo/) { ... }` — slash after explicit `(` still lexes as RegexMatch.
///
/// `(` always sets ExpectTerm, so this worked before the fix too. Kept as
/// a regression guard.
#[test]
fn lexer_given_when_regex_slash_after_when_with_parens() -> TestResult {
    let code = "when (/foo/) { 1 }";
    let mut lexer = PerlLexer::new(code);

    let mut saw_regex = false;
    while let Some(tok) = lexer.next_token() {
        if matches!(tok.token_type, TokenType::RegexMatch) {
            saw_regex = true;
            break;
        }
        if matches!(tok.token_type, TokenType::EOF) {
            break;
        }
    }

    assert!(saw_regex, "Expected RegexMatch in 'when (/foo/)'");
    Ok(())
}

/// Full `given ($x) { when (/foo/) { ... } }` — still works.
#[test]
fn lexer_given_when_regex_full_given_when_construct_with_parens() -> TestResult {
    let code = r#"given ($x) { when (/foo/) { 1 } }"#;
    let mut lexer = PerlLexer::new(code);

    let mut saw_regex = false;
    while let Some(tok) = lexer.next_token() {
        if matches!(tok.token_type, TokenType::RegexMatch) {
            saw_regex = true;
            break;
        }
        if matches!(tok.token_type, TokenType::EOF) {
            break;
        }
    }

    assert!(saw_regex, "Expected RegexMatch inside given/when construct");
    Ok(())
}

// ─── Division regressions ─────────────────────────────────────────────────────

/// Regression: division after a variable is still Division, not RegexMatch.
#[test]
fn lexer_given_when_regex_regression_division_after_variable() -> TestResult {
    let code = "my $r = $n / 2;";
    let mut lexer = PerlLexer::new(code);

    let mut found_div = false;
    while let Some(tok) = lexer.next_token() {
        if matches!(tok.token_type, TokenType::Division) {
            found_div = true;
            break;
        }
        if matches!(tok.token_type, TokenType::EOF) {
            break;
        }
    }

    assert!(found_div, "Expected Division in 'my $r = $n / 2'");
    Ok(())
}

/// Regression: division after a function call result is still Division.
#[test]
fn lexer_given_when_regex_regression_division_in_expression() -> TestResult {
    let code = "$x = func() / 3;";
    let mut lexer = PerlLexer::new(code);

    let mut found_div = false;
    while let Some(tok) = lexer.next_token() {
        if matches!(tok.token_type, TokenType::Division) {
            found_div = true;
            break;
        }
        if matches!(tok.token_type, TokenType::EOF) {
            break;
        }
    }

    assert!(found_div, "Expected Division in '$x = func() / 3'");
    Ok(())
}

/// Regression: `if (/foo/) {}` — slash after `if` must still be RegexMatch.
#[test]
fn lexer_given_when_regex_regression_if_regex_still_works() -> TestResult {
    let code = "if (/foo/) { 1 }";
    let mut lexer = PerlLexer::new(code);

    let mut saw_regex = false;
    while let Some(tok) = lexer.next_token() {
        if matches!(tok.token_type, TokenType::RegexMatch) {
            saw_regex = true;
            break;
        }
        if matches!(tok.token_type, TokenType::EOF) {
            break;
        }
    }

    assert!(saw_regex, "Expected RegexMatch in 'if (/foo/) {{ }}'");
    Ok(())
}

/// Regression: `while (/bar/) {}` — slash after `while` must still be RegexMatch.
#[test]
fn lexer_given_when_regex_regression_while_regex_still_works() -> TestResult {
    let code = "while (/bar/) { last }";
    let mut lexer = PerlLexer::new(code);

    let mut saw_regex = false;
    while let Some(tok) = lexer.next_token() {
        if matches!(tok.token_type, TokenType::RegexMatch) {
            saw_regex = true;
            break;
        }
        if matches!(tok.token_type, TokenType::EOF) {
            break;
        }
    }

    assert!(saw_regex, "Expected RegexMatch in 'while (/bar/) {{ }}'");
    Ok(())
}

/// `given` as a variable name (inside sigil) must not cause errors.
///
/// When `given` appears as `$given`, the lexer sees it as a variable, not a
/// keyword. This test checks no error tokens appear.
#[test]
fn lexer_given_when_regex_given_as_variable_name_parses() -> TestResult {
    let code = "my $given = 1;";
    let mut lexer = PerlLexer::new(code);

    let mut tokens = Vec::new();
    while let Some(tok) = lexer.next_token() {
        let is_eof = matches!(tok.token_type, TokenType::EOF);
        tokens.push(tok);
        if is_eof {
            break;
        }
    }

    let has_error = tokens.iter().any(|t| matches!(t.token_type, TokenType::Error(_)));
    assert!(!has_error, "Unexpected error token in 'my $given = 1;'");
    Ok(())
}
