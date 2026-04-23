use perl_lexer::{PerlLexer, TokenType};

fn main() {
    let input = "///A! ";
    println!("Original: {:?}", input);
    
    let mut lx = PerlLexer::new(input);
    let mut tokens = vec![];
    while let Some(t) = lx.next_token() {
        if !matches!(t.token_type, TokenType::Whitespace | TokenType::Newline | TokenType::EOF | TokenType::Comment(_)) {
            tokens.push((format!("{:?}", t.token_type), t.text.to_string()));
        }
    }
    println!("Tokens: {:?}", tokens);
}
