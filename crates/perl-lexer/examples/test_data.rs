use perl_lexer::{PerlLexer, TokenType};

fn main() {
    let input = "print \"hello\\n\";\n__DATA__\nnot perl code";
    let mut lexer = PerlLexer::new(input);

    println!("Tokenizing: {:?}", input);
    println!();

    while let Some(token) = lexer.next_token() {
        println!("Token: {:?} = {:?}", token.token_type, &input[token.start..token.end]);
        if matches!(token.token_type, TokenType::EOF) {
            break;
        }
    }
}
