//! Test the perl lexer to see what tokens it produces

use perl_lexer::PerlLexer;

fn main() {
    let code = "my $x = 42;";
    println!("Code: {}", code);
    println!("Tokens:");

    let mut lexer = PerlLexer::new(code);
    let mut count = 0;

    while let Some(token) = lexer.next_token() {
        println!("  {:?}", token);
        count += 1;

        // Safety limit
        if count > 20 {
            println!("  ... (stopped after 20 tokens)");
            break;
        }
    }

    println!("Total tokens: {}", count);
}
