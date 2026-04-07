//! Test tr/y operator in detail
use tree_sitter_perl::perl_lexer::{PerlLexer, TokenType};

#[test]
fn test_tr_operator_detail() {
    let cases = vec![
        ("tr/a-z/A-Z/", "basic tr"),
        ("y/a-z/A-Z/", "y variant"),
        ("tr[a-z][A-Z]", "bracket delimiters"),
        ("tr{a-z}{A-Z}", "brace delimiters"),
    ];
    
    for (input, desc) in cases {
        println!("\n=== Testing {} ===", desc);
        let mut lexer = PerlLexer::new(input);
        
        while let Some(token) = lexer.next_token() {
            println!("Token: {:?}", token);
            
            // tr and y should be tokenized as identifiers since they're special operators
            if token.text.as_ref() == "tr" || token.text.as_ref() == "y" {
                assert!(matches!(token.token_type, TokenType::Identifier(_)));
            }
        }
    }
}