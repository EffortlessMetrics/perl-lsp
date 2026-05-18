use perl_lexer::{PerlLexer, TokenType};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn first_non_whitespace_token(input: &str) -> Option<perl_lexer::Token> {
    let mut lexer = PerlLexer::new(input);
    loop {
        let token = lexer.next_token()?;
        if !matches!(token.token_type, TokenType::Whitespace) {
            return Some(token);
        }
    }
}

#[test]
fn unclosed_quote_like_tokens_return_unclosed_error() -> TestResult {
    let cases = [
        "qq{hello;",
        "q{hello;",
        "qx{cmd;",
        "qr{pat;",
        "s{a}{",
        "tr{a}{",
        "qq/hello;",
        "qq[hello;",
        "qq(hello;",
        "qq<hello;",
        "qq#hello;",
    ];

    for input in cases {
        let token = first_non_whitespace_token(input)
            .ok_or_else(|| format!("expected token for input {input}"))?;
        match token.token_type {
            TokenType::Error(message) => {
                assert!(
                    message.contains("unclosed"),
                    "expected unclosed message for {input}, got {message}"
                );
            }
            other => return Err(format!("expected error token for {input}, got {other:?}").into()),
        }
    }

    Ok(())
}

#[test]
fn substitution_empty_quoted_replacement_closes_before_next_statement() -> TestResult {
    let source = r#"if ($def =~ /=/) { $def =~ s/"/""/g; $def = qq["$def"]; }"#;
    let mut lexer = PerlLexer::new(source);
    let mut saw_substitution = false;

    while let Some(token) = lexer.next_token() {
        if matches!(token.token_type, TokenType::Error(_)) {
            return Err(format!("unexpected lexer error token: {token:?}").into());
        }
        if matches!(token.token_type, TokenType::Substitution) {
            assert_eq!(token.text.as_ref(), r#"s/"/""/g"#);
            saw_substitution = true;
        }
        if matches!(token.token_type, TokenType::EOF) {
            break;
        }
    }

    assert!(saw_substitution, "expected substitution token in {source}");
    Ok(())
}

#[test]
fn substitution_apostrophe_replacement_closes_before_comment_apostrophes() -> TestResult {
    let source = r#"$name =~ s/\bo'(\w)/O'\u$1/igo; # Irish names such as 'O'Malley, O'Reilly'
$name =~ s/\[[^\]]*\]//g;"#;
    let mut lexer = PerlLexer::new(source);
    let mut substitutions = Vec::new();

    while let Some(token) = lexer.next_token() {
        if matches!(token.token_type, TokenType::Error(_)) {
            return Err(format!("unexpected lexer error token: {token:?}").into());
        }
        if matches!(token.token_type, TokenType::Substitution) {
            substitutions.push(token.text.to_string());
        }
        if matches!(token.token_type, TokenType::EOF) {
            break;
        }
    }

    assert_eq!(
        substitutions,
        vec![r#"s/\bo'(\w)/O'\u$1/igo"#, r#"s/\[[^\]]*\]//g"#],
        "expected each substitution to close before the trailing comment or next statement",
    );
    Ok(())
}
