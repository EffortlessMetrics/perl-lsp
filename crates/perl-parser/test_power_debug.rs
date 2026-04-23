use perl_lexer::PerlLexer;

fn main() {
    let input = "2 ** 3";
    let mut lexer = PerlLexer::new(input);
    
    println!("Lexing: {}", input);
    while let Some(token) = lexer.next_token() {
        println!("  {:?}", token);
        if matches!(token.token_type, perl_lexer::TokenType::EOF) {
            break;
        }
    }
}