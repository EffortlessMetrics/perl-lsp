//! Debug heredoc tokenization
use perl_lexer::{PerlLexer, TokenType};

fn main() {
    let tests = vec![
        "print <<EOF;",
        "print <<'END';",
        "print <<\"TEXT\";",
        "my $x = <<~EOF;",
    ];

    for test in tests {
        println!("\nCode: {}", test);
        let mut lexer = PerlLexer::new(test);
        println!("Tokens:");

        while let Some(token) = lexer.next_token() {
            println!("  {:?}", token);
            if token.token_type == TokenType::EOF {
                break;
            }
        }
    }
}
