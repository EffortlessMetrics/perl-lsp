use perl_lexer::{Lexer, TokenKind};

fn main() {
    let input = "return if 1;";
    let mut lexer = Lexer::new(input);
    
    println!("Tokenizing: {}", input);
    loop {
        match lexer.next_token() {
            Ok(token) => {
                println!("  {:?} => '{}'", token.kind, &input[token.start..token.end]);
                if token.kind == TokenKind::Eof {
                    break;
                }
            }
            Err(e) => {
                println!("  Error: {}", e);
                break;
            }
        }
    }
}