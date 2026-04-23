//! Edge case tests for the perl-tokenizer crate.
//!
//! Verifies that the TokenStream correctly translates lexer tokens
//! for edge-case Perl constructs: heredocs, regex delimiters,
//! quote-like operators, special variables, and Unicode identifiers.

use perl_parser_core::tokens::token_stream::TokenStream;
use perl_tdd_support::must;
use perl_token::TokenKind;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn collect_kinds(src: &str) -> Vec<TokenKind> {
    let mut s = TokenStream::new(src);
    let mut kinds = Vec::new();
    while let Ok(t) = s.next() {
        if t.kind == TokenKind::Eof {
            break;
        }
        kinds.push(t.kind);
    }
    kinds
}

fn collect_texts(src: &str) -> Vec<String> {
    let mut s = TokenStream::new(src);
    let mut texts = Vec::new();
    while let Ok(t) = s.next() {
        if t.kind == TokenKind::Eof {
            break;
        }
        texts.push(t.text.to_string());
    }
    texts
}

fn first_kind(src: &str) -> TokenKind {
    let mut s = TokenStream::new(src);
    must(s.peek()).kind
}

fn first_text(src: &str) -> String {
    let mut s = TokenStream::new(src);
    must(s.peek()).text.to_string()
}

// ===========================================================================
// 1. Heredoc variants through TokenStream
// ===========================================================================

#[test]
fn ts_heredoc_bare_word() -> Result<(), Box<dyn std::error::Error>> {
    let kinds = collect_kinds("<<EOF\nhello\nEOF\n");
    assert!(kinds.contains(&TokenKind::HeredocStart), "Expected HeredocStart in {:?}", kinds);
    Ok(())
}

#[test]
fn ts_heredoc_double_quoted() -> Result<(), Box<dyn std::error::Error>> {
    let kinds = collect_kinds("<<\"EOF\"\nhello\nEOF\n");
    assert!(
        kinds.contains(&TokenKind::HeredocStart),
        "Expected HeredocStart for double-quoted heredoc"
    );
    Ok(())
}

#[test]
fn ts_heredoc_single_quoted() -> Result<(), Box<dyn std::error::Error>> {
    let kinds = collect_kinds("<<'EOF'\nhello\nEOF\n");
    assert!(
        kinds.contains(&TokenKind::HeredocStart),
        "Expected HeredocStart for single-quoted heredoc"
    );
    Ok(())
}

#[test]
fn ts_heredoc_backtick() -> Result<(), Box<dyn std::error::Error>> {
    let kinds = collect_kinds("<<`CMD`\necho hello\nCMD\n");
    assert!(kinds.contains(&TokenKind::HeredocStart), "Expected HeredocStart for backtick heredoc");
    Ok(())
}

#[test]
fn ts_heredoc_indented() -> Result<(), Box<dyn std::error::Error>> {
    let kinds = collect_kinds("<<~EOF\n    hello\n    EOF\n");
    assert!(kinds.contains(&TokenKind::HeredocStart), "Expected HeredocStart for indented heredoc");
    Ok(())
}

#[test]
fn ts_heredoc_followed_by_statement() -> Result<(), Box<dyn std::error::Error>> {
    let kinds = collect_kinds("<<EOF\nbody\nEOF\nmy $x = 1;\n");
    assert!(kinds.contains(&TokenKind::HeredocStart));
    assert!(kinds.contains(&TokenKind::My), "Expected 'my' after heredoc body");
    Ok(())
}

// ===========================================================================
// 2. Regex delimiters through TokenStream
// ===========================================================================

#[test]
fn ts_regex_m_brace() -> Result<(), Box<dyn std::error::Error>> {
    let kind = first_kind("m{pattern}");
    assert_eq!(kind, TokenKind::Regex, "m{{...}} should be Regex");
    Ok(())
}

#[test]
fn ts_regex_m_bracket() -> Result<(), Box<dyn std::error::Error>> {
    let kind = first_kind("m[pattern]");
    assert_eq!(kind, TokenKind::Regex, "m[...] should be Regex");
    Ok(())
}

#[test]
fn ts_regex_m_angle() -> Result<(), Box<dyn std::error::Error>> {
    let kind = first_kind("m<pattern>");
    assert_eq!(kind, TokenKind::Regex, "m<...> should be Regex");
    Ok(())
}

#[test]
fn ts_regex_m_paren() -> Result<(), Box<dyn std::error::Error>> {
    let kind = first_kind("m(pattern)");
    assert_eq!(kind, TokenKind::Regex, "m(...) should be Regex");
    Ok(())
}

#[test]
fn ts_regex_m_pipe() -> Result<(), Box<dyn std::error::Error>> {
    let kind = first_kind("m|pattern|");
    assert_eq!(kind, TokenKind::Regex, "m|...| should be Regex");
    Ok(())
}

#[test]
fn ts_regex_slash_with_modifiers() -> Result<(), Box<dyn std::error::Error>> {
    let text = first_text("/pattern/imsx");
    assert_eq!(text, "/pattern/imsx");
    let kind = first_kind("/pattern/imsx");
    assert_eq!(kind, TokenKind::Regex);
    Ok(())
}

#[test]
fn ts_substitution_brackets() -> Result<(), Box<dyn std::error::Error>> {
    let kind = first_kind("s[old][new]");
    assert_eq!(kind, TokenKind::Substitution, "s[...][...] should be Substitution");
    Ok(())
}

#[test]
fn ts_substitution_angles() -> Result<(), Box<dyn std::error::Error>> {
    let kind = first_kind("s<old><new>");
    assert_eq!(kind, TokenKind::Substitution, "s<...><...> should be Substitution");
    Ok(())
}

#[test]
fn ts_substitution_with_modifiers() -> Result<(), Box<dyn std::error::Error>> {
    let text = first_text("s/foo/bar/ge");
    assert_eq!(text, "s/foo/bar/ge");
    let kind = first_kind("s/foo/bar/ge");
    assert_eq!(kind, TokenKind::Substitution);
    Ok(())
}

#[test]
fn ts_transliteration_pipe() -> Result<(), Box<dyn std::error::Error>> {
    let kind = first_kind("tr|a-z|A-Z|");
    assert_eq!(kind, TokenKind::Transliteration, "tr|...|...| should be Transliteration");
    Ok(())
}

#[test]
fn ts_transliteration_y_alias() -> Result<(), Box<dyn std::error::Error>> {
    let kind = first_kind("y/a-z/A-Z/");
    assert_eq!(kind, TokenKind::Transliteration, "y/.../.../ should be Transliteration");
    Ok(())
}

#[test]
fn ts_qr_angle() -> Result<(), Box<dyn std::error::Error>> {
    let kind = first_kind("qr<pattern>i");
    assert_eq!(kind, TokenKind::Regex, "qr<...> should be Regex");
    Ok(())
}

#[test]
fn ts_qr_brace() -> Result<(), Box<dyn std::error::Error>> {
    let kind = first_kind("qr{pattern}");
    assert_eq!(kind, TokenKind::Regex, "qr{{...}} should be Regex");
    Ok(())
}

// ===========================================================================
// 3. Quote-like operators through TokenStream
// ===========================================================================

#[test]
fn ts_q_brace() -> Result<(), Box<dyn std::error::Error>> {
    let kind = first_kind("q{hello}");
    assert_eq!(kind, TokenKind::QuoteSingle, "q{{...}} should be QuoteSingle");
    Ok(())
}

#[test]
fn ts_q_paren() -> Result<(), Box<dyn std::error::Error>> {
    let kind = first_kind("q(hello)");
    assert_eq!(kind, TokenKind::QuoteSingle, "q(...) should be QuoteSingle");
    Ok(())
}

#[test]
fn ts_q_bracket() -> Result<(), Box<dyn std::error::Error>> {
    let kind = first_kind("q[hello]");
    assert_eq!(kind, TokenKind::QuoteSingle, "q[...] should be QuoteSingle");
    Ok(())
}

#[test]
fn ts_q_angle() -> Result<(), Box<dyn std::error::Error>> {
    let kind = first_kind("q<hello>");
    assert_eq!(kind, TokenKind::QuoteSingle, "q<...> should be QuoteSingle");
    Ok(())
}

#[test]
fn ts_q_pipe() -> Result<(), Box<dyn std::error::Error>> {
    let kind = first_kind("q|hello|");
    assert_eq!(kind, TokenKind::QuoteSingle, "q|...| should be QuoteSingle");
    Ok(())
}

#[test]
fn ts_qq_paren() -> Result<(), Box<dyn std::error::Error>> {
    let kind = first_kind("qq(hello $world)");
    assert_eq!(kind, TokenKind::QuoteDouble, "qq(...) should be QuoteDouble");
    Ok(())
}

#[test]
fn ts_qq_bracket() -> Result<(), Box<dyn std::error::Error>> {
    let kind = first_kind("qq[hello $world]");
    assert_eq!(kind, TokenKind::QuoteDouble, "qq[...] should be QuoteDouble");
    Ok(())
}

#[test]
fn ts_qq_angle() -> Result<(), Box<dyn std::error::Error>> {
    let kind = first_kind("qq<hello $world>");
    assert_eq!(kind, TokenKind::QuoteDouble, "qq<...> should be QuoteDouble");
    Ok(())
}

#[test]
fn ts_qw_bracket() -> Result<(), Box<dyn std::error::Error>> {
    let kind = first_kind("qw[foo bar baz]");
    assert_eq!(kind, TokenKind::QuoteWords, "qw[...] should be QuoteWords");
    Ok(())
}

#[test]
fn ts_qw_angle() -> Result<(), Box<dyn std::error::Error>> {
    let kind = first_kind("qw<foo bar baz>");
    assert_eq!(kind, TokenKind::QuoteWords, "qw<...> should be QuoteWords");
    Ok(())
}

#[test]
fn ts_qw_pipe() -> Result<(), Box<dyn std::error::Error>> {
    let kind = first_kind("qw|foo bar baz|");
    assert_eq!(kind, TokenKind::QuoteWords, "qw|...| should be QuoteWords");
    Ok(())
}

#[test]
fn ts_qx_brace() -> Result<(), Box<dyn std::error::Error>> {
    let kind = first_kind("qx{ls -la}");
    assert_eq!(kind, TokenKind::QuoteCommand, "qx{{...}} should be QuoteCommand");
    Ok(())
}

#[test]
fn ts_qx_paren() -> Result<(), Box<dyn std::error::Error>> {
    let kind = first_kind("qx(ls -la)");
    assert_eq!(kind, TokenKind::QuoteCommand, "qx(...) should be QuoteCommand");
    Ok(())
}

#[test]
fn ts_backtick_string() -> Result<(), Box<dyn std::error::Error>> {
    let kind = first_kind("`ls -la`");
    assert_eq!(kind, TokenKind::QuoteCommand, "backtick string should be QuoteCommand");
    Ok(())
}

#[test]
fn ts_q_nested_delimiters() -> Result<(), Box<dyn std::error::Error>> {
    let text = first_text("q{hello {nested} world}");
    assert_eq!(text, "q{hello {nested} world}");
    let kind = first_kind("q{hello {nested} world}");
    assert_eq!(kind, TokenKind::QuoteSingle);
    Ok(())
}

// ===========================================================================
// 4. Special variables through TokenStream
// ===========================================================================

#[test]
fn ts_special_var_dollar_underscore() -> Result<(), Box<dyn std::error::Error>> {
    let kinds = collect_kinds("$_");
    assert!(
        kinds.contains(&TokenKind::Identifier),
        "$_ should tokenize to Identifier, got {:?}",
        kinds
    );
    Ok(())
}

#[test]
fn ts_special_var_at_underscore() -> Result<(), Box<dyn std::error::Error>> {
    let kinds = collect_kinds("@_");
    assert!(
        kinds.contains(&TokenKind::Identifier),
        "@_ should tokenize to Identifier, got {:?}",
        kinds
    );
    Ok(())
}

#[test]
fn ts_special_var_percent_env() -> Result<(), Box<dyn std::error::Error>> {
    let kinds = collect_kinds("%ENV");
    assert!(
        kinds.contains(&TokenKind::Identifier) || kinds.contains(&TokenKind::HashSigil),
        "%ENV should tokenize, got {:?}",
        kinds
    );
    Ok(())
}

#[test]
fn ts_special_var_dollar_bang() -> Result<(), Box<dyn std::error::Error>> {
    let kinds = collect_kinds("$!");
    assert!(!kinds.is_empty(), "$! should produce at least one token");
    Ok(())
}

#[test]
fn ts_special_var_dollar_at() -> Result<(), Box<dyn std::error::Error>> {
    let kinds = collect_kinds("$@");
    assert!(!kinds.is_empty(), "$@ should produce at least one token");
    Ok(())
}

#[test]
fn ts_special_var_dollar_dollar() -> Result<(), Box<dyn std::error::Error>> {
    let kinds = collect_kinds("$$");
    assert!(!kinds.is_empty(), "$$ should produce at least one token");
    Ok(())
}

#[test]
fn ts_special_var_dollar_hash_array() -> Result<(), Box<dyn std::error::Error>> {
    let texts = collect_texts("$#array");
    assert!(
        texts.iter().any(|t| t.contains("#array") || t == "$#array"),
        "$#array should be in tokens, got {:?}",
        texts
    );
    Ok(())
}

#[test]
fn ts_capture_variables() -> Result<(), Box<dyn std::error::Error>> {
    for n in 1..=9 {
        let var = format!("${}", n);
        let kinds = collect_kinds(&var);
        assert!(!kinds.is_empty(), "Expected at least one token for '{}'", var);
    }
    Ok(())
}

#[test]
fn ts_package_qualified_var() -> Result<(), Box<dyn std::error::Error>> {
    let text = first_text("$Foo::Bar::baz");
    assert_eq!(text, "$Foo::Bar::baz");
    let kind = first_kind("$Foo::Bar::baz");
    assert_eq!(kind, TokenKind::Identifier);
    Ok(())
}

// ===========================================================================
// 5. Unicode identifiers through TokenStream
// ===========================================================================

#[test]
fn ts_unicode_identifier_cafe() -> Result<(), Box<dyn std::error::Error>> {
    let kinds = collect_kinds("my $caf\u{00e9} = 1;");
    assert!(kinds.contains(&TokenKind::My));
    assert!(kinds.contains(&TokenKind::Identifier));
    assert!(kinds.contains(&TokenKind::Number));
    Ok(())
}

#[test]
fn ts_unicode_identifier_cjk() -> Result<(), Box<dyn std::error::Error>> {
    let kinds = collect_kinds("my $\u{4e16}\u{754c} = 1;");
    assert!(kinds.contains(&TokenKind::My));
    assert!(kinds.contains(&TokenKind::Identifier));
    Ok(())
}

#[test]
fn ts_unicode_subroutine_name() -> Result<(), Box<dyn std::error::Error>> {
    let texts = collect_texts("sub caf\u{00e9} { }");
    assert!(
        texts.iter().any(|t| t.contains("caf\u{00e9}")),
        "expected unicode sub name in texts: {:?}",
        texts
    );
    Ok(())
}

// ===========================================================================
// 6. Slash disambiguation through TokenStream
// ===========================================================================

#[test]
fn ts_division_after_variable() -> Result<(), Box<dyn std::error::Error>> {
    let kinds = collect_kinds("$x / 2");
    assert!(
        kinds.contains(&TokenKind::Slash),
        "/ after variable should be Slash (division), got {:?}",
        kinds
    );
    Ok(())
}

#[test]
fn ts_regex_after_binding() -> Result<(), Box<dyn std::error::Error>> {
    let kinds = collect_kinds("$x =~ /pattern/i");
    assert!(kinds.contains(&TokenKind::Regex), "/ after =~ should be Regex, got {:?}", kinds);
    Ok(())
}

#[test]
fn ts_defined_or_after_variable() -> Result<(), Box<dyn std::error::Error>> {
    let kinds = collect_kinds("$x // $y");
    assert!(
        kinds.contains(&TokenKind::DefinedOr),
        "// after variable should be DefinedOr, got {:?}",
        kinds
    );
    Ok(())
}

#[test]
fn ts_regex_at_statement_start() -> Result<(), Box<dyn std::error::Error>> {
    let kind = first_kind("/pattern/");
    assert_eq!(kind, TokenKind::Regex, "/ at statement start should be Regex");
    Ok(())
}

// ===========================================================================
// 7. Combined edge cases
// ===========================================================================

#[test]
fn ts_heredoc_then_regex() -> Result<(), Box<dyn std::error::Error>> {
    let kinds = collect_kinds("<<EOF\nbody\nEOF\nif (/pattern/) { }\n");
    assert!(kinds.contains(&TokenKind::HeredocStart));
    assert!(kinds.contains(&TokenKind::Regex));
    Ok(())
}

#[test]
fn ts_multiple_quote_ops() -> Result<(), Box<dyn std::error::Error>> {
    let kinds = collect_kinds("my $a = q{one}; my $b = qq(two); my @c = qw[three four];");
    assert!(kinds.contains(&TokenKind::QuoteSingle));
    assert!(kinds.contains(&TokenKind::QuoteDouble));
    assert!(kinds.contains(&TokenKind::QuoteWords));
    Ok(())
}

#[test]
fn ts_special_vars_in_expression() -> Result<(), Box<dyn std::error::Error>> {
    let kinds = collect_kinds("if ($! && $@) { $_ = $0; }");
    let id_count = kinds.iter().filter(|&&k| k == TokenKind::Identifier).count();
    assert!(
        id_count >= 3,
        "expected at least 3 identifier tokens for special vars, got {}",
        id_count
    );
    Ok(())
}

#[test]
fn ts_substitution_escaped_delimiters() -> Result<(), Box<dyn std::error::Error>> {
    let text = first_text(r"s/foo\/bar/baz\/qux/");
    assert_eq!(text, r"s/foo\/bar/baz\/qux/");
    let kind = first_kind(r"s/foo\/bar/baz\/qux/");
    assert_eq!(kind, TokenKind::Substitution);
    Ok(())
}

#[test]
fn ts_transliteration_with_ranges() -> Result<(), Box<dyn std::error::Error>> {
    let text = first_text("tr/a-zA-Z/A-Za-z/");
    assert_eq!(text, "tr/a-zA-Z/A-Za-z/");
    let kind = first_kind("tr/a-zA-Z/A-Za-z/");
    assert_eq!(kind, TokenKind::Transliteration);
    Ok(())
}
