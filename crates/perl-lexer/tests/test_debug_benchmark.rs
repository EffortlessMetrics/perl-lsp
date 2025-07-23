use perl_lexer::{PerlLexer, TokenType};

#[test]
fn debug_simple_tokens() {
    let input = "my $x = 42; print $x;";
    let mut lexer = PerlLexer::new(input);
    
    let mut count = 0;
    while let Some(token) = lexer.next_token() {
        println!("Token {}: {:?}", count, token);
        count += 1;
        
        // Safety check
        if count > 100 {
            panic!("Too many tokens - possible infinite loop");
        }
    }
    
    println!("Total tokens: {}", count);
}

#[test]
fn test_format_termination() {
    // Test case with terminating dot
    let input = "Some format content\n.\n";
    let mut lexer = PerlLexer::new(input);
    lexer.enter_format_mode();
    
    let token = lexer.next_token();
    assert!(token.is_some());
    match token.unwrap().token_type {
        TokenType::FormatBody(content) => {
            println!("Format body: {:?}", content);
        }
        TokenType::Error(msg) => {
            println!("Error: {:?}", msg);
        }
        _ => panic!("Unexpected token type")
    }
}

#[test]
fn test_format_no_termination() {
    // Test case without terminating dot
    let input = "Some format content\nno dot here";
    let mut lexer = PerlLexer::new(input);
    lexer.enter_format_mode();
    
    let token = lexer.next_token();
    assert!(token.is_some());
    match token.unwrap().token_type {
        TokenType::Error(msg) => {
            assert_eq!(msg.as_ref(), "Unterminated format body");
        }
        _ => panic!("Expected error token")
    }
}