//! Debug token lexing

#[cfg(test)]
mod tests {
    use crate::simple_token::{PerlLexer, Token};
    use logos::Logos;

    #[test]
    fn debug_token_stream() {
        let input = "$x / 2";
        let mut lexer = Token::lexer(input);

        println!("Input: {:?}", input);
        println!("Tokens:");

        while let Some(token) = lexer.next() {
            match token {
                Ok(t) => println!("  {:?} at {:?} = {:?}", t, lexer.span(), lexer.slice()),
                Err(_) => println!("  ERROR at {:?} = {:?}", lexer.span(), lexer.slice()),
            }
        }
    }

    #[test]
    fn debug_perl_lexer() {
        let input = "$x / 2";
        let mut lexer = PerlLexer::new(input);

        println!("\nPerlLexer tokens:");
        loop {
            let token = lexer.next_token();
            println!("  {:?}", token);
            if token == Token::Eof {
                break;
            }
        }
    }
}
