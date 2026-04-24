use perl_lexer::{PerlLexer, Token, TokenType};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn significant_tokens(input: &str) -> Vec<Token> {
    PerlLexer::new(input)
        .collect_tokens()
        .into_iter()
        .filter(|token| {
            !matches!(token.token_type, TokenType::Whitespace | TokenType::Newline | TokenType::EOF)
        })
        .collect()
}

fn assert_valid_spans(tokens: &[Token], input: &str) {
    let mut previous_end = 0;
    for token in tokens {
        assert!(token.start <= token.end, "invalid span ordering: {token:?}");
        assert!(token.end <= input.len(), "span exceeds input length: {token:?}");
        assert!(token.start >= previous_end, "token spans should be monotonic: {token:?}");
        previous_end = token.end;
    }
}

#[test]
fn slash_ambiguity_division_token_has_expected_text() -> TestResult {
    let input = "42 / 7";
    let mut lexer = PerlLexer::new(input);

    let _number = lexer.next_token().ok_or("missing number token")?;
    let slash = lexer.next_token().ok_or("missing slash token")?;

    assert!(matches!(slash.token_type, TokenType::Division));
    assert_eq!(slash.text.as_ref(), "/");
    Ok(())
}

#[test]
fn slash_ambiguity_regex_match_at_statement_start() -> TestResult {
    let input = "/foo+/";
    let mut lexer = PerlLexer::new(input);

    let token = lexer.next_token().ok_or("missing regex token")?;
    assert!(matches!(token.token_type, TokenType::RegexMatch));
    assert_eq!(token.text.as_ref(), "/foo+/");
    Ok(())
}

#[test]
fn quote_like_qr_with_braces_uses_quote_regex_token() -> TestResult {
    let input = "qr{a+b}";
    let mut lexer = PerlLexer::new(input);

    let token = lexer.next_token().ok_or("missing qr token")?;
    assert!(matches!(token.token_type, TokenType::QuoteRegex));
    assert_eq!(token.text.as_ref(), "qr{a+b}");
    Ok(())
}

#[test]
fn transliteration_with_modifiers_preserves_full_text() -> TestResult {
    let input = "tr/a-z/A-Z/ds";
    let mut lexer = PerlLexer::new(input);

    let token = lexer.next_token().ok_or("missing tr token")?;
    assert!(matches!(token.token_type, TokenType::Transliteration));
    assert_eq!(token.text.as_ref(), "tr/a-z/A-Z/ds");
    Ok(())
}

#[test]
fn transliteration_alias_y_with_modifiers_is_transliteration() -> TestResult {
    let input = "y!abc!xyz!r";
    let mut lexer = PerlLexer::new(input);

    let token = lexer.next_token().ok_or("missing y token")?;
    assert!(matches!(token.token_type, TokenType::Transliteration));
    assert_eq!(token.text.as_ref(), "y!abc!xyz!r");
    Ok(())
}

#[test]
fn heredoc_with_crlf_line_endings_terminates_with_eof() -> TestResult {
    let input = "<<EOF\r\nline\r\nEOF\r\n";
    let mut lexer = PerlLexer::new(input);

    for _ in 0..64 {
        if let Some(token) = lexer.next_token() {
            if matches!(token.token_type, TokenType::EOF) {
                return Ok(());
            }
        } else {
            break;
        }
    }

    Err("lexer did not terminate heredoc input with EOF".into())
}

#[test]
fn utf8_bom_input_keeps_all_token_spans_in_bounds() {
    let input = "\u{feff}my $x = 1;";
    let tokens = significant_tokens(input);

    assert!(!tokens.is_empty(), "expected tokens for BOM-prefixed input");
    assert_valid_spans(&tokens, input);
}

#[test]
fn unicode_heredoc_trigger_input_does_not_panic() {
    let input = "¡<<'";
    let result = std::panic::catch_unwind(|| {
        let mut lexer = PerlLexer::new(input);
        let _ = lexer.next_token();
    });

    assert!(result.is_ok(), "lexer panicked on unicode heredoc trigger input");
}

#[test]
fn vstring_token_keeps_exact_text_and_span() -> TestResult {
    let input = "use v5.38.2;";
    let tokens = significant_tokens(input);

    let version = tokens
        .iter()
        .find(|token| matches!(token.token_type, TokenType::Version(_)))
        .ok_or("missing v-string token")?;

    assert_eq!(version.text.as_ref(), "v5.38.2");
    assert_eq!(&input[version.start..version.end], "v5.38.2");
    Ok(())
}

#[test]
fn deeply_nested_regex_input_degrades_without_hanging() {
    let nested = "(".repeat(8_000);
    let input = format!("m/{nested}/");

    let mut lexer = PerlLexer::new(&input);
    let mut saw_guard_token = false;

    for _ in 0..256 {
        match lexer.next_token() {
            Some(token) => {
                if matches!(token.token_type, TokenType::UnknownRest | TokenType::RegexMatch) {
                    saw_guard_token = true;
                    break;
                }
            }
            None => break,
        }
    }

    assert!(saw_guard_token, "expected graceful regex handling token");
}
