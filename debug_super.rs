use perl_lexer::PerlLexer;

fn main() {
    let code = "SUPER::method()";
    let mut lexer = PerlLexer::new(code);
    
    println!("Tokenizing: {}", code);
    while let Some(token) = lexer.next_token() {
        println!("  {:?}: '{}'", token.token_type, token.text);
    }
}