use perl_parser::Parser;

fn main() {
    let test_cases = vec![
        "5.",
        "my $x = 5.;",
        "print 5. + 3;",
        "5.0",
        "5.123",
        ".5",
        ".123",
        "1.2e10",
        "5.e10",
        ".5e10",
    ];
    
    for code in test_cases {
        println!("Testing: {}", code);
        
        let mut parser = Parser::new(code);
        match parser.parse() {
            Ok(ast) => {
                println!("✓ Success! AST: {}", ast.to_sexp());
            }
            Err(e) => {
                println!("✗ Error: {:?}", e);
            }
        }
        println!();
    }
}