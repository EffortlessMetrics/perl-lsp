use perl_parser::Parser;

fn main() {
    let test_cases = vec![
        "my $♥ = 'love';",
        "my $café = 123;",
        "my $π = 3.14159;",
        "my $Σ = 42;",
        "my $🚀 = 'rocket';",
        "sub 日本語 { return 'hello'; }",
        "my $αβγ = 'greek';",
    ];
    
    for code in test_cases {
        println!("\nTesting: {}", code);
        let mut parser = Parser::new(code);
        match parser.parse() {
            Ok(ast) => {
                println!("✓ Success! AST:\n{}", ast.to_sexp());
            }
            Err(e) => {
                println!("✗ Error: {}", e);
            }
        }
    }
}