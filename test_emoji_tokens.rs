use perl_lexer::{PerlLexer, TokenType};

fn main() {
    let test = "my $♥ = 'love';";
    println!("Testing: {}", test);
    println!("Tokens:");
    
    let mut lexer = PerlLexer::new(test);
    let mut count = 0;
    while let Some(token) = lexer.next_token() {
        println!("  {:?}: '{}' [{}..{}]", 
            token.token_type, 
            token.text, 
            token.start, 
            token.end
        );
        
        // Stop on EOF or Error tokens, or after 20 tokens to prevent infinite loop
        count += 1;
        if count > 20 || token.text.is_empty() || matches!(token.token_type, TokenType::Error(_)) {
            println!("Stopping after {} tokens", count);
            break;
        }
    }
    
    // Let's also check what happens with just the emoji
    println!("\nTesting just emoji: ♥");
    let mut lexer2 = PerlLexer::new("♥");
    count = 0;
    while let Some(token) = lexer2.next_token() {
        println!("  {:?}: '{}' [{}..{}]", 
            token.token_type, 
            token.text, 
            token.start, 
            token.end
        );
        
        count += 1;
        if count > 5 || token.text.is_empty() || matches!(token.token_type, TokenType::Error(_)) {
            println!("Stopping after {} tokens", count);
            break;
        }
    }
}