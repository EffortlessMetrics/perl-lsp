use perl_lexer::{PerlLexer, TokenType};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn debug_simple_tokens() {
    let input = "my $x = 42; print $x;";
    let mut lexer = PerlLexer::new(input);

    let mut count = 0;
    while let Some(token) = lexer.next_token() {
        println!("Token {}: {:?}", count, token);
        count += 1;

        // Check for EOF
        if token.token_type == TokenType::EOF {
            break;
        }

        // Safety check
        assert!(count <= 100, "Too many tokens - possible infinite loop");
    }

    println!("Total tokens: {}", count);
}

#[test]
fn test_format_termination() -> TestResult {
    // Test case with terminating dot
    let input = "Some format content\n.\n";
    let mut lexer = PerlLexer::new(input);
    lexer.enter_format_mode();

    let token = lexer.next_token().ok_or("Expected token")?;
    assert!(
        matches!(token.token_type, TokenType::FormatBody(_) | TokenType::Error(_)),
        "Expected FormatBody or Error, got {:?}",
        token.token_type
    );

    if let TokenType::FormatBody(content) = token.token_type {
        println!("Format body: {:?}", content);
    } else if let TokenType::Error(msg) = token.token_type {
        println!("Error: {:?}", msg);
    }

    Ok(())
}

#[test]
fn test_format_no_termination() -> TestResult {
    // Test case without terminating dot
    let input = "Some format content\nno dot here";
    let mut lexer = PerlLexer::new(input);
    lexer.enter_format_mode();

    let token = lexer.next_token().ok_or("Expected token")?;
    assert!(
        matches!(token.token_type, TokenType::Error(_)),
        "Expected error token, got {:?}",
        token.token_type
    );

    if let TokenType::Error(msg) = token.token_type {
        assert_eq!(msg.as_ref(), "Unterminated format body");
    }

    Ok(())
}
