//! Debug lexer division issue

use perl_lexer::{PerlLexer, TokenType};

fn main() {
    let test_cases = vec![
        ("$a / $b", "Variable division"),
        ("10 / 2", "Literal division"),
        ("$x/$y", "No spaces"),
    ];
    
    for (code, desc) in test_cases {
        println!("\n=== {} ===", desc);
        println!("Code: {}", code);
        
        let mut lexer = PerlLexer::new(code);
        let mut tokens = Vec::new();
        
        loop {
            if let Some(token) = lexer.next_token() {
                println!("  Token: {:?}", token);
                
                if matches!(token.token_type, TokenType::EOF) {
                    break;
                }
                tokens.push(token);
            } else {
                break;
            }
        }
        
        // Check if we got a division token
        let has_division = tokens.iter().any(|t| matches!(t.token_type, TokenType::Division));
        let has_regex = tokens.iter().any(|t| matches!(t.token_type, TokenType::RegexMatch));
        
        println!("  Has Division: {}", has_division);
        println!("  Has Regex: {}", has_regex);
    }
}