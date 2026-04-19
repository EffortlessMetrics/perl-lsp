//! Edge case tests for the Perl lexer.
//!
//! Covers:
//! - Heredoc variants: bare, double-quoted, single-quoted, backtick, indented (<<~)
//! - Regex delimiters: m{}, s[][], tr|||, qr<>
//! - Quote-like operators: q{}, qq(), qw[], qx``
//! - Special variables: $_, @_, %ENV, $!, $@, $$, $1..$9
//! - Unicode identifiers: my $cafe = 1;

use perl_lexer::{PerlLexer, TokenType};

type R = Result<(), Box<dyn std::error::Error>>;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn tokens(input: &str) -> Vec<perl_lexer::Token> {
    PerlLexer::new(input).collect_tokens()
}

fn significant(input: &str) -> Vec<perl_lexer::Token> {
    tokens(input)
        .into_iter()
        .filter(|t| {
            !matches!(
                t.token_type,
                TokenType::Whitespace | TokenType::Newline | TokenType::EOF
            )
        })
        .collect()
}

fn first_significant(input: &str) -> Option<perl_lexer::Token> {
    significant(input).into_iter().next()
}

/// Assert that every token span is within the input and the lexer terminates.
fn assert_terminates(input: &str) {
    let toks = tokens(input);
    for t in &toks {
        assert!(
            t.end <= input.len(),
            "Token {:?} end {} exceeds input len {}",
            t.token_type,
            t.end,
            input.len()
        );
        assert!(
            t.start <= t.end,
            "Token {:?} has start {} > end {}",
            t.token_type,
            t.start,
            t.end
        );
    }
    assert!(
        toks.iter().any(|t| matches!(t.token_type, TokenType::EOF)),
        "Expected EOF token in output"
    );
}

// ===========================================================================
// 1. Heredoc variants
// ===========================================================================

#[test]
fn heredoc_bare_word() -> R {
    let input = "<<EOF\nhello world\nEOF\n";
    assert_terminates(input);
    let sig = significant(input);
    let first = sig.first().ok_or("no tokens")?;
    assert!(
        matches!(first.token_type, TokenType::HeredocStart),
        "Expected HeredocStart, got {:?}",
        first.token_type
    );
    assert_eq!(first.text.as_ref(), "<<EOF");
    Ok(())
}

#[test]
fn heredoc_double_quoted_delimiter() -> R {
    let input = "<<\"EOF\"\nhello world\nEOF\n";
    assert_terminates(input);
    let sig = significant(input);
    let first = sig.first().ok_or("no tokens")?;
    assert!(
        matches!(first.token_type, TokenType::HeredocStart),
        "Expected HeredocStart, got {:?}",
        first.token_type
    );
    assert_eq!(first.text.as_ref(), "<<\"EOF\"");
    Ok(())
}

#[test]
fn heredoc_single_quoted_delimiter() -> R {
    let input = "<<'EOF'\nhello world\nEOF\n";
    assert_terminates(input);
    let sig = significant(input);
    let first = sig.first().ok_or("no tokens")?;
    assert!(
        matches!(first.token_type, TokenType::HeredocStart),
        "Expected HeredocStart, got {:?}",
        first.token_type
    );
    assert_eq!(first.text.as_ref(), "<<'EOF'");
    Ok(())
}

#[test]
fn heredoc_backtick_delimiter() -> R {
    let input = "<<`CMD`\necho hello\nCMD\n";
    assert_terminates(input);
    let sig = significant(input);
    let first = sig.first().ok_or("no tokens")?;
    assert!(
        matches!(first.token_type, TokenType::HeredocStart),
        "Expected HeredocStart, got {:?}",
        first.token_type
    );
    assert_eq!(first.text.as_ref(), "<<`CMD`");
    Ok(())
}

#[test]
fn heredoc_indented_bare() -> R {
    let input = "<<~EOF\n    hello\n    EOF\n";
    assert_terminates(input);
    let sig = significant(input);
    let first = sig.first().ok_or("no tokens")?;
    assert!(
        matches!(first.token_type, TokenType::HeredocStart),
        "Expected HeredocStart, got {:?}",
        first.token_type
    );
    assert_eq!(first.text.as_ref(), "<<~EOF");
    Ok(())
}

#[test]
fn heredoc_indented_double_quoted() -> R {
    let input = "<<~\"END\"\n    hello $world\n    END\n";
    assert_terminates(input);
    let sig = significant(input);
    let first = sig.first().ok_or("no tokens")?;
    assert!(
        matches!(first.token_type, TokenType::HeredocStart),
        "Expected HeredocStart, got {:?}",
        first.token_type
    );
    assert_eq!(first.text.as_ref(), "<<~\"END\"");
    Ok(())
}

#[test]
fn heredoc_indented_single_quoted() -> R {
    let input = "<<~'END'\n    hello $literal\n    END\n";
    assert_terminates(input);
    let sig = significant(input);
    let first = sig.first().ok_or("no tokens")?;
    assert!(
        matches!(first.token_type, TokenType::HeredocStart),
        "Expected HeredocStart, got {:?}",
        first.token_type
    );
    assert_eq!(first.text.as_ref(), "<<~'END'");
    Ok(())
}

#[test]
fn heredoc_body_consumed_before_next_statement() -> R {
    // After a heredoc, the next statement should be tokenized normally
    let input = "<<EOF\nbody\nEOF\nmy $x = 1;\n";
    assert_terminates(input);
    let sig = significant(input);
    // Should contain HeredocStart, then my, $x, =, 1, ;
    let has_heredoc = sig
        .iter()
        .any(|t| matches!(t.token_type, TokenType::HeredocStart));
    let has_my = sig
        .iter()
        .any(|t| matches!(&t.token_type, TokenType::Keyword(k) if k.as_ref() == "my"));
    assert!(has_heredoc, "expected HeredocStart");
    assert!(has_my, "expected 'my' keyword after heredoc body");
    Ok(())
}

#[test]
fn heredoc_empty_body() -> R {
    let input = "<<EOF\nEOF\n";
    assert_terminates(input);
    let sig = significant(input);
    let first = sig.first().ok_or("no tokens")?;
    assert!(
        matches!(first.token_type, TokenType::HeredocStart),
        "Expected HeredocStart, got {:?}",
        first.token_type
    );
    Ok(())
}

#[test]
fn heredoc_with_trailing_whitespace_on_terminator() -> R {
    // Perl allows trailing whitespace on the terminator line
    let input = "<<EOF\nhello\nEOF   \n";
    assert_terminates(input);
    let sig = significant(input);
    let has_heredoc = sig
        .iter()
        .any(|t| matches!(t.token_type, TokenType::HeredocStart));
    assert!(has_heredoc, "expected HeredocStart");
    Ok(())
}

// ===========================================================================
// 2. Regex delimiters
// ===========================================================================

#[test]
fn regex_m_brace_delimiter() -> R {
    let input = "m{pattern}";
    let sig = significant(input);
    let first = sig.first().ok_or("no tokens")?;
    assert!(
        matches!(first.token_type, TokenType::RegexMatch),
        "Expected RegexMatch for m{{...}}, got {:?}",
        first.token_type
    );
    assert_eq!(first.text.as_ref(), "m{pattern}");
    Ok(())
}

#[test]
fn regex_m_bracket_delimiter() -> R {
    let input = "m[pattern]";
    let sig = significant(input);
    let first = sig.first().ok_or("no tokens")?;
    assert!(
        matches!(first.token_type, TokenType::RegexMatch),
        "Expected RegexMatch for m[...], got {:?}",
        first.token_type
    );
    assert_eq!(first.text.as_ref(), "m[pattern]");
    Ok(())
}

#[test]
fn regex_m_angle_delimiter() -> R {
    let input = "m<pattern>";
    let sig = significant(input);
    let first = sig.first().ok_or("no tokens")?;
    assert!(
        matches!(first.token_type, TokenType::RegexMatch),
        "Expected RegexMatch for m<...>, got {:?}",
        first.token_type
    );
    assert_eq!(first.text.as_ref(), "m<pattern>");
    Ok(())
}

#[test]
fn regex_m_paren_delimiter() -> R {
    let input = "m(pattern)";
    let sig = significant(input);
    let first = sig.first().ok_or("no tokens")?;
    assert!(
        matches!(first.token_type, TokenType::RegexMatch),
        "Expected RegexMatch for m(...), got {:?}",
        first.token_type
    );
    assert_eq!(first.text.as_ref(), "m(pattern)");
    Ok(())
}

#[test]
fn substitution_bracket_bracket() -> R {
    let input = "s[old][new]";
    let sig = significant(input);
    let first = sig.first().ok_or("no tokens")?;
    assert!(
        matches!(first.token_type, TokenType::Substitution),
        "Expected Substitution for s[...][...], got {:?}",
        first.token_type
    );
    assert_eq!(first.text.as_ref(), "s[old][new]");
    Ok(())
}

#[test]
fn substitution_angle_angle() -> R {
    let input = "s<old><new>";
    let sig = significant(input);
    let first = sig.first().ok_or("no tokens")?;
    assert!(
        matches!(first.token_type, TokenType::Substitution),
        "Expected Substitution for s<...><...>, got {:?}",
        first.token_type
    );
    assert_eq!(first.text.as_ref(), "s<old><new>");
    Ok(())
}

#[test]
fn substitution_mixed_paired_delimiters() -> R {
    // Perl allows different paired delimiters for pattern and replacement: s{old}[new]
    let input = "s{old}[new]";
    let sig = significant(input);
    let first = sig.first().ok_or("no tokens")?;
    assert!(
        matches!(first.token_type, TokenType::Substitution),
        "Expected Substitution for s{{old}}[new], got {:?}",
        first.token_type
    );
    assert_eq!(first.text.as_ref(), "s{old}[new]");
    Ok(())
}

#[test]
fn substitution_with_modifiers_ge() -> R {
    let input = "s/foo/bar/ge";
    let sig = significant(input);
    let first = sig.first().ok_or("no tokens")?;
    assert!(
        matches!(first.token_type, TokenType::Substitution),
        "Expected Substitution, got {:?}",
        first.token_type
    );
    assert_eq!(first.text.as_ref(), "s/foo/bar/ge");
    Ok(())
}

#[test]
fn transliteration_pipe_delimiter() -> R {
    let input = "tr|a-z|A-Z|";
    let sig = significant(input);
    let first = sig.first().ok_or("no tokens")?;
    assert!(
        matches!(first.token_type, TokenType::Transliteration),
        "Expected Transliteration for tr|...|...|, got {:?}",
        first.token_type
    );
    assert_eq!(first.text.as_ref(), "tr|a-z|A-Z|");
    Ok(())
}

#[test]
fn transliteration_bracket_delimiter() -> R {
    let input = "tr[a-z][A-Z]";
    let sig = significant(input);
    let first = sig.first().ok_or("no tokens")?;
    assert!(
        matches!(first.token_type, TokenType::Transliteration),
        "Expected Transliteration for tr[...][...], got {:?}",
        first.token_type
    );
    assert_eq!(first.text.as_ref(), "tr[a-z][A-Z]");
    Ok(())
}

#[test]
fn transliteration_with_modifiers_cds() -> R {
    let input = "tr/a-z/A-Z/cds";
    let sig = significant(input);
    let first = sig.first().ok_or("no tokens")?;
    assert!(
        matches!(first.token_type, TokenType::Transliteration),
        "Expected Transliteration, got {:?}",
        first.token_type
    );
    assert_eq!(first.text.as_ref(), "tr/a-z/A-Z/cds");
    Ok(())
}

#[test]
fn transliteration_y_alias() -> R {
    let input = "y/a-z/A-Z/";
    let sig = significant(input);
    let first = sig.first().ok_or("no tokens")?;
    assert!(
        matches!(first.token_type, TokenType::Transliteration),
        "Expected Transliteration for y///,  got {:?}",
        first.token_type
    );
    assert_eq!(first.text.as_ref(), "y/a-z/A-Z/");
    Ok(())
}

#[test]
fn qr_angle_delimiter() -> R {
    let input = "qr<pattern>i";
    let sig = significant(input);
    let first = sig.first().ok_or("no tokens")?;
    assert!(
        matches!(first.token_type, TokenType::QuoteRegex),
        "Expected QuoteRegex for qr<...>, got {:?}",
        first.token_type
    );
    assert_eq!(first.text.as_ref(), "qr<pattern>i");
    Ok(())
}

#[test]
fn qr_brace_delimiter() -> R {
    let input = "qr{pattern}ms";
    let sig = significant(input);
    let first = sig.first().ok_or("no tokens")?;
    assert!(
        matches!(first.token_type, TokenType::QuoteRegex),
        "Expected QuoteRegex for qr{{...}}, got {:?}",
        first.token_type
    );
    assert_eq!(first.text.as_ref(), "qr{pattern}ms");
    Ok(())
}

#[test]
fn regex_with_escaped_delimiter() -> R {
    let input = r"/foo\/bar/";
    let sig = significant(input);
    let first = sig.first().ok_or("no tokens")?;
    assert!(
        matches!(first.token_type, TokenType::RegexMatch),
        "Expected RegexMatch with escaped delimiter, got {:?}",
        first.token_type
    );
    assert_eq!(first.text.as_ref(), r"/foo\/bar/");
    Ok(())
}

#[test]
fn regex_with_modifiers_imsx() -> R {
    let input = "/pattern/imsx";
    let sig = significant(input);
    let first = sig.first().ok_or("no tokens")?;
    assert!(
        matches!(first.token_type, TokenType::RegexMatch),
        "Expected RegexMatch, got {:?}",
        first.token_type
    );
    assert_eq!(first.text.as_ref(), "/pattern/imsx");
    Ok(())
}

// ===========================================================================
// 3. Quote-like operators
// ===========================================================================

#[test]
fn q_brace_operator() -> R {
    let input = "q{hello world}";
    let sig = significant(input);
    let first = sig.first().ok_or("no tokens")?;
    assert!(
        matches!(first.token_type, TokenType::QuoteSingle),
        "Expected QuoteSingle for q{{...}}, got {:?}",
        first.token_type
    );
    assert_eq!(first.text.as_ref(), "q{hello world}");
    Ok(())
}

#[test]
fn q_paren_operator() -> R {
    let input = "q(hello world)";
    let sig = significant(input);
    let first = sig.first().ok_or("no tokens")?;
    assert!(
        matches!(first.token_type, TokenType::QuoteSingle),
        "Expected QuoteSingle for q(...), got {:?}",
        first.token_type
    );
    assert_eq!(first.text.as_ref(), "q(hello world)");
    Ok(())
}

#[test]
fn q_bracket_operator() -> R {
    let input = "q[hello world]";
    let sig = significant(input);
    let first = sig.first().ok_or("no tokens")?;
    assert!(
        matches!(first.token_type, TokenType::QuoteSingle),
        "Expected QuoteSingle for q[...], got {:?}",
        first.token_type
    );
    assert_eq!(first.text.as_ref(), "q[hello world]");
    Ok(())
}

#[test]
fn q_angle_operator() -> R {
    let input = "q<hello world>";
    let sig = significant(input);
    let first = sig.first().ok_or("no tokens")?;
    assert!(
        matches!(first.token_type, TokenType::QuoteSingle),
        "Expected QuoteSingle for q<...>, got {:?}",
        first.token_type
    );
    assert_eq!(first.text.as_ref(), "q<hello world>");
    Ok(())
}

#[test]
fn q_pipe_operator() -> R {
    let input = "q|hello world|";
    let sig = significant(input);
    let first = sig.first().ok_or("no tokens")?;
    assert!(
        matches!(first.token_type, TokenType::QuoteSingle),
        "Expected QuoteSingle for q|...|, got {:?}",
        first.token_type
    );
    assert_eq!(first.text.as_ref(), "q|hello world|");
    Ok(())
}

#[test]
fn qq_paren_operator() -> R {
    let input = "qq(hello $world)";
    let sig = significant(input);
    let first = sig.first().ok_or("no tokens")?;
    assert!(
        matches!(first.token_type, TokenType::QuoteDouble),
        "Expected QuoteDouble for qq(...), got {:?}",
        first.token_type
    );
    assert_eq!(first.text.as_ref(), "qq(hello $world)");
    Ok(())
}

#[test]
fn qq_bracket_operator() -> R {
    let input = "qq[hello $world]";
    let sig = significant(input);
    let first = sig.first().ok_or("no tokens")?;
    assert!(
        matches!(first.token_type, TokenType::QuoteDouble),
        "Expected QuoteDouble for qq[...], got {:?}",
        first.token_type
    );
    assert_eq!(first.text.as_ref(), "qq[hello $world]");
    Ok(())
}

#[test]
fn qq_angle_operator() -> R {
    let input = "qq<hello $world>";
    let sig = significant(input);
    let first = sig.first().ok_or("no tokens")?;
    assert!(
        matches!(first.token_type, TokenType::QuoteDouble),
        "Expected QuoteDouble for qq<...>, got {:?}",
        first.token_type
    );
    assert_eq!(first.text.as_ref(), "qq<hello $world>");
    Ok(())
}

#[test]
fn qw_bracket_operator() -> R {
    let input = "qw[foo bar baz]";
    let sig = significant(input);
    let first = sig.first().ok_or("no tokens")?;
    assert!(
        matches!(first.token_type, TokenType::QuoteWords),
        "Expected QuoteWords for qw[...], got {:?}",
        first.token_type
    );
    assert_eq!(first.text.as_ref(), "qw[foo bar baz]");
    Ok(())
}

#[test]
fn qw_angle_operator() -> R {
    let input = "qw<foo bar baz>";
    let sig = significant(input);
    let first = sig.first().ok_or("no tokens")?;
    assert!(
        matches!(first.token_type, TokenType::QuoteWords),
        "Expected QuoteWords for qw<...>, got {:?}",
        first.token_type
    );
    assert_eq!(first.text.as_ref(), "qw<foo bar baz>");
    Ok(())
}

#[test]
fn qw_pipe_operator() -> R {
    let input = "qw|foo bar baz|";
    let sig = significant(input);
    let first = sig.first().ok_or("no tokens")?;
    assert!(
        matches!(first.token_type, TokenType::QuoteWords),
        "Expected QuoteWords for qw|...|, got {:?}",
        first.token_type
    );
    assert_eq!(first.text.as_ref(), "qw|foo bar baz|");
    Ok(())
}

#[test]
fn qx_brace_operator() -> R {
    let input = "qx{ls -la}";
    let sig = significant(input);
    let first = sig.first().ok_or("no tokens")?;
    assert!(
        matches!(first.token_type, TokenType::QuoteCommand),
        "Expected QuoteCommand for qx{{...}}, got {:?}",
        first.token_type
    );
    assert_eq!(first.text.as_ref(), "qx{ls -la}");
    Ok(())
}

#[test]
fn qx_paren_operator() -> R {
    let input = "qx(ls -la)";
    let sig = significant(input);
    let first = sig.first().ok_or("no tokens")?;
    assert!(
        matches!(first.token_type, TokenType::QuoteCommand),
        "Expected QuoteCommand for qx(...), got {:?}",
        first.token_type
    );
    assert_eq!(first.text.as_ref(), "qx(ls -la)");
    Ok(())
}

#[test]
fn q_with_nested_delimiters() -> R {
    let input = "q{hello {nested} world}";
    let sig = significant(input);
    let first = sig.first().ok_or("no tokens")?;
    assert!(
        matches!(first.token_type, TokenType::QuoteSingle),
        "Expected QuoteSingle for nested q{{...}}, got {:?}",
        first.token_type
    );
    assert_eq!(first.text.as_ref(), "q{hello {nested} world}");
    Ok(())
}

#[test]
fn qq_with_nested_parens() -> R {
    let input = "qq(hello (nested) world)";
    let sig = significant(input);
    let first = sig.first().ok_or("no tokens")?;
    assert!(
        matches!(first.token_type, TokenType::QuoteDouble),
        "Expected QuoteDouble for nested qq(...), got {:?}",
        first.token_type
    );
    assert_eq!(first.text.as_ref(), "qq(hello (nested) world)");
    Ok(())
}

#[test]
fn q_with_escaped_delimiter() -> R {
    let input = r"q|hello \| world|";
    let sig = significant(input);
    let first = sig.first().ok_or("no tokens")?;
    assert!(
        matches!(first.token_type, TokenType::QuoteSingle),
        "Expected QuoteSingle for q|...\\|...|, got {:?}",
        first.token_type
    );
    assert_eq!(first.text.as_ref(), r"q|hello \| world|");
    Ok(())
}

// ===========================================================================
// 4. Special variables
// ===========================================================================

#[test]
fn special_var_dollar_underscore() -> R {
    let tok = first_significant("$_").ok_or("no token")?;
    assert!(
        matches!(&tok.token_type, TokenType::Identifier(id) if id.as_ref() == "$_"),
        "Expected Identifier($_), got {:?}",
        tok.token_type
    );
    Ok(())
}

#[test]
fn special_var_at_underscore() -> R {
    let tok = first_significant("@_").ok_or("no token")?;
    assert!(
        matches!(&tok.token_type, TokenType::Identifier(id) if id.as_ref() == "@_"),
        "Expected Identifier(@_), got {:?}",
        tok.token_type
    );
    Ok(())
}

#[test]
fn special_var_percent_env() -> R {
    let tok = first_significant("%ENV").ok_or("no token")?;
    assert!(
        matches!(&tok.token_type, TokenType::Identifier(id) if id.as_ref() == "%ENV"),
        "Expected Identifier(%ENV), got {:?}",
        tok.token_type
    );
    Ok(())
}

#[test]
fn special_var_dollar_bang() -> R {
    let tok = first_significant("$!").ok_or("no token")?;
    assert!(
        matches!(&tok.token_type, TokenType::Identifier(id) if id.as_ref() == "$!"),
        "Expected Identifier($!), got {:?}",
        tok.token_type
    );
    Ok(())
}

#[test]
fn special_var_dollar_at() -> R {
    let tok = first_significant("$@").ok_or("no token")?;
    assert!(
        matches!(&tok.token_type, TokenType::Identifier(id) if id.as_ref() == "$@"),
        "Expected Identifier($@), got {:?}",
        tok.token_type
    );
    Ok(())
}

#[test]
fn special_var_dollar_dollar() -> R {
    let tok = first_significant("$$").ok_or("no token")?;
    assert!(
        matches!(&tok.token_type, TokenType::Identifier(id) if id.as_ref() == "$$"),
        "Expected Identifier($$), got {:?}",
        tok.token_type
    );
    Ok(())
}

#[test]
fn special_var_dollar_backslash() -> R {
    let tok = first_significant("$\\").ok_or("no token")?;
    assert!(
        matches!(&tok.token_type, TokenType::Identifier(id) if id.as_ref() == "$\\"),
        "Expected Identifier($\\), got {:?}",
        tok.token_type
    );
    Ok(())
}

#[test]
fn special_var_dollar_slash() -> R {
    let tok = first_significant("$/").ok_or("no token")?;
    assert!(
        matches!(&tok.token_type, TokenType::Identifier(id) if id.as_ref() == "$/"),
        "Expected Identifier($/), got {:?}",
        tok.token_type
    );
    Ok(())
}

#[test]
fn special_var_dollar_pipe() -> R {
    let tok = first_significant("$|").ok_or("no token")?;
    assert!(
        matches!(&tok.token_type, TokenType::Identifier(id) if id.as_ref() == "$|"),
        "Expected Identifier($|), got {:?}",
        tok.token_type
    );
    Ok(())
}

#[test]
fn special_var_dollar_bracket() -> R {
    let tok = first_significant("$[").ok_or("no token")?;
    assert!(
        matches!(&tok.token_type, TokenType::Identifier(id) if id.as_ref() == "$["),
        "Expected Identifier($[), got {:?}",
        tok.token_type
    );
    Ok(())
}

#[test]
fn capture_variables_1_through_9() -> R {
    for n in 1..=9 {
        let var = format!("${}", n);
        let toks = tokens(&var);
        // The variable should be tokenized (may be as Identifier or as $ + Number)
        assert!(
            !toks.is_empty(),
            "Expected at least one token for '{}'",
            var
        );
        // Verify the lexer terminates and produces valid spans
        for t in &toks {
            assert!(
                t.end <= var.len(),
                "Token {:?} end {} exceeds input len {}",
                t.token_type,
                t.end,
                var.len()
            );
        }
    }
    Ok(())
}

#[test]
fn special_var_dollar_question() -> R {
    let tok = first_significant("$?").ok_or("no token")?;
    assert!(
        matches!(&tok.token_type, TokenType::Identifier(id) if id.as_ref() == "$?"),
        "Expected Identifier($?), got {:?}",
        tok.token_type
    );
    Ok(())
}

#[test]
fn special_var_dollar_dot() -> R {
    let tok = first_significant("$.").ok_or("no token")?;
    assert!(
        matches!(&tok.token_type, TokenType::Identifier(id) if id.as_ref() == "$."),
        "Expected Identifier($.), got {:?}",
        tok.token_type
    );
    Ok(())
}

#[test]
fn special_var_dollar_plus() -> R {
    let tok = first_significant("$+").ok_or("no token")?;
    assert!(
        matches!(&tok.token_type, TokenType::Identifier(id) if id.as_ref() == "$+"),
        "Expected Identifier($+), got {:?}",
        tok.token_type
    );
    Ok(())
}

#[test]
fn special_var_dollar_minus() -> R {
    let tok = first_significant("$-").ok_or("no token")?;
    assert!(
        matches!(&tok.token_type, TokenType::Identifier(id) if id.as_ref() == "$-"),
        "Expected Identifier($-), got {:?}",
        tok.token_type
    );
    Ok(())
}

#[test]
fn special_var_dollar_ampersand() -> R {
    let tok = first_significant("$&").ok_or("no token")?;
    assert!(
        matches!(&tok.token_type, TokenType::Identifier(id) if id.as_ref() == "$&"),
        "Expected Identifier($&), got {:?}",
        tok.token_type
    );
    Ok(())
}

#[test]
fn special_var_dollar_backtick() -> R {
    let tok = first_significant("$`").ok_or("no token")?;
    assert!(
        matches!(&tok.token_type, TokenType::Identifier(id) if id.as_ref() == "$`"),
        "Expected Identifier($`), got {:?}",
        tok.token_type
    );
    Ok(())
}

#[test]
fn special_var_dollar_quote() -> R {
    // $' is the post-match variable
    let tok = first_significant("$'").ok_or("no token")?;
    assert!(
        matches!(&tok.token_type, TokenType::Identifier(id) if id.as_ref() == "$'"),
        "Expected Identifier($'), got {:?}",
        tok.token_type
    );
    Ok(())
}

#[test]
fn special_var_at_plus() -> R {
    // @+ is the array of match end positions
    let tok = first_significant("@+").ok_or("no token")?;
    assert!(
        matches!(&tok.token_type, TokenType::Identifier(id) if id.as_ref() == "@+"),
        "Expected Identifier(@+), got {:?}",
        tok.token_type
    );
    Ok(())
}

#[test]
fn special_var_at_minus() -> R {
    // @- is the array of match start positions
    let tok = first_significant("@-").ok_or("no token")?;
    assert!(
        matches!(&tok.token_type, TokenType::Identifier(id) if id.as_ref() == "@-"),
        "Expected Identifier(@-), got {:?}",
        tok.token_type
    );
    Ok(())
}

#[test]
fn special_var_percent_plus() -> R {
    // %+ is the hash of named capture groups (last match)
    let tok = first_significant("%+").ok_or("no token")?;
    assert!(
        matches!(&tok.token_type, TokenType::Identifier(id) if id.as_ref() == "%+"),
        "Expected Identifier(%+), got {:?}",
        tok.token_type
    );
    Ok(())
}

#[test]
fn special_var_percent_minus() -> R {
    // %- is the hash of named capture groups (all matches)
    let tok = first_significant("%-").ok_or("no token")?;
    assert!(
        matches!(&tok.token_type, TokenType::Identifier(id) if id.as_ref() == "%-"),
        "Expected Identifier(%-), got {:?}",
        tok.token_type
    );
    Ok(())
}

#[test]
fn special_var_dollar_hash_array() -> R {
    // $#array gives the last index of @array
    let tok = first_significant("$#array").ok_or("no token")?;
    assert!(
        matches!(&tok.token_type, TokenType::Identifier(id) if id.as_ref() == "$#array"),
        "Expected Identifier($#array), got {:?}",
        tok.token_type
    );
    Ok(())
}

#[test]
fn special_var_dollar_caret_match() -> R {
    // ${^MATCH} is the special variable for the matched string
    let tok = first_significant("${^MATCH}").ok_or("no token")?;
    assert!(
        matches!(&tok.token_type, TokenType::Identifier(_)),
        "Expected Identifier for ${{^MATCH}}, got {:?}",
        tok.token_type
    );
    Ok(())
}

#[test]
fn package_qualified_variable() -> R {
    let tok = first_significant("$Foo::Bar::baz").ok_or("no token")?;
    assert!(
        matches!(&tok.token_type, TokenType::Identifier(id) if id.as_ref() == "$Foo::Bar::baz"),
        "Expected Identifier($Foo::Bar::baz), got {:?}",
        tok.token_type
    );
    Ok(())
}

// ===========================================================================
// 5. Unicode identifiers
// ===========================================================================

#[test]
fn unicode_identifier_cafe() -> R {
    let input = "my $caf\u{00e9} = 1;";
    assert_terminates(input);
    let sig = significant(input);
    let has_cafe = sig.iter().any(|t| {
        matches!(&t.token_type, TokenType::Identifier(id) if id.as_ref().contains("caf\u{00e9}"))
    });
    assert!(has_cafe, "expected unicode identifier $caf\u{00e9}");
    Ok(())
}

#[test]
fn unicode_identifier_accented_variable() -> R {
    let input = "my $\u{00fc}ber = 42;";
    assert_terminates(input);
    let sig = significant(input);
    let has_uber = sig.iter().any(|t| {
        matches!(&t.token_type, TokenType::Identifier(id) if id.as_ref().contains("\u{00fc}ber"))
    });
    assert!(has_uber, "expected unicode identifier $\u{00fc}ber");
    Ok(())
}

#[test]
fn unicode_identifier_cjk() -> R {
    // CJK characters are in XID_Start/XID_Continue
    let input = "my $\u{4e16}\u{754c} = 1;";
    assert_terminates(input);
    let sig = significant(input);
    let has_cjk = sig.iter().any(|t| {
        matches!(&t.token_type, TokenType::Identifier(id) if id.as_ref().contains("\u{4e16}\u{754c}"))
    });
    assert!(has_cjk, "expected CJK unicode identifier");
    Ok(())
}

#[test]
fn unicode_identifier_cyrillic() -> R {
    let input = "my $\u{043f}\u{0440}\u{0438}\u{0432}\u{0435}\u{0442} = 1;";
    assert_terminates(input);
    let sig = significant(input);
    let has_cyrillic = sig.iter().any(|t| {
        matches!(&t.token_type, TokenType::Identifier(id) if id.as_ref().contains("\u{043f}\u{0440}\u{0438}\u{0432}\u{0435}\u{0442}"))
    });
    assert!(has_cyrillic, "expected Cyrillic unicode identifier");
    Ok(())
}

#[test]
fn unicode_subroutine_name() -> R {
    // Perl allows Unicode in subroutine names
    let input = "sub caf\u{00e9} { }";
    assert_terminates(input);
    let sig = significant(input);
    let has_name = sig.iter().any(
        |t| matches!(&t.token_type, TokenType::Identifier(id) if id.as_ref() == "caf\u{00e9}"),
    );
    assert!(has_name, "expected unicode subroutine name caf\u{00e9}");
    Ok(())
}

// ===========================================================================
// 6. Combined edge cases / integration-like tests
// ===========================================================================

#[test]
fn heredoc_followed_by_regex() -> R {
    let input = "<<EOF\nbody\nEOF\nif (/pattern/) { }\n";
    assert_terminates(input);
    let sig = significant(input);
    let has_heredoc = sig
        .iter()
        .any(|t| matches!(t.token_type, TokenType::HeredocStart));
    let has_regex = sig
        .iter()
        .any(|t| matches!(t.token_type, TokenType::RegexMatch));
    assert!(has_heredoc, "expected HeredocStart");
    assert!(has_regex, "expected RegexMatch after heredoc");
    Ok(())
}

#[test]
fn multiple_quote_operators_in_sequence() -> R {
    let input = "my $a = q{one}; my $b = qq(two); my @c = qw[three four];";
    assert_terminates(input);
    let sig = significant(input);
    let has_q = sig
        .iter()
        .any(|t| matches!(t.token_type, TokenType::QuoteSingle));
    let has_qq = sig
        .iter()
        .any(|t| matches!(t.token_type, TokenType::QuoteDouble));
    let has_qw = sig
        .iter()
        .any(|t| matches!(t.token_type, TokenType::QuoteWords));
    assert!(has_q, "expected QuoteSingle");
    assert!(has_qq, "expected QuoteDouble");
    assert!(has_qw, "expected QuoteWords");
    Ok(())
}

#[test]
fn special_variables_in_expressions() -> R {
    let input = "if ($! && $@ eq '') { $_ = $1; }";
    assert_terminates(input);
    let sig = significant(input);
    // Should produce several Identifier tokens for $!, $@, $_, $1
    let ident_count = sig
        .iter()
        .filter(|t| matches!(t.token_type, TokenType::Identifier(_)))
        .count();
    assert!(
        ident_count >= 3,
        "expected at least 3 identifier tokens, got {}",
        ident_count
    );
    Ok(())
}

#[test]
fn regex_after_binding_operator() -> R {
    // =~ should put lexer in ExpectTerm mode so / starts regex
    let input = "$x =~ /pattern/i";
    assert_terminates(input);
    let sig = significant(input);
    let has_regex = sig
        .iter()
        .any(|t| matches!(t.token_type, TokenType::RegexMatch));
    assert!(has_regex, "expected RegexMatch after =~ binding operator");
    Ok(())
}

#[test]
fn division_after_variable() -> R {
    // After a variable, / should be division, not regex
    let input = "$x / 2";
    assert_terminates(input);
    let sig = significant(input);
    let has_division = sig
        .iter()
        .any(|t| matches!(t.token_type, TokenType::Division));
    assert!(
        has_division,
        "expected Division after variable, got: {:?}",
        sig.iter().map(|t| &t.token_type).collect::<Vec<_>>()
    );
    Ok(())
}

#[test]
fn division_after_closing_paren() -> R {
    // After ), / should be division
    let input = "($x) / 2";
    assert_terminates(input);
    let sig = significant(input);
    let has_division = sig
        .iter()
        .any(|t| matches!(t.token_type, TokenType::Division));
    assert!(has_division, "expected Division after closing paren");
    Ok(())
}

#[test]
fn regex_after_keyword() -> R {
    // After keyword like 'if', / should start regex
    let input = "if /pattern/";
    assert_terminates(input);
    let sig = significant(input);
    let has_regex = sig
        .iter()
        .any(|t| matches!(t.token_type, TokenType::RegexMatch));
    assert!(has_regex, "expected RegexMatch after keyword 'if'");
    Ok(())
}

#[test]
fn backtick_string_basic() -> R {
    let input = "`ls -la`";
    let sig = significant(input);
    let first = sig.first().ok_or("no tokens")?;
    assert!(
        matches!(first.token_type, TokenType::QuoteCommand),
        "Expected QuoteCommand for backtick string, got {:?}",
        first.token_type
    );
    assert_eq!(first.text.as_ref(), "`ls -la`");
    Ok(())
}

#[test]
fn empty_regex() -> R {
    let input = "//";
    // At start of input, mode is ExpectTerm, so // is an empty regex
    let sig = significant(input);
    let first = sig.first().ok_or("no tokens")?;
    assert!(
        matches!(first.token_type, TokenType::RegexMatch),
        "Expected RegexMatch for empty regex //, got {:?}",
        first.token_type
    );
    Ok(())
}

#[test]
fn defined_or_after_variable() -> R {
    // After variable, // should be defined-or operator
    let input = "$x // $y";
    assert_terminates(input);
    let sig = significant(input);
    let has_defined_or = sig
        .iter()
        .any(|t| matches!(&t.token_type, TokenType::Operator(op) if op.as_ref() == "//"));
    assert!(
        has_defined_or,
        "expected // as defined-or operator after variable"
    );
    Ok(())
}

#[test]
fn heredoc_with_expression_on_same_line() -> R {
    // Perl allows expressions after heredoc marker on the same line
    let input = "my $x = <<EOF . \"suffix\";\nhello\nEOF\n";
    assert_terminates(input);
    let sig = significant(input);
    let has_heredoc = sig
        .iter()
        .any(|t| matches!(t.token_type, TokenType::HeredocStart));
    assert!(has_heredoc, "expected HeredocStart");
    Ok(())
}

#[test]
fn substitution_with_escaped_delimiters() -> R {
    let input = r"s/foo\/bar/baz\/qux/";
    let sig = significant(input);
    let first = sig.first().ok_or("no tokens")?;
    assert!(
        matches!(first.token_type, TokenType::Substitution),
        "Expected Substitution with escaped delimiters, got {:?}",
        first.token_type
    );
    assert_eq!(first.text.as_ref(), r"s/foo\/bar/baz\/qux/");
    Ok(())
}

#[test]
fn transliteration_with_ranges() -> R {
    let input = "tr/a-zA-Z/A-Za-z/";
    let sig = significant(input);
    let first = sig.first().ok_or("no tokens")?;
    assert!(
        matches!(first.token_type, TokenType::Transliteration),
        "Expected Transliteration, got {:?}",
        first.token_type
    );
    assert_eq!(first.text.as_ref(), "tr/a-zA-Z/A-Za-z/");
    Ok(())
}

// ===========================================================================
// 7. Prototype mode leak regression tests (after 'sub' keyword)
// ===========================================================================

/// `sub foo { }` — no prototype, after_sub must not leak into body.
/// Without the fix, `in_prototype` stays true and `$^W` inside the body
/// would be mis-lexed (the `^` treated as a literal prototype char).
#[test]
fn sub_no_prototype_does_not_leak() -> R {
    // $^W is a special variable; inside a prototype it would be mishandled
    let input = "sub foo { my $x = $^W; }";
    let sig = significant(input);

    // Find the $^W variable token — it should be a single Variable token
    // containing "^W", not split by prototype mode treating ^ as literal
    let var_tokens: Vec<_> = sig
        .iter()
        .filter(|t| matches!(&t.token_type, TokenType::Identifier(v) if v.as_ref() == "$^W"))
        .collect();
    assert!(
        !var_tokens.is_empty(),
        "Expected $^W to be lexed as a Variable containing '^W', got tokens: {:?}",
        sig.iter()
            .map(|t| (&t.token_type, t.text.as_ref()))
            .collect::<Vec<_>>()
    );
    Ok(())
}

/// `sub foo : lvalue { }` — attribute without prototype should not leak.
#[test]
fn sub_attribute_without_prototype_does_not_leak() -> R {
    let input = "sub foo : lvalue { my $x = $^W; }";
    let sig = significant(input);

    let var_tokens: Vec<_> = sig
        .iter()
        .filter(|t| matches!(&t.token_type, TokenType::Identifier(v) if v.as_ref() == "$^W"))
        .collect();
    assert!(
        !var_tokens.is_empty(),
        "Expected $^W to be lexed correctly after sub with attribute, got tokens: {:?}",
        sig.iter()
            .map(|t| (&t.token_type, t.text.as_ref()))
            .collect::<Vec<_>>()
    );
    Ok(())
}

/// `sub foo ($self) { }` — signature (not prototype) should not leak.
#[test]
fn sub_signature_does_not_leak() -> R {
    // After the signature parens close, prototype mode must be off
    let input = "sub foo ($self) { my $x = $^W; }";
    let sig = significant(input);

    let var_tokens: Vec<_> = sig
        .iter()
        .filter(|t| matches!(&t.token_type, TokenType::Identifier(v) if v.as_ref() == "$^W"))
        .collect();
    assert!(
        !var_tokens.is_empty(),
        "Expected $^W to be lexed correctly after sub with signature, got tokens: {:?}",
        sig.iter()
            .map(|t| (&t.token_type, t.text.as_ref()))
            .collect::<Vec<_>>()
    );
    Ok(())
}

/// `sub foo ($$) { }` — actual prototype, verify prototype mode engages and exits.
#[test]
fn sub_with_prototype_works_correctly() -> R {
    let input = "sub foo ($$) { my $x = $^W; }";
    let sig = significant(input);

    // After prototype parens close, $^W should still be lexed correctly
    let var_tokens: Vec<_> = sig
        .iter()
        .filter(|t| matches!(&t.token_type, TokenType::Identifier(v) if v.as_ref() == "$^W"))
        .collect();
    assert!(
        !var_tokens.is_empty(),
        "Expected $^W after prototype to be lexed correctly, got tokens: {:?}",
        sig.iter()
            .map(|t| (&t.token_type, t.text.as_ref()))
            .collect::<Vec<_>>()
    );
    Ok(())
}

/// Forward declaration `sub foo;` should not leak prototype mode.
#[test]
fn sub_forward_declaration_does_not_leak() -> R {
    let input = "sub foo; my $x = $^W;";
    let sig = significant(input);

    let var_tokens: Vec<_> = sig
        .iter()
        .filter(|t| matches!(&t.token_type, TokenType::Identifier(v) if v.as_ref() == "$^W"))
        .collect();
    assert!(
        !var_tokens.is_empty(),
        "Expected $^W after forward declaration to be lexed correctly, got tokens: {:?}",
        sig.iter()
            .map(|t| (&t.token_type, t.text.as_ref()))
            .collect::<Vec<_>>()
    );
    Ok(())
}
