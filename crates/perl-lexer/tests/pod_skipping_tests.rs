//! Tests for POD block skipping in the lexer's whitespace/comment skipper.
//!
//! POD (Plain Old Documentation) blocks start with `=head1`, `=pod`, `=over`, etc.
//! at the beginning of a line and end with `=cut` at the beginning of a line.
//! The lexer should skip these blocks entirely, producing no tokens for them.

use perl_lexer::{PerlLexer, Token, TokenType};

type R = Result<(), Box<dyn std::error::Error>>;

/// Collect all tokens from input.
fn tokens(input: &str) -> Vec<Token> {
    PerlLexer::new(input).collect_tokens()
}

/// Collect only significant (non-whitespace, non-newline, non-EOF) tokens.
fn significant(input: &str) -> Vec<Token> {
    tokens(input)
        .into_iter()
        .filter(|t| {
            !matches!(t.token_type, TokenType::Whitespace | TokenType::Newline | TokenType::EOF)
        })
        .collect()
}

// ===========================================================================
// 1. POD between statements
// ===========================================================================

#[test]
fn pod_between_statements_is_skipped() -> R {
    let code = "my $x = 1;\n=head1 NAME\nFoo\n=cut\nmy $y = 2;";
    let toks = significant(code);
    // Should have tokens for both `my $x = 1;` and `my $y = 2;`
    let texts: Vec<&str> = toks.iter().map(|t| t.text.as_ref()).collect();
    assert!(texts.contains(&"my"), "Should contain 'my' keyword: {texts:?}");
    // Should have two 'my' keywords (one for each statement)
    let my_count = texts.iter().filter(|&&t| t == "my").count();
    assert_eq!(my_count, 2, "Should have two 'my' keywords, got: {texts:?}");
    // Should NOT contain any POD-related tokens
    assert!(
        !texts.iter().any(|t| t.starts_with("=head") || t.starts_with("=cut")),
        "Should not contain POD tokens: {texts:?}"
    );
    Ok(())
}

// ===========================================================================
// 2. POD at start of file
// ===========================================================================

#[test]
fn pod_at_start_of_file_is_skipped() -> R {
    let code = "=head1 NAME\nFoo\n=cut\nmy $x = 1;";
    let toks = significant(code);
    let texts: Vec<&str> = toks.iter().map(|t| t.text.as_ref()).collect();
    assert!(texts.contains(&"my"), "Should contain 'my': {texts:?}");
    assert!(
        !texts.iter().any(|t| t.starts_with("=head") || t.starts_with("=cut")),
        "Should not contain POD tokens: {texts:?}"
    );
    Ok(())
}

// ===========================================================================
// 3. POD at EOF without =cut
// ===========================================================================

#[test]
fn pod_at_eof_without_cut_is_skipped() -> R {
    let code = "my $x = 1;\n=head1 NAME\nFoo";
    let toks = significant(code);
    let texts: Vec<&str> = toks.iter().map(|t| t.text.as_ref()).collect();
    assert!(texts.contains(&"my"), "Should contain 'my': {texts:?}");
    assert!(
        !texts.iter().any(|t| t.starts_with("=head")),
        "Should not contain POD tokens: {texts:?}"
    );
    Ok(())
}

// ===========================================================================
// 4. Multiple POD sections in one file
// ===========================================================================

#[test]
fn multiple_pod_sections_are_skipped() -> R {
    let code =
        "my $a = 1;\n=head1 FIRST\nstuff\n=cut\nmy $b = 2;\n=pod\nmore stuff\n=cut\nmy $c = 3;";
    let toks = significant(code);
    let texts: Vec<&str> = toks.iter().map(|t| t.text.as_ref()).collect();
    let my_count = texts.iter().filter(|&&t| t == "my").count();
    assert_eq!(my_count, 3, "Should have three 'my' keywords, got: {texts:?}");
    assert!(
        !texts
            .iter()
            .any(|t| t.starts_with("=head") || t.starts_with("=pod") || t.starts_with("=cut")),
        "Should not contain POD tokens: {texts:?}"
    );
    Ok(())
}

// ===========================================================================
// 5. Non-POD '=' at start of line should NOT be skipped
// ===========================================================================

#[test]
fn non_pod_equals_at_line_start_not_skipped() -> R {
    // $x\n= 1 — the '=' here is an assignment operator at column 0, not POD
    let code = "$x\n= 1;";
    let toks = significant(code);
    let texts: Vec<&str> = toks.iter().map(|t| t.text.as_ref()).collect();
    assert!(texts.contains(&"="), "Assignment '=' should be preserved: {texts:?}");
    assert!(texts.contains(&"1"), "Number '1' should be present: {texts:?}");
    Ok(())
}

#[test]
fn non_pod_equals_equals_at_line_start_not_skipped() -> R {
    // == at line start is not a POD directive
    let code = "$x\n== $y;";
    let toks = significant(code);
    let texts: Vec<&str> = toks.iter().map(|t| t.text.as_ref()).collect();
    assert!(texts.contains(&"=="), "Equality '==' should be preserved: {texts:?}");
    Ok(())
}

// ===========================================================================
// 6. Various POD directive types
// ===========================================================================

#[test]
fn pod_directive_types_are_all_skipped() -> R {
    let directives = [
        "=pod",
        "=head1",
        "=head2",
        "=over",
        "=item",
        "=back",
        "=begin",
        "=end",
        "=for",
        "=encoding",
    ];
    for directive in &directives {
        let code = format!("my $x = 1;\n{directive} stuff\ncontent\n=cut\nmy $y = 2;");
        let toks = significant(&code);
        let texts: Vec<&str> = toks.iter().map(|t| t.text.as_ref()).collect();
        let my_count = texts.iter().filter(|&&t| t == "my").count();
        assert_eq!(my_count, 2, "Directive '{directive}' should be skipped; tokens: {texts:?}");
    }
    Ok(())
}

// ===========================================================================
// 7. POD with only =cut on last line (no trailing newline)
// ===========================================================================

#[test]
fn pod_with_cut_at_eof_no_trailing_newline() -> R {
    let code = "my $x = 1;\n=head1 NAME\nFoo\n=cut";
    let toks = significant(code);
    let texts: Vec<&str> = toks.iter().map(|t| t.text.as_ref()).collect();
    assert!(texts.contains(&"my"), "Should contain 'my': {texts:?}");
    assert!(
        !texts.iter().any(|t| t.starts_with("=head") || t.starts_with("=cut")),
        "Should not contain POD tokens: {texts:?}"
    );
    Ok(())
}

// ===========================================================================
// 8. POD followed by code on same line as after =cut line
// ===========================================================================

#[test]
fn code_immediately_after_cut_line() -> R {
    let code = "=head1 NAME\nFoo\n=cut\nprint 42;";
    let toks = significant(code);
    let texts: Vec<&str> = toks.iter().map(|t| t.text.as_ref()).collect();
    assert!(texts.contains(&"print"), "Should contain 'print': {texts:?}");
    assert!(texts.contains(&"42"), "Should contain '42': {texts:?}");
    Ok(())
}

// ===========================================================================
// 9. POD with multi-byte UTF-8 content (byte-safety regression test)
// ===========================================================================

#[test]
fn pod_with_multibyte_utf8_content() -> R {
    // Ensure the byte-oriented POD scanner doesn't panic on multi-byte chars
    let code = "my $x = 1;\n=head1 NAÏVE ÜBERSICHT\n日本語テスト\n=cut\nmy $y = 2;";
    let toks = significant(code);
    let texts: Vec<&str> = toks.iter().map(|t| t.text.as_ref()).collect();
    let my_count = texts.iter().filter(|&&t| t == "my").count();
    assert_eq!(my_count, 2, "Should have two 'my' keywords: {texts:?}");
    Ok(())
}
