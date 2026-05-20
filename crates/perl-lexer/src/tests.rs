use super::*;

type TestResult = std::result::Result<(), Box<dyn std::error::Error>>;

#[test]
fn test_basic_tokens() -> TestResult {
    let mut lexer = PerlLexer::new("my $x = 42;");

    let token = lexer.next_token().ok_or("Expected keyword token")?;
    assert_eq!(token.token_type, TokenType::Keyword(Arc::from("my")));

    let token = lexer.next_token().ok_or("Expected identifier token")?;
    assert!(matches!(token.token_type, TokenType::Identifier(_)));

    let token = lexer.next_token().ok_or("Expected operator token")?;
    assert!(matches!(token.token_type, TokenType::Operator(_)));

    let token = lexer.next_token().ok_or("Expected number token")?;
    assert!(matches!(token.token_type, TokenType::Number(_)));

    let token = lexer.next_token().ok_or("Expected semicolon token")?;
    assert_eq!(token.token_type, TokenType::Semicolon);
    Ok(())
}

#[test]
fn test_slash_disambiguation() -> TestResult {
    // Division
    let mut lexer = PerlLexer::new("10 / 2");
    lexer.next_token(); // 10
    let token = lexer.next_token().ok_or("Expected division token")?;
    assert_eq!(token.token_type, TokenType::Division);

    // Regex
    let mut lexer = PerlLexer::new("if (/pattern/)");
    lexer.next_token(); // if
    lexer.next_token(); // (
    let token = lexer.next_token().ok_or("Expected regex token")?;
    assert_eq!(token.token_type, TokenType::RegexMatch);
    Ok(())
}

#[test]
fn test_percent_and_double_sigil_disambiguation() -> TestResult {
    // Hash variable
    let mut lexer = PerlLexer::new("%hash");
    let token = lexer.next_token().ok_or("Expected hash identifier token")?;
    assert!(matches!(token.token_type, TokenType::Identifier(ref id) if id.as_ref() == "%hash"));

    // Modulo operator
    let mut lexer = PerlLexer::new("10 % 3");
    lexer.next_token(); // 10
    let token = lexer.next_token().ok_or("Expected modulo operator token")?;
    assert!(matches!(token.token_type, TokenType::Operator(ref op) if op.as_ref() == "%"));
    Ok(())
}

#[test]
fn test_defined_or_and_exponent() -> TestResult {
    // Defined-or operator
    let mut lexer = PerlLexer::new("$a // $b");
    lexer.next_token(); // $a
    let token = lexer.next_token().ok_or("Expected defined-or operator token")?;
    assert!(matches!(token.token_type, TokenType::Operator(ref op) if op.as_ref() == "//"));

    // Regex after =~ should still parse
    let mut lexer = PerlLexer::new("$x =~ //");
    lexer.next_token(); // $x
    lexer.next_token(); // =~
    let token = lexer.next_token().ok_or("Expected regex token")?;
    assert_eq!(token.token_type, TokenType::RegexMatch);

    // Exponent operator
    let mut lexer = PerlLexer::new("2 ** 3");
    lexer.next_token(); // 2
    let token = lexer.next_token().ok_or("Expected exponent operator token")?;
    assert!(matches!(token.token_type, TokenType::Operator(ref op) if op.as_ref() == "**"));
    Ok(())
}

#[test]
fn test_join_regex_disambiguation() -> TestResult {
    let mut lexer = PerlLexer::new("join /,/, @parts");
    let token = lexer.next_token().ok_or("Expected join token")?;
    assert!(matches!(token.token_type, TokenType::Identifier(ref id) if id.as_ref() == "join"));

    let token = lexer.next_token().ok_or("Expected regex token")?;
    assert_eq!(token.token_type, TokenType::RegexMatch);
    Ok(())
}

#[test]
fn test_builtin_regex_disambiguation() -> TestResult {
    for code in ["print /pattern/", "defined /pattern/", "keys /pattern/"] {
        let mut lexer = PerlLexer::new(code);
        lexer.next_token();
        let token = lexer.next_token().ok_or("Expected regex token")?;
        assert_eq!(token.token_type, TokenType::RegexMatch, "{code}");
    }
    Ok(())
}

#[test]
fn test_nullary_builtin_division_disambiguation() -> TestResult {
    let mut lexer = PerlLexer::new("time / 2");
    let token = lexer.next_token().ok_or("Expected time token")?;
    assert!(matches!(token.token_type, TokenType::Identifier(ref id) if id.as_ref() == "time"));

    let token = lexer.next_token().ok_or("Expected division token")?;
    assert_eq!(token.token_type, TokenType::Division);
    Ok(())
}

#[test]
fn test_peek_token_does_not_mutate_paren_depth() -> TestResult {
    // Regression guard for issue #2750: peek_token() must save and restore
    // paren_depth so that a peek at `(` does not permanently increment
    // paren_depth and corrupt the heredoc/bitshift guard on a subsequent token.
    let mut lexer = PerlLexer::new("(1<<2)");
    assert_eq!(lexer.paren_depth, 0, "paren_depth must start at 0");

    // Peek at `(` — must not permanently increment paren_depth
    let peeked = lexer.peek_token().ok_or("peek at ( failed")?;
    assert_eq!(peeked.token_type, TokenType::LeftParen);
    assert_eq!(lexer.paren_depth, 0, "peek_token must not mutate paren_depth");

    // Consume `(` — paren_depth becomes 1
    lexer.next_token();
    assert_eq!(lexer.paren_depth, 1);

    // Peek at `1` (a number) — paren_depth must remain 1
    let peeked2 = lexer.peek_token().ok_or("peek at 1 failed")?;
    assert!(matches!(peeked2.token_type, TokenType::Number(_)));
    assert_eq!(lexer.paren_depth, 1, "peek at number must not change paren_depth");

    Ok(())
}

#[test]
fn test_comment_skipping_with_cr_line_endings() -> TestResult {
    let mut lexer = PerlLexer::new("my $x = 1;# comment\rmy $y = 2;");
    let mut saw_second_my = false;

    while let Some(token) = lexer.next_token() {
        if matches!(token.token_type, TokenType::EOF) {
            break;
        }

        if matches!(token.token_type, TokenType::Keyword(ref kw) if kw.as_ref() == "my")
            && token.start > 0
        {
            saw_second_my = true;
        }
    }

    assert!(saw_second_my, "lexer should continue after CR-terminated comment line");
    Ok(())
}

#[test]
fn test_pod_skipped_with_cr_only_line_endings() -> TestResult {
    // CR-only line endings (classic Mac): =pod and =cut must be detected
    // when preceded by \r instead of \n.
    let input = "my $before = 1;\r=pod\rThis is documentation.\r=cut\rmy $after = 2;";
    let mut lexer = PerlLexer::new(input);
    let mut token_texts: Vec<String> = Vec::new();

    while let Some(token) = lexer.next_token() {
        if matches!(token.token_type, TokenType::EOF) {
            break;
        }
        if matches!(token.token_type, TokenType::Keyword(_) | TokenType::Identifier(_)) {
            token_texts.push(token.text.to_string());
        }
    }

    assert!(
        token_texts.iter().any(|t| t == "my" && {
            // find the second 'my' (after the POD block)
            token_texts.iter().enumerate().filter(|(_, t)| t.as_str() == "my").nth(1).is_some()
        }),
        "lexer should produce tokens after CR-terminated =cut; got: {:?}",
        token_texts
    );

    // Ensure POD body text is not present as an identifier token
    assert!(
        !token_texts.iter().any(|t| t == "documentation"),
        "POD body should be consumed, not emitted as a token; got: {:?}",
        token_texts
    );
    Ok(())
}

#[test]
fn test_find_line_end_stops_at_crlf_and_cr() -> TestResult {
    let bytes = b"abc\r\ndef\rghi\n";
    assert_eq!(PerlLexer::find_line_end(bytes, 0), 3);
    assert_eq!(PerlLexer::find_line_end(bytes, 5), 8);
    assert_eq!(PerlLexer::find_line_end(bytes, 9), 12);
    Ok(())
}

#[test]
fn test_exponent_sign_no_digits_plus() -> TestResult {
    // .5e+x — 'e' is not a valid exponent (no digits follow), so the number
    // token must be ".5" only.  The 'e' becomes a separate identifier token.
    // Regression: old code produced Number(".5e") by backtracking to the sign
    // character instead of to the 'e' itself.
    let mut lexer = PerlLexer::new(".5e+x");
    let tok1 = lexer.next_token().ok_or("expected first token")?;
    assert!(
        matches!(&tok1.token_type, TokenType::Number(n) if n.as_ref() == ".5"),
        "expected Number(\".5\") but got {:?}",
        tok1.token_type
    );
    // The 'e' must NOT be swallowed into the number token.
    let tok2 = lexer.next_token().ok_or("expected second token")?;
    assert!(
        !matches!(&tok2.token_type, TokenType::Number(_)),
        "number token must not include 'e'; second token should not be a Number, got {:?}",
        tok2.token_type
    );
    Ok(())
}

#[test]
fn test_exponent_sign_no_digits_minus() -> TestResult {
    // 1.5e-y — 'e' is not a valid exponent (no digits follow), so the number
    // token must be "1.5" only.  The 'e' becomes a separate identifier token.
    // Regression: old code produced Number("1.5e") by backtracking to the '-'
    // character instead of to the 'e' itself.
    let mut lexer = PerlLexer::new("1.5e-y");
    let tok1 = lexer.next_token().ok_or("expected first token")?;
    assert!(
        matches!(&tok1.token_type, TokenType::Number(n) if n.as_ref() == "1.5"),
        "expected Number(\"1.5\") but got {:?}",
        tok1.token_type
    );
    // The 'e' must NOT be swallowed into the number token.
    let tok2 = lexer.next_token().ok_or("expected second token")?;
    assert!(
        !matches!(&tok2.token_type, TokenType::Number(_)),
        "number token must not include 'e'; second token should not be a Number, got {:?}",
        tok2.token_type
    );
    Ok(())
}
