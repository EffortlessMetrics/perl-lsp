use perl_lexer::{PerlLexer, Token, TokenType};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn next_significant(lexer: &mut PerlLexer<'_>) -> Result<Token, Box<dyn std::error::Error>> {
    loop {
        let tok = lexer.next_token().ok_or("expected token")?;
        if !matches!(
            tok.token_type,
            TokenType::Whitespace | TokenType::Newline | TokenType::Comment(_)
        ) {
            return Ok(tok);
        }
    }
}

fn assert_span_in_bounds(input: &str, token: &Token) {
    assert!(
        token.start <= token.end && token.end <= input.len(),
        "invalid span [{}, {}) for token {:?} in input len {}",
        token.start,
        token.end,
        token.token_type,
        input.len()
    );
}

#[test]
fn regression_slash_after_right_paren_is_division() -> TestResult {
    let input = "(1+2)/3";
    let mut lexer = PerlLexer::new(input);

    while !matches!(next_significant(&mut lexer)?.token_type, TokenType::RightParen) {}
    let slash = next_significant(&mut lexer)?;

    assert!(matches!(slash.token_type, TokenType::Division));
    Ok(())
}

#[test]
fn regression_slash_after_binding_is_regex_match() -> TestResult {
    let input = "$x =~ /foo/";
    let mut lexer = PerlLexer::new(input);

    while next_significant(&mut lexer)?.text.as_ref() != "=~" {}
    let regex = next_significant(&mut lexer)?;

    assert!(matches!(regex.token_type, TokenType::RegexMatch));
    Ok(())
}

#[test]
fn regression_quote_words_braces_preserves_text() -> TestResult {
    let input = "qw{alpha beta}";
    let token = next_significant(&mut PerlLexer::new(input))?;

    assert_eq!(token.text.as_ref(), input);
    Ok(())
}

#[test]
fn regression_qx_pipe_is_quote_command() -> TestResult {
    let token = next_significant(&mut PerlLexer::new("qx|echo hi|"))?;

    assert!(matches!(token.token_type, TokenType::QuoteCommand));
    Ok(())
}

#[test]
fn regression_transliteration_with_modifiers_is_single_token() -> TestResult {
    let input = "tr/a-z/A-Z/cdr";
    let token = next_significant(&mut PerlLexer::new(input))?;

    assert_eq!(token.text.as_ref(), input);
    Ok(())
}

#[test]
fn regression_y_transliteration_alias_kind() -> TestResult {
    let token = next_significant(&mut PerlLexer::new("y/abc/xyz/"))?;

    assert!(matches!(token.token_type, TokenType::Transliteration));
    Ok(())
}

#[test]
fn regression_heredoc_start_span_stays_in_bounds() -> TestResult {
    let input = "<<EOF\nline\nEOF\n";
    let mut lexer = PerlLexer::new(input);
    let heredoc_start = next_significant(&mut lexer)?;

    assert!(matches!(heredoc_start.token_type, TokenType::HeredocStart));
    assert_span_in_bounds(input, &heredoc_start);
    Ok(())
}

#[test]
fn regression_bom_prefix_does_not_prevent_lexing() -> TestResult {
    let input = "\u{feff}my $x = 1;";
    let mut lexer = PerlLexer::new(input);
    let first = next_significant(&mut lexer)?;

    assert!(matches!(first.token_type, TokenType::Keyword(_) | TokenType::Identifier(_)));
    Ok(())
}

#[test]
fn regression_vstring_token_kind_and_text() -> TestResult {
    let input = "v5.38.2";
    let token = next_significant(&mut PerlLexer::new(input))?;

    assert!(matches!(token.token_type, TokenType::Version(_)));
    assert_eq!(token.text.as_ref(), input);
    Ok(())
}

#[test]
fn regression_termination_on_hanging_heredoc_marker() -> TestResult {
    let input = "<<EOF\nunterminated body";
    let mut lexer = PerlLexer::new(input);

    for _ in 0..256 {
        let tok = lexer.next_token().ok_or("expected token")?;
        if matches!(tok.token_type, TokenType::EOF) {
            return Ok(());
        }
    }

    Err("lexer did not terminate within token budget".into())
}
