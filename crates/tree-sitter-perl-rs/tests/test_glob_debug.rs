//! Debug glob assignment
use tree_sitter_perl::perl_lexer::{PerlLexer, TokenType};

#[test]
fn test_glob_assignment_debug() {
    let input = "*foo = *bar;";
    let mut lexer = PerlLexer::new(input);

    println!("=== Tokenizing: {} ===", input);
    while let Some(token) = lexer.next_token() {
        println!("Token: {:?}", token);
    }
}
