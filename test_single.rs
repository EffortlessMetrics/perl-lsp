use perl_parser::Parser;

fn main() {
    let test_cases = vec![
        "sub test(_) { }",
        "$x // $y",
        "my $x = $a // $b;",
    ];
    
    for code in test_cases {
        println!("Testing: {}", code);
        
        let mut parser = Parser::new(code);
        match parser.parse() {
            Ok(ast) => println!("✓ Success: {}", ast.to_sexp()),
            Err(e) => println!("✗ Error: {:?}", e),
        }
        println!();
    }
}