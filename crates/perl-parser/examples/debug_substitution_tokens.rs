//! Debug substitution tokenization
use perl_lexer::{PerlLexer, TokenType};

fn main() {
    let tests = vec![
        "s/foo/bar/",
        "$str =~ s/old/new/",
        "tr/a-z/A-Z/",
        "$str =~ tr/a-z/A-Z/",
    ];
    
    for test in tests {
        println!("\nCode: {}", test);
        let mut lexer = PerlLexer::new(test);
        println!("Tokens:");
        
        loop {
            if let Some(token) = lexer.next_token() {
                println!("  {:?}", token);
                if token.token_type == TokenType::EOF {
                    break;
                }
            } else {
                break;
            }
        }
    }
}