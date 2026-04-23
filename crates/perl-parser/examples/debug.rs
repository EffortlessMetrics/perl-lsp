//! Debug example to understand parser issues

use perl_parser::Parser;

fn main() {
    // Test different code examples
    let test_cases =
        vec!["my $x = 42;", "if ($x > 10) { print $x; }", "sub greet { print \"Hello\"; }"];

    for code in test_cases {
        println!("\n=== Parsing: {} ===", code);

        let mut parser = Parser::new(code);
        match parser.parse() {
            Ok(ast) => {
                println!("Success!");
                println!("AST: {:?}", ast);
                println!("S-expression: {}", ast.to_sexp());
            }
            Err(e) => {
                println!("Error: {}", e);
            }
        }

        // Let's also test the lexer directly
        println!("\nLexer output:");
        use perl_lexer::PerlLexer;

        let mut lexer = PerlLexer::new(code);
        while let Some(token) = lexer.next_token() {
            println!("  {:?}", token);
            if matches!(token.token_type, perl_lexer::TokenType::EOF) {
                break;
            }
        }
    }
}
