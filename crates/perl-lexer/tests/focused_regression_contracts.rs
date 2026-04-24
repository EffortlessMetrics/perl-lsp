use perl_lexer::{PerlLexer, Token, TokenType};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn next_non_trivia(lexer: &mut PerlLexer<'_>) -> Option<Token> {
    loop {
        let token = lexer.next_token()?;
        if !matches!(token.token_type, TokenType::Whitespace | TokenType::Newline) {
            return Some(token);
        }
    }
}

#[test]
fn slash_after_block_is_division_with_expected_text_and_span() -> TestResult {
    let input = "{1} / 2";
    let mut lexer = PerlLexer::new(input);

    // { 1 }
    let _ = next_non_trivia(&mut lexer).ok_or("missing {")?;
    let _ = next_non_trivia(&mut lexer).ok_or("missing 1")?;
    let _ = next_non_trivia(&mut lexer).ok_or("missing }")?;

    let slash = next_non_trivia(&mut lexer).ok_or("missing slash token")?;
    assert_eq!(slash.token_type, TokenType::Division);
    assert_eq!(slash.text.as_ref(), "/");
    assert_eq!(&input[slash.start..slash.end], "/");
    Ok(())
}

#[test]
fn slash_budget_guard_emits_unknown_rest_with_valid_span() -> TestResult {
    let huge = "a".repeat(70_000);
    let input = format!("/{huge}/");

    let mut lexer = PerlLexer::new(&input);
    let first = lexer.next_token().ok_or("expected a token")?;

    assert_eq!(first.token_type, TokenType::UnknownRest);
    assert!(first.start < first.end, "UnknownRest should consume remaining input");
    assert!(first.end <= input.len(), "UnknownRest span must stay in bounds");
    Ok(())
}

#[test]
fn quote_like_q_brace_preserves_token_text() -> TestResult {
    let input = "q{abc def}";
    let mut lexer = PerlLexer::new(input);

    let token = next_non_trivia(&mut lexer).ok_or("expected q token")?;
    assert_eq!(token.token_type, TokenType::QuoteSingle);
    assert_eq!(token.text.as_ref(), input);
    assert_eq!(token.start, 0);
    assert_eq!(token.end, input.len());
    Ok(())
}

#[test]
fn quote_like_qr_bracket_is_quote_regex() -> TestResult {
    let input = "qr[abc]+";
    let mut lexer = PerlLexer::new(input);

    let token = next_non_trivia(&mut lexer).ok_or("expected qr token")?;
    assert_eq!(token.token_type, TokenType::QuoteRegex);
    assert_eq!(token.text.as_ref(), "qr[abc]");
    Ok(())
}

#[test]
fn transliteration_tr_with_modifiers_is_single_token() -> TestResult {
    let input = "tr/a-z/A-Z/cdr";
    let mut lexer = PerlLexer::new(input);

    let token = next_non_trivia(&mut lexer).ok_or("expected transliteration")?;
    assert_eq!(token.token_type, TokenType::Transliteration);
    assert_eq!(token.text.as_ref(), input);
    assert_eq!(token.start, 0);
    assert_eq!(token.end, input.len());
    Ok(())
}

#[test]
fn heredoc_with_cr_terminates_without_looping() {
    let mut lexer = PerlLexer::new("``<<TAG\r");

    for i in 0..32 {
        match lexer.next_token() {
            Some(token) if matches!(token.token_type, TokenType::EOF) => return,
            Some(_) => {}
            None => return,
        }
        assert!(i < 31, "lexer did not terminate within bounded token budget");
    }
}

#[test]
fn utf8_bom_then_vstring_keeps_version_span_valid() -> TestResult {
    let input = "\u{FEFF}use v5.38;";
    let mut lexer = PerlLexer::new(input);

    let first = next_non_trivia(&mut lexer).ok_or("missing first token")?;
    assert!(matches!(first.token_type, TokenType::Keyword(_)));

    let version = next_non_trivia(&mut lexer).ok_or("missing version token")?;
    assert!(matches!(version.token_type, TokenType::Version(_)));
    assert_eq!(&input[version.start..version.end], "v5.38");
    Ok(())
}

#[test]
fn unicode_heredoc_regression_input_does_not_panic() {
    let input = "¡<<'";
    let result = std::panic::catch_unwind(|| {
        let mut lexer = PerlLexer::new(input);
        let token = lexer.next_token();
        if let Some(tok) = token {
            assert!(tok.end <= input.len(), "token span must remain in bounds");
            assert!(tok.start <= tok.end, "token span must be well-formed");
        }
    });

    assert!(result.is_ok(), "lexer should not panic for unicode heredoc edge input");
}

#[test]
fn unterminated_quote_command_degrades_gracefully() -> TestResult {
    let mut lexer = PerlLexer::new("qx{unterminated");
    let token = next_non_trivia(&mut lexer).ok_or("expected token")?;

    assert!(matches!(token.token_type, TokenType::Error(_) | TokenType::QuoteCommand));
    assert!(token.end <= "qx{unterminated".len());
    Ok(())
}
