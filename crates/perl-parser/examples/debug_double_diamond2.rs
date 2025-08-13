use perl_lexer::{PerlLexer, TokenType};
use perl_parser::Parser;

fn main() {
    // Test different variations
    let test_cases = vec![
        "<>",               // Diamond operator
        "<<>>",             // Double diamond operator
        "<< >>",            // Space in between
        "< >",              // Space inside
        "while (<>) { }",   // Diamond in context
        "while (<<>>) { }", // Double diamond in context
    ];

    for code in test_cases {
        println!("Testing: {}", code);
        println!("Lexer tokens:");

        let mut lexer = PerlLexer::new(code);
        while let Some(token) = lexer.next_token() {
            println!("  {:?}", token);
            if matches!(token.token_type, TokenType::EOF) {
                break;
            }
        }

        println!("Parser result:");
        let mut parser = Parser::new(code);
        match parser.parse() {
            Ok(_) => println!("  ✅ Success!"),
            Err(e) => println!("  ❌ Error: {}", e),
        }
        println!();
    }
}
