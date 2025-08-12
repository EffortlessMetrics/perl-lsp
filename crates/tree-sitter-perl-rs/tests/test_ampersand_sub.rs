//! Test how & subroutines are tokenized
use tree_sitter_perl::perl_lexer::{PerlLexer, TokenType};

#[test]
fn test_ampersand_sub() {
    let input = "&mysub";
    let mut lexer = PerlLexer::new(input);

    println!("=== Testing &mysub ===");
    while let Some(token) = lexer.next_token() {
        println!("Token: {:?}", token);
    }
}
