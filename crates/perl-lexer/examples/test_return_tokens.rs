use perl_lexer::{PerlLexer, TokenType};

fn main() {
    let test_cases = vec![
        "return if 1;",
        "return;",
        "return $x if $cond;",
        "return $x or die if $error;",
    ];

    for input in test_cases {
        println!("\nTokenizing: {}", input);
        let mut lexer = PerlLexer::new(input);

        loop {
            match lexer.next_token() {
                Some(token) => {
                    println!(
                        "  {:?} => '{}'",
                        token.token_type,
                        &input[token.start..token.end]
                    );
                    if matches!(token.token_type, TokenType::EOF) {
                        break;
                    }
                }
                None => {
                    println!("  End of tokens");
                    break;
                }
            }
        }
    }
}
