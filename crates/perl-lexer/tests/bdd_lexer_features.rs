use perl_lexer::{PerlLexer, TokenType};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn tokenize(input: &str) -> Vec<(TokenType, String)> {
    let mut lexer = PerlLexer::new(input);
    let mut out = Vec::new();

    while let Some(token) = lexer.next_token() {
        let is_eof = matches!(token.token_type, TokenType::EOF);
        out.push((token.token_type, token.text.to_string()));
        if is_eof {
            break;
        }
    }

    out
}

fn tokenize_with_body_tokens(input: &str) -> Vec<(TokenType, String)> {
    let mut lexer = PerlLexer::with_body_tokens(input);
    let mut out = Vec::new();

    while let Some(token) = lexer.next_token() {
        let is_eof = matches!(token.token_type, TokenType::EOF);
        out.push((token.token_type, token.text.to_string()));
        if is_eof {
            break;
        }
    }

    out
}

#[test]
fn scenario_division_operator_after_number() -> TestResult {
    // Given a numeric expression with slash between operands
    let input = "10 / 2";

    // When the input is tokenized
    let tokens = tokenize(input);

    // Then slash is interpreted as division, not regex
    assert!(matches!(tokens[0].0, TokenType::Number(_)));
    assert!(matches!(tokens[1].0, TokenType::Division));
    assert!(matches!(tokens[2].0, TokenType::Number(_)));
    assert!(matches!(tokens[3].0, TokenType::EOF));
    Ok(())
}

#[test]
fn scenario_regex_literal_after_match_binding_operator() -> TestResult {
    // Given a match binding followed by a slash-delimited pattern
    let input = "$x =~ /abc/";

    // When the input is tokenized
    let tokens = tokenize(input);

    // Then slash construct is interpreted as a regex match token
    assert!(matches!(tokens[0].0, TokenType::Identifier(_)));
    assert!(matches!(tokens[1].0, TokenType::Operator(_)));
    assert!(matches!(tokens[2].0, TokenType::RegexMatch));
    assert_eq!(tokens[2].1, "/abc/");
    assert!(matches!(tokens[3].0, TokenType::EOF));
    Ok(())
}

#[test]
fn scenario_quote_operator_without_delimiter_is_identifier() -> TestResult {
    // Given a quote-like operator without a delimiter
    let input = "qq";

    // When the input is tokenized
    let tokens = tokenize(input);

    // Then it is treated as an identifier token
    assert!(matches!(tokens[0].0, TokenType::Identifier(_)));
    assert_eq!(tokens[0].1, "qq");
    assert!(matches!(tokens[1].0, TokenType::EOF));
    Ok(())
}

#[test]
fn scenario_quote_operator_with_delimiter_is_quote_token() -> TestResult {
    // Given a quote-like operator with a delimiter
    let input = "qq{hello world}";

    // When the input is tokenized
    let tokens = tokenize(input);

    // Then it is treated as a quoted string token
    assert!(matches!(tokens[0].0, TokenType::QuoteDouble));
    assert_eq!(tokens[0].1, "qq{hello world}");
    assert!(matches!(tokens[1].0, TokenType::EOF));
    Ok(())
}

#[test]
fn scenario_heredoc_start_is_emitted_before_body() -> TestResult {
    // Given a heredoc declaration
    let input = "print <<'EOF';\nhello\nEOF\n";

    // When the input is tokenized
    let tokens = tokenize_with_body_tokens(input);

    // Then the heredoc start marker is emitted distinctly
    assert!(matches!(tokens[0].0, TokenType::Keyword(_)));
    assert!(matches!(tokens[1].0, TokenType::HeredocStart));
    assert!(
        tokens.iter().any(|(kind, _)| matches!(kind, TokenType::HeredocBody(_))),
        "expected a HeredocBody token in stream"
    );
    Ok(())
}
